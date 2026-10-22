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
    let count = cx.state(0i32);
    let count_text = count.clone();
    let count_btn = count.clone();

    ColumnView::new()
        .spacing(12.0)
        .inset(24.0)
        .child(Text::new(move || count_text.get().to_string()).size(24.0))
        .child(Button::new(cx, "Increment").action(move || {
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

Each example has one job. Sources live in [`examples`](examples).

| Example | Topic |
|---------|-------|
| `counter` | Minimal signal, reactive text, and button |
| `reactivity` | Signals, memoized values, and effects (output in the terminal) |
| `components` | Independent hook state in keyed component instances |
| `keys` | Stable row identity during removal and reordering |
| `layout` | Rows, columns, spacing, alignment, and styled views |
| `custom` | A form centered in a view that fills the window |
| `form` | TextField, SecureField, Select, Slider, and scrollable submission history |
| `images` | In-memory RGBA pixels and shared image handles |
| `custom_select` | A mouse-operated picker composed from primitives |
| `animation` | Animation progress, opacity, and theme tokens |

Run any example with `cargo run --example <name>`. Start with:

```bash
cargo run --example counter
cargo run --example reactivity
cargo run --example form
```

## Application API

Lemon describes the intent of a view through fluent builders:

- `cx.state(initial)` declares shared component state; `cx.derive(...)` computes comparable data from it.
- `cx.watch(...)` runs a callback after mount and whenever its reactive reads change.
- `cx.motion(...)` declares an animation; `cx.theme()` reads the originating host's theme.
- `.fill()` follows the parent content size (the viewport for a root view); `.fill_width()` and `.fill_height()` select an axis.
- `.center()` and `.place(Placement::BottomRight)` position children in physical directions, the same for rows and columns.
- `.spacing(...)` separates children; `.inset(...)` adds inner space; `.outset(...)` adds outer space.
- `.surface(...)`, `.rounding(...)`, `.gradient(...)`, `.shadow(...)`, and `.layer(...)` decorate a container.
- Text uses `.size(...)`, `.font(...)`, `.leading(...)`, and `.color(...)`; primary activation uses `.action(...)`.

Component resources still require stable declaration order across renders. Widget constructors
also declare state, so put dynamic widget instances in component scopes rather than changing
loops on a shared `Cx`. `State<T>` clones share a value; element trees are built during rendering,
not cached with `cx.derive(...)`.

### Shared decoration

Every builder and custom widget implementing `Into<Element>` supports the same
root styling through `Decorate` (included in the prelude):

```rust
use lemon::prelude::*;

fn field(view: View) -> View {
    view.width(300.0).inset(12.0).rounding(10.0)
}

fn login(cx: &Cx) -> Element {
    let name = cx.state(TextFieldState::new(""));
    let password = cx.state(TextFieldState::new(""));
    ColumnView::new().spacing(12.0)
        .child(TextField::new(cx, name).placeholder("Name").decorate(field))
        .child(SecureField::new(cx, password).placeholder("Password").decorate(field))
        .into_element()
}
```

Decoration modifies the existing root after widget defaults; explicit changes win.
Containers preserve their kind, keys, handlers and children. Text, raw leaves and
components receive a surrounding view. Widget-specific `.style(...)` still controls
internal parts such as a slider fill or select dropdown. See `custom` for a runnable example.

## Mental Model

Lemon is intentionally built as a pipeline of small layers:

1. **Runtime**: signals, derived values, effects, and hook-like APIs through `Cx`
2. **Element tree**: builders such as `ColumnView`, `RowView`, `View`, `Text`, and `Button`
3. **Diff**: compare virtual trees and emit patches
4. **Retained tree**: store live nodes, handlers, and Taffy layout IDs
5. **Layout**: compute flex layout and text measurement
6. **Paint**: build a Vello scene
7. **Input and platform**: core dispatches logical input; desktop translates native events and presents frames

This layering matters. Lemon is designed so that state changes do not imply full re-execution of every lower-level concern beyond what is actually dirty.

## Repository Layout

Lemon is one Cargo package. Its implementation lives in `src/`, with modules
organized by responsibility:

| Path | Role |
|------|------|
| `src/lib.rs`, `src/prelude.rs` | Public API and application imports |
| `src/reactive/` | Signals, derived values, effects, and observer tracking |
| `src/runtime/` | Component lifecycle, state slots, and reactive rendering |
| `src/element/`, `src/widget/` | Element model, builders, shared styling, and controls |
| `src/diff/`, `src/retained/` | Reconciliation, patches, retained state and invalidation |
| `src/layout/`, `src/paint/` | Geometry, text measurement, and scene construction |
| `src/input/`, `src/ui.rs` | Logical input and headless UI processing |
| `src/host.rs`, `src/animation.rs`, `src/theme.rs` | Host resources, animation clocks and themes |
| `src/platform/` | Native windows, event conversion, clipboard, GPU surfaces and presentation |
| `examples/` | Runnable applications (`cargo run --example <name>`) |
| `tests/` | Integration tests and tests included by examples |

`lemon::widget` exposes containers and controls. Primitive implementations are
split into container, text, button and component modules under `element::builders`.
Reactive primitives use only the standard library. Rendering and input processing
stay independent of native windowing, which belongs to `platform`.

`lemon::ui::UiState` runs update, layout, input and scene construction without
opening a native window. `lemon::host::HostContext` owns window requests, themes,
animations and clipboard callbacks. The platform converts native events and
presents the resulting scene.

Run `make check` from the repository root. For focused work, use
`cargo test reactive::` or `cargo test --test reconciliation`. Publish the
single package with `cargo publish`.

### Headless engine APIs

- Applications import public types through `lemon` and `lemon::prelude`.
- Container builders are `RowView` and `ColumnView`. Element enum variants retain
  their engine names.
- `UiState` and `RetainedTree` expose read-only inspection through methods such as
  `retained()`, `root()`, `layout_map()`, `scene()` and dirty getters. Use
  `apply_updates()`, `apply_patches()`, `invalidate_layout()` or `invalidate_paint()`
  to change processing state. Font changes go through `edit_fonts(...)`.
- `UiState::update` detects viewport, scale, focus and caret changes automatically.
  Paint-only style and text-color changes keep geometry and measured text caches.
- Manual `StyleProps` initializers must set `scroll_content` (normally `false`)
  or use `..Default::default()`. `Scroll` sets this marker automatically.
- `Cx::theme` reads the originating host's live theme, not whichever
  unrelated window happened to activate a theme last.
- Access window UI explicitly through `WindowState::ui`; there is no dereferencing
  bridge to the engine. Native window requests are drained from `HostContext`.
- Animation clocks and frame requests use the host's `AnimRegistry`. Standalone
  `Cx` instances own independent resources; enter a host scope to share them.

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

## System appearance and custom themes

`run(...)` and `run_with_fonts(...)` follow the native light/dark appearance, both
when the window opens and when the system changes it. Widget defaults, implicit
text colors, and the window background update together without resetting component
state. Read semantic tokens with `cx.theme()` instead of hardcoding a light/dark
surface and relying on an unrelated foreground color.

The built-in palettes live in [`src/theme.rs`](src/theme.rs): edit
`Theme::default_dark()` or `Theme::default_light()` to change Lemon's defaults.
For application-specific changes, edit a `ThemePair` before passing it to
`run_with_theme`. Its `colors` controls fills, text and borders; `chrome` controls
focus rings, carets and scrollbars; `typography`, `spacing` and `radius` control
the remaining shared defaults.

Applications can supply their own light/dark pair using the same entry point:

```rust
use lemon::{run_with_theme, Theme, ThemePair, WindowConfig};
use lemon::prelude::*;

fn app(_cx: &Cx) -> Element {
    Text::new("Hello").into_element()
}

let mut light = Theme::default_light();
let mut dark = Theme::default_dark();
light.radius.md = 10.0;
dark.radius.md = 10.0;
dark.colors.background = Color::rgb8(24, 24, 24);
dark.colors.surface = Color::rgb8(36, 36, 36);
run_with_theme(WindowConfig::default(), ThemePair::new(light, dark), app);
```

Passing a single `Theme` to `run_with_theme` keeps that palette fixed. Local widget overrides continue to take precedence.
Native window decorations retain the system appearance.

Detection and live change events use winit's macOS and Windows support. When the
platform does not report an appearance (including the current X11/Wayland API),
Lemon uses the dark palette. `Theme::default()` remains a deterministic dark palette
for headless use; it does not query the OS. See the [winit platform limits](https://docs.rs/winit/0.30.13/winit/window/struct.Window.html#method.theme).

## Primitive-First Customization

Built-in widgets such as `Button`, `Select`, `Slider`, `Scroll`, `TextField`, and `SecureField` cover common
application UI, and their defaults come from the active `Theme`. Start there when the widget shape
matches your product. Use `run_with_theme` for app-wide tokens and widget style types such as
`SelectStyle`, `SliderStyle`, and `TextFieldStyle` for local overrides.

`SecureField` shares `TextFieldState` and `TextFieldStyle`, displays bullets, and
blocks clipboard copy/cut shortcuts. Its state retains the original value for app code.

When a widget almost fits but the visual structure does not, rebuild the control from primitives:
`View`, `ColumnView`, `RowView`, `Text`, `Button`, `Signal`, and event handlers. This keeps the app in the
normal Lemon pipeline while giving you control over layout, paint, hover/press state, and copy.
For repeated dynamic rows, give siblings stable `.key(id)` values so diffing preserves identity.
For colors that depend on live state, pass a closure to paint builders such as `.surface(...)`;
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
- the package has meaningful tests and CI gates
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
- pass `fmt`, `clippy`, and package tests

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
