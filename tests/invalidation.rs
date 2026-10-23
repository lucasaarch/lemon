use lemon::diff::{NodePath, Patch};
use lemon::element::style::PaintData;
use lemon::{RetainedTree, Text};

#[test]
fn paint_and_chrome_do_not_invalidate_clean_geometry() {
    let mut tree = RetainedTree::mount(Text::new("hello").into_element()).unwrap();
    lemon::layout_pass(
        &mut tree,
        &mut parley::FontContext::new(),
        lemon::Viewport {
            width: 300.0,
            height: 200.0,
        },
    )
    .unwrap();
    tree.apply_patch(Patch::UpdatePaint {
        node: NodePath::root(),
        paint: PaintData::default(),
    })
    .unwrap();
    assert!(!tree.layout_dirty());
    tree.apply_patch(Patch::UpdateWidgetChrome {
        node: NodePath::root(),
        text_input: None,
        scroll_viewport: None,
    })
    .unwrap();
    assert!(!tree.layout_dirty());
}

#[test]
fn geometry_invalidation_survives_later_paint_only_patches() {
    let mut tree = RetainedTree::mount(Text::new("hello").into_element()).unwrap();
    lemon::layout_pass(
        &mut tree,
        &mut parley::FontContext::new(),
        lemon::Viewport {
            width: 300.0,
            height: 200.0,
        },
    )
    .unwrap();
    tree.apply_patch(Patch::UpdateText {
        node: NodePath::root(),
        content: "longer text".into(),
    })
    .unwrap();
    tree.apply_patch(Patch::UpdatePaint {
        node: NodePath::root(),
        paint: PaintData::default(),
    })
    .unwrap();
    assert!(tree.layout_dirty());
}

#[test]
fn visual_style_and_text_color_preserve_layout_and_text_measurements() {
    use lemon::{element::style::Color, ui::UiState, ColumnView, Cursor, Viewport};
    let mut ui = UiState::default();
    ui.mount(|_| {
        ColumnView::new()
            .width(200.0)
            .child(Text::new("cached text"))
            .into_element()
    })
    .unwrap();
    let viewport = Viewport {
        width: 400.0,
        height: 300.0,
    };
    ui.update(viewport, 1.0, None, true).unwrap();
    let root = ui.retained().unwrap().root().unwrap();
    let mut style = root.style.clone();
    let mut text_style = root.children[0].text.as_ref().unwrap().style.clone();
    let map = ui.layout_map().clone();
    assert!(root.children[0]
        .text
        .as_ref()
        .unwrap()
        .parley_layout
        .is_some());
    style.opacity = 0.5;
    style.z_index = 10;
    style.cursor = Cursor::Pointer;
    style.focusable = true;
    text_style.color = Some(Color::rgb8(150, 70, 180));
    ui.apply_patches([
        Patch::UpdateStyle {
            node: NodePath::root(),
            style: style.clone(),
        },
        Patch::UpdateTextStyle {
            node: NodePath::root().child(0),
            style: text_style,
        },
    ])
    .unwrap();
    assert!(!ui.layout_dirty());
    assert!(ui.paint_dirty());
    let cache = ui.retained().unwrap().root().unwrap().children[0]
        .text
        .as_ref()
        .unwrap();
    assert!(cache.parley_layout.is_some());
    assert!(!cache.needs_layout);
    ui.update(viewport, 1.0, None, true).unwrap();
    assert_eq!(ui.layout_map(), &map);
    style.width = Some(lemon::element::style::Dimension::Points(100.0));
    ui.apply_patches([Patch::UpdateStyle {
        node: NodePath::root(),
        style,
    }])
    .unwrap();
    assert!(ui.layout_dirty());
    ui.update(viewport, 1.0, None, true).unwrap();
    let root = ui.retained().unwrap().root().unwrap();
    assert_eq!(
        ui.layout_map().get(root.taffy_id.unwrap()).unwrap().width,
        100.0
    );
}

#[test]
fn viewport_changes_invalidate_geometry_without_manual_flags() {
    use lemon::{ui::UiState, View, Viewport};
    let mut ui = UiState::default();
    ui.mount(|_| View::new().fill().into_element()).unwrap();
    for width in [400.0, 700.0] {
        ui.update(
            Viewport {
                width,
                height: 300.0,
            },
            1.0,
            None,
            true,
        )
        .unwrap();
        let root = ui.retained().unwrap().root().unwrap();
        assert_eq!(
            ui.layout_map().get(root.taffy_id.unwrap()).unwrap().width,
            width
        );
        assert!(!ui.layout_dirty());
    }
}

#[test]
fn invalid_structural_patches_return_errors_without_losing_nodes() {
    use lemon::{Component, Cx, View};
    fn component(_: &Cx) -> lemon::element::Element {
        Text::new("child").into_element()
    }
    let mut tree =
        RetainedTree::mount(View::new().child(Text::new("kept")).into_element()).unwrap();
    let before = tree.layout_tree().total_node_count();
    for patch in [
        Patch::RemoveChild {
            parent: NodePath::root(),
            index: 2,
        },
        Patch::MoveChild {
            parent: NodePath::root(),
            from: 0,
            to: 2,
        },
        Patch::InsertChild {
            parent: NodePath::root(),
            index: 3,
            element: Text::new("invalid").into_element(),
        },
        Patch::UnmountComponent {
            node: NodePath::root(),
        },
        Patch::MountComponent {
            node: NodePath::root().child(3),
            component: match Component::new(component).into_element() {
                lemon::element::Element::Component(value) => value,
                _ => unreachable!(),
            },
        },
    ] {
        assert!(tree.apply_patch(patch).is_err());
        assert_eq!(
            tree.root().unwrap().children[0].text_content(),
            Some("kept")
        );
        assert_eq!(tree.layout_tree().total_node_count(), before);
    }
}
