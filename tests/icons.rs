use lemon::{icons::lucide, paint::paint_pass, prelude::*, ui::UiState, Viewport};

#[test]
fn vector_icons_flow_through_freezing_diff_and_paint_without_layout_changes() {
    let color = State::new(Color::rgb8(10, 20, 30));
    let value = color.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        Icon::new(lucide::TAG)
            .size(32.0)
            .color(value.get())
            .stroke_width(1.5)
            .into_element()
    })
    .unwrap();
    let viewport = Viewport {
        width: 200.0,
        height: 200.0,
    };
    ui.update(viewport, 2.0, None, false).unwrap();
    let geometry = ui.layout_map().clone();
    let id = ui.retained().unwrap().root().unwrap().taffy_id;
    let stats = paint_pass(
        ui.retained().unwrap(),
        ui.layout_map(),
        &mut vello::Scene::new(),
        2.0,
        None,
        false,
    );
    assert_eq!(stats.strokes, 1);
    assert_eq!(stats.fills, 1, "filled Lucide details must also be drawn");
    color.set(Color::rgb8(100, 120, 130));
    ui.update(viewport, 2.0, None, false).unwrap();
    let root = ui.retained().unwrap().root().unwrap();
    assert_eq!(root.taffy_id, id);
    assert_eq!(root.paint.icon.as_ref().unwrap().color, color.peek());
    assert_eq!(ui.layout_map(), &geometry);
}

#[test]
fn zero_size_icons_draw_nothing() {
    let mut ui = UiState::default();
    ui.mount(|_| Icon::new(lucide::FOLDER).size(0.0).into_element())
        .unwrap();
    ui.update(
        Viewport {
            width: 200.0,
            height: 200.0,
        },
        1.0,
        None,
        false,
    )
    .unwrap();
    let stats = paint_pass(
        ui.retained().unwrap(),
        ui.layout_map(),
        &mut vello::Scene::new(),
        1.0,
        None,
        false,
    );
    assert_eq!(stats.strokes, 0);
    assert_eq!(stats.fills, 0);
}
