use crate::{
    element::Element,
    icons::{IconData, IconPaint},
    Color, View,
};

/// A crisp vector icon backed by an ordinary view.
///
/// Defaults to 24 logical points, a 2-unit stroke and the theme foreground.
/// Icons are decorative; attach actions and text labels to their containing control.
/// `.decorate(...)` can style the resulting view.
///
/// ```
/// use lemon::prelude::*;
/// let icon = Icon::new(lucide::SETTINGS).size(18.0).stroke_width(1.5);
/// ```
pub struct Icon {
    paint: IconPaint,
    size: f32,
}

impl Icon {
    /// Creates an icon from a bundled definition; see [`crate::icons::lucide`].
    pub fn new(data: IconData) -> Self {
        Self {
            paint: IconPaint {
                data,
                color: crate::theme::current_theme().colors.foreground,
                stroke_width: 2.0,
            },
            size: 24.0,
        }
    }
    /// Sets the square size in logical points. Must be finite and nonnegative.
    ///
    /// # Panics
    /// Panics for a negative or non-finite size.
    pub fn size(mut self, size: f32) -> Self {
        assert!(
            size.is_finite() && size >= 0.0,
            "icon size must be finite and nonnegative"
        );
        self.size = size;
        self
    }
    /// Overrides the theme foreground for both strokes and filled details.
    pub fn color(mut self, color: Color) -> Self {
        self.paint.color = color;
        self
    }
    /// Sets stroke width in the 24-unit grid, scaling with the icon's size.
    ///
    /// # Panics
    /// Panics for a negative or non-finite width. Zero suppresses strokes.
    pub fn stroke_width(mut self, width: f32) -> Self {
        assert!(
            width.is_finite() && width >= 0.0,
            "icon stroke width must be finite and nonnegative"
        );
        self.paint.stroke_width = width;
        self
    }
    /// Builds an ordinary view with vector paint and a fixed square size.
    pub fn into_element(self) -> Element {
        let mut element = View::new()
            .width(self.size)
            .height(self.size)
            .into_element();
        if let Element::View(view) = &mut element {
            view.style.flex_shrink = Some(0.0);
            view.paint.icon = Some(self.paint);
        }
        element
    }
}
impl From<Icon> for Element {
    fn from(icon: Icon) -> Self {
        icon.into_element()
    }
}
