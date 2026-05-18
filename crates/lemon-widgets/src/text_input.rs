use lemon::{
    element::{
        builders::{Row, Text, View},
        events::Cursor,
        style::{Color, ColorSource, Overflow},
        types::TextInputMeta,
        Element,
    },
    Cx, Signal,
};

use crate::TextFieldState;

/// Per-widget visual overrides for [`TextInput`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextInputStyle {
    placeholder_color: Option<Color>,
    border_color: Option<Color>,
    padding: Option<f32>,
    radius: Option<f32>,
    focus_ring_color: Option<Color>,
}

impl TextInputStyle {
    /// Overrides the placeholder text color.
    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.placeholder_color = Some(color);
        self
    }

    /// Overrides the border color.
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Overrides the container padding in logical points.
    pub fn padding(mut self, value: f32) -> Self {
        self.padding = Some(value);
        self
    }

    /// Overrides the border radius in logical points.
    pub fn radius(mut self, value: f32) -> Self {
        self.radius = Some(value);
        self
    }

    /// Overrides the focus ring color.
    pub fn focus_ring_color(mut self, color: Color) -> Self {
        self.focus_ring_color = Some(color);
        self
    }
}

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
/// use lemon::{Color, Cx, element::Element};
/// use lemon_widgets::{TextInput, TextFieldState, TextInputStyle};
///
/// fn my_view(cx: &Cx) -> Element {
///     let state = cx.use_signal(TextFieldState::new(""));
///     TextInput::new(cx, state)
///         .style(TextInputStyle::default().border_color(Color::rgb8(200, 80, 80)))
///         .placeholder("Type here…")
///         .width(240.0)
///         .into_element()
/// }
/// ```
pub struct TextInput {
    state: Signal<TextFieldState>,
    hovered: Signal<bool>,
    pressed: Signal<bool>,
    placeholder: String,
    width: Option<f32>,
    placeholder_color: Color,
    text_color: Color,
    surface_color: Color,
    surface_color_hover: Color,
    surface_color_pressed: Color,
    surface_color_disabled: Color,
    border_color: Color,
    text_color_disabled: Color,
    padding: f32,
    radius: f32,
    focus_ring_color: Color,
    style: TextInputStyle,
    disabled: bool,
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
            hovered: cx.use_signal(false),
            pressed: cx.use_signal(false),
            placeholder: String::new(),
            width: None,
            placeholder_color: theme.colors.foreground_secondary,
            text_color: theme.colors.foreground,
            surface_color: theme.colors.surface,
            surface_color_hover: theme.colors.surface_hover,
            surface_color_pressed: theme.colors.surface_pressed,
            surface_color_disabled: theme.colors.surface,
            border_color: theme.colors.border,
            text_color_disabled: theme.colors.foreground_disabled,
            padding: theme.spacing.sm,
            radius: theme.radius.sm,
            focus_ring_color: theme.chrome.focus_ring,
            style: TextInputStyle::default(),
            disabled: false,
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

    /// Replaces all text input style overrides.
    pub fn style(mut self, style: TextInputStyle) -> Self {
        self.style = style;
        self
    }

    /// Overrides the placeholder text color for this widget.
    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.style.placeholder_color = Some(color);
        self
    }

    /// Overrides the border color for this widget.
    pub fn border_color(mut self, color: Color) -> Self {
        self.style.border_color = Some(color);
        self
    }

    /// Overrides the container padding for this widget.
    pub fn padding(mut self, value: f32) -> Self {
        self.style.padding = Some(value);
        self
    }

    /// Overrides the border radius for this widget.
    pub fn radius(mut self, value: f32) -> Self {
        self.style.radius = Some(value);
        self
    }

    /// Overrides the focus ring color for this widget.
    pub fn focus_ring_color(mut self, color: Color) -> Self {
        self.style.focus_ring_color = Some(color);
        self
    }

    /// Marks this text input as disabled, preventing focus and keyboard input.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Builds and returns the [`Element`] for this input field.
    ///
    /// Call this at the end of the builder chain to insert the widget into a parent container.
    pub fn into_element(self) -> Element {
        let field = self.state.get();
        let disabled = self.disabled;
        let current_value = field.value.clone();
        let placeholder = self.placeholder.clone();
        let placeholder_color = self
            .style
            .placeholder_color
            .unwrap_or(self.placeholder_color);
        let border_color = self.style.border_color.unwrap_or(self.border_color);
        let padding = self.style.padding.unwrap_or(self.padding);
        let radius = self.style.radius.unwrap_or(self.radius);
        let focus_ring_color = self.style.focus_ring_color.unwrap_or(self.focus_ring_color);
        let surface_color = {
            let hovered = self.hovered.clone();
            let pressed = self.pressed.clone();
            let surface_color = self.surface_color;
            let surface_color_hover = self.surface_color_hover;
            let surface_color_pressed = self.surface_color_pressed;
            let surface_color_disabled = self.surface_color_disabled;
            ColorSource::Dynamic(std::rc::Rc::new(move || {
                if disabled {
                    surface_color_disabled
                } else if pressed.get() {
                    surface_color_pressed
                } else if hovered.get() {
                    surface_color_hover
                } else {
                    surface_color
                }
            }))
        };
        let text_color = if disabled {
            self.text_color_disabled
        } else {
            self.text_color
        };
        let placeholder_color = if disabled {
            self.text_color_disabled
        } else {
            placeholder_color
        };

        // Text shown in the field: current value or placeholder (dimmed).
        let text_child: Element = if current_value.is_empty() {
            Text::new(placeholder)
                .color(placeholder_color)
                .into_element()
        } else {
            Text::new(current_value).color(text_color).into_element()
        };

        let mut container = View::new()
            .background(surface_color)
            .padding(padding)
            .border(border_color, 1.5)
            .radius(radius)
            .overflow(Overflow::Hidden)
            .cursor(if disabled {
                Cursor::NotAllowed
            } else {
                Cursor::Text
            })
            .child(
                Row::new()
                    .overflow(Overflow::Hidden)
                    .flex_grow(1.0)
                    .child(text_child),
            );
        if !disabled {
            container = container
                .focusable()
                .text_input(TextInputMeta {
                    cursor: field.cursor,
                    value: field.value,
                })
                .on_key_down({
                    let state = self.state.clone();
                    move |ev| state.update(|s| s.handle_key(&ev))
                });
            let hovered_enter = self.hovered.clone();
            let hovered_leave = self.hovered.clone();
            let pressed_down = self.pressed.clone();
            let pressed_up = self.pressed.clone();
            let pressed_leave = self.pressed.clone();
            container = container
                .on_hover_enter(move || hovered_enter.set(true))
                .on_hover_leave(move || {
                    hovered_leave.set(false);
                    pressed_leave.set(false);
                })
                .on_pointer_down(move |_, _| pressed_down.set(true))
                .on_pointer_up(move |_, _| pressed_up.set(false));
        }

        if let Some(w) = self.width {
            container = container.width(w);
        }

        let mut element = container.into_element();
        if let Element::View(view) = &mut element {
            view.paint.focus_ring_color = Some(ColorSource::Static(focus_ring_color));
        }
        element
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
        assert_eq!(
            paint.border_color,
            lemon::element::style::Edges::all(Some(custom.colors.border))
        );
        assert_eq!(paint.radius, CornerRadii::all(custom.radius.sm));

        let Element::Row(row) = &view.children[0] else {
            panic!("expected inner Row");
        };
        let Element::Text(text) = &row.children[0] else {
            panic!("expected placeholder Text");
        };
        assert_eq!(text.style.color, Some(custom.colors.foreground_secondary));

        let mut field = TextFieldState::new("hello");
        field.cursor = 5;
        let state_with_text = Signal::new(field);
        let Element::View(filled) = TextInput::new(&cx, state_with_text).into_element() else {
            panic!("expected View element");
        };
        let Element::Row(filled_row) = &filled.children[0] else {
            panic!("expected inner Row");
        };
        let Element::Text(filled_text) = &filled_row.children[0] else {
            panic!("expected value Text");
        };
        assert_eq!(filled_text.style.color, Some(custom.colors.foreground));
        let filled_paint = filled.paint.resolve();
        assert_eq!(filled_paint.background, Some(custom.colors.surface));

        set_active_theme(previous);
    }

    #[test]
    fn text_input_style_overrides_and_fallbacks() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.foreground_secondary = Color::rgb8(120, 121, 122);
        custom.colors.border = Color::rgb8(12, 34, 56);
        custom.chrome.focus_ring = Color::rgb8(30, 40, 50);
        custom.spacing.sm = 10.0;
        custom.radius.sm = 7.0;
        set_active_theme(custom);

        let cx = Cx::new();
        let state = Signal::new(TextFieldState::new(""));
        let Element::View(view) = TextInput::new(&cx, state)
            .style(
                TextInputStyle::default()
                    .border_color(Color::rgb8(1, 2, 3))
                    .focus_ring_color(Color::rgb8(4, 5, 6)),
            )
            .placeholder("Type here")
            .into_element()
        else {
            panic!("expected View element");
        };

        assert_eq!(
            view.style.padding,
            Some(lemon::element::style::Edges::all(10.0)),
            "unset padding should fall back to theme default"
        );
        let paint = view.paint.resolve();
        assert_eq!(
            paint.border_color,
            lemon::element::style::Edges::all(Some(Color::rgb8(1, 2, 3)))
        );
        assert_eq!(paint.focus_ring_color, Some(Color::rgb8(4, 5, 6)));

        let Element::Row(row) = &view.children[0] else {
            panic!("expected inner Row");
        };
        let Element::Text(text) = &row.children[0] else {
            panic!("expected placeholder Text");
        };
        assert_eq!(
            text.style.color,
            Some(Color::rgb8(120, 121, 122)),
            "unset placeholder color should fall back to theme default"
        );

        set_active_theme(previous);
    }

    #[test]
    fn text_input_hover_and_press_states_update_surface_color() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.surface = Color::rgb8(1, 2, 3);
        custom.colors.surface_hover = Color::rgb8(4, 5, 6);
        custom.colors.surface_pressed = Color::rgb8(7, 8, 9);
        set_active_theme(custom.clone());

        let cx = Cx::new();
        let state = Signal::new(TextFieldState::new(""));
        let Element::View(view) = TextInput::new(&cx, state).into_element() else {
            panic!("expected View element");
        };

        assert_eq!(view.paint.resolve().background, Some(custom.colors.surface));
        view.handlers
            .on_hover_enter
            .as_ref()
            .expect("hover enter handler")();
        assert_eq!(
            view.paint.resolve().background,
            Some(custom.colors.surface_hover)
        );
        view.handlers
            .on_pointer_down
            .as_ref()
            .expect("pointer down handler")(0.5, 0.5);
        assert_eq!(
            view.paint.resolve().background,
            Some(custom.colors.surface_pressed)
        );
        view.handlers
            .on_pointer_up
            .as_ref()
            .expect("pointer up handler")(0.5, 0.5);
        assert_eq!(
            view.paint.resolve().background,
            Some(custom.colors.surface_hover)
        );

        set_active_theme(previous);
    }

    #[test]
    fn disabled_text_input_ignores_keyboard_and_uses_disabled_colors() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.foreground_disabled = Color::rgb8(9, 10, 11);
        set_active_theme(custom.clone());

        let cx = Cx::new();
        let state = Signal::new(TextFieldState::new(""));
        let Element::View(view) = TextInput::new(&cx, state.clone())
            .placeholder("Type here")
            .disabled(true)
            .into_element()
        else {
            panic!("expected View element");
        };

        assert!(!view.style.focusable);
        assert!(view.text_input.is_none());
        assert!(view.handlers.on_key_down.is_none());
        assert!(view.handlers.on_pointer_down.is_none());

        let Element::Row(row) = &view.children[0] else {
            panic!("expected inner Row");
        };
        let Element::Text(text) = &row.children[0] else {
            panic!("expected placeholder Text");
        };
        assert_eq!(text.style.color, Some(custom.colors.foreground_disabled));
        assert_eq!(state.get().value, "");

        set_active_theme(previous);
    }
}
