use lemon::{
    element::Element, ui::UiState, ColumnView, Distribute, Placement, RowView, View, Viewport,
};

#[test]
fn physical_placement_is_independent_of_container_direction() {
    for (placement, expected) in [
        (Placement::TopLeft, (10.0, 10.0)),
        (Placement::Top, (150.0, 10.0)),
        (Placement::TopRight, (290.0, 10.0)),
        (Placement::Left, (10.0, 115.0)),
        (Placement::Center, (150.0, 115.0)),
        (Placement::Right, (290.0, 115.0)),
        (Placement::BottomLeft, (10.0, 220.0)),
        (Placement::Bottom, (150.0, 220.0)),
        (Placement::BottomRight, (290.0, 220.0)),
    ] {
        for element in [
            View::new()
                .fill()
                .inset(10.0)
                .place(placement)
                .child(View::new().width(20.0).height(10.0))
                .into_element(),
            RowView::new()
                .fill()
                .inset(10.0)
                .place(placement)
                .child(View::new().width(20.0).height(10.0))
                .into_element(),
            ColumnView::new()
                .fill()
                .inset(10.0)
                .place(placement)
                .child(View::new().width(20.0).height(10.0))
                .into_element(),
        ] {
            let mut ui = UiState::default();
            ui.mount(move |_| element.clone()).unwrap();
            ui.layout(Viewport {
                width: 320.0,
                height: 240.0,
            })
            .unwrap();
            let root = ui.retained().unwrap().root().unwrap();
            let rect = ui
                .layout_map()
                .get(root.children[0].taffy_id.unwrap())
                .unwrap();
            assert_eq!(
                (rect.x, rect.y),
                expected,
                "{placement:?} in {:?}",
                root.kind
            );
        }
    }
}

#[test]
fn centered_group_keeps_column_spacing_after_reactive_repositioning() {
    use lemon::State;
    let placement = State::new(Placement::TopLeft);
    let source = placement.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        ColumnView::new()
            .fill()
            .place(source.get())
            .spacing(10.0)
            .child(View::new().width(40.0).height(20.0))
            .child(View::new().width(40.0).height(20.0))
            .into_element()
    })
    .unwrap();
    ui.update(
        Viewport {
            width: 400.0,
            height: 300.0,
        },
        1.0,
        None,
        true,
    )
    .unwrap();
    placement.set(Placement::Center);
    ui.update(
        Viewport {
            width: 400.0,
            height: 300.0,
        },
        1.0,
        None,
        true,
    )
    .unwrap();
    let root = ui.retained().unwrap().root().unwrap();
    let first = ui
        .layout_map()
        .get(root.children[0].taffy_id.unwrap())
        .unwrap();
    let second = ui
        .layout_map()
        .get(root.children[1].taffy_id.unwrap())
        .unwrap();
    assert_eq!((first.x, first.y), (180.0, 125.0));
    assert_eq!((second.x, second.y), (180.0, 155.0));
}

#[test]
fn distribution_uses_stacking_direction_without_overwriting_other_axis() {
    for element in [
        RowView::new()
            .width(100.0)
            .height(100.0)
            .center()
            .distribute(Distribute::Between)
            .child(View::new().width(10.0).height(10.0))
            .child(View::new().width(10.0).height(10.0))
            .into_element(),
        ColumnView::new()
            .width(100.0)
            .height(100.0)
            .center()
            .distribute(Distribute::Between)
            .child(View::new().width(10.0).height(10.0))
            .child(View::new().width(10.0).height(10.0))
            .into_element(),
    ] {
        let vertical = matches!(&element, Element::Column(_));
        let mut ui = UiState::default();
        ui.mount(move |_| element.clone()).unwrap();
        ui.layout(Viewport {
            width: 100.0,
            height: 100.0,
        })
        .unwrap();
        let root = ui.retained().unwrap().root().unwrap();
        let first = ui
            .layout_map()
            .get(root.children[0].taffy_id.unwrap())
            .unwrap();
        let second = ui
            .layout_map()
            .get(root.children[1].taffy_id.unwrap())
            .unwrap();
        let expected = if vertical {
            ((45.0, 0.0), (45.0, 90.0))
        } else {
            ((0.0, 45.0), (90.0, 45.0))
        };
        assert_eq!(((first.x, first.y), (second.x, second.y)), expected);
    }
}
