use lemon_core::diff::{NodePath, Patch};
use lemon_core::element::Element;
use lemon_core::host::HostContext;
use lemon_core::ui::UiState;
use lemon_core::{Color, Column, Signal, Text, Theme, Viewport};

fn viewport() -> Viewport {
    Viewport {
        width: 400.0,
        height: 300.0,
    }
}

#[test]
fn batches_reactive_updates_until_geometry_is_requested() {
    let text = Signal::new("first".to_owned());
    let source = text.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| Text::new(source.get()).into_element())
        .unwrap();
    ui.update(viewport(), 1.0, None, true).unwrap();
    assert!(!ui.layout_dirty);
    text.set("second".into());
    ui.apply_updates().unwrap();
    text.set("third".into());
    ui.apply_updates().unwrap();
    assert!(ui.layout_dirty);
    assert!(ui.paint_dirty);
    assert_eq!(
        ui.retained
            .as_ref()
            .unwrap()
            .root
            .as_ref()
            .unwrap()
            .text_content(),
        Some("third")
    );
    ui.update(viewport(), 1.0, None, true).unwrap();
    assert!(!ui.layout_dirty);
    assert!(!ui.paint_dirty);
}

#[test]
fn paint_only_runtime_updates_keep_text_layout_cache() {
    let color = Signal::new(Color::rgb8(1, 2, 3));
    let source = color.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        Column::new()
            .background(source.get())
            .child(Text::new("stable"))
            .into_element()
    })
    .unwrap();
    ui.update(viewport(), 1.0, None, true).unwrap();
    color.set(Color::rgb8(4, 5, 6));
    ui.apply_updates().unwrap();
    assert!(!ui.layout_dirty);
    assert!(ui.paint_dirty);
    assert!(!ui.retained.as_ref().unwrap().text_needs_reflow());
}

#[test]
fn render_uses_originating_theme_even_when_another_host_sets_signal() {
    let host = HostContext::new(Theme::default_light());
    let other = HostContext::new(Theme::default_dark());
    let signal = Signal::new(0);
    let source = signal.clone();
    let mut ui = UiState::default();
    {
        let _scope = host.enter();
        ui.mount(move |cx| {
            source.get();
            assert_eq!(cx.use_theme(), Theme::default_light());
            assert_eq!(lemon_core::current_theme(), Theme::default_light());
            Text::new("light").into_element()
        })
        .unwrap();
    }
    let _scope = other.enter();
    signal.set(1);
    ui.apply_updates().unwrap();
    assert_eq!(lemon_core::current_theme(), Theme::default_dark());
}

#[test]
fn scroll_content_marker_survives_an_extra_wrapper_and_reactive_updates() {
    use lemon_core::{layout, Overflow, View};
    let make = |offset: f32| -> Element {
        let mut content = View::new().height(600.0).into_element();
        if let Element::View(node) = &mut content {
            node.style.flex_shrink = Some(0.0);
        }
        View::new()
            .height(100.0)
            .overflow(Overflow::Hidden)
            .on_scroll(|_| {})
            .child(
                View::new().child(
                    View::new()
                        .scroll_content()
                        .margin_top(-offset)
                        .child(content),
                ),
            )
            .into_element()
    };
    let mut ui = UiState::default();
    ui.mount(move |_| make(0.0)).unwrap();
    ui.update(viewport(), 1.0, None, true).unwrap();
    let tree = ui.retained.as_mut().unwrap();
    let content = layout::scroll_content_node(tree.root.as_ref().unwrap()).unwrap();
    assert!(content.style.scroll_content);
    let extent =
        layout::scroll_content_extent(tree.root.as_ref().unwrap(), &ui.layout_map).unwrap();
    assert_eq!(extent, 600.0);
    tree.apply_patches(lemon_core::diff::diff(
        make(0.0),
        make(30.0),
        NodePath::root(),
    ))
    .unwrap();
    ui.layout(viewport()).unwrap();
    let root = ui.retained.as_ref().unwrap().root.as_ref().unwrap();
    assert_eq!(
        layout::scroll_content_extent(root, &ui.layout_map),
        Some(extent)
    );
    // A paint-only mutation must not undo geometry invalidation.
    ui.retained
        .as_mut()
        .unwrap()
        .apply_patch(Patch::UpdatePaint {
            node: NodePath::root(),
            paint: Default::default(),
        })
        .unwrap();
    assert!(!ui.retained.as_ref().unwrap().layout_dirty);
}
