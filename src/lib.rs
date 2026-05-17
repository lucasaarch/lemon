pub mod diff;
pub mod element;
pub mod layout;
pub mod paint;
pub mod platform;
pub mod retained;
pub mod runtime;

pub use element::builders::{Box_, Button, Column, Component, Row, Text};
pub use element::style::{Color, StyleProps};
pub use layout::{layout_pass, layout_pass_if_dirty, LayoutMap, LayoutRect, Viewport};
pub use paint::{paint_pass, PaintStats};
pub use platform::{run, AppState, WindowConfig};
pub use retained::RetainedTree;
pub use runtime::cx::Cx;
pub use runtime::signal::Signal;
pub use runtime::Runtime;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::Patch;
    use vello::Scene;

    #[test]
    fn counter_increments_produce_update_text_patches() {
        let count = Signal::new(0i32);
        let c = count.clone();

        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let c2 = c.clone();
            Column::new()
                .child(Text::new(move || format!("Count: {}", c2.get())))
                .into_element()
        });

        assert!(rt.take_patches().is_empty(), "no patches on first mount");

        count.set(1);
        rt.flush_effects();

        let patches = rt.take_patches();
        assert!(!patches.is_empty());
        let has_patch = patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateText { content, .. } if content == "Count: 1"));
        assert!(has_patch, "expected UpdateText 'Count: 1'");

        count.set(2);
        rt.flush_effects();
        let patches = rt.take_patches();
        let has_patch = patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateText { content, .. } if content == "Count: 2"));
        assert!(has_patch, "expected UpdateText 'Count: 2'");
    }

    #[test]
    fn conditional_child_produces_insert_and_remove_patches() {
        let show = Signal::new(false);
        let s = show.clone();

        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let visible = s.get();
            let mut col = Column::new();
            if visible {
                col = col.child(Text::new("visible"));
            }
            col.into_element()
        });

        assert!(rt.take_patches().is_empty());

        show.set(true);
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(
            patches
                .iter()
                .any(|p| matches!(p, Patch::InsertChild { .. })),
            "showing child must produce InsertChild"
        );

        show.set(false);
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(
            patches
                .iter()
                .any(|p| matches!(p, Patch::RemoveChild { .. })),
            "hiding child must produce RemoveChild"
        );
    }

    #[test]
    fn multiple_signals_each_trigger_patch() {
        let name = Signal::new("Alice".to_owned());
        let age = Signal::new(30u32);
        let n = name.clone();
        let a = age.clone();

        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let n2 = n.clone();
            let a2 = a.clone();
            Column::new()
                .child(Text::new(move || n2.get()))
                .child(Text::new(move || a2.get().to_string()))
                .into_element()
        });

        name.set("Bob".to_owned());
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateText { content, .. } if content == "Bob")));

        age.set(31);
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateText { content, .. } if content == "31")));
    }

    #[test]
    fn runtime_patches_drive_relayout_height_change() {
        let text = Signal::new("A".to_owned());
        let t = text.clone();

        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let t2 = t.clone();
            Column::new()
                .width(120.0)
                .child(Text::new(move || t2.get()).font_size(16.0))
                .into_element()
        });

        let initial = rt.root_element().expect("root element after mount");
        let mut tree = RetainedTree::mount(initial).unwrap();
        let viewport = Viewport {
            width: 400.0,
            height: 600.0,
        };

        let map_before = layout_pass(&mut tree, viewport, 1.0).unwrap();
        let text_id = tree.root.as_ref().unwrap().children[0].taffy_id.unwrap();
        let height_before = map_before.get(text_id).unwrap().height;

        text.set("Line one\nLine two\nLine three".to_owned());
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(
            patches.iter().any(|p| matches!(p, Patch::UpdateText { .. })),
            "expected UpdateText patch, got {patches:?}"
        );
        tree.apply_patches(patches).unwrap();

        let updated_content = &tree.root.as_ref().unwrap().children[0]
            .text
            .as_ref()
            .unwrap()
            .content;
        assert!(
            updated_content.contains("Line three"),
            "retained text not updated: {updated_content:?}"
        );
        assert!(tree.root.as_ref().unwrap().children[0]
            .text
            .as_ref()
            .unwrap()
            .needs_layout);

        assert!(tree.layout_dirty);
        let map_after = layout_pass_if_dirty(&mut tree, viewport, 1.0)
            .unwrap()
            .expect("layout should run after patches");
        let height_after = map_after.get(text_id).unwrap().height;

        assert!(
            height_after > height_before,
            "height should grow after longer text: before={height_before} after={height_after}"
        );
        assert!(!tree.layout_dirty);
        assert!(!tree.root.as_ref().unwrap().children[0]
            .text
            .as_ref()
            .unwrap()
            .needs_layout);
    }

    #[test]
    fn end_to_end_runtime_layout_paint_without_panic() {
        let count = Signal::new(0i32);
        let c = count.clone();

        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let c2 = c.clone();
            Column::new()
                .width(200.0)
                .gap(8.0)
                .child(Text::new(move || format!("Count: {}", c2.get())).font_size(16.0))
                .child(
                    Button::new("Increment")
                        .width(120.0)
                        .height(40.0)
                        .background(Color::rgb8(50, 100, 150)),
                )
                .into_element()
        });

        let mut tree = RetainedTree::mount(rt.root_element().unwrap()).unwrap();
        let viewport = Viewport {
            width: 400.0,
            height: 600.0,
        };

        let mut scene = Scene::new();
        let layout = layout_pass(&mut tree, viewport, 1.0).unwrap();
        let _stats = paint_pass(&tree, &layout, &mut scene, 1.0);

        count.set(1);
        rt.flush_effects();
        tree.apply_patches(rt.take_patches()).unwrap();
        let layout = layout_pass_if_dirty(&mut tree, viewport, 1.0)
            .unwrap()
            .expect("layout after patch");
        let _stats = paint_pass(&tree, &layout, &mut scene, 2.0);
    }

    #[test]
    fn crate_root_re_exports_component_builder() {
        fn child(_cx: &Cx) -> element::Element {
            Text::new("child").into_element()
        }

        let element = Component::new(child).key(9).into_element();
        let element::Element::Component(component) = element else {
            panic!("expected component element");
        };

        assert_eq!(component.key(), Some(&element::types::Key(9)));
    }
}
