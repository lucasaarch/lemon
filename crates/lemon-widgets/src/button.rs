use lemon::{
    element::{
        builders::Button as CoreButton,
        content::TextContent,
        style::ColorSource,
        Element,
    },
    Cx,
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
pub struct Button(CoreButton);

impl Button {
    /// Creates a new button whose default styling comes from the active theme.
    pub fn new(cx: &Cx, label: impl Into<TextContent>) -> Self {
        let theme = cx.use_theme();
        Self(
            CoreButton::new(label)
                .padding(theme.spacing.sm)
                .background(theme.colors.accent)
                .radius(theme.radius.md),
        )
    }

    /// Called when the button is clicked (after hit-testing).
    pub fn on_click(mut self, f: impl Fn() + 'static) -> Self {
        self.0 = self.0.on_click(f);
        self
    }

    /// Fill color behind the label ([`Color`] or reactive [`ColorSource`]).
    pub fn background(mut self, c: impl Into<ColorSource>) -> Self {
        self.0 = self.0.background(c);
        self
    }

    /// Uniform padding on all sides, in logical points.
    pub fn padding(mut self, v: f32) -> Self {
        self.0 = self.0.padding(v);
        self
    }

    /// Corner radius in logical points.
    pub fn radius(mut self, r: f32) -> Self {
        self.0 = self.0.radius(r);
        self
    }

    /// Fixed width in logical points.
    pub fn width(mut self, v: f32) -> Self {
        self.0 = self.0.width(v);
        self
    }

    /// Fixed height in logical points.
    pub fn height(mut self, v: f32) -> Self {
        self.0 = self.0.height(v);
        self
    }

    /// Finishes the builder and returns an [`Element`].
    pub fn into_element(self) -> Element {
        self.0.into_element()
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
        let Element::Button(button) = Button::new(&cx, "Save").into_element() else {
            panic!("expected Button element");
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
}
