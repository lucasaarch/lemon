use crate::element::events::Cursor;
use crate::element::Element;
use crate::{ColumnView, Cx, RowView, State, View};

/// An edge that can be dragged to resize a [`ResizableView`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeEdge {
    /// Resizes width by dragging the pane's left edge.
    Left,
    /// Resizes width by dragging the pane's right edge.
    Right,
    /// Resizes height by dragging the pane's top edge.
    Top,
    /// Resizes height by dragging the pane's bottom edge.
    Bottom,
}

/// A pane with explicitly selected draggable edges.
///
/// No edge is resizable by default. Set an initial `.width(...)` for `Left` or
/// `Right`, and an initial `.height(...)` for `Top` or `Bottom`. Drag handles use
/// the matching native resize cursor. The widget consumes one hook slot and must
/// be constructed in a stable order. Pane contents are clipped to its bounds so
/// they cannot paint over adjacent panes while resizing.
///
/// ```no_run
/// use lemon::{Cx, Text, widget::{ResizableView, ResizeEdge}};
/// fn editor(cx: &Cx) -> lemon::element::Element {
///     ResizableView::new(cx, Text::new("Files"))
///         .width(240.0)
///         .edges([ResizeEdge::Right])
///         .into_element()
/// }
/// ```
pub struct ResizableView {
    child: Element,
    edges: Vec<ResizeEdge>,
    initial_width: Option<f32>,
    initial_height: Option<f32>,
    min_width: f32,
    min_height: f32,
    size: State<(Option<f32>, Option<f32>)>,
    theme: crate::Theme,
}

impl ResizableView {
    /// Wraps one pane. No resize handles are shown until `.edges(...)` is set.
    pub fn new(cx: &Cx, child: impl Into<Element>) -> Self {
        Self {
            child: child.into(),
            edges: Vec::new(),
            initial_width: None,
            initial_height: None,
            min_width: 80.0,
            min_height: 60.0,
            size: cx.state((None, None)),
            theme: cx.theme(),
        }
    }

    /// Enables resize handles on the listed edges. Omitted edges stay fixed.
    ///
    /// Horizontal edges require `.width(...)`; vertical edges require `.height(...)`.
    pub fn edges(mut self, edges: impl IntoIterator<Item = ResizeEdge>) -> Self {
        self.edges = edges.into_iter().collect();
        self
    }

    /// Sets the initial width and the width used by left/right resize handles.
    pub fn width(mut self, width: f32) -> Self {
        assert!(
            width.is_finite() && width > 0.0,
            "width must be positive and finite"
        );
        self.initial_width = Some(width);
        self
    }

    /// Sets the initial height and the height used by top/bottom resize handles.
    pub fn height(mut self, height: f32) -> Self {
        assert!(
            height.is_finite() && height > 0.0,
            "height must be positive and finite"
        );
        self.initial_height = Some(height);
        self
    }

    /// Sets the minimum width enforced by left/right resize handles.
    pub fn min_width(mut self, width: f32) -> Self {
        assert!(
            width.is_finite() && width > 0.0,
            "minimum width must be positive and finite"
        );
        self.min_width = width;
        self
    }

    /// Sets the minimum height enforced by top/bottom resize handles.
    pub fn min_height(mut self, height: f32) -> Self {
        assert!(
            height.is_finite() && height > 0.0,
            "minimum height must be positive and finite"
        );
        self.min_height = height;
        self
    }

    /// Builds the pane and only the handles requested with `.edges(...)`.
    ///
    /// # Panics
    /// Panics if a horizontal edge is selected without an initial width, or a
    /// vertical edge is selected without an initial height.
    pub fn into_element(self) -> Element {
        let horizontal = self
            .edges
            .iter()
            .any(|edge| matches!(edge, ResizeEdge::Left | ResizeEdge::Right));
        let vertical = self
            .edges
            .iter()
            .any(|edge| matches!(edge, ResizeEdge::Top | ResizeEdge::Bottom));
        assert!(
            !horizontal || self.initial_width.is_some(),
            "left/right resize edges require width"
        );
        assert!(
            !vertical || self.initial_height.is_some(),
            "top/bottom resize edges require height"
        );

        let (stored_width, stored_height) = self.size.get();
        let width = stored_width.or(self.initial_width);
        let height = stored_height.or(self.initial_height);
        let size = self.size;
        let mut middle = RowView::new().fill_width().min_width(0.0).grow(1.0);
        if self.edges.contains(&ResizeEdge::Left) {
            middle = middle.child(resize_handle(
                ResizeEdge::Left,
                size.clone(),
                width.unwrap(),
                height.unwrap_or(0.0),
                self.min_width,
                self.min_height,
                self.theme.colors.border,
            ));
        }
        middle = middle.child(
            View::new()
                .grow(1.0)
                .fill_height()
                .min_width(0.0)
                .clip()
                .child(self.child),
        );
        if self.edges.contains(&ResizeEdge::Right) {
            middle = middle.child(resize_handle(
                ResizeEdge::Right,
                size.clone(),
                width.unwrap(),
                height.unwrap_or(0.0),
                self.min_width,
                self.min_height,
                self.theme.colors.border,
            ));
        }

        let mut root = ColumnView::new().fill().min_width(0.0).clip();
        if self.edges.contains(&ResizeEdge::Top) {
            root = root.child(resize_handle(
                ResizeEdge::Top,
                size.clone(),
                width.unwrap_or(0.0),
                height.unwrap(),
                self.min_width,
                self.min_height,
                self.theme.colors.border,
            ));
        }
        root = root.child(middle);
        if self.edges.contains(&ResizeEdge::Bottom) {
            root = root.child(resize_handle(
                ResizeEdge::Bottom,
                size,
                width.unwrap_or(0.0),
                height.unwrap(),
                self.min_width,
                self.min_height,
                self.theme.colors.border,
            ));
        }
        let fixed_size = width.is_some() || height.is_some();
        if let Some(width) = width {
            root = root.width(width);
        }
        if let Some(height) = height {
            root = root.height(height);
        }
        let mut element = root.into_element();
        if fixed_size {
            match &mut element {
                Element::Column(view) | Element::Row(view) | Element::View(view) => {
                    view.style.flex_shrink = Some(0.0);
                }
                Element::Text(_) | Element::Component(_) | Element::Fragment(_) | Element::None => {
                }
            }
        }
        element
    }
}

fn resize_handle(
    edge: ResizeEdge,
    size: State<(Option<f32>, Option<f32>)>,
    initial_width: f32,
    initial_height: f32,
    min_width: f32,
    min_height: f32,
    color: crate::Color,
) -> Element {
    // Keep the full 6pt region available for pointer hit testing, but paint only
    // the centered 1pt separator so it does not look like a scrollbar or pane.
    let mut handle = View::new();
    handle = match edge {
        ResizeEdge::Left | ResizeEdge::Right => handle
            .width(6.0)
            .fill_height()
            .center()
            .cursor(Cursor::ResizeHorizontal)
            .child(View::new().width(1.0).fill_height().surface(color))
            .on_drag(move |dx, _| {
                let amount = if edge == ResizeEdge::Right { dx } else { -dx };
                if amount.is_finite() {
                    size.update(|(width, _)| {
                        *width = Some((width.unwrap_or(initial_width) + amount).max(min_width));
                    });
                }
            }),
        ResizeEdge::Top | ResizeEdge::Bottom => handle
            .height(6.0)
            .fill_width()
            .center()
            .cursor(Cursor::ResizeVertical)
            .child(View::new().height(1.0).fill_width().surface(color))
            .on_drag(move |_, dy| {
                let amount = if edge == ResizeEdge::Bottom { dy } else { -dy };
                if amount.is_finite() {
                    size.update(|(_, height)| {
                        *height = Some((height.unwrap_or(initial_height) + amount).max(min_height));
                    });
                }
            }),
    };
    let mut element = handle.into_element();
    if let Element::View(view) = &mut element {
        view.style.flex_shrink = Some(0.0);
    }
    element
}

impl From<ResizableView> for Element {
    fn from(view: ResizableView) -> Self {
        view.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Text;

    #[test]
    fn only_requested_edge_renders_a_handle_and_resize_cursor() {
        let inactive = ResizableView::new(&Cx::new(), Text::new("static"))
            .width(220.0)
            .into_element();
        let Element::Column(inactive) = inactive else {
            panic!("resizable pane must render as a column")
        };
        let Element::Row(inactive_content) = &inactive.children[0] else {
            panic!("pane content must render as a row")
        };
        assert_eq!(inactive_content.children.len(), 1);

        let pane = ResizableView::new(&Cx::new(), Text::new("files"))
            .width(220.0)
            .edges([ResizeEdge::Right])
            .into_element();
        let Element::Column(root) = pane else {
            panic!("resizable pane must render as a column")
        };
        let Element::Row(content) = &root.children[0] else {
            panic!("pane content must render as a row")
        };
        assert_eq!(content.children.len(), 2);
        let Element::View(handle) = &content.children[1] else {
            panic!("right edge must render a handle")
        };
        assert_eq!(handle.style.cursor, Cursor::ResizeHorizontal);
        assert!(handle.handlers.on_drag.is_some());
    }

    #[test]
    fn left_edge_drag_resizes_in_the_opposite_direction_and_clamps_minimum() {
        let cx = Cx::new();
        let pane = ResizableView::new(&cx, Text::new("sidebar"))
            .width(220.0)
            .min_width(100.0)
            .edges([ResizeEdge::Left]);
        let size = pane.size.clone();
        let Element::Column(root) = pane.into_element() else {
            panic!("resizable pane must render as a column")
        };
        let Element::Row(content) = &root.children[0] else {
            panic!("pane content must render as a row")
        };
        let Element::View(handle) = &content.children[0] else {
            panic!("left edge must render its handle before the content")
        };
        handle.handlers.on_drag.as_ref().unwrap()(-30.0, 0.0);
        assert_eq!(size.peek().0, Some(250.0));
        handle.handlers.on_drag.as_ref().unwrap()(500.0, 0.0);
        assert_eq!(size.peek().0, Some(100.0));
    }

    #[test]
    fn vertical_edges_require_height_and_use_vertical_resize_cursor() {
        let pane = ResizableView::new(&Cx::new(), Text::new("panel"))
            .height(180.0)
            .edges([ResizeEdge::Top, ResizeEdge::Bottom])
            .into_element();
        let Element::Column(root) = pane else {
            panic!("resizable pane must render as a column")
        };
        assert_eq!(root.children.len(), 3);
        let Element::View(top) = &root.children[0] else {
            panic!("top edge must render a handle")
        };
        assert_eq!(top.style.cursor, Cursor::ResizeVertical);
        let Element::View(bottom) = &root.children[2] else {
            panic!("bottom edge must render a handle")
        };
        assert_eq!(bottom.style.cursor, Cursor::ResizeVertical);
    }

    #[test]
    fn ui_pointer_drag_changes_sidebar_layout_width() {
        use crate::{
            input::{hit_test::LogicalPoint, InputEvent},
            ui::UiState,
            Viewport,
        };

        fn find_resize_handle(
            node: &crate::retained::RetainedNode,
        ) -> Option<&crate::retained::RetainedNode> {
            if node.style.cursor == Cursor::ResizeHorizontal && node.handlers.on_drag.is_some() {
                return Some(node);
            }
            node.children.iter().find_map(find_resize_handle)
        }

        let mut ui = UiState::default();
        ui.mount(|cx| {
            RowView::new()
                .fill()
                .child(
                    ResizableView::new(cx, Text::new("sidebar"))
                        .width(220.0)
                        .edges([ResizeEdge::Right]),
                )
                .child(View::new().grow(1.0).fill())
                .into_element()
        })
        .unwrap();
        ui.layout(Viewport {
            width: 600.0,
            height: 400.0,
        })
        .unwrap();

        let handle = find_resize_handle(ui.retained().unwrap().root().unwrap()).unwrap();
        let bounds = *ui.layout_map().get(handle.taffy_id.unwrap()).unwrap();
        let start = LogicalPoint::new(bounds.x + bounds.width / 2.0, bounds.y + 100.0);
        assert!(
            ui.dispatch_input(InputEvent::PointerDown(start))
                .unwrap()
                .handled
        );
        ui.dispatch_input(InputEvent::PointerMove(LogicalPoint::new(
            start.x + 60.0,
            start.y,
        )))
        .unwrap();
        ui.dispatch_input(InputEvent::PointerUp(LogicalPoint::new(
            start.x + 60.0,
            start.y,
        )))
        .unwrap();

        let sidebar = &ui.retained().unwrap().root().unwrap().children[0];
        let resized = ui
            .layout_map()
            .get(sidebar.taffy_id.unwrap())
            .unwrap()
            .width;
        assert_eq!(resized, 280.0);
    }
}
