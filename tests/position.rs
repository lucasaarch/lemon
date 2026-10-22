use lemon::{
    element::Element,
    input::hit_test::{hit_test_on_click, LogicalPoint},
    layout::{layout_pass, scroll_content_extent, LayoutMap, LayoutRect},
    retained::{RetainedNode, RetainedTree},
    ColumnView, Position, View, Viewport,
};
use parley::FontContext;

fn layout(element: Element, width: f32, height: f32) -> (RetainedTree, LayoutMap) {
    let mut tree = RetainedTree::mount(element).unwrap();
    let map = layout_pass(
        &mut tree,
        &mut FontContext::new(),
        Viewport { width, height },
    )
    .unwrap();
    (tree, map)
}
fn rect(node: &RetainedNode, map: &LayoutMap) -> LayoutRect {
    *map.get(node.taffy_id.unwrap()).unwrap()
}

#[test]
fn absolute_edges_do_not_reserve_space_and_relative_offsets_preserve_flow() {
    let (tree, map) = layout(
        ColumnView::new()
            .width(200.0)
            .height(100.0)
            .child(
                View::new()
                    .position(Position::Absolute)
                    .right(10.0)
                    .bottom(5.0)
                    .width(20.0)
                    .height(15.0),
            )
            .child(View::new().top(8.0).height(20.0))
            .child(View::new().height(20.0))
            .into_element(),
        400.0,
        300.0,
    );
    let root = tree.root().unwrap();
    assert_eq!(
        rect(&root.children[0], &map),
        LayoutRect {
            x: 170.0,
            y: 80.0,
            width: 20.0,
            height: 15.0
        }
    );
    assert_eq!(rect(&root.children[1], &map).y, 8.0);
    assert_eq!(rect(&root.children[2], &map).y, 20.0);
}

#[test]
fn fixed_uses_viewport_dimensions_escapes_clips_and_tracks_resize() {
    let (mut tree, _) = layout(
        View::new()
            .left(50.0)
            .top(40.0)
            .width(80.0)
            .height(50.0)
            .clip()
            .child(
                View::new()
                    .position(Position::Fixed)
                    .right(10.0)
                    .bottom(20.0)
                    .fill_width()
                    .height(25.0)
                    .action(|| {}),
            )
            .into_element(),
        400.0,
        300.0,
    );
    for (width, height) in [(400.0, 300.0), (600.0, 500.0)] {
        let map = layout_pass(
            &mut tree,
            &mut FontContext::new(),
            Viewport { width, height },
        )
        .unwrap();
        let root = tree.root().unwrap();
        let fixed = &root.children[0];
        assert_eq!(
            rect(fixed, &map),
            LayoutRect {
                x: -10.0,
                y: height - 45.0,
                width,
                height: 25.0
            }
        );
        assert_eq!(
            hit_test_on_click(root, &map, LogicalPoint::new(200.0, height - 40.0))
                .unwrap()
                .taffy_id,
            fixed.taffy_id
        );
    }
}

#[test]
fn fixed_stretches_between_edges_and_nested_fixed_anchors_independently() {
    let (tree, map) = layout(
        View::new()
            .width(100.0)
            .height(100.0)
            .child(
                View::new()
                    .position(Position::Fixed)
                    .left(10.0)
                    .right(20.0)
                    .top(30.0)
                    .bottom(40.0)
                    .child(
                        View::new()
                            .position(Position::Fixed)
                            .left(2.0)
                            .top(3.0)
                            .width(5.0)
                            .height(6.0),
                    ),
            )
            .into_element(),
        400.0,
        300.0,
    );
    let outer = &tree.root().unwrap().children[0];
    assert_eq!(
        rect(outer, &map),
        LayoutRect {
            x: 10.0,
            y: 30.0,
            width: 370.0,
            height: 230.0
        }
    );
    assert_eq!(rect(&outer.children[0], &map).x, 2.0);
    assert_eq!(rect(&outer.children[0], &map).y, 3.0);
}

#[test]
fn sticky_stops_at_parent_and_preserves_scroll_extent() {
    for (offset, expected) in [(0.0, 0.0), (60.0, 0.0), (190.0, -10.0)] {
        let (tree, map) = layout(
            View::new()
                .width(200.0)
                .height(100.0)
                .clip()
                .on_scroll(|_| {})
                .child(
                    ColumnView::new()
                        .scroll_content()
                        .outset_top(-offset)
                        .height(200.0)
                        .child(
                            View::new()
                                .position(Position::Sticky)
                                .top(0.0)
                                .width(200.0)
                                .height(20.0)
                                .action(|| {}),
                        )
                        .child(View::new().height(180.0)),
                )
                .into_element(),
            200.0,
            100.0,
        );
        let root = tree.root().unwrap();
        assert_eq!(rect(&root.children[0].children[0], &map).y, expected);
        assert_eq!(scroll_content_extent(root, &map), Some(200.0));
        if offset == 60.0 {
            assert!(hit_test_on_click(root, &map, LogicalPoint::new(5.0, 5.0)).is_some());
        }
    }
}

#[test]
fn clipped_content_is_not_clickable_but_layers_follow_paint_order() {
    let (tree, map) = layout(
        View::new()
            .width(100.0)
            .height(40.0)
            .clip()
            .child(
                View::new()
                    .position(Position::Absolute)
                    .top(60.0)
                    .width(40.0)
                    .height(20.0)
                    .action(|| {}),
            )
            .child(
                View::new()
                    .position(Position::Fixed)
                    .top(5.0)
                    .width(40.0)
                    .height(20.0)
                    .action(|| {})
                    .layer(2),
            )
            .child(
                View::new()
                    .position(Position::Fixed)
                    .top(5.0)
                    .width(40.0)
                    .height(20.0)
                    .action(|| {})
                    .layer(1),
            )
            .into_element(),
        200.0,
        100.0,
    );
    let root = tree.root().unwrap();
    assert!(hit_test_on_click(root, &map, LogicalPoint::new(5.0, 65.0)).is_none());
    assert_eq!(
        hit_test_on_click(root, &map, LogicalPoint::new(5.0, 10.0))
            .unwrap()
            .taffy_id,
        root.children[1].taffy_id
    );
}

#[test]
fn sticky_uses_nearest_clip_and_leaves_sibling_flow_unchanged() {
    let (tree, map) = layout(
        View::new()
            .width(200.0)
            .height(200.0)
            .clip()
            .child(
                View::new()
                    .top(30.0)
                    .width(100.0)
                    .height(80.0)
                    .clip()
                    .child(
                        ColumnView::new()
                            .height(120.0)
                            .outset_top(-20.0)
                            .child(View::new().position(Position::Sticky).top(5.0).height(20.0))
                            .child(View::new().height(100.0)),
                    ),
            )
            .into_element(),
        200.0,
        200.0,
    );
    let content = &tree.root().unwrap().children[0].children[0];
    assert_eq!(rect(&content.children[0], &map).y, 35.0);
    assert_eq!(rect(&content.children[1], &map).y, 30.0);
}

#[test]
fn reactive_position_and_sticky_offsets_invalidate_layout() {
    use lemon::{ui::UiState, State};
    let position = State::new(Position::Absolute);
    let top = State::new(10.0);
    let source = position.clone();
    let offset = top.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        ColumnView::new()
            .fill()
            .child(
                View::new().top(30.0).width(100.0).height(100.0).child(
                    View::new()
                        .position(source.get())
                        .top(offset.get())
                        .width(20.0)
                        .height(20.0),
                ),
            )
            .into_element()
    })
    .unwrap();
    for (mode, offset, expected) in [
        (Position::Absolute, 10.0, 40.0),
        (Position::Fixed, 10.0, 10.0),
        (Position::Sticky, 10.0, 30.0),
        (Position::Sticky, 50.0, 50.0),
    ] {
        position.set(mode);
        top.set(offset);
        ui.update(
            Viewport {
                width: 300.0,
                height: 200.0,
            },
            1.0,
            None,
            true,
        )
        .unwrap();
        let node = &ui.retained().unwrap().root().unwrap().children[0].children[0];
        assert_eq!(rect(node, ui.layout_map()).y, expected);
    }
}

#[test]
fn fixed_intrinsic_text_uses_window_instead_of_narrow_ancestor() {
    let (tree, map) = layout(
        View::new()
            .width(30.0)
            .height(30.0)
            .child(
                View::new()
                    .position(Position::Fixed)
                    .top(10.0)
                    .left(10.0)
                    .child(lemon::Text::new("A fixed label with room to fit")),
            )
            .into_element(),
        600.0,
        300.0,
    );
    let fixed = &tree.root().unwrap().children[0];
    let bounds = rect(fixed, &map);
    assert!(bounds.width > 100.0 && bounds.width < 580.0, "{bounds:?}");
    assert!(bounds.height < 40.0, "{bounds:?}");
}
