use std::rc::Rc;

use crate::{
    element::{
        builders::{Text, View},
        content::TextContent,
        events::Cursor,
        style::{Align, Color, ColorSource, Distribute},
        Element,
    },
    Cx, KeyState, LemonKey, NamedKey, State,
};

/// Theme-aware button appearance, selected with [`Button::style`].
///
/// All presets share typography, padding, disabled behavior and interaction transitions.
/// `.surface(...)` and `.decorate(...)` can override their defaults.
///
/// ```
/// use lemon::prelude::*;
/// let cx = Cx::new();
/// let button = Button::new(&cx, "Save").style(ButtonStyle::Bordered);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonStyle {
    /// Accent text on a transparent surface, with subtle hover/pressed feedback.
    Clean,
    /// Accent border and text, with subtle hover/pressed feedback.
    Bordered,
    /// Accent background and contrasting text. This is the default.
    #[default]
    Filled,
}

/// Theme-aware button widget for app-facing examples and widgets.
///
/// By default this button reads the active theme through [`Cx::theme`] and applies:
///
/// - `theme.colors.accent` as the background
/// - theme typography and `theme.spacing.sm` padding, matching the default height
///   of [`TextField`](super::TextField) and [`SecureField`](super::SecureField) for single-line labels
/// - `theme.radius.md` as the corner radius
///
/// You can still override those defaults with the usual builder methods.
///
/// # Examples
///
/// ```no_run
/// use lemon::{Cx, element::Element};
/// use lemon::widget::Button;
///
/// fn my_view(cx: &Cx) -> Element {
///     Button::new(cx, "Save")
///         .width(120.0)
///         .into_element()
/// }
/// ```
pub struct Button {
    style: ButtonStyle,
    label: TextContent,
    on_click: Option<Rc<dyn Fn()>>,
    background: Option<ColorSource>,
    padding: f32,
    radius: f32,
    width: Option<f32>,
    height: Option<f32>,
    accent: Color,
    accent_hover: Color,
    accent_pressed: Color,
    disabled_background: Color,
    text_color: Color,
    disabled_text_color: Color,
    hovered: State<bool>,
    pressed: State<bool>,
    disabled: bool,
}

impl Button {
    /// Creates a new button whose default styling comes from the active theme.
    ///
    /// Single-line labels use the same font size, line height and padding as text
    /// fields. Explicit [`.height()`](Self::height) and [`.inset()`](Self::inset)
    /// overrides take precedence over this natural height.
    pub fn new(cx: &Cx, label: impl Into<TextContent>) -> Self {
        let theme = cx.theme();
        Self {
            style: ButtonStyle::default(),
            label: label.into(),
            on_click: None,
            background: None,
            padding: theme.spacing.sm,
            radius: theme.radius.md,
            width: None,
            height: None,
            accent: theme.colors.accent,
            accent_hover: theme.colors.accent_hover,
            accent_pressed: theme.colors.accent_pressed,
            disabled_background: theme.colors.surface,
            text_color: theme.colors.on_accent,
            disabled_text_color: theme.colors.foreground_disabled,
            hovered: cx.state(false),
            pressed: cx.state(false),
            disabled: false,
        }
    }

    /// Selects a theme-aware preset (default: [`ButtonStyle::Filled`]).
    ///
    /// Explicit surface, padding, size and radius overrides remain in effect.
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// Called when the button is clicked (after hit-testing).
    pub fn action(mut self, f: impl Fn() + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }

    /// Fill color behind the label ([`Color`] or reactive [`ColorSource`]).
    pub fn surface(mut self, c: impl Into<ColorSource>) -> Self {
        self.background = Some(c.into());
        self
    }

    /// Uniform padding on all sides, in logical points.
    pub fn inset(mut self, v: f32) -> Self {
        self.padding = v;
        self
    }

    /// Corner radius in logical points.
    pub fn rounding(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }

    /// Fixed width in logical points.
    pub fn width(mut self, v: f32) -> Self {
        self.width = Some(v);
        self
    }

    /// Fixed height in logical points.
    pub fn height(mut self, v: f32) -> Self {
        self.height = Some(v);
        self
    }

    /// Marks this button as disabled, preventing focus, keyboard and pointer interactions.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Finishes the builder and returns an [`Element`].
    pub fn into_element(self) -> Element {
        let label = self.label;
        let disabled = self.disabled;
        let text_color = if disabled {
            self.disabled_text_color
        } else if self.style == ButtonStyle::Filled {
            self.text_color
        } else {
            self.accent
        };

        let background = if let Some(background) = self.background {
            background
        } else {
            let hovered = self.hovered.clone();
            let pressed = self.pressed.clone();
            let accent = self.accent;
            let accent_hover = self.accent_hover;
            let accent_pressed = self.accent_pressed;
            let disabled_background = self.disabled_background;
            let filled = self.style == ButtonStyle::Filled;
            ColorSource::Dynamic(Rc::new(move || {
                if !filled {
                    if disabled {
                        Color::default()
                    } else if pressed.get() {
                        accent.with_alpha(0.16)
                    } else if hovered.get() {
                        accent.with_alpha(0.08)
                    } else {
                        Color::default()
                    }
                } else if disabled {
                    disabled_background
                } else if pressed.get() {
                    accent_pressed
                } else if hovered.get() {
                    accent_hover
                } else {
                    accent
                }
            }))
        };

        let mut button = View::new()
            .inset(self.padding)
            .surface(background)
            .rounding(self.radius)
            .border(
                if disabled {
                    self.disabled_text_color
                } else {
                    self.accent
                },
                if self.style == ButtonStyle::Bordered {
                    1.0
                } else {
                    0.0
                },
            )
            .transition(super::interaction_transition())
            .align(Align::Center)
            .distribute(Distribute::Center)
            .cursor(if disabled {
                Cursor::NotAllowed
            } else {
                Cursor::Pointer
            })
            .child(Text::new(label).weight(500).color(text_color));

        if let Some(width) = self.width {
            button = button.width(width);
        }
        if let Some(height) = self.height {
            button = button.height(height);
        }

        if !disabled {
            let hovered_enter = self.hovered.clone();
            let hovered_leave = self.hovered.clone();
            let pressed_down = self.pressed.clone();
            let pressed_up = self.pressed.clone();
            let pressed_leave = self.pressed.clone();
            button = button
                .focusable()
                .on_hover_enter(move || hovered_enter.set(true))
                .on_hover_leave(move || {
                    hovered_leave.set(false);
                    pressed_leave.set(false);
                })
                .on_pointer_down(move |_, _| pressed_down.set(true))
                .on_pointer_up(move |_, _| pressed_up.set(false));

            if let Some(on_click) = self.on_click {
                let keyboard_action = on_click.clone();
                button = button.action(move || on_click()).on_key_down(move |event| {
                    if event.state == KeyState::Pressed
                        && !event.repeat
                        && matches!(
                            event.key,
                            LemonKey::Named(NamedKey::Enter | NamedKey::Space)
                        )
                    {
                        keyboard_action();
                    }
                });
            }
        }

        let mut element = button.into_element();
        if let Element::View(view) = &mut element {
            view.paint.focus_ring_color = Some(self.accent.into());
        }
        element
    }
}

impl From<Button> for Element {
    fn from(button: Button) -> Self {
        button.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        current_theme,
        element::{style::CornerRadii, Element},
        set_active_theme, Color, Theme,
    };

    #[test]
    fn button_defaults_follow_active_theme() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.accent = Color::rgb8(200, 10, 20);
        custom.spacing.sm = 11.0;
        custom.radius.md = 9.0;
        set_active_theme(custom.clone());

        let cx = Cx::new();
        let Element::View(button) = Button::new(&cx, "Save").into_element() else {
            panic!("expected View element");
        };

        assert_eq!(
            button.style.padding,
            Some(crate::element::style::Edges::all(custom.spacing.sm))
        );
        let paint = button.paint.resolve();
        assert_eq!(paint.background, Some(custom.colors.accent));
        assert_eq!(paint.radius, CornerRadii::all(custom.radius.md));

        set_active_theme(previous);
    }

    #[test]
    fn button_hover_and_press_states_change_background() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.accent = Color::rgb8(1, 2, 3);
        custom.colors.accent_hover = Color::rgb8(4, 5, 6);
        custom.colors.accent_pressed = Color::rgb8(7, 8, 9);
        set_active_theme(custom.clone());

        let cx = Cx::new();
        let Element::View(button) = Button::new(&cx, "Save").into_element() else {
            panic!("expected View element");
        };

        assert_eq!(
            button.paint.resolve().background,
            Some(custom.colors.accent)
        );
        button
            .handlers
            .on_hover_enter
            .as_ref()
            .expect("hover enter handler")();
        assert_eq!(
            button.paint.resolve().background,
            Some(custom.colors.accent_hover)
        );
        button
            .handlers
            .on_pointer_down
            .as_ref()
            .expect("pointer down handler")(0.5, 0.5);
        assert_eq!(
            button.paint.resolve().background,
            Some(custom.colors.accent_pressed)
        );
        button
            .handlers
            .on_pointer_up
            .as_ref()
            .expect("pointer up handler")(0.5, 0.5);
        assert_eq!(
            button.paint.resolve().background,
            Some(custom.colors.accent_hover)
        );

        set_active_theme(previous);
    }

    #[test]
    fn disabled_button_ignores_click_and_uses_disabled_background() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.colors.surface = Color::rgb8(10, 11, 12);
        custom.colors.foreground_disabled = Color::rgb8(13, 14, 15);
        set_active_theme(custom.clone());

        let cx = Cx::new();
        let clicked = std::rc::Rc::new(std::cell::Cell::new(false));
        let clicked_flag = clicked.clone();
        let Element::View(button) = Button::new(&cx, "Save")
            .action(move || clicked_flag.set(true))
            .disabled(true)
            .into_element()
        else {
            panic!("expected View element");
        };

        assert!(button.handlers.on_click.is_none());
        assert!(button.handlers.on_hover_enter.is_none());
        assert!(button.handlers.on_pointer_down.is_none());
        assert_eq!(
            button.paint.resolve().background,
            Some(custom.colors.surface)
        );
        let Element::Text(label) = &button.children[0] else {
            panic!("expected Text child");
        };
        assert_eq!(label.style.color, Some(custom.colors.foreground_disabled));
        assert!(!clicked.get());

        set_active_theme(previous);
    }
}
