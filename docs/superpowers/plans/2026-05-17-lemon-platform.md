# Lemon Platform And Frame Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Lemon Layer 8: winit application shell, wgpu surface, Vello renderer, frame loop wiring runtime → patches → retained → layout → paint → present, and the public `lemon::run()` entry point.

**Architecture:** New `platform` module owns `AppState`. Examples crate (`examples/counter.rs`) becomes the manual verification path. Unit tests in platform are limited to state-machine helpers; full loop verified via `cargo run --example counter`.

**Tech Stack:** Rust 2021; `winit = 0.30`, `wgpu = 29`, `vello = 0.9`, `pollster = 0.4`; all upstream layers.

**Spec reference:** `docs/superpowers/specs/2026-05-17-lemon-architecture-design.md` (Camada 8)

**Depends on:** `2026-05-17-lemon-layout-pass.md`, `2026-05-17-lemon-paint-pass.md`

---

## Scope Check

In scope:
- `WindowConfig`, `lemon::run(config, root_component)`
- `AppState` per spec (window, render context, surface, renderer, scene, runtime, retained, layout_map, font context, dirty flags)
- winit `ApplicationHandler`: `resumed`, `window_event`, `about_to_wait`
- Frame tick: flush runtime effects → apply patches → layout if dirty → paint if dirty → render to surface
- Basic input: click hit-test on `LayoutMap` rects, invoke `on_click` handlers
- `request_redraw` when signals change (runtime effects mark paint/layout dirty)

Out of scope:
- Keyboard focus, text input, drag, scroll
- Multi-window
- Mobile targets

---

## File Structure

```text
src/
  platform/
    mod.rs           ← AppState, ApplicationHandler, run loop glue
    window.rs        ← WindowConfig
  lib.rs             ← pub fn run(...), re-exports
examples/
  counter.rs         ← spec counter demo using lemon::run
```

**Design decisions locked by this plan:**
- Patch queue flushes entirely between events (never mid-handler) per spec.
- First `resumed` mounts root component into `Runtime` + initial `RetainedTree::mount` without diff.
- `scale_factor` from winit passed to layout + paint each frame.

---

### Task 1: WindowConfig And AppState Skeleton

**Files:**
- Create: `src/platform/window.rs`, `src/platform/mod.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml` (expose `run` only; examples use path dep)

- [ ] **Step 1: Add `WindowConfig` with title, size, resizable**

- [ ] **Step 2: Add `AppState` struct fields per spec (allow dead_code until wired)**

- [ ] **Step 3: Compile-only test or `cargo check` gate**

- [ ] **Step 4: Commit**

---

### Task 2: wgpu Surface And Vello Renderer Bootstrap

**Files:**
- Modify: `src/platform/mod.rs`

- [ ] **Step 1: On `resumed`, create window, `RenderContext`, surface, `vello::Renderer`**

Follow Vello 0.9 + wgpu 29 examples pattern; use `pollster::block_on` for adapter request in `run`.

- [ ] **Step 2: Manual test — `cargo run --example counter` opens blank window**

- [ ] **Step 3: Commit**

---

### Task 3: Frame Loop — Runtime, Patches, Layout, Paint

**Files:**
- Modify: `src/platform/mod.rs`
- Modify: `examples/counter.rs`

- [ ] **Step 1: Mount root `fn(&Cx) -> Element` on first resume**

- [ ] **Step 2: On `about_to_wait` / redraw requested:**

```text
runtime.flush_effects()
for patch in runtime.take_patches() { retained.apply_patch(patch)?; layout_dirty = true }
if layout_dirty { layout_map = layout_pass(...); layout_dirty = false; paint_dirty = true }
if paint_dirty { scene.reset(); paint_pass(...); render_to_surface(); paint_dirty = false }
```

- [ ] **Step 3: Subscribe runtime/effects to call `window.request_redraw()`**

- [ ] **Step 4: Verify counter example updates label on click**

- [ ] **Step 5: Commit**

---

### Task 4: Hit-Test And on_click

**Files:**
- Modify: `src/platform/mod.rs`

- [ ] **Step 1: Map cursor to logical coords (divide by scale_factor)**

- [ ] **Step 2: Walk retained tree top-most first; call `handlers.on_click`**

- [ ] **Step 3: Example test: button click increments counter**

- [ ] **Step 4: Full verification + commit**

```bash
cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo test && cargo run --example counter
git commit -m "feat(platform): add winit frame loop and lemon::run"
```

---

## Self-Review

**Spec coverage:** Layer 8 platform, `lemon::run`, frame tick ordering, HiDPI, basic click routing.

**Deferred:** deferred `use_effect` (see runtime-semantics plan), keyboard, scroll, clip.
