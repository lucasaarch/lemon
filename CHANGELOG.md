# Changelog

Changes to Lemon's public API are recorded here. Releases follow [Semantic Versioning](https://semver.org/).

## Unreleased

### Application API

- `Toaster` supports `Info`, `Success`, `Warning` and `Error` notifications with distinct icons and colors; plain `show` and `show_for` remain informational.
- `Cx::on_mount` runs after the first frame and returns cleanup that runs when the component unmounts or its window closes.

### Reactive behavior

- Effects and derived values stop observing states that their latest evaluation no longer reads.
- Suppressed observation and effect execution restore their state after a caught panic.
- Pending mount-time watchers no longer run or retain resources after their component unmounts.

### Diagnostics

- Debug builds open a native Lemon DevTools window with F12: a retained-tree inspector with layout/paint details and highlighting, component/hook metadata, and a bounded CPU profiling history. Profiler timings exclude GPU presentation waits. Inspector code and instrumentation are excluded from release builds.
- `Tree`, `TreeNode` and `TreeStyle` provide collapsible application hierarchies with custom icons; `FileTree` now uses the same tree interaction and painting implementation.
- `Tree` starts with branches expanded and reports hovered rows; DevTools keeps all branches visible, mirrors hover as a soft target highlight, and colors node icons by element kind. `View`, `RowView`, `ColumnView`, `Text`, and `Component` accept `.id(...)` labels for the inspector.
- Scrollbar thumbs support pointer dragging and track clicks.
- `Table` renders themed headers and equal-width string cells. `ResizableView` wraps a pane with explicitly selected resize edges and direction-aware native resize cursors.

- Disabled traces skip message formatting. Runtime traces include component unmounts; layout and paint traces include pass durations.
- Debug builds warn when a component changes its hook count across renders. Patch traces report text length instead of content.

## [1.0.0] - 2026-10-23

Lemon 1.0 establishes the current native desktop API as the stable baseline.

### Application API

- One `run(|cx| ...)` entry point with shared fonts, named themes and explicit windows.
- `State`, derived values, effects and isolated keyed components.
- Adaptive system light/dark themes and configurable window chrome and traffic lights.
- Rows, columns, sizing, positioning and scrolling on either or both axes.
- Text editing, selection, secure fields, multiline fields, control styles and overlays.
- Icons, file trees, gradients, opacity, Gaussian content blur and brightness.
- Pointer, keyboard, hover, focus and native window lifecycle events.

### Repository and distribution

- Four complete applications: task inbox, UTF-8 editor, reader preferences and focus timer.
- Linux, macOS and Windows CI, example interaction tests and a Rust 1.88 compatibility check.
- Explicit tagged releases, verified Cargo packages and release notes tied to the package version.
- A focused README, application guide and contribution and release instructions.

### Migration from alpha

The alpha API changed substantially. Update to `lemon = "1"`, use `run(|cx| ...)`
and `cx.open_window(...)`, and consult the application guide and examples for the
current API. The old `run_with_*` entry points, compatibility aliases and workspace
crates are not included. The old alpha versions are retired through crates.io yank;
existing lockfiles and downloads remain available.

Native runtime verification is performed on macOS. CI compilation and headless
tests on other platforms do not establish complete native input or accessibility parity.
