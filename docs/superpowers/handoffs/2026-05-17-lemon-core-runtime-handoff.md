# Lemon Core Runtime Handoff

**Date:** 2026-05-17
**Repo state:** `master`
**Last completed commit:** `f93427c` — `feat: complete lemon core runtime implementation`

## Current Status

The implementation plan in `docs/superpowers/plans/2026-05-17-lemon-core-runtime.md` has been completed.

The pure core layers are implemented and validated:

- Layer 1: reactive runtime
  - `Signal<T>`
  - `Derived<T>`
  - `Effect`
  - observer stack
- Layer 2: component model core
  - `Cx`
  - hook index persistence across re-renders
- Layer 3: element tree
  - element data types
  - dynamic text/color closures
  - fluent builders
- Layer 4: diff + patch
  - recursive unkeyed diff
  - runtime patch queue
  - top-level component mount + re-render flow

## Validation Completed

All of the following were run successfully after the final fixes:

- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo check`
- `cargo test`

At the end of validation:

- `43` tests passed
- `0` test failures
- `0` clippy warnings

## Important Implementation Notes

### Runtime tree freezing

The runtime stores a frozen version of the rendered `Element` tree in `src/runtime/mod.rs`.

Reason:

- dynamic content like `TextContent::Dynamic` and `ColorSource::Dynamic` reads signals from closures
- if the runtime stores the old tree with live closures, the "old" side of the diff resolves against current signal values and changes disappear
- the fix was to freeze rendered trees into static resolved values before storing them as `previous` / `pending`

This behavior is intentional and currently required for correct diffing.

### Closure storage changed from `Box<dyn Fn>` to `Rc<dyn Fn>`

Several element-layer types were changed to store closures in `Rc` so the tree can be cloned safely:

- `TextContent::Dynamic`
- `ColorSource::Dynamic`
- `ButtonElement.on_click`
- `ComponentElement.view`

This supports frozen-tree handling and avoids re-running the view just to rebuild a consumed tree.

### Diff fixes beyond the original task list

Two real spec-alignment fixes were added during validation:

- `ButtonElement.style` now emits `Patch::UpdateStyle`
- `ImageElement` now diffs `style`, and `src` changes emit `Patch::ReplaceNode`

## Remaining Gaps vs Spec

These are the meaningful gaps still left relative to `docs/superpowers/specs/2026-05-17-lemon-architecture-design.md`.

### Still deferred / not implemented

- keyed child diffing
  - `MoveChild` exists but is not emitted
- real `ComponentElement` lifecycle in diff/runtime
  - no `MountComponent`
  - no `UnmountComponent`
  - no nested component preservation by `type_id + key`
- layers 5–8
  - retained tree
  - patch application
  - Taffy layout pass
  - Parley text measurement/cache
  - Vello paint pass
  - winit/wgpu platform integration
  - final `lemon::run()`

### Semantic differences still present in the core

- `Cx::use_effect(...)` currently runs immediately when mounted in the pure core.
  - The spec text describes effects as running after paint.
  - This likely needs a deferred-effect queue once retained tree / frame loop exists.
- `Derived<T>` currently invalidates downstream subscribers whenever a dependency changes.
  - The spec text says downstream notification should happen when the computed value changes.
  - That implies recomputation + equality-aware propagation, which is not implemented.

## Recommended Next Plan

**Completed since this handoff was written:**

| Plan | Status |
|------|--------|
| `2026-05-17-lemon-retained-tree.md` | Implemented |
| `2026-05-17-lemon-component-lifecycle.md` | Implemented |
| `2026-05-17-lemon-core-runtime.md` | Implemented (Layers 1–4) |

**Execute next (plans written 2026-05-17):**

1. `docs/superpowers/plans/2026-05-17-lemon-layout-pass.md` — Taffy compute + Parley measure + `LayoutMap`
2. `docs/superpowers/plans/2026-05-17-lemon-paint-pass.md` — Vello `Scene` traversal
3. `docs/superpowers/plans/2026-05-17-lemon-platform.md` — winit/wgpu frame loop + `lemon::run()`
4. `docs/superpowers/plans/2026-05-17-lemon-runtime-semantics.md` — keyed diff, derived equality, deferred `use_effect` (last item needs platform)

## Files Most Relevant for Resuming

- `docs/superpowers/specs/2026-05-17-lemon-architecture-design.md`
- `docs/superpowers/plans/2026-05-17-lemon-core-runtime.md`
- `src/runtime/mod.rs`
- `src/runtime/cx.rs`
- `src/runtime/derived.rs`
- `src/runtime/effect.rs`
- `src/diff/mod.rs`
- `src/lib.rs`

## Recommended Resume Prompt

Use this exact prompt in the next session:

```text
Continue the `lemon` implementation from the current repository state.

Read these files first:
- docs/superpowers/handoffs/2026-05-17-lemon-core-runtime-handoff.md
- docs/superpowers/specs/2026-05-17-lemon-architecture-design.md
- docs/superpowers/plans/2026-05-17-lemon-core-runtime.md

Context:
- The plan `docs/superpowers/plans/2026-05-17-lemon-core-runtime.md` is already complete.
- The last completed commit is `f93427c` on `master`.
- Core layers 1–4 are implemented and validated.

Task:
1. Identify the next implementation slice needed to close the remaining gaps against the spec.
2. Write a new implementation plan in `docs/superpowers/plans/`.
3. Start executing that plan immediately.

Requirements:
- Follow the architecture spec closely.
- Preserve existing behavior unless you are fixing a spec mismatch.
- Run `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo check`, and `cargo test` after meaningful changes.
- Call out any spec ambiguities explicitly before making irreversible architectural choices.
```
