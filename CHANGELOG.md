# Changelog

Changes to Lemon's public API are recorded here. Releases follow [Semantic Versioning](https://semver.org/).

## Unreleased

### Application API

- `Toaster` supports `Info`, `Success`, `Warning` and `Error` notifications with distinct icons and colors; plain `show` and `show_for` remain informational.

### Reactive behavior

- Effects and derived values stop observing states that their latest evaluation no longer reads.
- Suppressed observation and effect execution restore their state after a caught panic.

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
