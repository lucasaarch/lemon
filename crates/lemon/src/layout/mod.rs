use std::collections::HashMap;

use parley::{
    Alignment, AlignmentOptions, FontContext, FontWeight, GenericFamily, Layout, LayoutContext,
    StyleProperty,
};
use taffy::geometry::{Point, Size};
use taffy::{AvailableSpace, NodeId};

use crate::element::style::TextStyle;
use crate::retained::{RetainedError, RetainedKind, RetainedNode, RetainedTree};

type ParleyBrush = [u8; 4];

/// Absolute bounds of a node after layout, in logical coordinates relative to the window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Maps Taffy [`NodeId`] values to absolute [`LayoutRect`]s for one layout pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutMap {
    rects: HashMap<NodeId, LayoutRect>,
}

impl LayoutMap {
    pub fn get(&self, node_id: NodeId) -> Option<&LayoutRect> {
        self.rects.get(&node_id)
    }

    pub fn len(&self) -> usize {
        self.rects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }
}

/// Viewport size in logical points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
}

struct TextMeasureOutput {
    layout: Layout<ParleyBrush>,
    max_width: Option<f32>,
}

struct MeasureContext<'a> {
    font_cx: &'a mut FontContext,
    layout_cx: &'a mut LayoutContext<ParleyBrush>,
    results: &'a mut HashMap<NodeId, TextMeasureOutput>,
}

/// Parley multiplies font metrics by this factor. Layout uses logical points; HiDPI is applied
/// in [`crate::paint::paint_pass`], not here.
const PARLEY_LAYOUT_SCALE: f32 = 1.0;

#[derive(Clone)]
struct TextSnapshot {
    content: String,
    style: TextStyle,
    needs_layout: bool,
    layout_max_width: Option<f32>,
    parley_layout: Option<Layout<ParleyBrush>>,
}

/// Runs Taffy flex layout and text measurement on `tree`, returning absolute logical rects.
///
/// Clears [`RetainedTree::layout_dirty`](crate::retained::RetainedTree::layout_dirty). Pass the
/// same [`Viewport`] size the window uses in logical points. `font_size` and other style sizes are
/// logical pixels; the platform applies the window scale factor when painting.
pub fn layout_pass(
    tree: &mut RetainedTree,
    viewport: Viewport,
) -> Result<LayoutMap, RetainedError> {
    let root_node = tree
        .root
        .as_ref()
        .ok_or(RetainedError::UnsupportedElement("empty retained tree"))?;
    let root_id = root_node
        .layout_node_id()
        .ok_or(RetainedError::UnsupportedElement(
            "root without layout node",
        ))?;

    let mut snapshots: HashMap<NodeId, TextSnapshot> = HashMap::new();
    collect_text_snapshots(root_node, &mut snapshots);

    let mut font_cx = FontContext::new();
    let mut layout_cx = LayoutContext::new();
    let mut measure_results: HashMap<NodeId, TextMeasureOutput> = HashMap::new();
    let mut measure_ctx = MeasureContext {
        font_cx: &mut font_cx,
        layout_cx: &mut layout_cx,
        results: &mut measure_results,
    };

    let available_space = Size {
        width: AvailableSpace::Definite(viewport.width),
        height: AvailableSpace::Definite(viewport.height),
    };

    tree.taffy.compute_layout_with_measure(
        root_id,
        available_space,
        |known_dimensions, available_space, node_id, _, _| {
            measure_text_node(
                snapshots.get(&node_id),
                known_dimensions,
                available_space,
                &mut measure_ctx,
                node_id,
            )
        },
    )?;

    {
        if let Some(root) = tree.root.as_mut() {
            apply_text_measurements(root, &measure_results);
        }
    }

    let root_node = tree
        .root
        .as_ref()
        .ok_or(RetainedError::UnsupportedElement("empty retained tree"))?;
    let mut map = LayoutMap::default();
    collect_layouts(&tree.taffy, root_node, Point::ZERO, &mut map);
    fix_absolute_overlay_bounds(root_node, &mut map);
    tree.layout_dirty = false;
    Ok(map)
}

/// Like [`layout_pass`], but returns `Ok(None)` when the tree is not dirty (cheap no-op).
pub fn layout_pass_if_dirty(
    tree: &mut RetainedTree,
    viewport: Viewport,
) -> Result<Option<LayoutMap>, RetainedError> {
    if !tree.layout_dirty {
        return Ok(None);
    }
    Ok(Some(layout_pass(tree, viewport)?))
}

fn collect_text_snapshots(node: &RetainedNode, map: &mut HashMap<NodeId, TextSnapshot>) {
    if let (Some(id), Some(text)) = (node.taffy_id, node.text.as_ref()) {
        map.insert(
            id,
            TextSnapshot {
                content: text.content.clone(),
                style: text.style.clone(),
                needs_layout: text.needs_layout,
                layout_max_width: text.layout_max_width,
                parley_layout: text.parley_layout.clone(),
            },
        );
    }
    for child in &node.children {
        collect_text_snapshots(child, map);
    }
}

fn measure_text_node(
    snapshot: Option<&TextSnapshot>,
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
    ctx: &mut MeasureContext<'_>,
    node_id: NodeId,
) -> Size<f32> {
    let Some(snapshot) = snapshot else {
        return Size::ZERO;
    };

    // Taffy may supply both dimensions for sized leaves (e.g. buttons with `.width(...)`).
    // Still run Parley when `needs_layout` is set so `apply_text_measurements` can clear it.
    if let (Some(width), Some(height)) = (known_dimensions.width, known_dimensions.height) {
        if !snapshot.needs_layout {
            return Size { width, height };
        }
    }

    let max_width = known_dimensions.width.or(match available_space.width {
        AvailableSpace::Definite(width) => Some(width),
        AvailableSpace::MinContent | AvailableSpace::MaxContent => None,
    });

    if !snapshot.needs_layout && snapshot.layout_max_width == max_width {
        if let Some(layout) = &snapshot.parley_layout {
            return Size {
                width: layout.width(),
                height: layout.height(),
            };
        }
    }

    let font_size = effective_font_size(&snapshot.style);
    let weight = FontWeight::new(snapshot.style.font_weight as f32);

    let mut builder =
        ctx.layout_cx
            .ranged_builder(ctx.font_cx, &snapshot.content, PARLEY_LAYOUT_SCALE, true);
    builder.push_default(GenericFamily::SystemUi);
    builder.push_default(StyleProperty::FontSize(font_size));
    builder.push_default(StyleProperty::FontWeight(weight));
    let mut layout = builder.build(&snapshot.content);
    layout.break_all_lines(max_width);
    layout.align(Alignment::Start, AlignmentOptions::default());

    let size = Size {
        width: layout.width(),
        height: layout.height(),
    };
    ctx.results
        .insert(node_id, TextMeasureOutput { layout, max_width });
    size
}

fn effective_font_size(style: &TextStyle) -> f32 {
    if style.font_size > 0.0 {
        style.font_size
    } else {
        16.0
    }
}

/// Measures the width of a single-line string using the same defaults as layout.
pub fn measure_single_line_width(content: &str, style: &TextStyle) -> f32 {
    if content.is_empty() {
        return 0.0;
    }

    let mut font_cx = FontContext::new();
    let mut layout_cx = LayoutContext::<ParleyBrush>::new();
    let font_size = effective_font_size(style);
    let weight = FontWeight::new(style.font_weight as f32);

    let mut builder = layout_cx.ranged_builder(&mut font_cx, content, PARLEY_LAYOUT_SCALE, true);
    builder.push_default(GenericFamily::SystemUi);
    builder.push_default(StyleProperty::FontSize(font_size));
    builder.push_default(StyleProperty::FontWeight(weight));
    let mut layout = builder.build(content);
    layout.break_all_lines(None);
    layout.align(Alignment::Start, AlignmentOptions::default());
    layout.width()
}

fn apply_text_measurements(node: &mut RetainedNode, results: &HashMap<NodeId, TextMeasureOutput>) {
    if let Some(id) = node.taffy_id {
        if let (Some(text), Some(output)) = (&mut node.text, results.get(&id)) {
            text.parley_layout = Some(output.layout.clone());
            text.layout_max_width = output.max_width;
            text.needs_layout = false;
        }
    }
    for child in &mut node.children {
        apply_text_measurements(child, results);
    }
}

fn collect_layouts(
    taffy: &taffy::TaffyTree<()>,
    node: &RetainedNode,
    offset: Point<f32>,
    map: &mut LayoutMap,
) {
    if matches!(node.kind, RetainedKind::Component { .. }) {
        for child in &node.children {
            collect_layouts(taffy, child, offset, map);
        }
        return;
    }

    let Some(taffy_id) = node.taffy_id else {
        for child in &node.children {
            collect_layouts(taffy, child, offset, map);
        }
        return;
    };

    let layout = taffy
        .layout(taffy_id)
        .expect("layout exists after compute_layout");
    let abs = LayoutRect {
        x: offset.x + layout.location.x,
        y: offset.y + layout.location.y,
        width: layout.size.width,
        height: layout.size.height,
    };
    map.rects.insert(taffy_id, abs);

    let child_offset = Point { x: abs.x, y: abs.y };
    for child in &node.children {
        collect_layouts(taffy, child, child_offset, map);
    }
}

/// Taffy often reports `0` height for absolutely positioned flex containers whose size should
/// follow their children (e.g. `Select` dropdown panels). Expand those rects so paint can fill a
/// background over the full option list.
fn fix_absolute_overlay_bounds(node: &RetainedNode, map: &mut LayoutMap) {
    if matches!(node.kind, RetainedKind::Component { .. }) {
        for child in &node.children {
            fix_absolute_overlay_bounds(child, map);
        }
        return;
    }

    if let Some(taffy_id) = node.taffy_id {
        if node.style.position_absolute {
            if let Some(rect) = map.rects.get(&taffy_id).copied() {
                if let Some(content) = union_descendant_bounds(node, map) {
                    let height = (content.y + content.height) - rect.y;
                    let width = (content.x + content.width) - rect.x;
                    if height > rect.height || width > rect.width {
                        map.rects.insert(
                            taffy_id,
                            LayoutRect {
                                x: rect.x,
                                y: rect.y,
                                width: rect.width.max(width),
                                height: rect.height.max(height),
                            },
                        );
                    }
                }
            }
        }
    }

    for child in &node.children {
        fix_absolute_overlay_bounds(child, map);
    }
}

fn union_descendant_bounds(node: &RetainedNode, map: &LayoutMap) -> Option<LayoutRect> {
    let mut union: Option<LayoutRect> = None;
    for child in &node.children {
        if let Some(child_id) = child.taffy_id {
            if let Some(rect) = map.get(child_id) {
                union = Some(match union {
                    None => *rect,
                    Some(u) => union_layout_rects(u, *rect),
                });
            }
        }
        if let Some(desc) = union_descendant_bounds(child, map) {
            union = Some(match union {
                None => desc,
                Some(u) => union_layout_rects(u, desc),
            });
        }
    }
    union
}

fn union_layout_rects(a: LayoutRect, b: LayoutRect) -> LayoutRect {
    let left = a.x.min(b.x);
    let top = a.y.min(b.y);
    let right = (a.x + a.width).max(b.x + b.width);
    let bottom = (a.y + a.height).max(b.y + b.height);
    LayoutRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{NodePath, Patch};
    use crate::element::builders::{Column, Text, View};
    use crate::element::types::ComponentElement;

    #[test]
    fn font_size_16px_measures_in_logical_points_not_display_scale() {
        let mut tree = RetainedTree::mount(Text::new("Ag").font_size(16.0).into_element()).unwrap();

        let map = layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 600.0,
            },
        )
        .unwrap();

        let text_id = tree.root.as_ref().unwrap().taffy_id.unwrap();
        let height = map.get(text_id).unwrap().height;
        assert!(
            height > 12.0 && height < 26.0,
            "16px logical font should measure near 16px tall, got {height}"
        );
    }

    #[test]
    fn absolute_column_overlay_expands_to_descendant_bounds() {
        let mut tree = RetainedTree::mount(
            Column::new()
                .width(100.0)
                .child(View::new().width(100.0).height(40.0))
                .child(
                    Column::new()
                        .absolute()
                        .top(40.0)
                        .left(0.0)
                        .width(100.0)
                        .child(
                            View::new()
                                .padding(8.0)
                                .child(Text::new("A").font_size(14.0)),
                        )
                        .child(
                            View::new()
                                .padding(8.0)
                                .child(Text::new("B").font_size(14.0)),
                        ),
                )
                .into_element(),
        )
        .unwrap();

        let map = layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 400.0,
            },
        )
        .unwrap();

        let dropdown = &tree.root.as_ref().unwrap().children[1];
        let rect = map.get(dropdown.taffy_id.unwrap()).unwrap();
        assert!(
            rect.height > 40.0,
            "absolute dropdown column should cover option rows, got {:?}",
            rect
        );
    }

    #[test]
    fn text_node_measured_with_parley_clears_needs_layout() {
        let mut tree = RetainedTree::mount(
            View::new()
                .width(120.0)
                .child(Text::new("hello").font_size(16.0))
                .into_element(),
        )
        .unwrap();

        let text_id = tree.root.as_ref().unwrap().children[0].taffy_id.unwrap();
        assert!(
            tree.root.as_ref().unwrap().children[0]
                .text
                .as_ref()
                .unwrap()
                .needs_layout
        );

        let map = layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 600.0,
            },
        )
        .unwrap();

        let text = &tree.root.as_ref().unwrap().children[0];
        let cache = text.text.as_ref().unwrap();
        let rect = map.get(text_id).unwrap();

        assert!(rect.height > 0.0);
        assert!(cache.parley_layout.is_some());
        assert!(!cache.needs_layout);
        assert!(cache.layout_max_width.is_some());
    }

    #[test]
    fn button_with_fixed_width_clears_label_needs_layout() {
        use crate::element::builders::Button;

        let mut tree = RetainedTree::mount(
            View::new()
                .width(200.0)
                .child(Button::new("+").width(44.0))
                .into_element(),
        )
        .unwrap();

        let button = &tree.root.as_ref().unwrap().children[0];
        assert!(button.text.as_ref().is_some_and(|text| text.needs_layout));

        layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 600.0,
            },
        )
        .unwrap();

        let button = &tree.root.as_ref().unwrap().children[0];
        let label = button.text.as_ref().unwrap();
        assert!(label.parley_layout.is_some());
        assert!(!label.needs_layout);
        assert!(!tree.text_needs_reflow());
    }

    #[test]
    fn column_children_stack_vertically() {
        let mut tree = RetainedTree::mount(
            Column::new()
                .gap(8.0)
                .child(Text::new("hi"))
                .child(Text::new("hello world"))
                .into_element(),
        )
        .unwrap();

        let map = layout_pass(
            &mut tree,
            Viewport {
                width: 400.0,
                height: 600.0,
            },
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        let short_id = root.children[0].taffy_id.unwrap();
        let long_id = root.children[1].taffy_id.unwrap();

        let short = map.get(short_id).unwrap();
        let long = map.get(long_id).unwrap();

        assert!(long.y > short.y);
        assert!(short.width > 0.0 && short.height > 0.0);
        assert!(long.width > 0.0 && long.height > 0.0);
        assert_eq!(
            short.width, long.width,
            "column stretches children to same width"
        );
    }

    #[test]
    fn button_in_column_sizes_to_content_not_full_width() {
        use crate::element::builders::Button;

        let mut tree = RetainedTree::mount(
            Column::new()
                .padding(24.0)
                .child(Button::new("Incrementar"))
                .into_element(),
        )
        .unwrap();

        let map = layout_pass(
            &mut tree,
            Viewport {
                width: 900.0,
                height: 600.0,
            },
        )
        .unwrap();

        let button_id = tree.root.as_ref().unwrap().children[0].taffy_id.unwrap();
        let button_rect = map.get(button_id).unwrap();
        assert!(
            button_rect.width < 400.0,
            "button should hug label, got width {}",
            button_rect.width
        );
    }

    #[test]
    fn same_fn_sibling_components_update_text_at_index_five_reflows() {
        use crate::element::builders::{Button, Column, Component, Row, Text};
        use crate::runtime::Runtime;
        use crate::Cx;

        fn mini(cx: &Cx) -> crate::element::Element {
            let n = cx.use_signal(0i32);
            let label = n.clone();
            Row::new()
                .child(Text::new(move || format!("{}", label.get())).font_size(16.0))
                .child(Button::new("+").width(44.0))
                .into_element()
        }

        let mut runtime = Runtime::new();
        runtime.mount(|_cx| {
            Column::new()
                .child(Text::new("a"))
                .child(Text::new("b"))
                .child(Text::new("c"))
                .child(Text::new("d"))
                .child(Text::new("e"))
                .child(Component::new(mini).key(1))
                .child(Component::new(mini).key(2))
                .into_element()
        });

        let mut tree = RetainedTree::mount(runtime.root_element().expect("root")).unwrap();
        tree.apply_patches(runtime.take_patches()).unwrap();
        layout_pass(
            &mut tree,
            Viewport {
                width: 520.0,
                height: 627.0,
            },
        )
        .unwrap();
        assert!(!tree.text_needs_reflow());

        tree.apply_patch(Patch::UpdateText {
            node: NodePath(vec![5, 0]),
            content: "1".to_owned(),
        })
        .unwrap();
        layout_pass(
            &mut tree,
            Viewport {
                width: 520.0,
                height: 627.0,
            },
        )
        .unwrap();

        let text = tree
            .root
            .as_ref()
            .unwrap()
            .children
            .get(5)
            .expect("first component")
            .children
            .first()
            .expect("row")
            .children
            .first()
            .expect("counter text");
        let cache = text.text.as_ref().expect("text cache");
        let first_id = text.taffy_id.expect("text taffy_id");
        let second_id = tree
            .root
            .as_ref()
            .unwrap()
            .children
            .get(6)
            .unwrap()
            .children
            .first()
            .unwrap()
            .children
            .first()
            .unwrap()
            .taffy_id
            .expect("second text taffy_id");
        assert_ne!(
            first_id, second_id,
            "sibling counter text nodes must not share a Taffy NodeId (first={first_id:?}, second={second_id:?})"
        );
        let first_row = tree.taffy.parent(first_id).expect("text parent row");
        assert!(
            tree.taffy.parent(first_row).is_some(),
            "first mini-counter row must stay attached in the Taffy tree"
        );
        assert_eq!(cache.content, "1");
        assert!(
            cache.parley_layout.is_some(),
            "counter text should have parley layout after layout_pass (needs_layout={}, first_id={first_id:?})",
            cache.needs_layout
        );
        assert!(!cache.needs_layout);
        assert!(
            !tree.text_needs_reflow(),
            "no text node should still need reflow"
        );
    }

    #[test]
    fn component_wrapper_is_transparent_in_layout_collection() {
        fn child(_cx: &crate::runtime::cx::Cx) -> crate::element::Element {
            Text::new("child").into_element()
        }

        let mut tree = RetainedTree::mount(Text::new("root").into_element()).unwrap();
        tree.apply_patch(Patch::MountComponent {
            node: NodePath::root(),
            component: ComponentElement::from_component_fn(child)
                .with_key(crate::element::types::Key(1)),
        })
        .unwrap();

        let map = layout_pass(
            &mut tree,
            Viewport {
                width: 200.0,
                height: 200.0,
            },
        )
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        assert!(matches!(root.kind, RetainedKind::Component { .. }));
        assert!(root.taffy_id.is_none());

        let text_id = root.children[0].taffy_id.unwrap();
        let rect = map.get(text_id).unwrap();
        assert!(rect.width > 0.0 && rect.height > 0.0);
    }
}
