use lemon_core::{element::Element, ui::UiState, ColumnView, RowView, View, Viewport};

#[test]
fn fill_tracks_viewport_resizes_for_every_container() {
    for element in [
        View::new().fill().into_element(),
        RowView::new().fill().into_element(),
        ColumnView::new().fill().into_element(),
    ] {
        let mut ui = UiState::default();
        ui.mount(move |_| element.clone()).unwrap();
        for (width, height) in [(800.0, 600.0), (480.0, 320.0)] {
            ui.layout(Viewport { width, height }).unwrap();
            let root = ui.retained().unwrap().root().unwrap();
            let rect = ui.layout_map().get(root.taffy_id.unwrap()).unwrap();
            assert_eq!((rect.width, rect.height), (width, height));
        }
    }
}

#[test]
fn fill_axes_preserve_fixed_other_axis_and_respect_builder_order() {
    for (element, expected) in [
        (
            View::new().height(120.0).fill_width().into_element(),
            (640.0, 120.0),
        ),
        (
            View::new().width(180.0).fill_height().into_element(),
            (180.0, 480.0),
        ),
        (
            View::new().fill().width(180.0).height(120.0).into_element(),
            (180.0, 120.0),
        ),
    ] {
        let mut ui = UiState::default();
        ui.mount(move |_| element.clone()).unwrap();
        ui.layout(Viewport {
            width: 640.0,
            height: 480.0,
        })
        .unwrap();
        let root = ui.retained().unwrap().root().unwrap();
        let rect = ui.layout_map().get(root.taffy_id.unwrap()).unwrap();
        assert_eq!((rect.width, rect.height), expected);
    }
}

#[test]
fn nested_fill_uses_parent_content_area() {
    let root: Element = View::new()
        .width(400.0)
        .height(300.0)
        .inset(20.0)
        .child(View::new().fill())
        .into_element();
    let mut ui = UiState::default();
    ui.mount(move |_| root.clone()).unwrap();
    ui.layout(Viewport {
        width: 800.0,
        height: 600.0,
    })
    .unwrap();
    let child = &ui.retained().unwrap().root().unwrap().children[0];
    let rect = ui.layout_map().get(child.taffy_id.unwrap()).unwrap();
    assert_eq!((rect.width, rect.height), (360.0, 260.0));
}

#[test]
fn reactive_fill_keeps_columns_vertical() {
    use lemon_core::Signal;
    let fill = Signal::new(false);
    let source = fill.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        let column = ColumnView::new()
            .child(View::new().width(20.0).height(10.0))
            .child(View::new().width(20.0).height(10.0));
        if source.get() {
            column.fill()
        } else {
            column.width(100.0).height(100.0)
        }
        .into_element()
    })
    .unwrap();
    let viewport = Viewport {
        width: 400.0,
        height: 300.0,
    };
    ui.update(viewport, 1.0, None, true).unwrap();
    fill.set(true);
    ui.update(viewport, 1.0, None, true).unwrap();
    let root = ui.retained().unwrap().root().unwrap();
    let first = ui
        .layout_map()
        .get(root.children[0].taffy_id.unwrap())
        .unwrap();
    let second = ui
        .layout_map()
        .get(root.children[1].taffy_id.unwrap())
        .unwrap();
    assert_eq!(first.x, second.x);
    assert_eq!(second.y, first.y + first.height);
}
