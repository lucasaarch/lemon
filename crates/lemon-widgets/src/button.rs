use std::rc::Rc;

use lemon::{
    element::{
        builders::{Text, View},
        content::TextContent,
        events::Cursor,
        style::{Align, Color, ColorSource, Justify},
        Element,
    },
    Cx, Signal,
};

/// Theme-aware button widget for app-facing examples and widgets.
///
/// By default this button reads the active theme through [`Cx::use_theme`] and applies:
///
/// - `theme.colors.accent` as the background
/// - `theme.spacing.sm` as uniform padding
/// - `theme.radius.md` as the corner radius
///
/// You can still override those defaults with the usual builder methods.
///
/// # Examples
///
/// ```no_run
/// use lemon::{Cx, element::Element};
/// use lemon_widgets::Button;
///
/// fn my_view(cx: &Cx) -> Element {
///     Button::new(cx, "Save")
///         .width(120.0)
///         .into_element()
/// }
/// ```
pub struct Button {
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
    hovered: Signal<bool>,
    pressed: Signal<bool>,
    disabled: bool,
}

impl Button {
    /// Creates a new button whose default styling comes from the active theme.
    pub fn new(cx: &Cx, label: impl Into<TextContent>) -> Self {
        let theme = cx.use_theme();
        Self {
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
            hovered: cx.use_signal(false),
            pressed: cx.use_signal(false),
            disabled: false,
        }
    }

    /// Called when the button is clicked (after hit-testing).
    pub fn on_click(mut self, f: impl Fn() + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }

    /// Fill color behind the label ([`Color`] or reactive [`ColorSource`]).
    pub fn background(mut self, c: impl Into<ColorSource>) -> Self {
        self.background = Some(c.into());
        self
    }

    /// Uniform padding on all sides, in logical points.
    pub fn padding(mut self, v: f32) -> Self {
        self.padding = v;
        self
    }

    /// Corner radius in logical points.
    pub fn radius(mut self, r: f32) -> Self {
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

    /// Marks this button as disabled, preventing click and pointer interactions.
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
        } else {
            self.text_color
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
            ColorSource::Dynamic(Rc::new(move || {
                if disabled {
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
            .padding(self.padding)
            .background(background)
            .radius(self.radius)
            .align_items(Align::Center)
            .justify_content(Justify::Center)
            .cursor(if disabled {
                Cursor::NotAllowed
            } else {
                Cursor::Pointer
            })
            .child(
                Text::new(label)
                    .font_size(16.0)
                    .weight(500)
                    .color(text_color),
            );

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
                .on_hover_enter(move || hovered_enter.set(true))
                .on_hover_leave(move || {
                    hovered_leave.set(false);
                    pressed_leave.set(false);
                })
                .on_pointer_down(move |_, _| pressed_down.set(true))
                .on_pointer_up(move |_, _| pressed_up.set(false));

            if let Some(on_click) = self.on_click {
                button = button.on_click(move || on_click());
            }
        }

        button.into_element()
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
    use lemon::{
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
            Some(lemon::element::style::Edges::all(custom.spacing.sm))
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
            .on_click(move || clicked_flag.set(true))
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
