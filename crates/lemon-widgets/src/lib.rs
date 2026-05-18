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
pub mod scroll;
pub use scroll::Scroll;

/// Deprecated alias for [`View`].
#[deprecated(
    since = "0.2.0",
    note = "renamed to `View`; see the `lemon` crate-level docs"
)]
pub use lemon::element::builders::View as Box_;

#[cfg(test)]
mod tests {
    use super::*;
    use lemon::{element::Element, Cx, Overflow, Signal};

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

    #[test]
    fn scroll_new_compiles_with_signal_from_cx() {
        fn _check(cx: &Cx) -> Element {
            let offset = cx.use_signal(0.0f64);
            Scroll::new(Element::None, offset)
                .height(180.0)
                .into_element()
        }
    }

    #[test]
    fn scroll_builds_hidden_viewport_and_updates_offset() {
        let offset = Signal::new(10.0f64);
        let root = Scroll::new(Text::new("item"), offset.clone())
            .height(200.0)
            .width(300.0)
            .into_element();

        let Element::View(viewport) = root else {
            panic!("expected scroll viewport to be Element::View");
        };

        assert_eq!(viewport.style.overflow, Overflow::Hidden);
        assert!(viewport.handlers.on_scroll.is_some());
        assert_eq!(viewport.children.len(), 1);

        let Element::View(inner) = &viewport.children[0] else {
            panic!("expected inner scroll content wrapper to be Element::View");
        };
        assert_eq!(inner.style.margin.as_ref().map(|m| m.top), Some(-10.0));

        let on_scroll = viewport.handlers.on_scroll.as_ref().unwrap();
        on_scroll(-20.0);
        assert_eq!(offset.get(), 30.0);
        on_scroll(1000.0);
        assert_eq!(offset.get(), 0.0);
    }
}
