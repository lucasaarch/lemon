use crate::{
    element::{
        builders::{RowView, Text, View},
        events::Cursor,
        style::{Color, ColorSource, Distribute},
        types::{TextAlignment, TextInputMeta},
        Element,
    },
    Cx, LemonKey, State,
};

use super::field_editor::FieldEditor;

/// Theme-aware visual preset shared by [`TextField`] and [`super::SecureField`].
///
/// Presets have the same typography and padding. Explicit widget overrides apply
/// after the preset; `.decorate(...)` can then customize the resulting root view.
///
/// ```
/// use lemon::prelude::*;
/// let cx = Cx::new();
/// let field = TextField::new(&cx, State::new(String::new())).style(TextFieldStyle::Clean);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextFieldStyle {
    /// Transparent background without a border or focus ring by default.
    Clean,
    /// Themed surface and border. This is the default.
    #[default]
    Bordered,
    /// Themed surface without a resting border.
    Filled,
}

#[derive(Default)]
struct FieldOverrides {
    placeholder_color: Option<Color>,
    border_color: Option<Color>,
    padding: Option<f32>,
    radius: Option<f32>,
    focus_ring_color: Option<Color>,
}

/// Single-line editable text field.
///
/// `TextField` is a **pure builder** — it reads reactive state at build time and registers
/// reactive dependencies so the parent re-renders when the field value changes.
///
/// Keyboard focus is handled by the platform ([`FocusManager`](crate::retained::focus::FocusManager)):
/// click or Tab to focus, then type. The paint pass draws the focus ring and text caret.
///
/// # Examples
///
/// ```no_run
/// use lemon::{Color, Cx, element::Element};
/// use lemon::widget::{TextField, TextFieldStyle};
///
/// fn my_view(cx: &Cx) -> Element {
///     let state = cx.state("".to_owned());
///     TextField::new(cx, state)
///         .style(TextFieldStyle::Bordered)
///         .border_color(Color::rgb8(200, 80, 80))
///         .placeholder("Type here…")
///         .width(240.0)
///         .into_element()
/// }
/// ```
pub struct TextField {
    state: State<String>,
    editor: State<FieldEditor>,
    hovered: State<bool>,
    pressed: State<bool>,
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
    style: TextFieldStyle,
    alignment: TextAlignment,
    overrides: FieldOverrides,
    disabled: bool,
    secure: bool,
}

impl TextField {
    /// Creates a new `TextField` backed by `state`.
    ///
    /// The returned builder uses theme defaults and is enabled with no placeholder.
    /// The app owns only the text. Cursor and selection persist internally across renders.
    /// App writes clear selection and clamp the cursor to a valid UTF-8 boundary.
    /// Cursor movement does not notify observers of the string.
    /// This constructor consumes hook slots; call it in a stable order on every render.
    pub fn new(cx: &Cx, state: State<String>) -> Self {
        let theme = cx.theme();
        TextField {
            editor: cx.state(FieldEditor::new(state.peek())),
            state,
            hovered: cx.state(false),
            pressed: cx.state(false),
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
            style: TextFieldStyle::default(),
            alignment: TextAlignment::default(),
            overrides: FieldOverrides::default(),
            disabled: false,
            secure: false,
        }
    }

    pub(super) fn new_secure(cx: &Cx, state: State<String>) -> Self {
        Self {
            secure: true,
            ..Self::new(cx, state)
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

    /// Aligns the value, placeholder, and caret horizontally (default: start/left).
    ///
    /// This affects content placement, not text direction or keyboard navigation.
    ///
    /// ```
    /// use lemon::prelude::*;
    /// let cx = Cx::new();
    /// let field = TextField::new(&cx, State::new(String::new()))
    ///     .width(240.0)
    ///     .alignment(TextAlignment::End);
    /// ```
    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Selects a theme-aware visual preset (default: [`TextFieldStyle::Bordered`]).
    ///
    /// Explicit color, padding and radius overrides remain in effect regardless of call order.
    pub fn style(mut self, style: TextFieldStyle) -> Self {
        self.style = style;
        self
    }

    /// Overrides the placeholder text color for this widget.
    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.overrides.placeholder_color = Some(color);
        self
    }

    /// Overrides the border color for this widget.
    pub fn border_color(mut self, color: Color) -> Self {
        self.overrides.border_color = Some(color);
        self
    }

    /// Overrides the container padding for this widget.
    pub fn inset(mut self, value: f32) -> Self {
        self.overrides.padding = Some(value);
        self
    }

    /// Overrides the border radius for this widget.
    pub fn rounding(mut self, value: f32) -> Self {
        self.overrides.radius = Some(value);
        self
    }

    /// Overrides the focus ring color, opting into a ring even with the clean preset.
    pub fn focus_ring_color(mut self, color: Color) -> Self {
        self.overrides.focus_ring_color = Some(color);
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
        let mut field = self.editor.get();
        field.sync_value(self.state.get());
        let disabled = self.disabled;
        let secure = self.secure;
        // Paint and retained metadata only receive display text and display byte offsets.
        // Editing continues to use the original UTF-8 value/cursor in the signal.
        let (current_value, cursor) = if secure {
            let cursor_chars = field
                .value
                .char_indices()
                .take_while(|(offset, _)| *offset < field.cursor)
                .count();
            (
                "•".repeat(field.value.chars().count()),
                cursor_chars * '•'.len_utf8(),
            )
        } else {
            (field.value, field.cursor)
        };
        let placeholder = self.placeholder.clone();
        let placeholder_color = self
            .overrides
            .placeholder_color
            .unwrap_or(self.placeholder_color);
        let border_color = self.overrides.border_color.unwrap_or(self.border_color);
        let padding = self.overrides.padding.unwrap_or(self.padding);
        let radius = self.overrides.radius.unwrap_or(self.radius);
        let focus_ring_color =
            self.overrides
                .focus_ring_color
                .unwrap_or(if self.style == TextFieldStyle::Clean {
                    Color::default()
                } else {
                    self.focus_ring_color
                });
        let surface_color = {
            let clean = self.style == TextFieldStyle::Clean;
            let hovered = self.hovered.clone();
            let pressed = self.pressed.clone();
            let surface_color = self.surface_color;
            let surface_color_hover = self.surface_color_hover;
            let surface_color_pressed = self.surface_color_pressed;
            let surface_color_disabled = self.surface_color_disabled;
            ColorSource::Dynamic(std::rc::Rc::new(move || {
                if clean {
                    Color::default()
                } else if disabled {
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
            Text::new(current_value.clone())
                .color(text_color)
                .into_element()
        };

        let mut container = View::new()
            .surface(surface_color)
            .inset(padding)
            .border(
                border_color,
                if self.style == TextFieldStyle::Bordered || self.overrides.border_color.is_some() {
                    1.5
                } else {
                    0.0
                },
            )
            .transition(super::interaction_transition())
            .rounding(radius)
            .clip()
            .cursor(if disabled {
                Cursor::NotAllowed
            } else {
                Cursor::Text
            })
            .child(
                RowView::new()
                    .clip()
                    .grow(1.0)
                    .distribute(match self.alignment {
                        TextAlignment::Start => Distribute::Start,
                        TextAlignment::Center => Distribute::Center,
                        TextAlignment::End => Distribute::End,
                    })
                    .child(text_child),
            );
        if !disabled {
            container = container
                .focusable()
                .text_input(TextInputMeta {
                    alignment: self.alignment,
                    cursor,
                    value: current_value,
                })
                .on_key_down({
                    let state = self.state.clone();
                    let editor = self.editor.clone();
                    move |ev| {
                        if secure
                            && (ev.modifiers.ctrl || ev.modifiers.meta)
                            && matches!(&ev.key, LemonKey::Character(key)
                                if key.eq_ignore_ascii_case("c") || key.eq_ignore_ascii_case("x"))
                        {
                            return;
                        }
                        let value = state.peek();
                        let mut next = editor.peek();
                        next.sync_value(value.clone());
                        next.handle_key(&ev);
                        if next.value != value {
                            state.set(next.value.clone());
                        }
                        if next != editor.peek() {
                            editor.set(next);
                        }
                    }
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

impl From<TextField> for Element {
    fn from(input: TextField) -> Self {
        input.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        current_theme,
        element::{style::CornerRadii, Element},
        set_active_theme, Theme,
    };

    #[test]
    fn text_field_defaults_follow_active_theme() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.foreground_secondary = Color::rgb8(120, 121, 122);
        custom.colors.border = Color::rgb8(12, 34, 56);
        custom.spacing.sm = 10.0;
        custom.radius.sm = 7.0;
        set_active_theme(custom.clone());

        let cx = Cx::new();
        let state = State::new("".to_owned());
        let Element::View(view) = TextField::new(&cx, state)
            .placeholder("Type here")
            .into_element()
        else {
            panic!("expected View element");
        };

        assert_eq!(
            view.style.padding,
            Some(crate::element::style::Edges::all(custom.spacing.sm))
        );
        let paint = view.paint.resolve();
        assert_eq!(
            paint.border_color,
            crate::element::style::Edges::all(Some(custom.colors.border))
        );
        assert_eq!(paint.radius, CornerRadii::all(custom.radius.sm));

        let Element::Row(row) = &view.children[0] else {
            panic!("expected inner Row");
        };
        let Element::Text(text) = &row.children[0] else {
            panic!("expected placeholder Text");
        };
        assert_eq!(text.style.color, Some(custom.colors.foreground_secondary));

        let state_with_text = State::new("hello".to_owned());
        let Element::View(filled) = TextField::new(&cx, state_with_text).into_element() else {
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
    fn text_field_style_overrides_and_fallbacks() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.foreground_secondary = Color::rgb8(120, 121, 122);
        custom.colors.border = Color::rgb8(12, 34, 56);
        custom.chrome.focus_ring = Color::rgb8(30, 40, 50);
        custom.spacing.sm = 10.0;
        custom.radius.sm = 7.0;
        set_active_theme(custom);

        let cx = Cx::new();
        let state = State::new("".to_owned());
        let Element::View(view) = TextField::new(&cx, state)
            .style(TextFieldStyle::Bordered)
            .border_color(Color::rgb8(1, 2, 3))
            .focus_ring_color(Color::rgb8(4, 5, 6))
            .placeholder("Type here")
            .into_element()
        else {
            panic!("expected View element");
        };

        assert_eq!(
            view.style.padding,
            Some(crate::element::style::Edges::all(10.0)),
            "unset padding should fall back to theme default"
        );
        let paint = view.paint.resolve();
        assert_eq!(
            paint.border_color,
            crate::element::style::Edges::all(Some(Color::rgb8(1, 2, 3)))
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
    fn text_field_hover_and_press_states_update_surface_color() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.surface = Color::rgb8(1, 2, 3);
        custom.colors.surface_hover = Color::rgb8(4, 5, 6);
        custom.colors.surface_pressed = Color::rgb8(7, 8, 9);
        set_active_theme(custom.clone());

        let cx = Cx::new();
        let state = State::new("".to_owned());
        let Element::View(view) = TextField::new(&cx, state).into_element() else {
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
    fn disabled_text_field_ignores_keyboard_and_uses_disabled_colors() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.foreground_disabled = Color::rgb8(9, 10, 11);
        set_active_theme(custom.clone());

        let cx = Cx::new();
        let state = State::new("".to_owned());
        let Element::View(view) = TextField::new(&cx, state.clone())
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
        assert_eq!(state.get(), "");

        set_active_theme(previous);
    }
}
