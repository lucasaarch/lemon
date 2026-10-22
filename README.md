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
use lemon::prelude::*;

fn counter(cx: &Cx) -> Element {
    let count = cx.use_signal(0i32);
    let count_text = count.clone();
    let count_btn = count.clone();

    Column::new()
        .gap(12.0)
        .padding(24.0)
        .child(Text::new(move || count_text.get().to_string()).font_size(24.0))
        .child(Button::new(cx, "Increment").on_click(move || {
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

Run the counter example:

```bash
cargo run --example counter
```

## Examples

Each example has one job. Sources live in [`crates/lemon/examples`](crates/lemon/examples).

| Example | Topic |
|---------|-------|
| `counter` | Minimal signal, reactive text, and button |
| `reactivity` | Signals, memoized values, and effects (output in the terminal) |
| `components` | Independent hook state in keyed component instances |
| `keys` | Stable row identity during removal and reordering |
| `layout` | Rows, columns, spacing, alignment, and styled views |
| `form` | TextInput, Select, Slider, and scrollable submission history |
| `images` | In-memory RGBA pixels and shared image handles |
| `custom_select` | A mouse-operated picker composed from primitives |
| `animation` | Animation progress, opacity, and theme tokens |

Run any example with `cargo run --example <name>`. Start with:

```bash
cargo run --example counter
cargo run --example reactivity
cargo run --example form
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

## Repository Layout

The repository is a Cargo workspace. Applications continue to depend on `lemon`
and use the same root/prelude imports.

| Path | Role |
|------|------|
| `crates/lemon/` | Public facade, compatibility reexports, and app examples |
| `crates/lemon-reactive/` | Signals, derived values, effects, and observer tracking; no dependencies |
| `crates/lemon-core/` | Elements, component runtime, diff, retained state, layout, scene construction, and theme |
| `crates/lemon-ui/` | Composed widgets (`TextInput`, `Scroll`, `Select`, `Slider`, `Image`, …) |
| `crates/lemon-desktop/` | Native windows, input dispatch, GPU surfaces, and presentation |
| `crates/lemon/examples/*.rs` | Runnable examples (`cargo run --example <name>`) |

Dependencies flow from `lemon` to `lemon-ui` and `lemon-desktop`, both to
`lemon-core`, then to `lemon-reactive`. Core does not depend on UI or desktop.
Primitive element builders remain in core; composed/themed widgets live in UI.
Layout and paint stay together in core until a concrete independent consumer
justifies another crate boundary.

`lemon-core::ui::UiState` runs the update/layout/scene pipeline without a native
window. `lemon-core::host::HostContext` owns an application's window requests and
scopes its theme. Desktop windows delegate UI processing to this headless state.

Run `make check` from the workspace root to validate every crate and example.
For focused work, use `cargo test -p lemon-reactive` or `cargo test -p lemon-core`.
For crates.io releases, publish dependencies before their consumers:
`lemon-reactive`, `lemon-core`, `lemon-ui`/`lemon-desktop`, then `lemon`.
All internal dependencies carry both a local path and an exact version.

### Low-level migration notes

- Application imports through `lemon` and `lemon::prelude` remain unchanged.
- `WindowState` now owns `ui: UiState`; normal field access to the old UI fields
  forwards through `Deref`/`DerefMut`. Code destructuring `WindowState` must use
  the new `ui` field.
- Manual `StyleProps` initializers must set `scroll_content` (normally `false`)
  or use `..Default::default()`. `Scroll` sets this marker automatically.
- `Cx::use_theme` reads the originating host's captured theme, not whichever
  unrelated window happened to activate a theme last.
- Existing public mutable engine fields remain available for custom hosts.
  This migration does not remove them or claim to fully encapsulate that API.

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

## Primitive-First Customization

Built-in widgets such as `Button`, `Select`, `Slider`, `Scroll`, and `TextInput` cover common
application UI, and their defaults come from the active `Theme`. Start there when the widget shape
matches your product. Use `run_with_theme` for app-wide tokens and widget style types such as
`SelectStyle`, `SliderStyle`, and `TextInputStyle` for local overrides.

When a widget almost fits but the visual structure does not, rebuild the control from primitives:
`View`, `Column`, `Row`, `Text`, `Button`, `Signal`, and event handlers. This keeps the app in the
normal Lemon pipeline while giving you control over layout, paint, hover/press state, and copy.
For repeated dynamic rows, give siblings stable `.key(id)` values so diffing preserves identity.
For colors that depend on live state, pass a closure to paint builders such as `.background(...)`;
the closure is stored as a dynamic `ColorSource` and is resolved as the tree is frozen.

The `custom_select` example demonstrates this escape hatch by building a select-like control from
regular nodes:

```bash
cargo run --example custom_select
```

Use this path when the customization is mostly visual or structural. Prefer the built-in widget
plus theme/style overrides when the interaction model is the same. Fork or write a custom widget
when you need to own identity, overlay positioning, pointer capture, keyboard handling, or internal
state transitions as a reusable app abstraction.

Current limits live at the paint layer. Widget-managed details such as scrollbars and text-input
carets are painted by Lemon's retained widget metadata, not by arbitrary app paint hooks. If those
details need brand-specific behavior beyond existing style overrides, use a custom primitive
composition where possible and track the relevant roadmap work:
[#40](https://github.com/lucasaarch/lemon/issues/40) for theme tokens,
[#42](https://github.com/lucasaarch/lemon/issues/42) for theme-backed widget defaults,
[#92](https://github.com/lucasaarch/lemon/pull/92) for positioned select overlays, and
[#93](https://github.com/lucasaarch/lemon/pull/93) for scroll behavior polish.

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
cargo build
cargo test
make build-examples
cargo run --example counter
cargo run --example form
```

Full quality gates:

```bash
make check
# or:
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --examples
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
