//! Rust port of [PortScanner.swift](PortScanner.swift).
//!
//! Phase 8 — per-platform port scanner.
//! - Linux: `/proc/net/tcp` + `procfs` crate.
//! - macOS: `libproc` + `lsof` fallback.
//! - Windows: `GetExtendedTcpTable` via `windows` crate.

// TODO(rewrite): port from PortScanner.swift
#[allow(dead_code)]
pub(crate) fn __link() {}
