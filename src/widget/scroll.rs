use std::cell::Cell;
use std::rc::Rc;

use crate::{
    element::{builders::View, style::Color, style::ColorSource, Element},
    Cx, Overflow, Signal,
};

/// Per-widget visual overrides for [`Scroll`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScrollStyle {
    scrollbar_track_color: Option<Color>,
    scrollbar_thumb_color: Option<Color>,
}

impl ScrollStyle {
    /// Overrides the scrollbar track color.
    pub fn scrollbar_track_color(mut self, color: Color) -> Self {
        self.scrollbar_track_color = Some(color);
        self
    }

    /// Overrides the scrollbar thumb color.
    pub fn scrollbar_thumb_color(mut self, color: Color) -> Self {
        self.scrollbar_thumb_color = Some(color);
        self
    }
}

/// Vertical scroll viewport with internal offset state and a proportional scrollbar thumb.
///
/// Scroll range is clamped using the **measured** inner content height after each layout pass
/// (see [`layout::sync_scroll_layout_max`](crate::layout::sync_scroll_layout_max)). The track and
/// thumb are painted in the paint pass from live layout data, so thumb size tracks content
/// without manual height estimates.
pub struct Scroll {
    child: Element,
    offset: Signal<f64>,
    height: f32,
    width: Option<f32>,
    /// Fallback max scroll before the first layout measurement (virtualized lists).
    content_height: Option<f32>,
    layout_max: Rc<Cell<f64>>,
    style: ScrollStyle,
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
            layout_max: Rc::new(Cell::new(f64::MAX)),
            style: ScrollStyle::default(),
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

    /// Optional fallback max scroll before layout measurement is available.
    ///
    /// Prefer the default measured behaviour; use this only for virtualized lists where the
    /// mounted subtree is shorter than the logical list.
    pub fn content_height(mut self, height: f32) -> Self {
        self.content_height = Some(height);
        self
    }

    /// Replaces all scrollbar style overrides.
    pub fn style(mut self, style: ScrollStyle) -> Self {
        self.style = style;
        self
    }

    /// Overrides the scrollbar track color for this widget.
    pub fn scrollbar_track_color(mut self, color: Color) -> Self {
        self.style.scrollbar_track_color = Some(color);
        self
    }

    /// Overrides the scrollbar thumb color for this widget.
    pub fn scrollbar_thumb_color(mut self, color: Color) -> Self {
        self.style.scrollbar_thumb_color = Some(color);
        self
    }

    /// Builds and returns the [`Element`] for this scroll region.
    pub fn into_element(self) -> Element {
        let offset_value = self.offset.get().max(0.0) as f32;
        let offset = self.offset.clone();
        let viewport_h = self.height;
        let width = self.width;
        let content_height = self.content_height;
        let layout_max = self.layout_max;
        let layout_max_for_scroll = layout_max.clone();

        let inner = View::new().margin_top(-offset_value).child(self.child);

        let mut viewport = View::new()
            .height(viewport_h)
            .overflow(Overflow::Hidden)
            .scroll_bar()
            .scroll_layout_max(layout_max)
            .on_scroll(move |delta| {
                let measured_max = layout_max_for_scroll.get();
                let max_offset = if measured_max < f64::MAX / 2.0 {
                    measured_max
                } else {
                    content_height
                        .map_or(f64::MAX, |ch| ((ch as f64) - (viewport_h as f64)).max(0.0))
                };
                let next = (offset.get() - delta).max(0.0).min(max_offset);
                offset.set(next);
            })
            .child(inner);

        if let Some(w) = width {
            viewport = viewport.width(w);
        }
        let mut element = viewport.into_element();
        if let Element::View(viewport) = &mut element {
            if let Some(track) = self.style.scrollbar_track_color {
                viewport.paint.scroll_track_color = Some(ColorSource::Static(track));
            }
            if let Some(thumb) = self.style.scrollbar_thumb_color {
                viewport.paint.scroll_thumb_color = Some(ColorSource::Static(thumb));
            }
        }
        element
    }
}

impl From<Scroll> for Element {
    fn from(scroll: Scroll) -> Self {
        scroll.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Element;

    #[test]
    fn scroll_viewport_uses_scroll_bar_paint_flag() {
        let offset = Signal::new(10.0f64);
        let root = Scroll::with_offset(offset.clone(), crate::element::builders::Text::new("item"))
            .height(200.0)
            .width(300.0)
            .into_element();

        let Element::View(viewport) = root else {
            panic!("expected scroll viewport View");
        };
        assert_eq!(viewport.style.overflow, Overflow::Hidden);
        assert!(viewport.scroll_bar);
        assert!(!viewport.scroll_viewport);
        assert!(viewport.handlers.on_scroll.is_some());
        assert!(viewport.handlers.scroll_layout_max.is_some());

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
    fn scroll_with_content_height_clamps_offset_at_max_table_driven() {
        let cases = [
            ("matches_estimate", 596.0_f32, 396.0_f64),
            ("over_estimate", 604.0_f32, 404.0_f64),
        ];

        for (name, content_height, expected_max_offset) in cases {
            let offset = Signal::new(0.0f64);
            let root =
                Scroll::with_offset(offset.clone(), crate::element::builders::Text::new("item"))
                    .height(200.0)
                    .content_height(content_height)
                    .into_element();

            let Element::View(viewport) = root else {
                panic!("expected View viewport");
            };
            let on_scroll = viewport.handlers.on_scroll.as_ref().unwrap();

            on_scroll(-9999.0);
            assert_eq!(
                offset.get(),
                expected_max_offset,
                "{name}: offset must clamp at content_height - viewport_h"
            );

            on_scroll(9999.0);
            assert_eq!(offset.get(), 0.0, "{name}: offset must clamp at 0");
        }
    }

    #[test]
    fn scroll_clamps_to_measured_content_after_layout_sync() {
        use crate::element::builders::{Column, Row, Text};
        use crate::{layout_pass, sync_scroll_layout_max, RetainedTree, Viewport};

        let mut list = Column::new().gap(4.0);
        for i in 0..12 {
            list = list.child(
                Row::new()
                    .key(i as u64)
                    .padding(4.0)
                    .child(Text::new(format!("{:02}. track", i + 1)).font_size(14.0)),
            );
        }

        let offset = Signal::new(0.0f64);
        let root = Scroll::with_offset(offset.clone(), list)
            .height(180.0)
            .width(400.0)
            .into_element();

        let mut tree = RetainedTree::mount(root).unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 500.0,
                height: 600.0,
            },
        )
        .unwrap();
        sync_scroll_layout_max(tree.root.as_ref().unwrap(), &layout);

        let viewport = tree.root.as_ref().unwrap();
        let cell = viewport.handlers.scroll_layout_max.as_ref().unwrap();
        let max_offset = cell.get();

        let on_scroll = viewport.handlers.on_scroll.as_ref().unwrap();
        on_scroll(-9999.0);
        assert_eq!(
            offset.get(),
            max_offset,
            "scroll must stop when last item meets viewport bottom"
        );
    }

    #[test]
    fn scroll_layout_max_cell_is_stable_across_into_element_calls() {
        let offset = Signal::new(0.0f64);
        let scroll = Scroll::with_offset(offset, crate::element::builders::Text::new("x"));
        let cell_a = scroll.layout_max.clone();
        let el_a = scroll.into_element();
        let Element::View(view_a) = el_a else {
            panic!()
        };
        let cell_b = view_a.handlers.scroll_layout_max.as_ref().unwrap();

        assert!(Rc::ptr_eq(&cell_a, cell_b));
    }

    #[test]
    fn scroll_style_overrides_set_scrollbar_paint_colors() {
        let offset = Signal::new(0.0f64);
        let root = Scroll::with_offset(offset, crate::element::builders::Text::new("item"))
            .style(
                ScrollStyle::default()
                    .scrollbar_track_color(Color::rgb8(1, 2, 3))
                    .scrollbar_thumb_color(Color::rgb8(4, 5, 6)),
            )
            .into_element();

        let Element::View(viewport) = root else {
            panic!("expected scroll viewport View");
        };
        let paint = viewport.paint.resolve();
        assert_eq!(paint.scroll_track_color, Some(Color::rgb8(1, 2, 3)));
        assert_eq!(paint.scroll_thumb_color, Some(Color::rgb8(4, 5, 6)));
    }

    #[test]
    fn scroll_style_unset_fields_fall_back_to_theme_tokens() {
        let offset = Signal::new(0.0f64);
        let root = Scroll::with_offset(offset, crate::element::builders::Text::new("item"))
            .style(ScrollStyle::default().scrollbar_thumb_color(Color::rgb8(4, 5, 6)))
            .into_element();

        let Element::View(viewport) = root else {
            panic!("expected scroll viewport View");
        };
        let paint = viewport.paint.resolve();
        assert_eq!(paint.scroll_thumb_color, Some(Color::rgb8(4, 5, 6)));
        assert_eq!(
            paint.scroll_track_color, None,
            "unset track color should keep theme fallback path"
        );
    }
}
