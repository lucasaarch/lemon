# AGENTS.md

Operational policy for agents working on Lemon, a reactive native desktop UI toolkit in Rust.

## Authority and scope

- Follow this file over other repository guidance. Use current source and tests to verify implementation details; prose documentation may lag behind them.
- Ignore `CONTEXT.md` and everything under `docs/superpowers/` for repository guidance.
- Preserve unrelated working tree changes. Keep changes focused on the requested task.
- Prefer existing patterns, explicit structs and field propagation, fluent builders, and focused helpers. Introduce abstractions only when the task needs them.

## Workspace and ownership

Run Cargo and Make commands from the workspace root.

| Path | Owns |
|------|------|
| `crates/lemon/src/lib.rs`, `prelude.rs` | Public facade and application imports |
| `crates/lemon/examples/` | Runnable application examples |
| `crates/lemon/tests/examples/` | Example tests, included by the corresponding example modules |
| `crates/lemon-reactive/src/` | Signals, derived values, effects, observer tracking |
| `crates/lemon-core/src/runtime/` | Component rendering, hook state, tree freezing |
| `crates/lemon-core/src/element/` | Element model, primitive builders, styles, events |
| `crates/lemon-core/src/diff/` | Patches, property comparisons, child reconciliation |
| `crates/lemon-core/src/retained/` | Retained nodes, patch application, focus traversal |
| `crates/lemon-core/src/layout/`, `paint/` | Geometry/text layout and Vello scene generation |
| `crates/lemon-core/src/ui.rs` | Headless update, layout, and scene orchestration (`UiState`) |
| `crates/lemon-core/src/host.rs`, `host_requests.rs` | Host context, scoped theme, window request queue/types |
| `crates/lemon-ui/src/widget/` | Composed widgets and their exports in `mod.rs` |
| `crates/lemon-desktop/src/platform/` | Native windows, input dispatch, hit testing, frame scheduling, presentation |
| `crates/lemon-core/tests/` | Headless pipeline, reconciliation, and invalidation integration tests |
| `Makefile`, `.github/workflows/ci.yml` | Local quality gates and CI |

Internal dependencies flow from `lemon` to `lemon-ui` and `lemon-desktop`, from both to `lemon-core`, and from core to `lemon-reactive`. Reactive has no dependencies. Do not introduce dependencies from core to UI or desktop, including in tests.

## Architecture

User views build an `Element` tree. `Runtime` renders components and freezes dynamic values; diffing emits patches; `RetainedTree` applies them; layout computes geometry and text layout; paint builds a Vello scene. The desktop host supplies input and presentation around this pipeline. `UiState` runs UI processing without a native window or GPU surface.

- Identify the failing stage before editing. For reactive primitives start in `lemon-reactive`; for hook/render behavior start in core runtime; for reconciliation start in diff/retained; for geometry start in layout; for scene output start in paint; for native input start in desktop platform/hit testing and the affected widget.
- Keep windowing and native input dispatch in desktop. Keep widget interaction in the owning widget and core event representation. Do not couple core rendering to desktop backends.
- Apply UI updates through the runtime/diff/retained pipeline; do not bypass it by mutating unrelated layers.
- Preserve host ownership: window requests and captured themes belong to their originating `HostContext`. Do not replace that ownership with unrelated active global state.

## Components, hooks, and identity

- Preserve fluent builders: `Column::new()`, `Row::new()`, `View::new()`, `Text::new(...)`.
- The app-facing `Button` exported by `lemon`/its prelude is the composed widget: `Button::new(cx, label)`. Core also has a lower-level `element::builders::Button::new(label)`; use the appropriate layer.
- `Component::new` takes `fn(&Cx) -> Element`. Capturing closures do not fit this API.
- `Component::new_with_props` takes a typed function pointer and `Clone + PartialEq + 'static` props. Use it for components that need per-instance data and isolated hook state. Use keyed primitive rows when isolated component state is unnecessary.
- Call hooks in a stable order on every render. Widget constructors such as `Button::new` consume hook slots too: do not put them in a changing loop or conditional on a shared `Cx`. Use component scopes or primitives there.
- Component identity depends on the view function and optional key. Do not change this model incidentally.
- Keys must be stable and unique among siblings. Use item IDs for reorderable/removable lists. Preserve keyed and unkeyed reconciliation behavior; update diff tests when changing it.

## Propagation and invalidation

When adding or changing element/style/widget fields, inspect every applicable path:

1. Types, defaults, constructors, and builder methods.
2. `freeze_element` / `freeze_box` and paint freezing in `crates/lemon-core/src/runtime/mod.rs`.
3. Diff comparisons and patch payloads in `crates/lemon-core/src/diff/`.
4. Retained construction and patch application.
5. Layout, paint, input dispatch, and manual struct literals in tests.

Preserve dirty flag semantics. Determine whether a change needs layout, paint, or both. Accumulate invalidation across update batches and preserve it on failure. Do not eagerly rerun layout or paint where existing dirty flags control the work.

For input changes, verify the complete path: element/widget API → retained handlers/metadata → dispatch or hit testing → state updates and invalidation → tests. Account for focus, hover, and widget state where relevant.

## Public APIs and documentation

- Document every introduced or materially changed public API with `///` at its definition, including APIs in the owning crates, builders, hooks, event/style types, and configuration.
- Describe behavior, parameters/return values, relevant defaults, errors/panics, and non-obvious constraints. Include a compilable example for non-trivial APIs; use `no_run` when a native event loop is required.
- README changes do not replace Rust API documentation. Use `#[doc(inline)]` on reexports only when the underlying documentation is complete.
- Check exports in the owning crate, `crates/lemon-ui/src/widget/mod.rs` for widgets, and the facade `crates/lemon/src/lib.rs` / `prelude.rs` as appropriate. Keep lower-level APIs out of the prelude unless the change calls for them.
- Update an existing example when user-visible behavior changes. Keep examples concise and focused; avoid adding another demo for an already covered capability.

## Verification and completion

Before editing, read the owning implementation and nearby tests. For behavior changes, add or update regression tests unless the layer has no meaningful unit-test surface. Prefer headless tests for UI processing; they do not prove native window/input/presentation behavior.

Run focused checks first, for example:

```bash
cargo test -p lemon-reactive
cargo test -p lemon-core
cargo test -p lemon-ui
cargo test -p lemon --examples
cargo build --example form
```

`cargo test` does not run the tests included by example modules. Run `cargo test -p lemon --examples` when changing those examples or their tests.

Before completing non-trivial work, run `make check`, matching CI. It runs:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --examples
```

For public API changes, also run `cargo doc --no-deps` when practical and resolve documentation warnings. Compilation of all examples is required after app-facing changes.

Report what changed, what was verified, and any failed or unavailable checks. Distinguish build/test evidence from visual or native runtime verification. Do not claim completion while required checks fail without explaining the blocker.
