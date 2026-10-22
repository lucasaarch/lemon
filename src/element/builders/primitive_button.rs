use crate::element::{
    content::TextContent,
    style::{BoxShadow, ColorSource, CornerRadii, Edges, LinearGradient, PaintProps, StyleProps},
    Element,
};
use std::rc::Rc;

// ── Button ────────────────────────────────────────────────────────────────

/// Clickable control with a text label and default padding / background.
pub struct Button {
    label: TextContent,
    style: StyleProps,
    paint: PaintProps,
    on_click: Option<Rc<dyn Fn()>>,
}

impl Button {
    /// Creates a button; label accepts the same static or closure forms as [`crate::Text::new`].
    pub fn new(label: impl Into<TextContent>) -> Self {
        let theme = crate::theme::current_theme();
        Button {
            label: label.into(),
            style: StyleProps {
                padding: Some(Edges::all(theme.spacing.sm)),
                align_self: Some(crate::element::style::Align::Start),
                ..Default::default()
            },
            paint: PaintProps {
                background: Some(theme.colors.accent.into()),
                radius: CornerRadii::all(theme.radius.md),
                ..Default::default()
            },
            on_click: None,
        }
    }
    /// Called when the button is clicked (after hit-testing).
    pub fn action(mut self, f: impl Fn() + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }

    /// Uniform padding on all sides, in logical points.
    pub fn inset(mut self, v: f32) -> Self {
        self.style.padding = Some(Edges::all(v));
        self
    }

    /// Solid fill behind the label ([`crate::Color`] or reactive [`ColorSource`]).
    ///
    /// Calling this clears any previously configured [`gradient`](Self::gradient).
    pub fn surface(mut self, c: impl Into<ColorSource>) -> Self {
        self.paint.background = Some(c.into());
        self.paint.background_gradient = None;
        self
    }

    /// Two-stop linear gradient fill painted behind the label.
    pub fn gradient(mut self, gradient: LinearGradient) -> Self {
        self.paint.background_gradient = Some(gradient);
        self.paint.background = None;
        self
    }

    /// Blurred outer shadow painted before the button background.
    pub fn shadow(mut self, shadow: BoxShadow) -> Self {
        self.paint.box_shadow = Some(shadow);
        self
    }

    /// Sets a uniform corner radius in logical points.
    pub fn rounding(mut self, r: f32) -> Self {
        self.paint.radius = CornerRadii::all(r);
        self
    }

    /// Fixed width in logical points.
    pub fn width(mut self, v: f32) -> Self {
        self.style.width = Some(crate::element::style::Dimension::Points(v));
        self
    }
    /// Fixed height in logical points.
    pub fn height(mut self, v: f32) -> Self {
        self.style.height = Some(crate::element::style::Dimension::Points(v));
        self
    }
    /// Finishes the builder and returns an [`Element`].
    pub fn into_element(self) -> Element {
        Element::Button(crate::element::types::ButtonElement {
            label: self.label,
            style: self.style,
            paint: self.paint,
            on_click: self.on_click,
            key: None,
        })
    }
}

impl From<Button> for Element {
    fn from(b: Button) -> Self {
        b.into_element()
    }
}
