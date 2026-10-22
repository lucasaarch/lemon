use lemon_core::diff::{NodePath, Patch};
use lemon_core::element::style::PaintData;
use lemon_core::{RetainedTree, Text};

#[test]
fn paint_and_chrome_do_not_invalidate_clean_geometry() {
    let mut tree = RetainedTree::mount(Text::new("hello").into_element()).unwrap();
    tree.layout_dirty = false;
    tree.apply_patch(Patch::UpdatePaint {
        node: NodePath::root(),
        paint: PaintData::default(),
    })
    .unwrap();
    assert!(!tree.layout_dirty);
    tree.apply_patch(Patch::UpdateWidgetChrome {
        node: NodePath::root(),
        text_input: None,
        scroll_viewport: false,
        scroll_bar: false,
    })
    .unwrap();
    assert!(!tree.layout_dirty);
}

#[test]
fn geometry_invalidation_survives_later_paint_only_patches() {
    let mut tree = RetainedTree::mount(Text::new("hello").into_element()).unwrap();
    tree.layout_dirty = false;
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
    assert!(tree.layout_dirty);
}
