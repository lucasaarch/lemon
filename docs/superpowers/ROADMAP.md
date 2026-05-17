# Lemon Implementation Roadmap

**Spec:** `specs/2026-05-17-lemon-architecture-design.md`  
**GitHub tracker:** [Issue #14](https://github.com/lucasaarch/lemon/issues/14) (issues `#1`–`#13` in execution order)

Execute plans **in this order**. Each plan is self-contained (TDD steps, `cargo test` gates). Do not skip ahead — later layers assume earlier ones.

---

## Phase 0 — Done (do not re-implement)

| # | Plan | Layers | Validates with |
|---|------|--------|----------------|
| ✅ | `plans/2026-05-17-lemon-core-runtime.md` | 1–4 | `cargo test` |
| ✅ | `plans/2026-05-17-lemon-retained-tree.md` | 5 (concrete nodes + patches) | `cargo test` |
| ✅ | `plans/2026-05-17-lemon-component-lifecycle.md` | 2, 4, 5 (nested components) | `cargo test` |

**Current baseline:** 68 unit tests, no window, no GPU.

---

## Phase 1 — Layout (pure, testable)

| # | Plan | Delivers | Gate |
|---|------|----------|------|
| 1 | `plans/2026-05-17-lemon-layout-pass.md` | `LayoutMap`, `layout_pass`, Parley text measure, Taffy `compute_layout_with_measure` | `cargo test` |

**Why first:** Paint and platform need absolute rects. No winit required.

**Exit criteria:** Integration test: `Runtime` → patches → `RetainedTree::apply_patch` → `layout_pass` → non-empty rects for a `Column` + `Text`.

---

## Phase 2 — Paint (CPU scene, no window yet)

| # | Plan | Delivers | Gate |
|---|------|----------|------|
| 2 | `plans/2026-05-17-lemon-paint-pass.md` | `paint_pass` → `vello::Scene` (fills, strokes, glyphs) | `cargo test` |

**Depends on:** Phase 1 (`LayoutMap`).

**Exit criteria:** `layout_pass` + `paint_pass` on a mounted tree completes without panic; scene has content for colored column + text.

---

## Phase 3 — Platform (first real window)

| # | Plan | Delivers | Gate |
|---|------|----------|------|
| 3 | `plans/2026-05-17-lemon-platform.md` | `lemon::run`, winit loop, wgpu + Vello present, click → signal | `cargo run --example counter` |

**Depends on:** Phases 1–2.

**Wire-up per frame:**

```text
flush_effects() → take_patches() → apply_patch* → layout_pass (if dirty) → paint_pass (if dirty) → render_to_surface
```

**Exit criteria:** Counter example opens a window; click increments label; resize triggers relayout.

---

## Phase 4 — Runtime semantics (can split across PRs)

| # | Plan | Delivers | When |
|---|------|----------|------|
| 4a | `plans/2026-05-17-lemon-runtime-semantics.md` Task 1 | Keyed child diff (`MoveChild`) | Anytime after Phase 0 |
| 4b | Same plan Task 2 | `Derived` equality-aware notify | Anytime after Phase 0 |
| 4c | Same plan Task 3 | Deferred `use_effect` (after paint) | **After Phase 3** |

**Recommendation:** Do **4a + 4b** before Phase 3 if you want a cleaner diff/runtime before UI; do **4c** only once `platform` can call `flush_deferred_effects()` after present.

---

## Dependency graph

```text
[Phase 0: core + retained + components]  (done)
        │
        ▼
[Phase 1: layout-pass]
        │
        ▼
[Phase 2: paint-pass]
        │
        ▼
[Phase 3: platform + lemon::run]
        │
        ▼
[Phase 4c: deferred use_effect]

Phase 4a/4b ──► optional anytime after Phase 0 (parallel to 1–3)
```

---

## Per-session checklist

After each plan (or task commit):

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo check
cargo test
```

After Phase 3 only:

```bash
cargo run --example counter
```

---

## What is intentionally out of v1

- Image rendering (beyond placeholder)
- Keyboard / text input / scroll
- `overflow: hidden` clipping
- Multi-window
- Full keyed list reconciliation at runtime level (diff only in 4a)

See each plan's **Self-Review** section for details.
