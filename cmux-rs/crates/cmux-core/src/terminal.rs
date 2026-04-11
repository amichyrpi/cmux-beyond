//! Terminal core for the Rust rewrite.
//!
//! This is the Phase 6 starting point: a PTY-backed session wrapper with
//! an `alacritty_terminal::Term` screen model. The frontend plumbing and
//! richer render/surface integration will land later.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;

use alacritty_terminal::event::{Event as TermEvent, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::{Config as TermConfig, Term};
use alacritty_terminal::vte::ansi;
use parking_lot::Mutex;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSize {
    pub columns: usize,
    pub rows: usize,
}

impl TerminalSize {
    pub fn new(columns: usize, rows: usize) -> Self {
        Self { columns, rows }
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self { columns: 80, rows: 24 }
    }
}

impl Dimensions for TerminalSize {
    fn columns(&self) -> usize {
        self.columns
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn total_lines(&self) -> usize {
        self.rows
    }
}

impl From<TerminalSize> for PtySize {
    fn from(value: TerminalSize) -> Self {
        Self {
            rows: value.rows as u16,
            cols: value.columns as u16,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerminalSnapshot {
    pub id: Uuid,
    pub working_directory: Option<String>,
    pub columns: usize,
    pub rows: usize,
    pub title: Option<String>,
    pub bell_count: u64,
    pub is_running: bool,
    pub child_process_id: Option<u32>,
    pub visible_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSearchMatch {
    pub line_index: usize,
    pub column_index: usize,
    pub matched_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSearchResult {
    pub query: String,
    pub matches: Vec<TerminalSearchMatch>,
}

#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("pty error: {0}")]
    Pty(#[from] anyhow::Error),

    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Default)]
struct TerminalEventState {
    title: Option<String>,
    bell_count: u64,
}

#[derive(Clone, Debug)]
struct TerminalEventSink {
    state: Arc<Mutex<TerminalEventState>>,
}

impl TerminalEventSink {
    fn new(state: Arc<Mutex<TerminalEventState>>) -> Self {
        Self { state }
    }
}

impl EventListener for TerminalEventSink {
    fn send_event(&self, event: TermEvent) {
        let mut state = self.state.lock();
        match event {
            TermEvent::Title(title) => state.title = Some(title),
            TermEvent::ResetTitle => state.title = None,
            TermEvent::Bell => state.bell_count = state.bell_count.saturating_add(1),
            _ => {}
        }
    }
}

pub struct TerminalSession {
    id: Uuid,
    working_directory: Option<PathBuf>,
    size: TerminalSize,
    term: Term<TerminalEventSink>,
    parser: ansi::Processor,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    output_rx: Receiver<Vec<u8>>,
    writer: Box<dyn Write + Send>,
    event_state: Arc<Mutex<TerminalEventState>>,
}

impl TerminalSession {
    pub fn spawn_shell(
        working_directory: Option<PathBuf>,
        size: TerminalSize,
    ) -> Result<Self, TerminalError> {
        let mut builder = CommandBuilder::new(default_shell_program());
        if let Some(ref cwd) = working_directory {
            builder.cwd(cwd);
        }
        builder.env("TERM", "xterm-256color");
        builder.env("COLORTERM", "truecolor");
        Self::spawn(builder, working_directory, size)
    }

    pub fn spawn(
        mut command: CommandBuilder,
        working_directory: Option<PathBuf>,
        size: TerminalSize,
    ) -> Result<Self, TerminalError> {
        if let Some(ref cwd) = working_directory {
            command.cwd(cwd);
        }

        let pty_system = native_pty_system();
        let pair = pty_system.openpty(size.into())?;
        let child = pair.slave.spawn_command(command)?;
        drop(pair.slave);

        let reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || read_pty_output(reader, tx));

        let event_state = Arc::new(Mutex::new(TerminalEventState::default()));
        let listener = TerminalEventSink::new(event_state.clone());
        let term = Term::new(TermConfig::default(), &size, listener);

        Ok(Self {
            id: Uuid::new_v4(),
            working_directory,
            size,
            term,
            parser: ansi::Processor::default(),
            master: pair.master,
            child,
            output_rx: rx,
            writer,
            event_state,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn working_directory(&self) -> Option<&Path> {
        self.working_directory.as_deref()
    }

    pub fn size(&self) -> TerminalSize {
        self.size
    }

    pub fn resize(&mut self, size: TerminalSize) -> Result<(), TerminalError> {
        self.size = size;
        self.term.resize(size);
        self.master.resize(size.into())?;
        Ok(())
    }

    pub fn write_input(&mut self, bytes: &[u8]) -> Result<(), TerminalError> {
        self.writer.write_all(bytes)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn pump_output(&mut self) -> Result<usize, TerminalError> {
        let mut total_bytes = 0;
        loop {
            match self.output_rx.try_recv() {
                Ok(chunk) => {
                    total_bytes += chunk.len();
                    for byte in chunk {
                        self.parser.advance(&mut self.term, byte);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        Ok(total_bytes)
    }

    pub fn snapshot(&mut self) -> Result<TerminalSnapshot, TerminalError> {
        self.pump_output()?;
        let event_state = self.event_state.lock();
        let is_running = self.child.try_wait()?.is_none();
        let child_process_id = self.child.process_id();

        Ok(TerminalSnapshot {
            id: self.id,
            working_directory: self
                .working_directory
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            columns: self.size.columns,
            rows: self.size.rows,
            title: event_state.title.clone(),
            bell_count: event_state.bell_count,
            is_running,
            child_process_id,
            visible_text: visible_text(&self.term),
        })
    }

    pub fn search_visible_text(&mut self, needle: &str) -> Result<Vec<usize>, TerminalError> {
        self.pump_output()?;
        if needle.trim().is_empty() {
            return Ok(Vec::new());
        }
        let haystack = visible_text(&self.term);
        Ok(haystack
            .lines()
            .enumerate()
            .filter_map(|(index, line)| line.contains(needle).then_some(index))
            .collect())
    }

    pub fn search_visible_text_matches(
        &mut self,
        needle: &str,
    ) -> Result<TerminalSearchResult, TerminalError> {
        self.pump_output()?;
        let query = needle.trim().to_string();
        if query.is_empty() {
            return Ok(TerminalSearchResult {
                query,
                matches: Vec::new(),
            });
        }

        let haystack = visible_text(&self.term);
        let matches = haystack
            .lines()
            .enumerate()
            .flat_map(|(line_index, line)| {
                line.match_indices(&query).map(move |(column_index, matched_text)| {
                    TerminalSearchMatch {
                        line_index,
                        column_index,
                        matched_text: matched_text.to_string(),
                    }
                })
            })
            .collect();

        Ok(TerminalSearchResult { query, matches })
    }

    pub fn search_visible_text_regex(
        &mut self,
        pattern: &Regex,
    ) -> Result<TerminalSearchResult, TerminalError> {
        self.pump_output()?;
        let haystack = visible_text(&self.term);
        let query = pattern.as_str().to_string();
        let matches = haystack
            .lines()
            .enumerate()
            .flat_map(|(line_index, line)| {
                pattern.find_iter(line).map(move |mat| TerminalSearchMatch {
                    line_index,
                    column_index: mat.start(),
                    matched_text: mat.as_str().to_string(),
                })
            })
            .collect();

        Ok(TerminalSearchResult { query, matches })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedSSHSession {
    pub destination: String,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub config_file: Option<String>,
    pub jump_host: Option<String>,
    pub control_path: Option<String>,
    pub use_ipv4: bool,
    pub use_ipv6: bool,
    pub forward_agent: bool,
    pub compression_enabled: bool,
    pub ssh_options: Vec<String>,
}

pub struct TerminalSSHSessionDetector;

impl TerminalSSHSessionDetector {
    pub fn detect(for_tty: &str) -> Option<DetectedSSHSession> {
        let tty = normalize_tty_name(for_tty);
        if tty.is_empty() {
            return None;
        }

        #[cfg(unix)]
        {
            detect_ssh_session_unix(&tty)
        }

        #[cfg(windows)]
        {
            let _ = tty;
            None
        }
    }
}

#[cfg(unix)]
fn detect_ssh_session_unix(tty: &str) -> Option<DetectedSSHSession> {
    let output = std::process::Command::new("ps")
        .args(["-t", tty, "-o", "command="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let commands = String::from_utf8_lossy(&output.stdout);
    for line in commands.lines() {
        if let Some(session) = parse_ssh_command_line(line) {
            return Some(session);
        }
    }

    None
}

#[cfg(unix)]
fn parse_ssh_command_line(command_line: &str) -> Option<DetectedSSHSession> {
    let ssh_binary = Regex::new(r"(?i)(?:^|\s)(?:[\w./-]+/)?ssh(?:\.exe)?(?:\s|$)").ok()?;
    if !ssh_binary.is_match(command_line) {
        return None;
    }

    let tokens: Vec<&str> = command_line.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let ssh_index = tokens
        .iter()
        .position(|token| token.ends_with("/ssh") || *token == "ssh")?;

    let mut destination: Option<String> = None;
    let mut port: Option<u16> = None;
    let mut identity_file: Option<String> = None;
    let mut config_file: Option<String> = None;
    let mut jump_host: Option<String> = None;
    let mut control_path: Option<String> = None;
    let mut use_ipv4 = false;
    let mut use_ipv6 = false;
    let mut forward_agent = false;
    let mut compression_enabled = false;
    let mut ssh_options: Vec<String> = Vec::new();

    let mut index = ssh_index + 1;
    while index < tokens.len() {
        let token = tokens[index];
        match token {
            "-4" => use_ipv4 = true,
            "-6" => use_ipv6 = true,
            "-A" => forward_agent = true,
            "-C" => compression_enabled = true,
            "-p" if index + 1 < tokens.len() => {
                port = tokens[index + 1].parse::<u16>().ok();
                index += 1;
            }
            "-i" if index + 1 < tokens.len() => {
                identity_file = Some(tokens[index + 1].to_string());
                index += 1;
            }
            "-F" if index + 1 < tokens.len() => {
                config_file = Some(tokens[index + 1].to_string());
                index += 1;
            }
            "-J" if index + 1 < tokens.len() => {
                jump_host = Some(tokens[index + 1].to_string());
                index += 1;
            }
            "-o" if index + 1 < tokens.len() => {
                let option = tokens[index + 1];
                if let Some((key, value)) = option.split_once('=') {
                    match key.to_ascii_lowercase().as_str() {
                        "controlpath" => control_path = Some(value.to_string()),
                        _ => ssh_options.push(option.to_string()),
                    }
                } else {
                    ssh_options.push(option.to_string());
                }
                index += 1;
            }
            value if !value.starts_with('-') => {
                destination = Some(value.to_string());
            }
            _ => {}
        }
        index += 1;
    }

    let destination = destination?;
    Some(DetectedSSHSession {
        destination,
        port,
        identity_file,
        config_file,
        jump_host,
        control_path,
        use_ipv4,
        use_ipv6,
        forward_agent,
        compression_enabled,
        ssh_options,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalImageTransferMode {
    Paste,
    Drop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalRemoteUploadTarget {
    WorkspaceRemote,
    DetectedSSH(DetectedSSHSession),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalImageTransferTarget {
    Local,
    Remote(TerminalRemoteUploadTarget),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalImageTransferPlan {
    InsertText(String),
    UploadFiles(Vec<String>, TerminalRemoteUploadTarget),
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalImageTransferExecutionError {
    Cancelled,
}

#[derive(Debug, Default, Clone)]
pub struct TerminalImageTransferOperation {
    cancelled: Arc<Mutex<bool>>,
    finished: Arc<Mutex<bool>>,
}

impl TerminalImageTransferOperation {
    pub fn cancel(&self) {
        *self.cancelled.lock() = true;
    }

    pub fn finish(&self) -> bool {
        let mut finished = self.finished.lock();
        if *finished {
            return false;
        }
        *finished = true;
        true
    }

    pub fn is_cancelled(&self) -> bool {
        *self.cancelled.lock()
    }

    pub fn throw_if_cancelled(&self) -> Result<(), TerminalImageTransferExecutionError> {
        if self.is_cancelled() {
            Err(TerminalImageTransferExecutionError::Cancelled)
        } else {
            Ok(())
        }
    }
}

pub struct TerminalImageTransferPlanner;

impl TerminalImageTransferPlanner {
    pub fn plan(file_urls: &[String], target: TerminalImageTransferTarget) -> TerminalImageTransferPlan {
        if file_urls.is_empty() {
            return TerminalImageTransferPlan::Reject;
        }

        match target {
            TerminalImageTransferTarget::Local => {
                TerminalImageTransferPlan::InsertText(join_shell_escaped(file_urls))
            }
            TerminalImageTransferTarget::Remote(remote_target) => {
                TerminalImageTransferPlan::UploadFiles(file_urls.to_vec(), remote_target)
            }
        }
    }

    pub fn escape_for_shell(value: &str) -> String {
        format!("'{}'", value.replace('\'', r"'\''"))
    }
}

fn join_shell_escaped(values: &[String]) -> String {
    values
        .iter()
        .map(|value| TerminalImageTransferPlanner::escape_for_shell(value))
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_tty_name(tty_name: &str) -> String {
    let trimmed = tty_name.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed.rsplit('/').next().unwrap_or(trimmed).to_string()
}

fn visible_text<T>(term: &Term<T>) -> String {
    let rows = term.screen_lines();
    let columns = term.columns();
    if rows == 0 || columns == 0 {
        return String::new();
    }

    term.bounds_to_string(
        Point::new(Line(0), Column(0)),
        Point::new(Line((rows - 1) as i32), Column(columns - 1)),
    )
}

fn read_pty_output(mut reader: Box<dyn Read + Send>, tx: mpsc::Sender<Vec<u8>>) {
    let mut buf = [0u8; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(count) => {
                if tx.send(buf[..count].to_vec()).is_err() {
                    break;
                }
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
}

fn default_shell_program() -> String {
    default_shell_program_impl().to_string_lossy().into_owned()
}

#[cfg(unix)]
fn default_shell_program_impl() -> PathBuf {
    std::env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from("/bin/sh"))
}

#[cfg(windows)]
fn default_shell_program_impl() -> PathBuf {
    std::env::var_os("COMSPEC")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from("cmd.exe"))
}
