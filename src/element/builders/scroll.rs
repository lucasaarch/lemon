use std::cell::Cell;
use std::rc::Rc;

use crate::{
    element::{builders::View, style::Color, style::ColorSource, Element},
    Cx, ScrollAxis, ScrollOffset, State,
};

/// Per-widget visual overrides for [`ScrollView`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScrollViewStyle {
    scrollbar_track_color: Option<Color>,
    scrollbar_thumb_color: Option<Color>,
}

impl ScrollViewStyle {
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

/// Scroll viewport with independent axis offsets and proportional scrollbar thumbs.
///
/// Vertical content fills the viewport width; horizontal content fills its height.
/// With [`ScrollAxis::Both`], content keeps its natural size in both directions.
/// Set a definite viewport width/height and use sized content for a bounded canvas.
///
/// ScrollView range is clamped using the **measured** content extent on the selected axis after each layout pass
/// (see [`layout::sync_scroll_layout_max`](crate::layout::sync_scroll_layout_max)). The track and
/// thumb are painted in the paint pass from live layout data, so thumb size tracks content
/// without manual height estimates.
pub struct ScrollView {
    child: Element,
    axis: ScrollAxis,
    offset: State<ScrollOffset>,
    height: f32,
    width: Option<f32>,
    /// Fallback max scroll before the first layout measurement (virtualized lists).
    content_size: Option<(f32, f32)>,
    layout_max: Rc<Cell<ScrollOffset>>,
    style: ScrollViewStyle,
}

impl ScrollView {
    /// Creates a vertical scroll region that owns its scroll offset (starts at `0`).
    ///
    /// Consumes one hook slot; construct it in a stable order on every render.
    pub fn new(cx: &Cx, child: impl Into<Element>) -> Self {
        Self::with_offset(cx.state(ScrollOffset::default()), child)
    }

    /// Creates a scroll region backed by a caller-owned offset in logical points.
    ///
    /// This constructor consumes no hooks. Use `ScrollOffset { x, y }` for independent
    /// axes instead of a scalar offset. Measured bounds clamp enabled axes after layout.
    ///
    /// ```
    /// use lemon::prelude::*;
    /// let offset = State::new(ScrollOffset::default());
    /// let view = ScrollView::with_offset(offset, View::new().width(800.0).height(600.0))
    ///     .axis(ScrollAxis::Both).width(240.0).height(160.0);
    /// ```
    pub fn with_offset(offset: State<ScrollOffset>, child: impl Into<Element>) -> Self {
        Self {
            child: child.into(),
            axis: ScrollAxis::Vertical,
            offset,
            height: 200.0,
            width: None,
            content_size: None,
            layout_max: Rc::new(Cell::new(ScrollOffset::new(f64::MAX, f64::MAX))),
            style: ScrollViewStyle::default(),
        }
    }

    /// Chooses the scroll direction (default: vertical).
    ///
    /// Horizontal views use native horizontal movement or Shift+wheel. Set a
    /// definite width and height for a bounded viewport. Content retains its natural
    /// size along scrolling axes and fills any non-scrolling cross axis. `Both`
    /// accepts diagonal gestures and paints a scrollbar for each overflowing axis.
    /// Calling this method again replaces the previous selection.
    ///
    /// ```
    /// use lemon::prelude::*;
    /// let cx = Cx::new();
    /// let view = ScrollView::new(&cx, RowView::new().child(Text::new("Wide content")))
    ///     .axis(ScrollAxis::Horizontal).width(240.0).height(80.0);
    /// ```
    pub fn axis(mut self, axis: ScrollAxis) -> Self {
        self.axis = axis;
        self
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

    /// Sets fallback content width and height in logical points, before measured layout.
    ///
    /// Prefer measured content bounds. Use this for virtualized content where an
    /// estimate is needed before the first layout. Replaces the scalar `content_extent`.
    ///
    /// # Panics
    /// Panics if either size is negative or non-finite.
    pub fn content_size(mut self, width: f32, height: f32) -> Self {
        assert!(
            width.is_finite() && height.is_finite() && width >= 0.0 && height >= 0.0,
            "scroll content size must be finite and nonnegative"
        );
        self.content_size = Some((width, height));
        self
    }

    /// Replaces all scrollbar style overrides.
    pub fn style(mut self, style: ScrollViewStyle) -> Self {
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
        let axis = self.axis;
        let offset_value = self
            .offset
            .get()
            .clamped(ScrollOffset::new(f32::MAX.into(), f32::MAX.into()));
        let offset = self.offset.clone();
        let viewport_h = self.height;
        let width = self.width;
        let content_size = self.content_size;
        let layout_max = self.layout_max;
        let layout_max_for_scroll = layout_max.clone();

        let inner = View::new()
            .align(crate::element::style::Align::Start)
            .scroll_content()
            .child(self.child);
        let mut inner = match axis {
            ScrollAxis::Vertical => inner.fill_width().outset_top(-offset_value.y as f32),
            ScrollAxis::Horizontal => inner.fill_height().outset_left(-offset_value.x as f32),
            ScrollAxis::Both => inner
                .outset_top(-offset_value.y as f32)
                .outset_left(-offset_value.x as f32),
        }
        .into_element();
        if let Element::View(view) = &mut inner {
            view.style.flex_shrink = Some(0.0);
        }

        let mut viewport = View::new()
            .height(viewport_h)
            .clip()
            .scroll_viewport(axis)
            .scroll_layout_max(layout_max)
            .on_scroll(move |delta| {
                let measured = layout_max_for_scroll.get();
                let fallback = content_size.map(|(w, h)| {
                    ScrollOffset::new(
                        f64::from((w - width.unwrap_or(0.0)).max(0.0)),
                        f64::from((h - viewport_h).max(0.0)),
                    )
                });
                let maximum = ScrollOffset::new(
                    if !axis.horizontal() {
                        0.0
                    } else if measured.x < f64::MAX / 2.0 {
                        measured.x
                    } else {
                        fallback.map_or(f64::MAX, |size| size.x)
                    },
                    if !axis.vertical() {
                        0.0
                    } else if measured.y < f64::MAX / 2.0 {
                        measured.y
                    } else {
                        fallback.map_or(f64::MAX, |size| size.y)
                    },
                );
                let current = offset.get();
                let next =
                    ScrollOffset::new(current.x - delta.x, current.y - delta.y).clamped(maximum);
                if next != current {
                    offset.set(next);
                }
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

impl From<ScrollView> for Element {
    fn from(scroll: ScrollView) -> Self {
        scroll.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Element;
    use crate::layout::{LayoutMap, Viewport};
    use crate::Overflow;
    use parley::FontContext;

    fn layout_pass(
        tree: &mut crate::RetainedTree,
        viewport: Viewport,
    ) -> Result<LayoutMap, crate::retained::RetainedError> {
        let mut font_cx = FontContext::new();
        crate::layout::layout_pass(tree, &mut font_cx, viewport)
    }

    #[test]
    fn both_axes_use_independent_fallback_sizes_before_layout() {
        let offset = State::new(ScrollOffset::default());
        let Element::View(view) = ScrollView::with_offset(offset.clone(), View::new())
            .axis(ScrollAxis::Both)
            .width(200.0)
            .height(100.0)
            .content_size(700.0, 400.0)
            .into_element()
        else {
            panic!("expected viewport");
        };
        let scroll = view.handlers.on_scroll.unwrap();
        scroll(crate::ScrollDelta::new(-9999.0, -9999.0));
        assert_eq!(offset.get(), ScrollOffset::new(500.0, 300.0));
        scroll(crate::ScrollDelta::new(100.0, 0.0));
        assert_eq!(offset.get(), ScrollOffset::new(400.0, 300.0));
    }

    #[test]
    fn scroll_uses_shared_viewport_marker() {
        let offset = State::new(ScrollOffset::new(0.0, 10.0));
        let root =
            ScrollView::with_offset(offset.clone(), crate::element::builders::Text::new("item"))
                .height(200.0)
                .width(300.0)
                .into_element();

        let Element::View(viewport) = root else {
            panic!("expected scroll viewport View");
        };
        assert_eq!(viewport.style.overflow, Overflow::Hidden);
        assert_eq!(viewport.scroll_viewport, Some(crate::ScrollAxis::Vertical));
        assert!(viewport.handlers.on_scroll.is_some());
        assert!(viewport.handlers.scroll_layout_max.is_some());

        let Element::View(inner) = &viewport.children[0] else {
            panic!("expected inner content wrapper to be Element::View");
        };
        assert_eq!(inner.style.margin.as_ref().map(|m| m.top), Some(-10.0));

        let on_scroll = viewport.handlers.on_scroll.as_ref().unwrap();
        on_scroll(crate::ScrollDelta::new(0.0, -20.0));
        assert_eq!(offset.get(), ScrollOffset::new(0.0, 30.0));
        on_scroll(crate::ScrollDelta::new(0.0, 1000.0));
        assert_eq!(offset.get(), ScrollOffset::default());
    }

    #[test]
    fn scroll_with_content_size_clamps_offset_at_max_table_driven() {
        let cases = [
            ("matches_estimate", 596.0_f32, 396.0_f64),
            ("over_estimate", 604.0_f32, 404.0_f64),
        ];

        for (name, content_extent, expected_max_offset) in cases {
            let offset = State::new(ScrollOffset::default());
            let root = ScrollView::with_offset(
                offset.clone(),
                crate::element::builders::Text::new("item"),
            )
            .height(200.0)
            .content_size(0.0, content_extent)
            .into_element();

            let Element::View(viewport) = root else {
                panic!("expected View viewport");
            };
            let on_scroll = viewport.handlers.on_scroll.as_ref().unwrap();

            on_scroll(crate::ScrollDelta::new(0.0, -9999.0));
            assert_eq!(
                offset.get(),
                ScrollOffset::new(0.0, expected_max_offset),
                "{name}: offset must clamp at content_extent - viewport_h"
            );

            on_scroll(crate::ScrollDelta::new(0.0, 9999.0));
            assert_eq!(
                offset.get(),
                ScrollOffset::default(),
                "{name}: offset must clamp at 0"
            );
        }
    }

    #[test]
    fn scroll_clamps_to_measured_content_after_layout_sync() {
        use crate::element::builders::{ColumnView, RowView, Text};
        use crate::{sync_scroll_layout_max, RetainedTree};

        let mut list = ColumnView::new().spacing(4.0);
        for i in 0..12 {
            list = list.child(
                RowView::new()
                    .key(i as u64)
                    .inset(4.0)
                    .child(Text::new(format!("{:02}. track", i + 1)).size(14.0)),
            );
        }

        let offset = State::new(ScrollOffset::default());
        let root = ScrollView::with_offset(offset.clone(), list)
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
        on_scroll(crate::ScrollDelta::new(0.0, -9999.0));
        assert_eq!(
            offset.get(),
            max_offset,
            "scroll must stop when last item meets viewport bottom"
        );
    }

    #[test]
    fn scroll_layout_max_cell_is_stable_across_into_element_calls() {
        let offset = State::new(ScrollOffset::default());
        let scroll = ScrollView::with_offset(offset, crate::element::builders::Text::new("x"));
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
        let offset = State::new(ScrollOffset::default());
        let root = ScrollView::with_offset(offset, crate::element::builders::Text::new("item"))
            .style(
                ScrollViewStyle::default()
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
        let offset = State::new(ScrollOffset::default());
        let root = ScrollView::with_offset(offset, crate::element::builders::Text::new("item"))
            .style(ScrollViewStyle::default().scrollbar_thumb_color(Color::rgb8(4, 5, 6)))
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
