use lemon::{
    element::{
        builders::{Row, Text, View},
        events::Cursor,
        style::Color,
        types::TextInputMeta,
        Element,
    },
    Signal,
};

use crate::TextFieldState;

/// Single-line text input widget.
///
/// `TextInput` is a **pure builder** — it reads reactive state at build time and registers
/// reactive dependencies so the parent re-renders when the field value changes.
///
/// Keyboard focus is handled by the platform ([`FocusManager`](lemon::retained::focus::FocusManager)):
/// click or Tab to focus, then type. The paint pass draws the focus ring and text caret.
///
/// # Examples
///
/// ```no_run
/// use lemon::{Cx, element::Element};
/// use lemon_widgets::{TextInput, TextFieldState};
///
/// fn my_view(cx: &Cx) -> Element {
///     let state = cx.use_signal(TextFieldState::new(""));
///     TextInput::new(state)
///         .placeholder("Type here…")
///         .width(240.0)
///         .into_element()
/// }
/// ```
pub struct TextInput {
    state: Signal<TextFieldState>,
    placeholder: String,
    width: Option<f32>,
}

impl TextInput {
    /// Creates a new `TextInput` backed by `state`.
    ///
    /// The builder reads the signal immediately so the parent component re-renders
    /// whenever the field value or cursor changes.
    pub fn new(state: Signal<TextFieldState>) -> Self {
        TextInput {
            state,
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
        let field = self.state.get();
        let current_value = field.value.clone();
        let placeholder = self.placeholder.clone();

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
            .border(Color::rgb8(156, 163, 175), 1.5)
            .radius(4.0)
            .focusable()
            .cursor(Cursor::Text)
            .text_input(TextInputMeta {
                cursor: field.cursor,
                value: field.value,
            })
            .on_key_down({
                let state = self.state.clone();
                move |ev| state.update(|s| s.handle_key(&ev))
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
