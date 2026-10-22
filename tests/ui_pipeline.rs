use lemon::diff::{NodePath, Patch};
use lemon::element::Element;
use lemon::host::HostContext;
use lemon::ui::UiState;
use lemon::{Color, ColumnView, State, Text, Theme, Viewport};

fn viewport() -> Viewport {
    Viewport {
        width: 400.0,
        height: 300.0,
    }
}

#[test]
fn batches_reactive_updates_until_geometry_is_requested() {
    let text = State::new("first".to_owned());
    let source = text.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| Text::new(source.get()).into_element())
        .unwrap();
    ui.update(viewport(), 1.0, None, true).unwrap();
    assert!(!ui.layout_dirty());
    text.set("second".into());
    ui.apply_updates().unwrap();
    text.set("third".into());
    ui.apply_updates().unwrap();
    assert!(ui.layout_dirty());
    assert!(ui.paint_dirty());
    assert_eq!(
        ui.retained().unwrap().root().unwrap().text_content(),
        Some("third")
    );
    ui.update(viewport(), 1.0, None, true).unwrap();
    assert!(!ui.layout_dirty());
    assert!(!ui.paint_dirty());
}

#[test]
fn paint_only_runtime_updates_keep_text_layout_cache() {
    let color = State::new(Color::rgb8(1, 2, 3));
    let source = color.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        ColumnView::new()
            .surface(source.get())
            .child(Text::new("stable"))
            .into_element()
    })
    .unwrap();
    ui.update(viewport(), 1.0, None, true).unwrap();
    color.set(Color::rgb8(4, 5, 6));
    ui.apply_updates().unwrap();
    assert!(!ui.layout_dirty());
    assert!(ui.paint_dirty());
    assert!(!ui.retained().unwrap().text_needs_reflow());
}

#[test]
fn render_uses_originating_theme_even_when_another_host_sets_signal() {
    let host = HostContext::new(Theme::default_light());
    let other = HostContext::new(Theme::default_dark());
    let signal = State::new(0);
    let source = signal.clone();
    let mut ui = UiState::default();
    {
        let _scope = host.enter();
        ui.mount(move |cx| {
            source.get();
            assert_eq!(cx.theme(), Theme::default_light());
            assert_eq!(lemon::current_theme(), Theme::default_light());
            Text::new("light").into_element()
        })
        .unwrap();
    }
    let _scope = other.enter();
    signal.set(1);
    ui.apply_updates().unwrap();
    assert_eq!(lemon::current_theme(), Theme::default_dark());
}

#[test]
fn scroll_content_marker_survives_an_extra_wrapper_and_reactive_updates() {
    use lemon::{layout, View};
    let make = |offset: f32| -> Element {
        let mut content = View::new().height(600.0).into_element();
        if let Element::View(node) = &mut content {
            node.style.flex_shrink = Some(0.0);
        }
        View::new()
            .height(100.0)
            .clip()
            .on_scroll(|_| {})
            .child(
                View::new().child(
                    View::new()
                        .scroll_content()
                        .outset_top(-offset)
                        .child(content),
                ),
            )
            .into_element()
    };
    let mut ui = UiState::default();
    ui.mount(move |_| make(0.0)).unwrap();
    ui.update(viewport(), 1.0, None, true).unwrap();
    let tree = ui.retained().unwrap();
    let content = layout::scroll_content_node(tree.root().unwrap()).unwrap();
    assert!(content.style.scroll_content);
    let extent = layout::scroll_content_extent(tree.root().unwrap(), ui.layout_map()).unwrap();
    assert_eq!(extent, 600.0);
    ui.apply_patches(lemon::diff::diff(make(0.0), make(30.0), NodePath::root()))
        .unwrap();
    ui.layout(viewport()).unwrap();
    let root = ui.retained().unwrap().root().unwrap();
    assert_eq!(
        layout::scroll_content_extent(root, ui.layout_map()),
        Some(extent)
    );
    // A paint-only mutation must not undo geometry invalidation.
    ui.apply_patches([Patch::UpdatePaint {
        node: NodePath::root(),
        paint: Default::default(),
    }])
    .unwrap();
    assert!(!ui.retained().unwrap().layout_dirty());
}

#[test]
fn unmarked_scroll_wrappers_are_not_inferred_from_tree_depth() {
    use lemon::{layout, View};
    let mut ui = UiState::default();
    ui.mount(|_| {
        View::new()
            .height(80.0)
            .clip()
            .on_scroll(|_| {})
            .child(View::new().child(View::new().height(600.0)))
            .into_element()
    })
    .unwrap();
    ui.layout(viewport()).unwrap();
    let root = ui.retained().unwrap().root().unwrap();
    assert!(layout::scroll_content_node(root).is_none());
    assert!(layout::scroll_content_extent(root, ui.layout_map()).is_none());
}
