use lemon::{
    element::{
        builders::{Row, View},
        style::Color,
        Element,
    },
    Cx, Overflow, Signal,
};

const SCROLLBAR_WIDTH: f32 = 8.0;
const THUMB_MIN_HEIGHT: f32 = 20.0;

/// Vertical scroll viewport with internal offset state and an optional scrollbar visual.
///
/// Call [`.content_height`](Scroll::content_height) to enable the scrollbar — the thumb size
/// and position are derived from the viewport height and the supplied content height.
/// Without a content height the widget behaves exactly as it did before (backward compatible).
pub struct Scroll {
    child: Element,
    offset: Signal<f64>,
    height: f32,
    width: Option<f32>,
    /// Estimated total content height. When set, a scrollbar track and thumb are rendered.
    content_height: Option<f32>,
}

impl Scroll {
    /// Creates a scroll region that owns its scroll offset (starts at `0`).
    pub fn new(cx: &Cx, child: impl Into<Element>) -> Self {
        Self::with_offset(cx.use_signal(0.0f64), child)
    }

    /// Creates a scroll region backed by a caller-owned offset signal (advanced use).
    pub fn with_offset(offset: Signal<f64>, child: impl Into<Element>) -> Self {
        Self {
            child: child.into(),
            offset,
            height: 200.0,
            width: None,
            content_height: None,
        }
    }

    /// Sets the viewport height in logical points (default: `200.0`).
    pub fn height(mut self, value: f32) -> Self {
        self.height = value;
        self
    }

    /// Sets the viewport width in logical points.
    pub fn width(mut self, value: f32) -> Self {
        self.width = Some(value);
        self
    }

    /// Supplies the estimated total content height and enables the scrollbar visual.
    ///
    /// The scrollbar thumb height is proportional to `viewport_height / content_height`,
    /// clamped to a minimum of 20 logical points. The thumb position tracks the scroll offset.
    pub fn content_height(mut self, height: f32) -> Self {
        self.content_height = Some(height);
        self
    }

    /// Builds and returns the [`Element`] for this scroll region.
    pub fn into_element(self) -> Element {
        let offset_value = self.offset.get().max(0.0) as f32;
        let offset = self.offset.clone();
        let viewport_h = self.height;
        let width = self.width;
        let has_custom_scrollbar = self.content_height.is_some();

        let inner = View::new().margin_top(-offset_value).child(self.child);

        // scroll_viewport enables the paint-pass auto-scrollbar; suppress it when a
        // custom scrollbar track is rendered alongside the viewport to avoid doubling.
        let mut viewport = View::new()
            .height(viewport_h)
            .overflow(Overflow::Hidden)
            .on_scroll(move |delta| {
                let next = (offset.get() - delta).max(0.0);
                offset.set(next);
            })
            .child(inner);

        if !has_custom_scrollbar {
            viewport = viewport.scroll_viewport();
        }

        let Some(content_h) = self.content_height else {
            // Backward-compatible: no scrollbar.
            if let Some(w) = width {
                viewport = viewport.width(w);
            }
            return viewport.into_element();
        };

        // Scrollbar path: wrap viewport + track in a Row.
        viewport = viewport.flex_grow(1.0);

        let track = build_scrollbar_track(viewport_h, content_h, offset_value);

        let mut outer = Row::new().height(viewport_h);
        if let Some(w) = width {
            outer = outer.width(w);
        }
        outer.child(viewport).child(track).into_element()
    }
}

fn build_scrollbar_track(viewport_h: f32, content_h: f32, offset: f32) -> Element {
    let ch = content_h.max(viewport_h);
    let ratio = viewport_h / ch;
    let thumb_h = (viewport_h * ratio).clamp(THUMB_MIN_HEIGHT, viewport_h);
    let max_scroll = ch - viewport_h;
    let thumb_top = if max_scroll > 0.0 {
        let clamped = offset.min(max_scroll);
        clamped / max_scroll * (viewport_h - thumb_h)
    } else {
        0.0
    };

    let thumb = View::new()
        .width(SCROLLBAR_WIDTH)
        .height(thumb_h)
        .margin_top(thumb_top)
        .radius(SCROLLBAR_WIDTH / 2.0)
        .background(Color::rgb8(120, 120, 140));

    View::new()
        .width(SCROLLBAR_WIDTH)
        .height(viewport_h)
        .background(Color::rgb8(30, 30, 40))
        .child(thumb)
        .into_element()
}

impl From<Scroll> for Element {
    fn from(scroll: Scroll) -> Self {
        scroll.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lemon::element::{style::Dimension, Element};

    #[test]
    fn scroll_without_content_height_returns_viewport_view() {
        let offset = Signal::new(10.0f64);
        let root = Scroll::with_offset(offset.clone(), lemon::element::builders::Text::new("item"))
            .height(200.0)
            .width(300.0)
            .into_element();

        let Element::View(viewport) = root else {
            panic!("expected scroll viewport to be Element::View");
        };
        assert_eq!(viewport.style.overflow, Overflow::Hidden);
        assert!(viewport.scroll_viewport);
        assert!(viewport.handlers.on_scroll.is_some());
        assert_eq!(viewport.children.len(), 1);

        let Element::View(inner) = &viewport.children[0] else {
            panic!("expected inner content wrapper to be Element::View");
        };
        assert_eq!(inner.style.margin.as_ref().map(|m| m.top), Some(-10.0));

        let on_scroll = viewport.handlers.on_scroll.as_ref().unwrap();
        on_scroll(-20.0);
        assert_eq!(offset.get(), 30.0);
        on_scroll(1000.0);
        assert_eq!(offset.get(), 0.0);
    }

    #[test]
    fn scroll_with_content_height_returns_row_with_viewport_and_track() {
        let offset = Signal::new(0.0f64);
        let root = Scroll::with_offset(offset, lemon::element::builders::Text::new("content"))
            .height(200.0)
            .width(300.0)
            .content_height(600.0)
            .into_element();

        let Element::Row(row) = root else {
            panic!("expected Row outer element when content_height is set");
        };
        // Row contains viewport + scrollbar track
        assert_eq!(row.children.len(), 2);

        let Element::View(viewport) = &row.children[0] else {
            panic!("expected View viewport as first child");
        };
        // scroll_viewport is suppressed when a custom scrollbar track is present
        // to avoid the paint pass rendering a second auto-scrollbar.
        assert!(!viewport.scroll_viewport);
        assert!(viewport.handlers.on_scroll.is_some());
        assert_eq!(viewport.style.overflow, Overflow::Hidden);

        let Element::View(track) = &row.children[1] else {
            panic!("expected View scrollbar track as second child");
        };
        assert_eq!(track.style.width, Some(Dimension::Points(SCROLLBAR_WIDTH)));
        assert_eq!(track.children.len(), 1, "track should contain a thumb");
    }

    #[test]
    fn scroll_thumb_height_is_proportional_to_viewport_content_ratio() {
        let offset = Signal::new(0.0f64);
        let root = Scroll::with_offset(offset, lemon::element::builders::Text::new("content"))
            .height(200.0)
            .content_height(400.0)
            .into_element();

        let Element::Row(row) = root else {
            panic!("expected Row");
        };
        let Element::View(track) = &row.children[1] else {
            panic!("expected track View");
        };
        let Element::View(thumb) = &track.children[0] else {
            panic!("expected thumb View");
        };

        // viewport=200, content=400 → ratio=0.5 → thumb_h=100
        assert_eq!(thumb.style.height, Some(Dimension::Points(100.0)));
    }

    #[test]
    fn scroll_thumb_min_height_is_enforced() {
        let offset = Signal::new(0.0f64);
        // Very tall content → thumb would be tiny without minimum
        let root = Scroll::with_offset(offset, lemon::element::builders::Text::new("content"))
            .height(200.0)
            .content_height(10_000.0)
            .into_element();

        let Element::Row(row) = root else {
            panic!("expected Row");
        };
        let Element::View(track) = &row.children[1] else {
            panic!("expected track View");
        };
        let Element::View(thumb) = &track.children[0] else {
            panic!("expected thumb View");
        };

        let thumb_h = match thumb.style.height {
            Some(Dimension::Points(h)) => h,
            _ => panic!("expected Points height"),
        };
        assert!(
            thumb_h >= THUMB_MIN_HEIGHT,
            "thumb height {thumb_h} should be at least {THUMB_MIN_HEIGHT}"
        );
    }

    #[test]
    fn scroll_thumb_top_moves_with_offset() {
        // At offset=0: thumb_top should be 0
        let root0 = Scroll::with_offset(
            Signal::new(0.0f64),
            lemon::element::builders::Text::new("c"),
        )
        .height(200.0)
        .content_height(400.0)
        .into_element();

        // At offset=200 (half way): thumb_top should be at 50 (half of 200-100=100)
        let root_half = Scroll::with_offset(
            Signal::new(200.0f64),
            lemon::element::builders::Text::new("c"),
        )
        .height(200.0)
        .content_height(400.0)
        .into_element();

        let thumb_top_at = |root: Element| -> f32 {
            let Element::Row(row) = root else { panic!() };
            let Element::View(track) = &row.children[1] else {
                panic!()
            };
            let Element::View(thumb) = &track.children[0] else {
                panic!()
            };
            thumb.style.margin.as_ref().map_or(0.0, |m| m.top)
        };

        assert_eq!(thumb_top_at(root0), 0.0);
        assert_eq!(thumb_top_at(root_half), 100.0);
    }
}
