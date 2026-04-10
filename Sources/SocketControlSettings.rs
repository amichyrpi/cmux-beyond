//! Rust port of [SocketControlSettings.swift](SocketControlSettings.swift).
//!
//! Phase 3 — socket access control modes: `off`, `cmuxOnly`, `automation`,
//! `password`, `allowAll`. Ancestry checks on Unix use `/proc` on Linux and
//! `libproc` on macOS; on Windows we fall back to the password mode.

// TODO(rewrite): port from SocketControlSettings.swift
#[allow(dead_code)]
pub(crate) fn __link() {}
