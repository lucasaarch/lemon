//! App-facing re-exports of Lemon’s UI builders.
//!
//! Use this crate in examples and applications so imports stay short:
//!
//! ```no_run
//! use lemon::{run, Cx, WindowConfig};
//! use lemon_widgets::{Button, Column, Text};
//! ```
//!
//! Everything here is also available from [`lemon::element::builders`] if you prefer a single
//! dependency.
//!
//! # Migrating from `Box_`
//!
//! The generic container builder was renamed to [`View`] in the `lemon` crate (see `cargo doc -p lemon`).
//! [`Box_`] is still re-exported here as a deprecated alias.

pub use lemon::element::builders::{Button, Column, Component, Row, Text, View};

/// Deprecated alias for [`View`].
#[deprecated(
    since = "0.2.0",
    note = "renamed to `View`; see the `lemon` crate-level docs"
)]
pub use lemon::element::builders::View as Box_;

#[cfg(test)]
mod tests {
    use super::*;
    use lemon::Cx;

    #[test]
    fn all_builders_are_accessible_from_lemon_widgets() {
        fn _check(_cx: &Cx) -> lemon::element::Element {
            Column::new()
                .child(Row::new().child(Text::new("hello")))
                .child(Button::new("ok"))
                .child(View::new().child(Text::new("view")))
                .into_element()
        }
    }
}
