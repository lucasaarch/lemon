//! Hit-testing in logical coordinates against a [`LayoutMap`].

use crate::layout::{LayoutMap, LayoutRect};
use crate::retained::{RetainedKind, RetainedNode};

/// Cursor position in logical points (pre-HiDPI layout space).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalPoint {
    pub x: f32,
    pub y: f32,
}

impl LogicalPoint {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Convert physical pixel coordinates to logical points using the window scale factor.
pub fn physical_to_logical(physical_x: f64, physical_y: f64, scale_factor: f32) -> LogicalPoint {
    let scale = f64::from(scale_factor);
    LogicalPoint::new((physical_x / scale) as f32, (physical_y / scale) as f32)
}

fn point_in_rect(point: LogicalPoint, rect: &LayoutRect) -> bool {
    point.x >= rect.x
        && point.x < rect.x + rect.width
        && point.y >= rect.y
        && point.y < rect.y + rect.height
}

/// Normalize `point` into the `[0.0, 1.0]` coordinate space of `rect`.
///
/// Values outside the rect are clamped so handlers always receive a value in `[0.0, 1.0]`.
pub fn normalize_coords(point: LogicalPoint, rect: &LayoutRect) -> (f32, f32) {
    let nx = if rect.width > 0.0 {
        ((point.x - rect.x) / rect.width).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let ny = if rect.height > 0.0 {
        ((point.y - rect.y) / rect.height).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (nx, ny)
}

/// Walk the retained tree in post-order and return the top-most node under `point` that has
/// an [`on_pointer_down`](crate::retained::EventHandlers::on_pointer_down) handler,
/// together with the hit coordinates normalized to `[0.0, 1.0]` within the node's bounds.
pub fn hit_test_pointer_down<'a>(
    node: &'a RetainedNode,
    layout: &LayoutMap,
    point: LogicalPoint,
) -> Option<(&'a RetainedNode, (f32, f32))> {
    if matches!(node.kind, RetainedKind::Component { .. }) {
        let mut hit = None;
        for child in &node.children {
            hit = hit_test_pointer_down(child, layout, point).or(hit);
        }
        return hit;
    }

    let mut hit = None;
    for child in &node.children {
        hit = hit_test_pointer_down(child, layout, point).or(hit);
    }

    if node.handlers.on_pointer_down.is_some() {
        if let Some(id) = node.taffy_id {
            if let Some(rect) = layout.get(id) {
                if point_in_rect(point, rect) {
                    hit = Some((node, normalize_coords(point, rect)));
                }
            }
        }
    }

    hit
}

/// Walk the entire retained tree and call [`on_click_outside`](crate::retained::EventHandlers::on_click_outside)
/// on every node whose handler is set but whose layout bounds do **not** contain `point`.
///
/// Used by the platform layer to close popovers or dropdowns when the user clicks elsewhere.
pub fn dispatch_outside_clicks(node: &RetainedNode, layout: &LayoutMap, point: LogicalPoint) {
    if matches!(node.kind, RetainedKind::Component { .. }) {
        for child in &node.children {
            dispatch_outside_clicks(child, layout, point);
        }
        return;
    }

    if let Some(handler) = node.handlers.on_click_outside.as_ref() {
        let is_outside = node
            .taffy_id
            .and_then(|id| layout.get(id))
            .is_none_or(|rect| !point_in_rect(point, rect));
        if is_outside {
            handler();
        }
    }

    for child in &node.children {
        dispatch_outside_clicks(child, layout, point);
    }
}

/// Walk the retained tree in post-order (children before parents) and return the top-most
/// node under `point` that has an `on_click` handler.
pub fn hit_test_on_click<'a>(
    node: &'a RetainedNode,
    layout: &LayoutMap,
    point: LogicalPoint,
) -> Option<&'a RetainedNode> {
    if matches!(node.kind, RetainedKind::Component { .. }) {
        let mut hit = None;
        for child in &node.children {
            hit = hit_test_on_click(child, layout, point).or(hit);
        }
        return hit;
    }

    let mut hit = None;
    for child in &node.children {
        hit = hit_test_on_click(child, layout, point).or(hit);
    }

    if node.handlers.on_click.is_some() {
        if let Some(id) = node.taffy_id {
            if let Some(rect) = layout.get(id) {
                if point_in_rect(point, rect) {
                    hit = Some(node);
                }
            }
        }
    }

    hit
}

pub fn hit_test_hover<'a>(
    node: &'a RetainedNode,
    layout: &LayoutMap,
    point: LogicalPoint,
) -> Option<&'a RetainedNode> {
    if matches!(node.kind, RetainedKind::Component { .. }) {
        let mut hit = None;
        for child in &node.children {
            hit = hit_test_hover(child, layout, point).or(hit);
        }
        return hit;
    }

    let mut hit = None;
    for child in &node.children {
        hit = hit_test_hover(child, layout, point).or(hit);
    }

    let is_hoverable = node.handlers.on_hover_enter.is_some()
        || node.handlers.on_hover_leave.is_some()
        || node.style.cursor != crate::element::events::Cursor::Default;

    if is_hoverable {
        if let Some(id) = node.taffy_id {
            if let Some(rect) = layout.get(id) {
                if point_in_rect(point, rect) {
                    hit = Some(node);
                }
            }
        }
    }

    hit
}

/// Walk the retained tree in post-order and return the top-most focusable node under `point`.
pub fn hit_test_focusable<'a>(
    node: &'a RetainedNode,
    layout: &LayoutMap,
    point: LogicalPoint,
) -> Option<&'a RetainedNode> {
    if matches!(node.kind, RetainedKind::Component { .. }) {
        let mut hit = None;
        for child in &node.children {
            hit = hit_test_focusable(child, layout, point).or(hit);
        }
        return hit;
    }

    let mut hit = None;
    for child in &node.children {
        hit = hit_test_focusable(child, layout, point).or(hit);
    }

    if node.style.focusable {
        if let Some(id) = node.taffy_id {
            if let Some(rect) = layout.get(id) {
                if point_in_rect(point, rect) {
                    hit = Some(node);
                }
            }
        }
    }

    hit
}

pub fn find_node_by_taffy_id(node: &RetainedNode, id: taffy::NodeId) -> Option<&RetainedNode> {
    if node.taffy_id == Some(id) {
        return Some(node);
    }

    for child in &node.children {
        if let Some(found) = find_node_by_taffy_id(child, id) {
            return Some(found);
        }
    }

    None
}

/// Invoke `on_click` on `node` if present. Returns whether a handler ran.
pub fn dispatch_click(node: &RetainedNode) -> bool {
    if let Some(handler) = node.handlers.on_click.as_ref() {
        handler();
        true
    } else {
        false
    }
}

/// Returns the deepest node under `point` that has an [`on_scroll`](crate::element::builders::Column::on_scroll) handler.
///
/// Used by the platform layer to route `MouseWheel` events before paint.
pub fn hit_test_scroll<'a>(
    node: &'a RetainedNode,
    layout: &LayoutMap,
    point: LogicalPoint,
) -> Option<&'a RetainedNode> {
    if matches!(node.kind, RetainedKind::Component { .. }) {
        let mut hit = None;
        for child in &node.children {
            hit = hit_test_scroll(child, layout, point).or(hit);
        }
        return hit;
    }

    let mut hit = None;
    for child in &node.children {
        hit = hit_test_scroll(child, layout, point).or(hit);
    }

    if node.handlers.on_scroll.is_some() {
        if let Some(id) = node.taffy_id {
            if let Some(rect) = layout.get(id) {
                if point_in_rect(point, rect) {
                    hit = Some(node);
                }
            }
        }
    }

    hit
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::builders::{Button, Column, Text, View};
    use crate::layout::{LayoutMap, Viewport};
    use crate::retained::RetainedTree;
    use parley::FontContext;
    use std::cell::Cell;
    use std::rc::Rc;

    fn layout_pass(tree: &mut RetainedTree, viewport: Viewport) -> Result<LayoutMap, crate::retained::RetainedError> {
        let mut font_cx = FontContext::new();
        crate::layout::layout_pass(tree, &mut font_cx, viewport)
    }

    #[test]
    fn physical_to_logical_divides_by_scale_factor() {
        let p = physical_to_logical(200.0, 100.0, 2.0);
        assert_eq!(p, LogicalPoint::new(100.0, 50.0));
    }

    #[test]
    fn hit_test_returns_deepest_clickable_node() {
        let clicked = Rc::new(Cell::new(false));
        let flag = clicked.clone();
        let mut tree = RetainedTree::mount(
            Column::new()
                .width(200.0)
                .height(200.0)
                .child(
                    Button::new("Click")
                        .width(80.0)
                        .height(40.0)
                        .on_click(move || flag.set(true)),
                )
                .into_element(),
        )
        .unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 200.0,
                height: 200.0,
            },
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let button = &root.children[0];
        let rect = layout.get(button.taffy_id.unwrap()).unwrap();
        let hit = hit_test_on_click(root, &layout, LogicalPoint::new(rect.x + 4.0, rect.y + 4.0));

        assert!(hit.is_some());
        assert!(dispatch_click(hit.unwrap()));
        assert!(clicked.get());
    }

    #[test]
    fn hit_test_skips_non_clickable_sibling() {
        let button_clicked = Rc::new(Cell::new(false));
        let flag = button_clicked.clone();
        let mut tree = RetainedTree::mount(
            Column::new()
                .width(200.0)
                .height(120.0)
                .child(Text::new("label").font_size(16.0))
                .child(
                    Button::new("OK")
                        .width(60.0)
                        .height(30.0)
                        .on_click(move || flag.set(true)),
                )
                .into_element(),
        )
        .unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 200.0,
                height: 120.0,
            },
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let text_rect = layout.get(root.children[0].taffy_id.unwrap()).unwrap();
        let miss = hit_test_on_click(
            root,
            &layout,
            LogicalPoint::new(text_rect.x + 2.0, text_rect.y + 2.0),
        );
        assert!(miss.is_none());

        let button_rect = layout.get(root.children[1].taffy_id.unwrap()).unwrap();
        let hit = hit_test_on_click(
            root,
            &layout,
            LogicalPoint::new(button_rect.x + 2.0, button_rect.y + 2.0),
        );
        assert!(hit.is_some());
        dispatch_click(hit.unwrap());
        assert!(button_clicked.get());
    }

    #[test]
    fn hover_hit_test_finds_node_with_hover_enter_handler() {
        let entered = Rc::new(Cell::new(false));
        let e = entered.clone();

        let mut tree = RetainedTree::mount(
            Column::new()
                .width(200.0)
                .height(200.0)
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .on_hover_enter(move || e.set(true)),
                )
                .into_element(),
        )
        .unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 200.0,
                height: 200.0,
            },
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let child = &root.children[0];
        let rect = layout.get(child.taffy_id.unwrap()).unwrap();

        let hit = hit_test_hover(root, &layout, LogicalPoint::new(rect.x + 4.0, rect.y + 4.0));
        assert!(hit.is_some());

        let miss = hit_test_hover(
            root,
            &layout,
            LogicalPoint::new(rect.x + rect.width + 10.0, rect.y),
        );
        assert!(miss.is_none());
    }

    #[test]
    fn find_node_by_taffy_id_returns_correct_node() {
        let mut tree = RetainedTree::mount(
            Column::new()
                .child(Text::new("a"))
                .child(Text::new("b"))
                .into_element(),
        )
        .unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 200.0,
                height: 200.0,
            },
        )
        .unwrap();
        let _ = layout;

        let root = tree.root.as_ref().unwrap();
        let child_id = root.children[1].taffy_id.unwrap();
        let found = find_node_by_taffy_id(root, child_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().text_content(), Some("b"));
    }

    #[test]
    fn hit_test_scroll_finds_node_with_on_scroll() {
        use std::cell::Cell;
        let scrolled = Rc::new(Cell::new(false));
        let s = scrolled.clone();

        let mut tree = RetainedTree::mount(
            View::new()
                .width(200.0)
                .height(150.0)
                .on_scroll(move |_| s.set(true))
                .into_element(),
        )
        .unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 400.0,
            },
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let hit = hit_test_scroll(root, &layout, LogicalPoint::new(50.0, 50.0));
        assert!(hit.is_some());
        if let Some(node) = hit {
            node.handlers.on_scroll.as_ref().unwrap()(10.0);
        }
        assert!(scrolled.get());

        let miss = hit_test_scroll(root, &layout, LogicalPoint::new(300.0, 300.0));
        assert!(miss.is_none());
    }

    #[test]
    fn hit_test_focusable_finds_node_with_focusable_flag() {
        let mut tree = RetainedTree::mount(
            Column::new()
                .width(200.0)
                .height(200.0)
                .child(View::new().width(80.0).height(40.0).focusable())
                .child(View::new().width(80.0).height(40.0))
                .into_element(),
        )
        .unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 200.0,
                height: 200.0,
            },
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let focusable_rect = layout.get(root.children[0].taffy_id.unwrap()).unwrap();

        let hit = hit_test_focusable(
            root,
            &layout,
            LogicalPoint::new(focusable_rect.x + 4.0, focusable_rect.y + 4.0),
        );
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().taffy_id, root.children[0].taffy_id);

        let miss = hit_test_focusable(
            root,
            &layout,
            LogicalPoint::new(focusable_rect.x + 4.0, focusable_rect.y + 60.0),
        );
        assert!(miss.is_none());
    }

    #[test]
    fn hit_test_pointer_down_returns_node_with_normalized_coords() {
        let pressed = Rc::new(Cell::new((0.0_f32, 0.0_f32)));
        let flag = pressed.clone();
        let mut tree = RetainedTree::mount(
            Column::new()
                .width(200.0)
                .height(200.0)
                .child(
                    View::new()
                        .width(100.0)
                        .height(50.0)
                        .on_pointer_down(move |x, y| flag.set((x, y))),
                )
                .into_element(),
        )
        .unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 200.0,
                height: 200.0,
            },
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let rect = layout.get(root.children[0].taffy_id.unwrap()).unwrap();

        // Hit at the center of the node.
        let cx = rect.x + rect.width * 0.5;
        let cy = rect.y + rect.height * 0.5;
        let hit = hit_test_pointer_down(root, &layout, LogicalPoint::new(cx, cy));
        assert!(hit.is_some());
        let (node, (nx, ny)) = hit.unwrap();
        node.handlers.on_pointer_down.as_ref().unwrap()(nx, ny);
        let (rx, ry) = pressed.get();
        assert!((rx - 0.5).abs() < 1e-4);
        assert!((ry - 0.5).abs() < 1e-4);

        // Miss outside the node.
        let miss = hit_test_pointer_down(
            root,
            &layout,
            LogicalPoint::new(rect.x + rect.width + 10.0, rect.y),
        );
        assert!(miss.is_none());
    }

    #[test]
    fn dispatch_outside_clicks_fires_for_nodes_outside_point() {
        let outside_fired = Rc::new(Cell::new(false));
        let flag = outside_fired.clone();

        let mut tree = RetainedTree::mount(
            Column::new()
                .width(200.0)
                .height(200.0)
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .on_click_outside(move || flag.set(true)),
                )
                .into_element(),
        )
        .unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 200.0,
                height: 200.0,
            },
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let rect = layout.get(root.children[0].taffy_id.unwrap()).unwrap();

        // Click well outside the node → handler must fire.
        dispatch_outside_clicks(
            root,
            &layout,
            LogicalPoint::new(rect.x + rect.width + 20.0, rect.y),
        );
        assert!(outside_fired.get());
    }

    #[test]
    fn dispatch_outside_clicks_skips_node_containing_point() {
        let outside_fired = Rc::new(Cell::new(false));
        let flag = outside_fired.clone();

        let mut tree = RetainedTree::mount(
            Column::new()
                .width(200.0)
                .height(200.0)
                .child(
                    View::new()
                        .width(80.0)
                        .height(40.0)
                        .on_click_outside(move || flag.set(true)),
                )
                .into_element(),
        )
        .unwrap();
        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 200.0,
                height: 200.0,
            },
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let rect = layout.get(root.children[0].taffy_id.unwrap()).unwrap();

        // Click inside the node → handler must NOT fire.
        dispatch_outside_clicks(root, &layout, LogicalPoint::new(rect.x + 4.0, rect.y + 4.0));
        assert!(!outside_fired.get());
    }

    #[test]
    fn normalize_coords_produces_clamped_fractions() {
        use crate::layout::LayoutRect;
        let rect = LayoutRect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };

        // Center of the rect → (0.5, 0.5)
        let (nx, ny) = normalize_coords(LogicalPoint::new(60.0, 45.0), &rect);
        assert!((nx - 0.5).abs() < 1e-4);
        assert!((ny - 0.5).abs() < 1e-4);

        // Outside the rect → clamped to 1.0
        let (nx, ny) = normalize_coords(LogicalPoint::new(200.0, 200.0), &rect);
        assert_eq!(nx, 1.0);
        assert_eq!(ny, 1.0);

        // Before the rect → clamped to 0.0
        let (nx, ny) = normalize_coords(LogicalPoint::new(0.0, 0.0), &rect);
        assert_eq!(nx, 0.0);
        assert_eq!(ny, 0.0);
    }
}
