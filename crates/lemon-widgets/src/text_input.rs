use lemon::{
    element::{
        builders::{Row, Text, View},
        events::Cursor,
        style::{Color, Overflow},
        types::TextInputMeta,
        Element,
    },
    Cx, Signal,
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
///     TextInput::new(cx, state)
///         .placeholder("Type here…")
///         .width(240.0)
///         .into_element()
/// }
/// ```
pub struct TextInput {
    state: Signal<TextFieldState>,
    placeholder: String,
    width: Option<f32>,
    placeholder_color: Color,
    border_color: Color,
    padding: f32,
    radius: f32,
}

impl TextInput {
    /// Creates a new `TextInput` backed by `state`.
    ///
    /// The builder reads the signal immediately so the parent component re-renders
    /// whenever the field value or cursor changes.
    pub fn new(cx: &Cx, state: Signal<TextFieldState>) -> Self {
        let theme = cx.use_theme();
        TextInput {
            state,
            placeholder: String::new(),
            width: None,
            placeholder_color: theme.colors.foreground_secondary,
            border_color: theme.colors.border,
            padding: theme.spacing.sm,
            radius: theme.radius.sm,
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
                .color(self.placeholder_color)
                .into_element()
        } else {
            Text::new(current_value).into_element()
        };

        let mut container = View::new()
            .padding(self.padding)
            .border(self.border_color, 1.5)
            .radius(self.radius)
            .overflow(Overflow::Hidden)
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
            .child(
                Row::new()
                    .overflow(Overflow::Hidden)
                    .flex_grow(1.0)
                    .child(text_child),
            );

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

#[cfg(test)]
mod tests {
    use super::*;
    use lemon::{
        current_theme,
        element::{style::CornerRadii, Element},
        set_active_theme, Theme,
    };

    #[test]
    fn text_input_defaults_follow_active_theme() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.foreground_secondary = Color::rgb8(120, 121, 122);
        custom.colors.border = Color::rgb8(12, 34, 56);
        custom.spacing.sm = 10.0;
        custom.radius.sm = 7.0;
        set_active_theme(custom.clone());

        let cx = Cx::new();
        let state = Signal::new(TextFieldState::new(""));
        let Element::View(view) = TextInput::new(&cx, state)
            .placeholder("Type here")
            .into_element()
        else {
            panic!("expected View element");
        };

        assert_eq!(
            view.style.padding,
            Some(lemon::element::style::Edges::all(custom.spacing.sm))
        );
        let paint = view.paint.resolve();
        assert_eq!(paint.border_color, Some(custom.colors.border));
        assert_eq!(paint.radius, CornerRadii::all(custom.radius.sm));

        let Element::Row(row) = &view.children[0] else {
            panic!("expected inner Row");
        };
        let Element::Text(text) = &row.children[0] else {
            panic!("expected placeholder Text");
        };
        assert_eq!(text.style.color, Some(custom.colors.foreground_secondary));

        set_active_theme(previous);
    }
}
