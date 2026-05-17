# Lemon

A cross-platform UI toolkit for building native graphical applications in Rust.

Lemon aims to give you the ergonomics of a modern widget framework—think along the lines of [Iced](https://iced.rs/) or GPU-driven toolkits like [GPUI](https://www.gpui.rs/)—while staying close to a small, composable stack you can understand and extend.

## Vision

Desktop UI toolkits usually split into three concerns:

1. **Windowing & input** — connect to the OS event loop, keyboards, pointers, and displays.
2. **Layout** — turn a tree of elements and style rules into positions and sizes.
3. **Rendering** — paint backgrounds, borders, vector shapes, images, and text on the GPU.

Lemon is being built around that model explicitly, rather than hiding it behind a monolithic runtime. The goal is a toolkit that feels native on macOS, Windows, and Linux, scales with Retina/high-DPI displays, and stays fast enough for interactive tools, editors, and utilities—not only static forms.

## Stack

| Layer | Crate | Role |
|-------|--------|------|
| Windowing | [winit](https://github.com/rust-windowing/winit) | Cross-platform windows and events |
| GPU | [wgpu](https://github.com/gfx-rs/wgpu) | Safe graphics API (Vulkan, Metal, DX12, …) |
| 2D render | [Vello](https://github.com/linebender/vello) | GPU vector rendering (paths, fills, layers) |
| Layout | [Taffy](https://github.com/DioxusLabs/taffy) | Flexbox & grid (CSS-like layout) |
| Text | [Parley](https://github.com/linebender/parley) | Rich text layout and shaping |
| Async glue | [pollster](https://github.com/RustAsync/pollster) | Block on wgpu initialization on the main thread |

Together, these pieces mirror how many production UI engines work internally: **element tree → layout pass → paint pass → present**.

## Project status

Early exploration. The public API, widget set, and application model are not defined yet.

A small graphics bootstrap (window, Vello scene, Taffy flex layout, Parley text) lives under [`archive/bootstrap-wgpu-vello-taffy/`](archive/bootstrap-wgpu-vello-taffy/) for reference. It is not part of the crate and is ignored by rust-analyzer.

## Building

```bash
cargo build
cargo run
```

Requires a recent stable Rust toolchain and a GPU with compute shader support (for Vello).

## License

Not specified yet.
