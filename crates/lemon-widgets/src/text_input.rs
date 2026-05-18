use lemon::{
    element::{
        builders::{Row, Text, View},
        events::Cursor,
        style::Color,
        Element,
    },
    Signal,
};

use crate::TextFieldState;

/// Single-line text input widget.
///
/// `TextInput` is a **pure builder** — it reads reactive signals at build time and registers
/// reactive dependencies so the parent re-renders when either signal changes.
///
/// # Parameters
///
/// - `state`: [`Signal<TextFieldState>`] owned by the parent; updated on every key press.
/// - `focused`: [`Signal<bool>`] that controls the visual focus border; set to `true` by the
///   parent's `.on_click()` handler and to `false` when another field is clicked.
///
/// # Examples
///
/// ```no_run
/// use lemon::{Cx, element::Element};
/// use lemon_widgets::{TextInput, TextFieldState};
///
/// fn my_view(cx: &Cx) -> Element {
///     let state = cx.use_signal(TextFieldState::new(""));
///     let focused = cx.use_signal(false);
///     TextInput::new(state.clone(), focused.clone())
///         .placeholder("Type here…")
///         .width(240.0)
///         .into_element()
/// }
/// ```
pub struct TextInput {
    state: Signal<TextFieldState>,
    focused: Signal<bool>,
    placeholder: String,
    width: Option<f32>,
}

impl TextInput {
    /// Creates a new `TextInput` backed by `state` and `focused`.
    ///
    /// The builder reads both signals immediately so the parent component re-renders
    /// whenever either one changes.
    pub fn new(state: Signal<TextFieldState>, focused: Signal<bool>) -> Self {
        TextInput {
            state,
            focused,
            placeholder: String::new(),
            width: None,
        }
    }

    /// Sets the placeholder text shown when the field value is empty.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Fixed width in logical points (defaults to auto).
    pub fn width(mut self, v: f32) -> Self {
        self.width = Some(v);
        self
    }

    /// Builds and returns the [`Element`] for this input field.
    ///
    /// Call this at the end of the builder chain to insert the widget into a parent container.
    pub fn into_element(self) -> Element {
        let is_focused = self.focused.get();
        let current_value = self.state.get().value.clone();
        let placeholder = self.placeholder.clone();

        let state = self.state.clone();
        let focused = self.focused.clone();

        // Border color: blue when focused, gray when not.
        let border_color = if is_focused {
            Color::rgb8(59, 130, 246) // blue-500
        } else {
            Color::rgb8(156, 163, 175) // gray-400
        };

        // Text shown in the field: current value or placeholder (dimmed).
        let text_child: Element = if current_value.is_empty() {
            Text::new(placeholder)
                .color(Color::rgb8(156, 163, 175))
                .into_element()
        } else {
            Text::new(current_value).into_element()
        };

        let mut container = View::new()
            .padding(6.0)
            .border(border_color, 1.5)
            .radius(4.0)
            .focusable()
            .cursor(Cursor::Text)
            .on_click(move || {
                focused.set(true);
            })
            .on_key_down(move |ev| {
                state.update(|s| s.handle_key(&ev));
            })
            .child(Row::new().child(text_child));

        if let Some(w) = self.width {
            container = container.width(w);
        }

        container.into_element()
    }
}

impl From<TextInput> for Element {
    fn from(input: TextInput) -> Self {
        input.into_element()
    }
}
