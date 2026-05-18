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

pub use lemon::element::builders::{Box_, Button, Column, Component, Row, Text};

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
                .child(Box_::new().child(Text::new("box")))
                .into_element()
        }
    }
}
