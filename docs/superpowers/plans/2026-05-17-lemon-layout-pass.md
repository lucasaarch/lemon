# Lemon Layout Pass Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Lemon Layer 6: run Taffy layout on the retained tree, measure text nodes with Parley, and produce a `LayoutMap` of absolute logical rects for paint and hit-testing.

**Architecture:** Keep layout independent from winit/Vello. `RetainedTree` already owns `TaffyTree` and `TextCache`; this plan adds a `layout` module that computes layouts when `layout_dirty` would be true in the full app, but validates with unit tests using fixed viewport sizes.

**Tech Stack:** Rust 2021; `taffy = 0.7`; `parley = 0.9`; existing `RetainedTree`, `RetainedNode`, `StyleProps`, `TextCache`.

**Spec reference:** `docs/superpowers/specs/2026-05-17-lemon-architecture-design.md` (Camada 6)

**Depends on:** `2026-05-17-lemon-retained-tree.md`, `2026-05-17-lemon-component-lifecycle.md` (transparent component nodes)

---

## Scope Check

In scope:
- `LayoutMap` (`HashMap<NodeId, Rect>` in logical points)
- `layout_pass(&mut RetainedTree, viewport: Size, scale_factor: f32) -> LayoutMap`
- Parley measure callback for `RetainedKind::Text` and button label text
- `TextCache` invalidation wired to `needs_layout` from `UpdateText` patches
- Transparent skip for `RetainedKind::Component` when collecting layouts

Out of scope (later plans):
- Vello scene building (paint pass)
- winit resize events / frame loop
- Hit-testing and input routing
- `overflow: hidden` clipping layers

---

## File Structure

```text
src/
  layout/
    mod.rs          ← LayoutMap, layout_pass, collect_layouts
  retained/mod.rs   ← expose taffy + root for layout; optional layout_dirty helper
  lib.rs            ← pub mod layout; re-export LayoutMap
```

**Design decisions locked by this plan:**
- All layout math stays in **logical points**; `scale_factor` is passed only into Parley shaping.
- `layout_node_id()` on `RetainedNode` is the map key (never component wrapper ids).
- Re-measure text only when `TextCache.needs_layout` is true or cache fields changed.

---

### Task 1: LayoutMap Skeleton And Transparent Collection

**Files:**
- Create: `src/layout/mod.rs`
- Modify: `src/lib.rs`
- Test: `src/layout/mod.rs`

- [ ] **Step 1: Write failing tests for `collect_layouts` on a mounted column**

Test that a `Column` with two `Text` children produces two rects with increasing `y` and correct widths from Taffy after `layout_pass`.

- [ ] **Step 2: Run tests (expect compile error)**

```bash
cargo test layout::tests::column_children_stack_vertically -- --nocapture
```

- [ ] **Step 3: Implement `LayoutMap` and `collect_layouts`**

```rust
pub struct LayoutMap {
    rects: HashMap<taffy::NodeId, Rect>,
}

pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
```

Walk retained tree; skip `RetainedKind::Component` by recursing into children without consuming offset; use `layout_node_id()` for Taffy lookups.

- [ ] **Step 4: Run tests (expect PASS)**

- [ ] **Step 5: Commit**

```bash
git add src/layout/mod.rs src/lib.rs
git commit -m "feat(layout): add LayoutMap and transparent layout collection"
```

---

### Task 2: Parley Text Measurement Callback

**Files:**
- Modify: `src/layout/mod.rs`
- Modify: `src/retained/mod.rs` (if `TextCache` needs `parley_layout` field per spec)
- Test: `src/layout/mod.rs`

- [ ] **Step 1: Write failing test — text node gets non-zero height after measure**

Mount `Text::new("hello")` with explicit width; assert measured height > 0 and `needs_layout` becomes false.

- [ ] **Step 2: Run test (expect FAIL)**

- [ ] **Step 3: Implement `measure_node` using Parley**

Use `parley::LayoutContext` / `FontContext` owned by `layout_pass` (thread-local or passed struct). Cache result on `TextCache` when content/style/max_width unchanged.

- [ ] **Step 4: Wire `taffy::compute_layout_with_measure`**

Call from `layout_pass` with viewport size; measurement closure reads `RetainedNode` text cache.

- [ ] **Step 5: Run tests (expect PASS)**

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(layout): measure text nodes with Parley in Taffy pass"
```

---

### Task 3: Integration With RetainedTree And Runtime Patches

**Files:**
- Modify: `src/layout/mod.rs`
- Modify: `src/retained/mod.rs`
- Test: `src/lib.rs` (integration test)

- [ ] **Step 1: Write failing integration test**

```text
Runtime mount → flush_effects → take_patches → RetainedTree::mount → apply each patch → layout_pass
```

Assert `UpdateText` patch causes relayout height change in `LayoutMap`.

- [ ] **Step 2: Implement `RetainedTree::apply_patches` helper (batch)**

Optional convenience wrapping sequential `apply_patch` + `layout_dirty` flag.

- [ ] **Step 3: Run integration test (expect PASS)**

- [ ] **Step 4: Run full verification**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo check
cargo test
```

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(layout): integrate layout pass with retained tree patches"
```

---

## Self-Review

**Spec coverage:** Layer 6 layout pass, Parley measurement, `LayoutMap`, HiDPI note (scale_factor to Parley only), transparent component traversal.

**Deferred:** paint pass, platform loop, hit-test, overflow clipping.
