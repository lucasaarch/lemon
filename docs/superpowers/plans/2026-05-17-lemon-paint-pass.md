# Lemon Paint Pass Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Lemon Layer 7: walk the retained tree in pre-order and emit Vello `Scene` commands (fills, strokes, glyphs) using `LayoutMap` rects and resolved `PaintData`.

**Architecture:** Pure paint logic testable without a window by building a `vello::Scene` in memory and asserting command counts or using Vello's debug APIs. GPU submission stays in the platform plan.

**Tech Stack:** Rust 2021; `vello = 0.9`; existing `RetainedTree`, `LayoutMap`, `PaintData`, `TextCache` (with Parley layout from layout plan).

**Spec reference:** `docs/superpowers/specs/2026-05-17-lemon-architecture-design.md` (Camada 7)

**Depends on:** `2026-05-17-lemon-layout-pass.md`

---

## Scope Check

In scope:
- `paint_pass(tree: &RetainedTree, layout: &LayoutMap, scene: &mut Scene, scale_factor: f32)`
- Pre-order traversal; transparent `Component` skip
- Box/Row/Column: background fill + border stroke with `CornerRadii`
- Text: glyph runs from cached Parley layout
- Button: box paint + label text (inline label or child text node — match retained model)
- Global HiDPI transform: `scene.push_layer(..., Affine::scale(scale_factor), ...)`

Out of scope:
- wgpu surface / present
- Images (`RetainedKind::Image`) — stub or solid placeholder rect only
- `overflow: hidden` clip layers
- Animations / opacity

---

## File Structure

```text
src/
  paint/
    mod.rs       ← paint_pass, helpers for rect/glyph emission
  lib.rs         ← pub mod paint
```

**Design decisions locked by this plan:**
- Paint uses **logical coordinates** inside the scaled layer (spec global transform at root).
- Missing `LayoutMap` entry for a node → skip paint for that subtree (test asserts no panic).
- Colors come from resolved `PaintData` / `TextStyle.color` only (no dynamic ColorSource in paint).

---

### Task 1: Scene Setup And Container Backgrounds

**Files:**
- Create: `src/paint/mod.rs`
- Modify: `src/lib.rs`
- Test: `src/paint/mod.rs`

- [ ] **Step 1: Write failing test — column with colored background emits fill**

Mount column with `background` color; layout + paint; assert scene is non-empty (or use a test helper counting fills).

- [ ] **Step 2: Implement `paint_pass` skeleton + box background fill**

Map `PaintData.background` to `vello::peniko::Color`; rounded rect from `PaintData.radius`.

- [ ] **Step 3: Run test (expect PASS)**

- [ ] **Step 4: Commit**

---

### Task 2: Borders And Text Glyphs

**Files:**
- Modify: `src/paint/mod.rs`
- Test: `src/paint/mod.rs`

- [ ] **Step 1: Write failing tests for border stroke and text glyphs**

- [ ] **Step 2: Implement border stroke helper**

- [ ] **Step 3: Implement text glyph emission from `TextCache` Parley layout**

If `parley_layout` missing, skip or trigger debug panic in tests only.

- [ ] **Step 4: Run tests (expect PASS)**

- [ ] **Step 5: Commit**

---

### Task 3: Button, Component Transparency, HiDPI Layer

**Files:**
- Modify: `src/paint/mod.rs`
- Test: `src/paint/mod.rs`

- [ ] **Step 1: Write failing test — button paints background then label**

- [ ] **Step 2: Skip `RetainedKind::Component` identically to layout collection**

- [ ] **Step 3: Apply root `push_layer` / `pop_layer` with `scale_factor`**

- [ ] **Step 4: Integration test: mount → patches → layout → paint without panic**

- [ ] **Step 5: Full verification + commit**

```bash
cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo test
git commit -m "feat(paint): add Vello paint pass for retained tree"
```

---

## Self-Review

**Spec coverage:** Layer 7 paint traversal, HiDPI transform, container/text/button paint paths, component transparency.

**Deferred:** GPU present, image rendering, clipping, platform event loop.
