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
pub mod text_field_state;
pub mod text_input;
pub use scroll::Scroll;
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
    use lemon::{element::Element, Cx, Signal};

    #[test]
    fn text_input_renders_placeholder_when_value_is_empty() {
        fn root(cx: &Cx) -> Element {
            let state = cx.use_signal(TextFieldState::new(""));
            let focused = cx.use_signal(false);
            TextInput::new(state, focused)
                .placeholder("Enter text...")
                .into_element()
        }
        // Compile-only check: builder API is correct and returns Element.
        let _ = root;
    }

    #[test]
    fn text_input_with_value_compiles() {
        fn root(cx: &Cx) -> Element {
            let state = cx.use_signal(TextFieldState::new("hello"));
            let focused = cx.use_signal(false);
            TextInput::new(state, focused).into_element()
        }
        let _ = root;
    }

    #[test]
    fn text_input_is_focusable_and_has_text_cursor() {
        let state = Signal::new(TextFieldState::new(""));
        let focused = Signal::new(false);
        let el = TextInput::new(state, focused).into_element();

        let Element::View(view) = el else {
            panic!("expected View element from TextInput");
        };
        assert!(view.style.focusable, "TextInput must be focusable");
        assert_eq!(
            view.style.cursor,
            lemon::Cursor::Text,
            "TextInput must use Text cursor"
        );
    }

    #[test]
    fn text_input_on_click_sets_focused() {
        let state = Signal::new(TextFieldState::new(""));
        let focused = Signal::new(false);
        let el = TextInput::new(state, focused.clone()).into_element();

        let Element::View(view) = el else {
            panic!("expected View element from TextInput");
        };
        let on_click = view.handlers.on_click.as_ref().expect("must have on_click");
        on_click();
        assert!(focused.get(), "on_click must set focused to true");
    }

    #[test]
    fn text_input_on_key_down_updates_state() {
        use lemon::{KeyEvent, KeyState, LemonKey, Modifiers};

        let state = Signal::new(TextFieldState::new(""));
        let focused = Signal::new(true);
        let el = TextInput::new(state.clone(), focused).into_element();

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
        let focused = Signal::new(false);
        let el = TextInput::new(state, focused).into_element();

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

    #[test]
    fn text_input_width_builder_sets_width() {
        use lemon::element::style::Dimension;

        let state = Signal::new(TextFieldState::new(""));
        let focused = Signal::new(false);
        let el = TextInput::new(state, focused).width(200.0).into_element();

        let Element::View(view) = el else {
            panic!("expected View element from TextInput");
        };
        assert_eq!(view.style.width, Some(Dimension::Points(200.0)));
    }
}

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
