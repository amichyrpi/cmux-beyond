# cmux-rs — Rust + Tauri v2 rewrite

This directory holds the Rust + Tauri v2 rewrite of cmux. It runs **alongside** the existing Swift / Objective-C app in [../Sources/](../Sources/), not in place of it. See [../PLAN.md](../PLAN.md) for the phased plan and progress.

## Layout

```
cmux-rs/
├── Cargo.toml            # workspace
├── rust-toolchain.toml
├── crates/
│   ├── cmux-core/        # platform-agnostic domain logic
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── bonsplit.rs   # model port of vendor/bonsplit/
│   │       ├── config.rs
│   │       ├── pane.rs
│   │       ├── socket.rs
│   │       ├── tab.rs
│   │       ├── terminal.rs
│   │       └── workspace.rs
│   └── cmux-app/         # Tauri v2 binary
│       ├── Cargo.toml
│       ├── build.rs
│       ├── tauri.conf.json
│       └── src/
│           ├── main.rs
│           └── sources.rs    # #[path] wiring into ../../Sources/*.rs
└── ui/                    # Vite + React + TS frontend
    ├── package.json
    ├── vite.config.ts
    ├── tsconfig.json
    ├── index.html
    └── src/
        ├── main.tsx
        ├── App.tsx
        ├── styles.css
        └── bonsplit/
            └── README.md
```

Each Swift file in [../Sources/](../Sources/) has a sibling `.rs` file of the same base name (e.g. [../Sources/AppDelegate.swift](../Sources/AppDelegate.swift) → [../Sources/AppDelegate.rs](../Sources/AppDelegate.rs)). `cmux-app` pulls them into its crate via `#[path]` module declarations in [crates/cmux-app/src/sources.rs](crates/cmux-app/src/sources.rs). This preserves the "same folder, same name" constraint from the plan even though Cargo needs its own workspace root.

The only documented exception is **Bonsplit**: per the locked decision in [../PLAN.md](../PLAN.md), [../vendor/bonsplit/](../vendor/bonsplit/) is not modified. Its Rust model lives at [crates/cmux-core/src/bonsplit.rs](crates/cmux-core/src/bonsplit.rs) and its UI reimplementation at [ui/src/bonsplit/](ui/src/bonsplit/).

## One-time bootstrap

If you haven't used Tauri on this machine before, follow the platform deps at https://v2.tauri.app/start/prerequisites/. Then:

```bash
# Rust side
cd cmux-rs
cargo fetch

# Frontend side
cd ui
pnpm install
```

You can also scaffold a reference Tauri v2 app separately with:

```bash
npm create tauri-app@latest
```

That generator produces the same shape (`src-tauri/` + `src/` with Vite + React), which is what the files in this directory mirror.

## Dev loop

```bash
cd cmux-rs
cargo tauri dev     # builds cmux-app, runs Vite in parallel, opens the window
```

Or build + run the Rust binary only:

```bash
cargo run -p cmux-app
```

## Running tests

```bash
cd cmux-rs
cargo test --workspace
```

UI type-check:

```bash
cd cmux-rs/ui
pnpm build
```

## CI

[../.github/workflows/rust-build.yml](../.github/workflows/rust-build.yml) runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo build --workspace`, and `cargo test --workspace` on macOS, Linux, and Windows. The existing Swift workflows are untouched.

## Relation to the Swift app

The Swift app at the repo root is still the **production** build target. Nothing here replaces [../GhosttyTabs.xcodeproj](../GhosttyTabs.xcodeproj) — `./scripts/reload.sh --tag <tag>` still builds and launches the Swift debug app exactly as before. This directory is additive.

## Current status

Phases 0–2 of [../PLAN.md](../PLAN.md) are complete: the workspace skeleton exists, every Swift file has a sibling Rust stub, and `cmux-app` references them all via `sources.rs`. Phases 3+ (real ports of config, sockets, workspace model, terminal, browser panel, updater, macOS parity) are next.
