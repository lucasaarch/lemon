use crate::{element::Element, Color, Cx, State};

use super::{TextField, TextFieldStyle};

/// Single-line field that displays one bullet per Unicode scalar value.
///
/// Uses `State<String>` and the same theme, focus, and style overrides as [`TextField`].
/// The element tree, retained metadata, and caret use masked text; `state` keeps
/// the original value for application code. This masks the display; it does not
/// encrypt or zeroize the string stored in the application state.
///
/// Ctrl/Cmd+C and Ctrl/Cmd+X are ignored to avoid exporting the secret. Paste,
/// select-all, navigation, and deletion use the regular text editing behavior.
///
/// # Examples
///
/// ```no_run
/// use lemon::prelude::*;
///
/// fn login(cx: &Cx) -> Element {
///     let password = cx.state("".to_owned());
///     SecureField::new(cx, password)
///         .placeholder("Password")
///         .style(TextFieldStyle::Bordered)
///         .rounding(8.0)
///         .width(240.0)
///         .into_element()
/// }
/// ```
pub struct SecureField(TextField);

impl SecureField {
    /// Creates an enabled secure field backed by `state`, with theme defaults.
    ///
    /// No placeholder or fixed width is set. Building the element subscribes the
    /// parent to state changes. This constructor consumes hook slots and must be
    /// called in a stable order on every render.
    pub fn new(cx: &Cx, state: State<String>) -> Self {
        Self(TextField::new_secure(cx, state))
    }

    /// Sets the unmasked placeholder shown only while the value is empty.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.0 = self.0.placeholder(text);
        self
    }

    /// Sets a fixed width in logical points; defaults to automatic sizing.
    pub fn width(mut self, value: f32) -> Self {
        self.0 = self.0.width(value);
        self
    }

    /// Selects the same theme-aware preset as a plain field; explicit overrides are preserved.
    pub fn style(mut self, style: TextFieldStyle) -> Self {
        self.0 = self.0.style(style);
        self
    }

    /// Overrides the placeholder text color.
    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.0 = self.0.placeholder_color(color);
        self
    }

    /// Overrides the border color.
    pub fn border_color(mut self, color: Color) -> Self {
        self.0 = self.0.border_color(color);
        self
    }

    /// Overrides the container padding in logical points.
    pub fn inset(mut self, value: f32) -> Self {
        self.0 = self.0.inset(value);
        self
    }

    /// Overrides the corner radius in logical points.
    pub fn rounding(mut self, value: f32) -> Self {
        self.0 = self.0.rounding(value);
        self
    }

    /// Overrides the focus ring color.
    pub fn focus_ring_color(mut self, color: Color) -> Self {
        self.0 = self.0.focus_ring_color(color);
        self
    }

    /// Prevents focus and keyboard input when `disabled` is true; text stays masked.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.0 = self.0.disabled(disabled);
        self
    }

    /// Builds the masked element, using display byte offsets for its caret metadata.
    pub fn into_element(self) -> Element {
        self.0.into_element()
    }
}

impl From<SecureField> for Element {
    fn from(field: SecureField) -> Self {
        field.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::Effect;
    use crate::{
        element::types::BoxElement, ui::UiState, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey,
        Viewport,
    };
    use std::{cell::Cell, rc::Rc};

    fn view(element: Element) -> BoxElement {
        let Element::View(view) = element else {
            panic!("expected field container")
        };
        view
    }

    fn displayed(view: &BoxElement) -> String {
        let Element::Row(row) = &view.children[0] else {
            panic!("expected text wrapper")
        };
        let Element::Text(text) = &row.children[0] else {
            panic!("expected display text")
        };
        text.content.resolve()
    }

    fn key(key: LemonKey) -> KeyEvent {
        KeyEvent {
            key,
            modifiers: Modifiers::default(),
            repeat: false,
            state: KeyState::Pressed,
        }
    }

    #[test]
    fn unicode_secrets_only_reach_retained_layout_and_paint_as_bullets() {
        let state = State::new("".to_owned());
        let source = state.clone();
        let mut ui = UiState::default();
        ui.mount(move |cx| {
            SecureField::new(cx, source.clone())
                .width(240.0)
                .into_element()
        })
        .unwrap();
        let viewport = Viewport {
            width: 300.0,
            height: 100.0,
        };
        let focused = ui
            .retained
            .as_ref()
            .unwrap()
            .root
            .as_ref()
            .unwrap()
            .taffy_id;
        let initial = view(ui.runtime.root_element().unwrap());
        initial.handlers.on_key_down.as_ref().unwrap()(key(LemonKey::Character("aé🔒".into())));
        ui.update(viewport, 1.0, focused, true).unwrap();

        assert_eq!(state.get(), "aé🔒");
        let root = ui.retained.as_ref().unwrap().root.as_ref().unwrap();
        let meta = root.text_input.as_ref().unwrap();
        assert_eq!(meta.value, "•••");
        assert_eq!(meta.cursor, "•••".len());
        let text = root.children[0].children[0].text.as_ref().unwrap();
        assert_eq!(text.content, "•••");
        assert_eq!(text.caret, meta.cursor);
        assert!(text.parley_layout.is_some());
        assert!(!ui.layout_dirty);
        assert!(!ui.paint_dirty);

        // Moving inside a Unicode secret changes only caret metadata, not mask text.
        let current = view(ui.runtime.root_element().unwrap());
        current.handlers.on_key_down.as_ref().unwrap()(key(LemonKey::Named(NamedKey::ArrowLeft)));
        ui.apply_updates().unwrap();
        assert!(!ui.layout_dirty);
        assert!(ui.paint_dirty);
        let root = ui.retained.as_ref().unwrap().root.as_ref().unwrap();
        assert_eq!(root.text_input.as_ref().unwrap().cursor, "••".len());
        assert_eq!(
            root.children[0].children[0].text.as_ref().unwrap().caret,
            "••".len()
        );
        ui.update(viewport, 1.0, focused, true).unwrap();

        current.handlers.on_key_down.as_ref().unwrap()(key(LemonKey::Named(NamedKey::Backspace)));
        ui.update(viewport, 1.0, focused, true).unwrap();
        assert_eq!(state.get(), "a🔒");
        let root = ui.retained.as_ref().unwrap().root.as_ref().unwrap();
        assert_eq!(root.text_input.as_ref().unwrap().value, "••");
        assert_eq!(root.text_input.as_ref().unwrap().cursor, "•".len());
    }

    #[test]
    fn mask_cursor_uses_display_byte_offsets_for_every_character_boundary() {
        for (steps, mask_cursor) in [(0, 0), (1, 3), (2, 6), (3, 9)] {
            let cx = Cx::new();
            let state = State::new("aé🔒".to_owned());
            let initial = view(SecureField::new(&cx, state.clone()).into_element());
            for _ in 0..steps {
                initial.handlers.on_key_down.as_ref().unwrap()(key(LemonKey::Named(
                    NamedKey::ArrowRight,
                )));
            }
            cx.reset_hooks();
            let field = view(SecureField::new(&cx, state).into_element());
            assert_eq!(displayed(&field), "•••");
            assert_eq!(field.text_input.as_ref().unwrap().cursor, mask_cursor);
        }
    }

    #[test]
    fn empty_placeholder_and_disabled_mask_share_plain_field_styles() {
        let cx = Cx::new();
        let style = TextFieldStyle::Bordered;
        let plain = view(
            TextField::new(&cx, State::new("".to_owned()))
                .placeholder("Password")
                .style(style)
                .placeholder_color(Color::rgb8(1, 2, 3))
                .border_color(Color::rgb8(4, 5, 6))
                .focus_ring_color(Color::rgb8(7, 8, 9))
                .inset(10.0)
                .rounding(8.0)
                .width(220.0)
                .into_element(),
        );
        let secure = view(
            SecureField::new(&cx, State::new("".to_owned()))
                .placeholder("Password")
                .style(style)
                .placeholder_color(Color::rgb8(1, 2, 3))
                .border_color(Color::rgb8(4, 5, 6))
                .focus_ring_color(Color::rgb8(7, 8, 9))
                .inset(10.0)
                .rounding(8.0)
                .width(220.0)
                .into_element(),
        );
        assert_eq!(displayed(&secure), "Password");
        assert_eq!(secure.text_input.as_ref().unwrap().value, "");
        assert_eq!(secure.text_input.as_ref().unwrap().cursor, 0);
        assert_eq!(secure.style, plain.style);
        assert_eq!(secure.paint.resolve(), plain.paint.resolve());

        let disabled = view(
            SecureField::new(&cx, State::new("secret".to_owned()))
                .disabled(true)
                .into_element(),
        );
        assert_eq!(displayed(&disabled), "••••••");
        assert!(!disabled.style.focusable);
        assert!(disabled.text_input.is_none());
        assert!(disabled.handlers.on_key_down.is_none());
        assert!(disabled.handlers.on_pointer_down.is_none());
    }

    #[test]
    fn select_all_deletion_and_typing_keep_secret_state_separate_from_display() {
        let state = State::new("old password".to_owned());
        let cx = Cx::new();
        let field = view(SecureField::new(&cx, state.clone()).into_element());
        let handler = field.handlers.on_key_down.as_ref().unwrap();
        let mut select_all = key(LemonKey::Character("a".into()));
        select_all.modifiers.meta = true;
        handler(select_all);
        handler(key(LemonKey::Named(NamedKey::Backspace)));
        assert_eq!(state.get(), "");
        handler(key(LemonKey::Character("new🔑".into())));
        assert_eq!(state.get(), "new🔑");
        cx.reset_hooks();
        let field = view(SecureField::new(&cx, state).into_element());
        assert_eq!(displayed(&field), "••••");
        assert_eq!(field.text_input.as_ref().unwrap().cursor, "••••".len());
    }

    #[test]
    fn clipboard_export_shortcuts_never_enter_the_editor() {
        let state = State::new("secret".to_owned());
        let read = state.clone();
        let updates = Rc::new(Cell::new(0));
        let count = updates.clone();
        let _effect = Effect::new(move || {
            read.get();
            count.set(count.get() + 1);
        });
        let field = view(SecureField::new(&Cx::new(), state.clone()).into_element());
        for modifiers in [
            Modifiers {
                ctrl: true,
                ..Default::default()
            },
            Modifiers {
                meta: true,
                ..Default::default()
            },
        ] {
            for shortcut in ["c", "C", "x", "X"] {
                let mut event = key(LemonKey::Character(shortcut.into()));
                event.modifiers = modifiers.clone();
                field.handlers.on_key_down.as_ref().unwrap()(event);
            }
        }
        assert_eq!(
            updates.get(),
            1,
            "blocked exports must bypass state/clipboard handling"
        );
        assert_eq!(state.get(), "secret");
        // Navigation updates only the internal caret, not the app's string.
        field.handlers.on_key_down.as_ref().unwrap()(key(LemonKey::Named(NamedKey::End)));
        assert_eq!(updates.get(), 1);
        field.handlers.on_key_down.as_ref().unwrap()(key(LemonKey::Character("!".into())));
        assert_eq!(updates.get(), 2);
    }
}
