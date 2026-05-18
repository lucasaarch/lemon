# Lemon

[![CI](https://github.com/lucasaarch/lemon/actions/workflows/ci.yml/badge.svg)](https://github.com/lucasaarch/lemon/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

**Reactive native UI for Rust, with an architecture you can still hold in your head.**

Lemon is a reactive UI toolkit for native desktop applications in Rust.

It combines a small reactive runtime, a fluent builder API, incremental tree diffing, retained layout state, and a modern GPU rendering stack. The goal is simple: make native UI in Rust feel composable, explicit, and fast without burying the rendering model under a giant framework.

Lemon is already useful as a serious experimental codebase and an early foundation for desktop UI work. It is not yet a fully mature application framework, but it is far enough along to demonstrate a credible direction for building Rust-native UI.

**Core stack:** [winit](https://github.com/rust-windowing/winit) · [wgpu](https://github.com/gfx-rs/wgpu) · [Vello](https://github.com/linebender/vello) · [Taffy](https://github.com/DioxusLabs/taffy) · [Parley](https://github.com/linebender/parley)

## Why Lemon

- Reactive state with explicit `Signal`, `Effect`, and `Cx` primitives
- Fluent UI builders for composing trees in plain Rust
- Incremental updates through diff + retained tree, rather than full rebuilds of every downstream stage
- Modern native rendering pipeline built on `wgpu` and `vello`
- Architecture that stays understandable: runtime, element tree, diff, retained tree, layout, paint, platform

## Positioning

Lemon sits between a toy UI experiment and a production-complete framework.

It is best understood today as:

- a credible foundation for exploring native Rust UI architecture
- a serious experimental toolkit for early adopters
- a codebase designed to stay legible as it grows

It is not trying to imitate a web framework in Rust, and it is not trying to claim feature parity with large established UI toolkits. Its value is the combination of explicit reactivity, incremental updates, and a rendering pipeline that contributors can actually understand.

## Quick Example

```rust
use lemon::{run, Cx, WindowConfig};
use lemon_widgets::{Button, Column, Text};

fn counter(cx: &Cx) -> lemon::element::Element {
    let count = cx.use_signal(0i32);
    let count_text = count.clone();
    let count_btn = count.clone();

    Column::new()
        .gap(12.0)
        .padding(24.0)
        .child(Text::new(move || count_text.get().to_string()).font_size(24.0))
        .child(Button::new("Increment").on_click(move || {
            count_btn.update(|n| *n += 1);
        }))
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon")
            .size(900.0, 600.0),
        counter,
    );
}
```

Run the example workspace app:

```bash
cargo run -p counter
```

## Examples

The workspace includes a few small but representative examples:

- `counter`: minimal reactive state and button interaction
- `card`: nested layout, styling, and simple interaction
- `list_keyed`: keyed children and dynamic add/remove updates

Run them with:

```bash
cargo run -p counter
cargo run -p card
cargo run -p list_keyed
```

## Mental Model

Lemon is intentionally built as a pipeline of small layers:

1. **Runtime**: signals, derived values, effects, and hook-like APIs through `Cx`
2. **Element tree**: builders such as `Column`, `Row`, `View`, `Text`, and `Button` (`Box_` was renamed to `View`; see `cargo doc -p lemon`)
3. **Diff**: compare virtual trees and emit patches
4. **Retained tree**: store live nodes, handlers, and Taffy layout IDs
5. **Layout**: compute flex layout and text measurement
6. **Paint**: build a Vello scene
7. **Platform**: dispatch input, manage the window, and present frames

This layering matters. Lemon is designed so that state changes do not imply full re-execution of every lower-level concern beyond what is actually dirty.

## Workspace Layout

This repository is a Cargo workspace:

| Crate | Path | Role |
|------|------|------|
| `lemon` | `crates/lemon` | Core runtime, element model, diffing, retained tree, layout, paint, platform |
| `lemon-widgets` | `crates/lemon-widgets` | App-facing widget and builder re-exports |
| `counter` | `examples/counter` | Minimal reactive counter |
| `card` | `examples/card` | Composed card layout example |
| `list_keyed` | `examples/list_keyed` | Keyed list update example |

## Current Capabilities

Today, the codebase includes:

- reactive rendering through signals and effects
- builder-based composition for boxes, rows, columns, text, buttons, and components
- incremental patch generation and retained tree updates
- flex layout via Taffy
- text measurement and painting via Parley + Vello
- mouse click handling
- keyboard event dispatch and focus cycling
- hover enter/leave handling and cursor selection
- example applications that exercise composition, keyed updates, and interaction

## Current Status

Lemon should be understood as an early-stage library with a real architecture, not a finished framework.

That means:

- the internal model is strong enough to build on
- the workspace has meaningful tests and CI gates
- the public surface is still evolving
- some application-level widgets and interaction patterns are still missing

If you are evaluating Lemon, the right expectation is: **serious experimental library, not production-stable UI platform**.

## Stability

What is stable enough to rely on conceptually:

- the layered architecture
- the reactive rendering model
- the use of retained state and incremental updates as core design principles

What is still evolving:

- app-facing ergonomics
- widget breadth
- public API shape
- interaction and platform coverage

## What Is Not There Yet

Lemon is still missing parts that a production UI framework would need, including areas such as:

- richer text input controls
- broader widget set
- scrolling and overflow behavior
- image and asset workflows beyond the current core
- accessibility and deeper platform integration
- long-term API stability guarantees

## Roadmap Direction

Near-term work is likely to continue in the following areas:

- richer built-in widgets
- stronger text and input handling
- scrolling and overflow semantics
- broader interaction coverage
- clearer application-facing APIs on top of the core engine
- continued hardening of tests, examples, and internal boundaries

## Developing In This Repository

Prerequisites:

- recent stable Rust
- a machine capable of running the `wgpu` / `vello` stack

Useful commands:

```bash
cargo build -p lemon
cargo test -p lemon
cargo test -p lemon-widgets
cargo build -p counter
cargo build -p card
cargo build -p list_keyed
```

Full workspace checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Contributing

Contributions should preserve the current architecture rather than blur it.

In practice, good changes in this repository usually:

- keep logic in the correct layer
- stay small and explicit
- add or update tests near the affected module
- verify examples when public APIs change
- pass `fmt`, `clippy`, and workspace tests

For repository-specific contributor guidance, see [AGENTS.md](AGENTS.md).

## Design Direction

Lemon is aiming for a style of Rust UI development that is:

- reactive, but explicit
- ergonomic, but not magical
- layered, so the rendering model stays inspectable
- small enough that contributors can understand the whole stack

The project is especially opinionated about keeping the pipeline legible. If the framework grows, it should grow by strengthening these layers rather than hiding them.

## License

MIT
