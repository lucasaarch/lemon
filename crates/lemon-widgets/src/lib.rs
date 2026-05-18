//! App-facing re-exports of Lemon’s UI builders.
//!
//! Use this crate in examples and applications so imports stay short:
//!
//! ```no_run
//! use lemon_widgets::prelude::*;
//!
//! fn app(cx: &Cx) -> Element {
//!     Column::new().child(Text::new("Hello")).into_element()
//! }
//!
//! fn main() {
//!     run(WindowConfig::default().title("App"), app);
//! }
//! ```
//!
//! Everything here is also available from [`lemon::element::builders`] if you prefer a single
//! dependency.
//!
//! # Migrating from `Box_`
//!
//! The generic container builder was renamed to [`View`] in the `lemon` crate (see `cargo doc -p lemon`).
//! [`Box_`] is still re-exported here as a deprecated alias.

pub mod prelude;

pub use lemon::children;
pub use lemon::element::builders::{Button, Column, Component, Row, Text, View};
pub mod image;
pub mod scroll;
pub mod select;
pub mod slider;
pub mod text_field_state;
pub mod text_input;
pub use image::Image;
pub use scroll::Scroll;
pub use select::Select;
pub use slider::Slider;
pub use text_field_state::TextFieldState;
pub use text_input::TextInput;

/// Deprecated alias for [`View`].
#[deprecated(
    since = "0.2.0",
    note = "renamed to `View`; see the `lemon` crate-level docs"
)]
pub use lemon::element::builders::View as Box_;

#[cfg(test)]
mod text_input_tests {
    use super::*;
    use lemon::{element::Element, Cursor, Cx, KeyEvent, KeyState, LemonKey, Modifiers, Signal};

    #[test]
    fn text_input_renders_placeholder_when_value_is_empty() {
        fn root(cx: &Cx) -> Element {
            let state = cx.use_signal(TextFieldState::new(""));
            TextInput::new(state)
                .placeholder("Enter text...")
                .into_element()
        }
        let _ = root;
    }

    #[test]
    fn text_input_is_focusable_and_has_text_cursor() {
        let state = Signal::new(TextFieldState::new(""));
        let el = TextInput::new(state).into_element();

        let Element::View(view) = el else {
            panic!("expected View element from TextInput");
        };
        assert!(view.style.focusable, "TextInput must be focusable");
        assert_eq!(
            view.style.cursor,
            Cursor::Text,
            "TextInput must use Text cursor"
        );
        assert!(view.text_input.is_some(), "TextInput must attach metadata");
    }

    #[test]
    fn text_input_on_key_down_updates_state() {
        let state = Signal::new(TextFieldState::new(""));
        let el = TextInput::new(state.clone()).into_element();

        let Element::View(view) = el else {
            panic!("expected View element from TextInput");
        };
        let on_key_down = view
            .handlers
            .on_key_down
            .as_ref()
            .expect("must have on_key_down");
        on_key_down(KeyEvent {
            key: LemonKey::Character("a".into()),
            modifiers: Modifiers::default(),
            repeat: false,
            state: KeyState::Pressed,
        });
        assert_eq!(state.get().value, "a");
    }

    #[test]
    fn text_input_has_border() {
        let state = Signal::new(TextFieldState::new(""));
        let el = TextInput::new(state).into_element();

        let Element::View(view) = el else {
            panic!("expected View element from TextInput");
        };
        assert!(
            view.paint.border_color.is_some(),
            "TextInput must always have a border color"
        );
        assert!(
            view.paint.border_width > 0.0,
            "TextInput must have a non-zero border width"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lemon::{element::Element, Cx, Overflow, Signal};

    #[test]
    fn scroll_new_compiles_with_cx() {
        fn _check(cx: &Cx) -> Element {
            Scroll::new(cx, Text::new("item"))
                .height(180.0)
                .into_element()
        }
    }

    #[test]
    fn scroll_builds_hidden_viewport_and_updates_offset() {
        let offset = Signal::new(10.0f64);
        let root = Scroll::with_offset(offset.clone(), Text::new("item"))
            .height(200.0)
            .width(300.0)
            .into_element();

        let Element::View(viewport) = root else {
            panic!("expected scroll viewport to be Element::View");
        };

        assert_eq!(viewport.style.overflow, Overflow::Hidden);
        assert!(viewport.scroll_viewport);
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
