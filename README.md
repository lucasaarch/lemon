# Lemon

A reactive UI toolkit for native desktop apps in Rust.

Describe UI with components and fluent builders; state flows through **signals**. When something changes, Lemon diffs the virtual element tree and updates only what changed—layout, paint, and GPU present follow in one frame loop.

**Stack:** [winit](https://github.com/rust-windowing/winit) · [wgpu](https://github.com/gfx-rs/wgpu) · [Vello](https://github.com/linebender/vello) · [Taffy](https://github.com/DioxusLabs/taffy) · [Parley](https://github.com/linebender/parley)

## Quick start

```rust
use lemon::{run, Button, Column, Cx, Text, WindowConfig};

fn counter(cx: &Cx) -> lemon::element::Element {
    let count = cx.use_signal(0i32);
    let label = count.clone();
    let btn = count.clone();

    Column::new()
        .gap(12.0)
        .padding(24.0)
        .child(Text::new(move || label.get().to_string()).font_size(24.0))
        .child(Button::new("Increment").on_click(move || btn.update(|n| *n += 1)))
        .into_element()
}

fn main() {
    run(
        WindowConfig::default().title("Lemon").size(900.0, 600.0),
        counter,
    );
}
```

```bash
cargo run -p counter
```

Add `lemon = "0.1"` to your app’s `Cargo.toml`, or use this repo as a workspace (see below).

## Workspace

This repository is a [Cargo workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html):

| Crate | Path | Published |
|-------|------|-----------|
| `lemon` | `lemon/` | yes (crates.io) |
| `counter` | `examples/counter/` | no (`publish = false`) |

Future crates (e.g. `lemon-widgets`) can be added as workspace members with aligned versions via `[workspace.package]`.

```bash
cargo test              # all workspace crates
cargo test -p lemon     # core only
cargo run -p counter    # demo app
```

## Architecture

Eight layers, inside-out:

1. **Reactive runtime** — `Signal`, `Derived`, `Effect`, hooks (`use_signal`, `use_memo`, `use_effect`)
2. **Components** — `fn(&Cx) -> Element`
3. **Element tree** — virtual tree built with builders
4. **Diff** — patches (`UpdateText`, `InsertChild`, `MoveChild`, …)
5. **Retained tree** — live nodes + Taffy layout IDs
6. **Layout** — flexbox + Parley text measurement
7. **Paint** — Vello scene (fills, borders, glyphs)
8. **Platform** — window, input, GPU present

Layers 1–4 are pure Rust and covered by unit tests. Layers 5–8 run when you call `lemon::run`.

## Project status

**v1** ships a working desktop shell: window, layout, paint, mouse click, and the counter example. Not yet included: text fields, keyboard focus, scroll, image rendering, or overflow clipping.

## Development

```bash
cargo test -p lemon
cargo build -p lemon
cargo run -p counter
```

Requires a recent stable Rust toolchain and GPU compute support (Vello).

## License

MIT OR Apache-2.0
