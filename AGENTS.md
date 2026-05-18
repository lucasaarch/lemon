# AGENTS.md

This document is the operational policy for AI agents working in this repository.

It is intended to be stable, strict, and difficult to misinterpret.

If another local document conflicts with this file, follow this file.

## Authority

Agents working in this repository MUST treat the following as authoritative, in this order:

1. `AGENTS.md`
2. the current source tree
3. the current test suite

Agents MUST ignore `CONTEXT.md` and everything under `docs/superpowers/` for repository-specific guidance. Pretend those files do not exist.

## Mission

The mission is to improve Lemon without degrading its architecture.

Lemon is a reactive native desktop UI toolkit in Rust. The repository is a Cargo workspace containing:

- `crates/lemon`: core engine
- `crates/lemon-widgets`: app-facing widget re-exports
- `examples/counter`
- `examples/card`
- `examples/list_keyed`

Primary technologies:

- `winit`
- `wgpu`
- `vello`
- `taffy`
- `parley`

## Non-Negotiable Policies

These are hard rules.

### 1. Preserve layer boundaries

Agents MUST preserve the current architecture:

1. `runtime`
2. `element`
3. `diff`
4. `retained`
5. `layout`
6. `paint`
7. `platform`

Agents MUST place logic in the layer that owns it.

Agents MUST NOT:

- put windowing or input semantics into `runtime`
- bypass diffing by mutating unrelated layers directly
- couple `paint` logic to `platform`
- couple `element` APIs to backend details unless the codebase already does so

### 2. Prefer minimal, local changes

Agents MUST make the smallest coherent change that solves the problem.

Agents MUST NOT introduce large abstractions unless the existing code already demands one.

This repository prefers:

- explicit structs
- explicit field propagation
- fluent builders
- focused helper functions
- tests close to the module they verify

### 3. Keep public APIs deliberate

The public surface is defined primarily by:

- `crates/lemon/src/lib.rs`
- `crates/lemon-widgets/src/lib.rs`

If an agent changes app-facing APIs, it MUST also verify:

- exports remain correct
- examples still compile
- tests still pass

### 4. Behavior changes require verification

If behavior changes, agents MUST add or update tests unless the affected layer has no meaningful unit-test surface.

At minimum, agents MUST run the smallest relevant test or build command before claiming success.

### 5. Final quality gates are mandatory

Before considering non-trivial work complete, agents MUST run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If one of these cannot be run, the agent MUST explicitly say so.

## Repository Map

Agents MUST use the real codebase, not assumptions.

High-value files:

- `crates/lemon/src/lib.rs`: public exports
- `crates/lemon/src/runtime/`: reactive state and rendering loop
- `crates/lemon/src/element/`: virtual element model, builders, styles, events
- `crates/lemon/src/diff/mod.rs`: patch generation
- `crates/lemon/src/retained/mod.rs`: retained tree and patch application
- `crates/lemon/src/retained/focus.rs`: focus traversal
- `crates/lemon/src/layout/mod.rs`: layout pass and text measurement
- `crates/lemon/src/paint/mod.rs`: Vello scene generation
- `crates/lemon/src/platform/mod.rs`: app shell, input dispatch, redraw loop
- `crates/lemon/src/platform/hit_test.rs`: click and hover hit testing
- `examples/`: canonical consumer code
- `.github/workflows/ci.yml`: CI quality gates

## Architecture Contract

Agents SHOULD think of Lemon as this pipeline:

1. user code builds an `Element` tree
2. `Runtime` renders and re-renders using signals/effects
3. `diff` emits `Patch` values
4. `RetainedTree` applies patches
5. `layout` computes rectangles and text layout
6. `paint` generates the Vello scene
7. `platform` handles input and presentation

When debugging, agents MUST identify which stage is failing before editing code.

## API Contract

### Builders

The canonical builder style is:

- `Column::new()`
- `Row::new()`
- `View::new()` (formerly `Box_`; see crate docs)
- `Text::new(...)`
- `Button::new(...)`
- `Component::new(...)`

Container builders are backed by `BoxElement`. They own:

- layout style
- paint style
- child elements
- keys
- event handlers

Agents MUST preserve the fluent builder style.

### Components

This is important:

`Component::new(...)` currently accepts a function pointer API, not an arbitrary capturing closure API.

Agents MUST assume the following:

- `Component::new(some_fn)` is valid
- `Component::new(move |_cx| ...)` is only valid if it does not capture state
- captured-state rows in repeated lists should usually be built with keyed elements, not captured components

Agents MUST NOT silently refactor the component identity model unless the task explicitly requires it.

### Keys

Keyed diffing is already part of the architecture.

Agents MUST:

- use stable keys for stable identity
- preserve keyed vs unkeyed behavior
- add or update tests in `crates/lemon/src/diff/mod.rs` when keyed reconciliation changes

### Events and input

Current event model lives in:

- `crates/lemon/src/element/events.rs`
- `crates/lemon/src/retained/mod.rs`
- `crates/lemon/src/platform/hit_test.rs`
- `crates/lemon/src/platform/mod.rs`

Important concepts:

- `KeyEvent`
- `LemonKey`
- `NamedKey`
- `Modifiers`
- `KeyState`
- `Cursor`
- retained `EventHandlers`
- `StyleProps.focusable`

If an agent adds a new input capability, it MUST verify the full chain:

1. element-level API
2. retained representation
3. dispatch or hit testing
4. dirty flag effects
5. tests

## Explicit Implementation Rules

### Rule: propagate new fields completely

This codebase has several manual propagation paths.

If an agent adds a field to `StyleProps`, `BoxElement`, `ButtonElement`, or related public data structures, it MUST inspect all of:

- default constructors
- builder methods
- `freeze_element` / `freeze_box` in `runtime/mod.rs`
- retained tree construction
- tests with manual struct literals

Missing one propagation site is a common source of regressions.

### Rule: protect dirty-flag semantics

Layout and paint are intentionally dirty-flag driven.

Agents MUST NOT eagerly recompute large subsystems when a dirty flag is the established mechanism.

If a change affects:

- text content
- style
- input handlers
- focus
- hover

then the agent MUST confirm whether `layout_dirty`, `paint_dirty`, or both should change.

### Rule: verify examples after app-facing changes

If an agent changes builders, exports, keys, runtime behavior, or public events, it MUST verify the examples that exercise those surfaces.

Relevant commands:

```bash
cargo build -p counter
cargo build -p card
cargo build -p list_keyed
```

### Rule: prefer extending existing patterns

Agents SHOULD follow patterns already present in the repository instead of inventing new styles.

Examples:

- tests near the module under test
- builder methods in `element/builders.rs`
- exports in `lib.rs`
- app-facing re-exports in `lemon-widgets`

## Known Pitfalls

Agents MUST be aware of these repository-specific traps:

1. Adding fields to `StyleProps` or `BoxElement` often breaks manual initializers in tests.
2. Adding builder capabilities without exposing them through `lemon-widgets` leaves examples behind.
3. Input changes often require edits in more than one place: builders, retained handlers, hit testing, and platform dispatch.
4. `clippy -D warnings` is strict. Structural lint failures count as broken work.
5. `README.md` may lag behind the actual code. Source and tests win.
6. The component system has identity semantics tied to function identity and keys. Do not casually rewrite that model.

## Playbook

This section is procedural. Agents SHOULD follow it in order.

### Playbook A: when implementing a feature

1. Identify the owning layer.
2. Read the nearest existing implementation in that layer.
3. Find tests in that module before editing.
4. Make the smallest coherent code change.
5. Add or update tests.
6. Run the smallest relevant command.
7. Run full workspace gates.

### Playbook B: when debugging a bug

Classify the failure first.

If it is:

- state/update bug: start in `runtime/`
- reconciliation bug: start in `diff/mod.rs`
- stale retained state bug: start in `retained/mod.rs`
- click/hover/focus/keyboard bug: start in `platform/mod.rs` and `platform/hit_test.rs`
- visual geometry bug: start in `layout/mod.rs`
- paint-only bug: start in `paint/mod.rs`

Agents MUST NOT start patching random downstream files before locating the failing stage.

### Playbook C: when adding a new app-facing capability

1. implement the core data or behavior in `crates/lemon`
2. expose the builder or type from the appropriate module
3. re-export from `crates/lemon/src/lib.rs` if public
4. re-export from `crates/lemon-widgets` if end-user-facing
5. update or add an example if the feature is visible to users
6. run builds for affected examples

### Playbook D: when reviewing a proposed change

Review in this order:

1. architecture violation risk
2. missing propagation risk
3. dirty-flag correctness
4. keyed identity correctness
5. public API breakage
6. missing tests
7. formatting/lint/test status

## Completion Standard

An agent SHOULD only consider work complete when all of the following are true:

- the change is in the correct architectural layer
- tests covering the changed behavior exist or were updated
- examples still build if the public surface changed
- `cargo fmt --all -- --check` passes
- `cargo clippy --workspace --all-targets -- -D warnings` passes
- `cargo test --workspace` passes

## Short Version

If an agent reads nothing else, it MUST remember this:

- obey this file over local prose docs
- preserve the runtime → diff → retained → layout → paint → platform pipeline
- keep changes small and local
- propagate new fields everywhere
- do not misuse `Component::new(...)` as if it accepted arbitrary captured closures
- verify examples after public API changes
- never skip `fmt`, `clippy`, or workspace tests
