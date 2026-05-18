use lemon::{
    element::{builders::View, events::Cursor, style::Color, Element},
    Signal,
};

/// Horizontal slider widget backed by a user-owned `f32` signal.
///
/// The value is always in the range `[0.0, 1.0]`. Pointer-down sets the value immediately;
/// pointer-move continues updating it while the button is held (mouse capture keeps drag
/// gestures routed to the track even when the pointer leaves its bounds).
///
/// # Examples
///
/// ```no_run
/// use lemon::{Cx, element::Element};
/// use lemon_widgets::Slider;
///
/// fn my_view(cx: &Cx) -> Element {
///     let volume = cx.use_signal(0.5_f32);
///     Slider::new(volume)
///         .width(240.0)
///         .into_element()
/// }
/// ```
pub struct Slider {
    value: Signal<f32>,
    width: f32,
    height: f32,
}

impl Slider {
    /// Creates a `Slider` backed by `value` (a `f32` signal in `[0.0, 1.0]`).
    pub fn new(value: Signal<f32>) -> Self {
        Self {
            value,
            width: 200.0,
            height: 16.0,
        }
    }

    /// Fixed track width in logical points (default: `200.0`).
    pub fn width(mut self, v: f32) -> Self {
        self.width = v;
        self
    }

    /// Fixed track height in logical points (default: `16.0`).
    pub fn height(mut self, v: f32) -> Self {
        self.height = v;
        self
    }

    /// Builds and returns the [`Element`] for this slider.
    ///
    /// Call this at the end of the builder chain to insert the widget into a parent container.
    pub fn into_element(self) -> Element {
        let v = self.value.get().clamp(0.0, 1.0);
        let fill_width = v * self.width;
        let radius = self.height / 2.0;

        let val_down = self.value.clone();
        let val_move = self.value.clone();

        View::new()
            .width(self.width)
            .height(self.height)
            .radius(radius)
            .background(Color::rgb8(55, 65, 81))
            .cursor(Cursor::Pointer)
            .on_pointer_down(move |nx, _| val_down.set(nx.clamp(0.0, 1.0)))
            .on_pointer_move(move |nx, _| val_move.set(nx.clamp(0.0, 1.0)))
            .child(
                View::new()
                    .width(fill_width)
                    .height(self.height)
                    .radius(radius)
                    .background(Color::rgb8(59, 130, 246)),
            )
            .into_element()
    }
}

impl From<Slider> for Element {
    fn from(slider: Slider) -> Self {
        slider.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lemon::element::{style::Dimension, Element};

    #[test]
    fn slider_has_pointer_down_and_move_handlers() {
        let value = Signal::new(0.0_f32);
        let el = Slider::new(value).into_element();

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        assert!(
            track.handlers.on_pointer_down.is_some(),
            "Slider track must have on_pointer_down"
        );
        assert!(
            track.handlers.on_pointer_move.is_some(),
            "Slider track must have on_pointer_move"
        );
    }

    #[test]
    fn slider_pointer_down_updates_value() {
        let value = Signal::new(0.0_f32);
        let el = Slider::new(value.clone()).into_element();

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        let on_down = track
            .handlers
            .on_pointer_down
            .as_ref()
            .expect("must have on_pointer_down");
        on_down(0.5, 0.0);
        assert!(
            (value.get() - 0.5).abs() < 1e-6,
            "value should be 0.5 after pointer_down at nx=0.5"
        );
    }

    #[test]
    fn slider_pointer_move_updates_value() {
        let value = Signal::new(0.0_f32);
        let el = Slider::new(value.clone()).into_element();

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        let on_move = track
            .handlers
            .on_pointer_move
            .as_ref()
            .expect("must have on_pointer_move");
        on_move(0.75, 0.0);
        assert!(
            (value.get() - 0.75).abs() < 1e-6,
            "value should be 0.75 after pointer_move at nx=0.75"
        );
    }

    #[test]
    fn slider_fill_child_width_reflects_initial_value() {
        let value = Signal::new(0.25_f32);
        let el = Slider::new(value).width(200.0).into_element();

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        assert_eq!(track.children.len(), 1);
        let Element::View(fill) = &track.children[0] else {
            panic!("expected View fill child");
        };
        assert_eq!(
            fill.style.width,
            Some(Dimension::Points(50.0)),
            "fill width should be 0.25 * 200 = 50"
        );
    }

    #[test]
    fn slider_default_dimensions() {
        let value = Signal::new(0.0_f32);
        let el = Slider::new(value).into_element();

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        assert_eq!(track.style.width, Some(Dimension::Points(200.0)));
        assert_eq!(track.style.height, Some(Dimension::Points(16.0)));
    }

    #[test]
    fn slider_pointer_coords_are_clamped_to_unit_range() {
        let value = Signal::new(0.5_f32);
        let el = Slider::new(value.clone()).into_element();

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        let on_down = track
            .handlers
            .on_pointer_down
            .as_ref()
            .expect("must have on_pointer_down");

        // Attempt to set a value above 1.0 — should clamp.
        on_down(1.5, 0.0);
        assert_eq!(value.get(), 1.0, "value above 1.0 should clamp to 1.0");

        // Attempt to set a value below 0.0 — should clamp.
        on_down(-0.5, 0.0);
        assert_eq!(value.get(), 0.0, "value below 0.0 should clamp to 0.0");
    }

    #[test]
    fn slider_cursor_is_pointer() {
        let value = Signal::new(0.0_f32);
        let el = Slider::new(value).into_element();

        let Element::View(track) = el else {
            panic!("expected View element from Slider");
        };
        assert_eq!(
            track.style.cursor,
            Cursor::Pointer,
            "Slider track should use Pointer cursor"
        );
    }
}
