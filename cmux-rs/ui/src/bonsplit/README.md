# ui/src/bonsplit

TypeScript reimplementation of the Swift [Bonsplit](../../../../vendor/bonsplit/Sources/Bonsplit/) split-pane / tab-bar controller.

This directory is the UI half of the Bonsplit port. The Rust model half lives at [cmux-rs/crates/cmux-core/src/bonsplit.rs](../../../crates/cmux-core/src/bonsplit.rs).

Per the locked decision in [PLAN.md](../../../../PLAN.md), neither of these touch [vendor/bonsplit/](../../../../vendor/bonsplit/). The submodule stays frozen.

Phase 5 of the plan populates this directory with:

- `SplitContainer.tsx` — resizable split panes (via `react-resizable-panels`)
- `TabBar.tsx` — drag-to-reorder tabs
- `PaneHost.tsx` — pane container with drop zones
- `bonsplit-types.ts` — TS mirror of the Rust model types
