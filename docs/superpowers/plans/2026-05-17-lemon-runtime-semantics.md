# Lemon Runtime Semantics Gaps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close documented gaps between the architecture spec and the pure runtime (Layers 1–4) that were intentionally deferred from the core-runtime and component-lifecycle plans.

**Architecture:** Three independent slices, each testable with `cargo test` without GPU. Can land as separate commits inside one PR or three small PRs.

**Tech Stack:** Rust 2021; existing `diff`, `runtime`, `signal`, `derived`, `effect`.

**Spec reference:** `docs/superpowers/specs/2026-05-17-lemon-architecture-design.md`

**Depends on:** `2026-05-17-lemon-core-runtime.md`, `2026-05-17-lemon-component-lifecycle.md`

**Note:** `use_effect` deferral until after paint **requires** `2026-05-17-lemon-platform.md` for the effect queue; implement Task 3 only after platform exists.

---

## Scope Check

| Slice | Spec intent | When |
|-------|-------------|------|
| Keyed child diff | `MoveChild` patches; preserve state on reorder | Anytime (pure diff) |
| `Derived<T>` equality | Notify subscribers only when computed value changes | Anytime (pure runtime) |
| Deferred `use_effect` | Run mount effects after first paint, not during render | After platform plan |

---

## File Structure

```text
src/
  diff/mod.rs           ← keyed diff for children with Key
  runtime/derived.rs    ← equality-aware invalidation
  runtime/cx.rs         ← deferred effect queue hook
  runtime/mod.rs        ← flush mount effects after paint (platform calls)
  platform/mod.rs       ← call runtime.flush_deferred_effects() post-present
```

---

### Task 1: Keyed Child Diffing

**Files:**
- Modify: `src/diff/mod.rs`
- Test: `src/diff/mod.rs`

- [ ] **Step 1: Write failing tests**

Cases:
- reorder keyed children → `MoveChild` only, no spurious remove/insert
- insert keyed child → `InsertChild` at correct index
- key change → remove + insert (or unmount/mount for components)

- [ ] **Step 2: Implement key map diff in `diff_children`**

Use `Option<Key>` on `BoxElement` children; fall back to index diff when keys absent.

- [ ] **Step 3: Run tests + commit**

```bash
git commit -m "feat(diff): add keyed child diffing with MoveChild patches"
```

---

### Task 2: Derived Equality-Aware Propagation

**Files:**
- Modify: `src/runtime/derived.rs`
- Test: `src/runtime/derived.rs`

- [ ] **Step 1: Write failing test — signal changes but derived value identical → downstream effect not re-run**

Example: `use_memo(|| s.get() / 10 * 10)` with `s.set(11)` then `s.set(21)` both map to same bucket.

- [ ] **Step 2: Store last computed `T` in `DerivedInner`; compare with `PartialEq` before `mark_dirty`**

- [ ] **Step 3: Run tests + commit**

---

### Task 3: Deferred use_effect (post-platform)

**Files:**
- Modify: `src/runtime/cx.rs`, `src/runtime/effect.rs`, `src/runtime/mod.rs`, `src/platform/mod.rs`
- Test: `src/runtime/cx.rs`, `src/platform/mod.rs` (integration)

- [ ] **Step 1: Write failing test — `use_effect` on mount does not run until `flush_deferred_effects`**

- [ ] **Step 2: Queue first-run effects instead of `Effect::new` immediate for `use_effect` hooks only**

- [ ] **Step 3: Platform calls `flush_deferred_effects` after first paint**

- [ ] **Step 4: Verify spec counter example still works**

- [ ] **Step 5: Commit**

---

## Self-Review

**Spec coverage:** Keyed diff, derived equality, deferred effects (last item blocked on platform).

**Not in scope:** prop-capturing components, `Fragment` in retained tree, image paint.
