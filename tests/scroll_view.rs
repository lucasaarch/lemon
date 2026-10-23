use lemon::{
    input::{hit_test::LogicalPoint, InputEvent},
    prelude::*,
    ui::UiState,
    Viewport,
};

#[test]
fn both_axes_scroll_clamp_and_paint_with_explicit_content_wrappers() {
    for axis in [ScrollAxis::Vertical, ScrollAxis::Horizontal] {
        let offset = State::new(0.0);
        let value = offset.clone();
        let mut ui = UiState::default();
        ui.mount(move |_| {
            ScrollView::with_offset(value.clone(), View::new().width(600.0).height(600.0))
                .axis(axis)
                .width(200.0)
                .height(100.0)
                .into_element()
        })
        .unwrap();
        let viewport = Viewport {
            width: 400.0,
            height: 300.0,
        };
        ui.update(viewport, 1.0, None, false).unwrap();
        let delta = match axis {
            ScrollAxis::Vertical => ScrollDelta::new(0.0, -9999.0),
            ScrollAxis::Horizontal => ScrollDelta::new(-9999.0, 0.0),
        };
        ui.dispatch_input(InputEvent::Scroll {
            point: LogicalPoint::new(20.0, 20.0),
            delta,
        })
        .unwrap();
        ui.update(viewport, 1.0, None, false).unwrap();
        assert_eq!(
            offset.peek(),
            match axis {
                ScrollAxis::Vertical => 500.0,
                ScrollAxis::Horizontal => 400.0,
            }
        );
        let root = ui.retained().unwrap().root().unwrap();
        let stats = lemon::paint_pass(
            ui.retained().unwrap(),
            ui.layout_map(),
            &mut vello::Scene::new(),
            1.0,
            None,
            false,
        );
        assert_eq!(
            stats.fills, 2,
            "one track and one thumb on the selected axis"
        );
        let content = &root.children[0];
        let rect = ui.layout_map().get(content.taffy_id.unwrap()).unwrap();
        match axis {
            ScrollAxis::Vertical => assert_eq!(rect.y, -500.0),
            ScrollAxis::Horizontal => assert_eq!(rect.x, -400.0),
        }
    }
}

#[test]
fn perpendicular_wheel_movement_reaches_the_parent_viewport() {
    let vertical = State::new(0.0);
    let horizontal = State::new(0.0);
    let v = vertical.clone();
    let h = horizontal.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        ScrollView::with_offset(
            v.clone(),
            ColumnView::new()
                .child(
                    ScrollView::with_offset(h.clone(), View::new().width(600.0).height(80.0))
                        .axis(ScrollAxis::Horizontal)
                        .width(200.0)
                        .height(80.0),
                )
                .child(View::new().height(500.0).width(200.0)),
        )
        .width(200.0)
        .height(100.0)
        .into_element()
    })
    .unwrap();
    let viewport = Viewport {
        width: 400.0,
        height: 300.0,
    };
    ui.update(viewport, 1.0, None, false).unwrap();
    ui.dispatch_input(InputEvent::Scroll {
        point: LogicalPoint::new(20.0, 20.0),
        delta: ScrollDelta::new(-30.0, 0.0),
    })
    .unwrap();
    ui.update(viewport, 1.0, None, false).unwrap();
    assert_eq!(horizontal.peek(), 30.0);
    assert_eq!(vertical.peek(), 0.0);
    ui.dispatch_input(InputEvent::Scroll {
        point: LogicalPoint::new(20.0, 20.0),
        delta: ScrollDelta::new(0.0, -20.0),
    })
    .unwrap();
    ui.update(viewport, 1.0, None, false).unwrap();
    assert_eq!(horizontal.peek(), 30.0);
    assert_eq!(vertical.peek(), 20.0);
}

#[test]
fn horizontal_content_keeps_its_natural_text_width() {
    let mut ui = UiState::default();
    ui.mount(|cx| {
        ScrollView::new(
            cx,
            Text::new(
                "Long content should scroll sideways rather than wrap into several narrow lines",
            ),
        )
        .axis(ScrollAxis::Horizontal)
        .width(120.0)
        .height(40.0)
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
        false,
    )
    .unwrap();
    let root = ui.retained().unwrap().root().unwrap();
    let text = &root.children[0].children[0];
    let rect = ui.layout_map().get(text.taffy_id.unwrap()).unwrap();
    assert!(
        rect.width > 120.0,
        "horizontal content must remain unwrapped: {rect:?}"
    );
    assert!(
        rect.height < 30.0,
        "single-line text must keep its natural height: {rect:?}"
    );
}

#[test]
fn paint_only_parent_renders_keep_measured_scroll_limits_before_input() {
    let color = State::new(Color::rgb8(1, 2, 3));
    let paint = color.clone();
    let offset = State::new(0.0);
    let scroll = offset.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        View::new()
            .surface(paint.get())
            .child(
                ScrollView::with_offset(scroll.clone(), View::new().width(600.0).height(40.0))
                    .axis(ScrollAxis::Horizontal)
                    .width(200.0)
                    .height(60.0),
            )
            .into_element()
    })
    .unwrap();
    let viewport = Viewport {
        width: 400.0,
        height: 300.0,
    };
    ui.update(viewport, 1.0, None, false).unwrap();
    color.set(Color::rgb8(3, 2, 1));
    ui.dispatch_input(InputEvent::Scroll {
        point: LogicalPoint::new(20.0, 20.0),
        delta: ScrollDelta::new(-9999.0, 0.0),
    })
    .unwrap();
    assert_eq!(
        offset.peek(),
        400.0,
        "new handlers must receive the existing measured limit without waiting for layout"
    );
}
