//! Rust port of [Backport.swift](Backport.swift).
//!
//! The original is a macOS / Swift version-compat shim. In Rust this mostly
//! becomes empty — crate versioning + `rust-toolchain.toml` handle most of
//! what Backport.swift provides.

// TODO(rewrite): port from Backport.swift (likely mostly empty)
#[allow(dead_code)]
pub(crate) fn __link() {}
