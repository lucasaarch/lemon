use lemon::{
    input::{hit_test::LogicalPoint, InputEvent},
    prelude::*,
    ui::UiState,
    Viewport,
};

#[test]
fn both_axes_scroll_clamp_and_paint_with_explicit_content_wrappers() {
    for axis in [
        ScrollAxis::Vertical,
        ScrollAxis::Horizontal,
        ScrollAxis::Both,
    ] {
        let offset = State::new(ScrollOffset::default());
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
            ScrollAxis::Both => ScrollDelta::new(-9999.0, -9999.0),
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
                ScrollAxis::Vertical => ScrollOffset::new(0.0, 500.0),
                ScrollAxis::Horizontal => ScrollOffset::new(400.0, 0.0),
                ScrollAxis::Both => ScrollOffset::new(400.0, 500.0),
            }
        );
        let root = ui.retained().unwrap().root().unwrap();
        let stats = lemon::paint_pass(
            ui.retained().unwrap(),
            ui.layout_map(),
            &mut lemon::Scene::new(),
            1.0,
            None,
            false,
        );
        assert_eq!(
            stats.fills,
            if axis == ScrollAxis::Both { 4 } else { 2 },
            "one track and one thumb on each enabled overflowing axis"
        );
        let content = &root.children[0];
        let rect = ui.layout_map().get(content.taffy_id.unwrap()).unwrap();
        match axis {
            ScrollAxis::Vertical => assert_eq!(rect.y, -500.0),
            ScrollAxis::Horizontal => assert_eq!(rect.x, -400.0),
            ScrollAxis::Both => {
                assert_eq!(rect.x, -400.0);
                assert_eq!(rect.y, -500.0);
            }
        }
    }
}

#[test]
fn perpendicular_wheel_movement_reaches_the_parent_viewport() {
    let vertical = State::new(ScrollOffset::default());
    let horizontal = State::new(ScrollOffset::default());
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
    assert_eq!(horizontal.peek(), ScrollOffset::new(30.0, 0.0));
    assert_eq!(vertical.peek(), ScrollOffset::default());
    ui.dispatch_input(InputEvent::Scroll {
        point: LogicalPoint::new(20.0, 20.0),
        delta: ScrollDelta::new(0.0, -20.0),
    })
    .unwrap();
    ui.update(viewport, 1.0, None, false).unwrap();
    assert_eq!(horizontal.peek(), ScrollOffset::new(30.0, 0.0));
    assert_eq!(vertical.peek(), ScrollOffset::new(0.0, 20.0));
}

#[test]
fn horizontally_scrolling_content_keeps_its_natural_text_width() {
    for axis in [ScrollAxis::Horizontal, ScrollAxis::Both] {
        let mut ui = UiState::default();
        ui.mount(move |cx| {
            ScrollView::new(
            cx,
            Text::new(
                "Long content should scroll sideways rather than wrap into several narrow lines",
            ),
        )
        .axis(axis)
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
}

#[test]
fn paint_only_parent_renders_keep_measured_scroll_limits_before_input() {
    let color = State::new(Color::rgb8(1, 2, 3));
    let paint = color.clone();
    let offset = State::new(ScrollOffset::default());
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
        ScrollOffset::new(400.0, 0.0),
        "new handlers must receive the existing measured limit without waiting for layout"
    );
}

const VIEWPORT: Viewport = Viewport {
    width: 400.0,
    height: 300.0,
};
fn update(ui: &mut UiState) {
    ui.update(VIEWPORT, 1.0, ui.focused(), false).unwrap();
}
fn wheel(ui: &mut UiState, point: (f32, f32), x: f64, y: f64) {
    ui.dispatch_input(InputEvent::Scroll {
        point: LogicalPoint::new(point.0, point.1),
        delta: ScrollDelta::new(x, y),
    })
    .unwrap();
    update(ui);
}
#[test]
fn both_axes_accept_diagonal_and_independent_movement_including_opposite_signs() {
    let offset = State::new(ScrollOffset::new(100.0, 100.0));
    let value = offset.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        ScrollView::with_offset(value.clone(), View::new().width(600.0).height(400.0))
            .axis(ScrollAxis::Both)
            .width(200.0)
            .height(100.0)
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    wheel(&mut ui, (20.0, 20.0), -30.0, 30.0);
    assert_eq!(offset.get(), ScrollOffset::new(130.0, 70.0));
    wheel(&mut ui, (20.0, 20.0), -20.0, 0.0);
    assert_eq!(offset.get(), ScrollOffset::new(150.0, 70.0));
    wheel(&mut ui, (20.0, 20.0), 0.0, -40.0);
    assert_eq!(offset.get(), ScrollOffset::new(150.0, 110.0));
    wheel(&mut ui, (20.0, 20.0), 9999.0, -9999.0);
    assert_eq!(offset.get(), ScrollOffset::new(0.0, 300.0));
}
#[test]
fn both_limits_reclamp_after_content_viewport_and_external_offset_changes() {
    let offset = State::new(ScrollOffset::new(9999.0, 9999.0));
    let value = offset.clone();
    let content = State::new((600.0, 300.0));
    let size = content.clone();
    let viewport = State::new((200.0, 100.0));
    let port = viewport.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        let (w, h) = size.get();
        let (vw, vh) = port.get();
        ScrollView::with_offset(value.clone(), View::new().width(w).height(h))
            .axis(ScrollAxis::Both)
            .width(vw)
            .height(vh)
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    assert_eq!(offset.get(), ScrollOffset::new(400.0, 200.0));
    content.set((250.0, 300.0));
    update(&mut ui);
    assert_eq!(offset.get(), ScrollOffset::new(50.0, 200.0));
    viewport.set((260.0, 100.0));
    update(&mut ui);
    assert_eq!(offset.get(), ScrollOffset::new(0.0, 200.0));
    offset.set(ScrollOffset::new(5000.0, 5000.0));
    update(&mut ui);
    assert_eq!(offset.get(), ScrollOffset::new(0.0, 200.0));
    content.set((200.0, 60.0));
    update(&mut ui);
    assert_eq!(offset.get(), ScrollOffset::default());
}
#[test]
fn nested_both_viewports_keep_their_clips_and_scroll_ranges_separate() {
    let outer = State::new(ScrollOffset::default());
    let o = outer.clone();
    let inner = State::new(ScrollOffset::default());
    let i = inner.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        ScrollView::with_offset(
            o.clone(),
            View::new().width(300.0).height(220.0).child(
                ScrollView::with_offset(i.clone(), View::new().width(800.0).height(600.0))
                    .axis(ScrollAxis::Both)
                    .width(120.0)
                    .height(80.0)
                    .decorate(|v| v.position(Position::Absolute).left(20.0).top(20.0)),
            ),
        )
        .axis(ScrollAxis::Both)
        .width(200.0)
        .height(100.0)
        .into_element()
    })
    .unwrap();
    update(&mut ui);
    let root = ui.retained().unwrap().root().unwrap();
    assert_eq!(
        lemon::scroll_content_max_offset(root, ui.layout_map()),
        Some(ScrollOffset::new(100.0, 120.0))
    );
    let nested = &root.children[0].children[0].children[0];
    let rect = ui.layout_map().get(nested.taffy_id.unwrap()).unwrap();
    assert_eq!((rect.width, rect.height), (120.0, 80.0));
    wheel(&mut ui, (40.0, 40.0), -30.0, -40.0);
    assert_eq!(inner.get(), ScrollOffset::new(30.0, 40.0));
    assert_eq!(outer.get(), ScrollOffset::default());
    wheel(&mut ui, (180.0, 90.0), -9999.0, -9999.0);
    assert_eq!(outer.get(), ScrollOffset::new(100.0, 120.0));
    assert_eq!(inner.get(), ScrollOffset::new(30.0, 40.0));
}
#[test]
fn both_paints_only_overflowing_axes_and_refreshes_limits_after_paint_only_renders() {
    for (w, h, fills) in [
        (600.0, 400.0, 4),
        (100.0, 400.0, 2),
        (600.0, 50.0, 2),
        (100.0, 50.0, 0),
    ] {
        let offset = State::new(ScrollOffset::default());
        let value = offset.clone();
        let color = State::new(Color::rgb8(1, 2, 3));
        let tint = color.clone();
        let mut ui = UiState::default();
        ui.mount(move |_| {
            View::new()
                .surface(tint.get())
                .child(
                    ScrollView::with_offset(value.clone(), View::new().width(w).height(h))
                        .axis(ScrollAxis::Both)
                        .width(200.0)
                        .height(100.0),
                )
                .into_element()
        })
        .unwrap();
        update(&mut ui);
        color.set(Color::rgb8(3, 2, 1));
        wheel(&mut ui, (20.0, 20.0), -9999.0, -9999.0);
        assert_eq!(
            offset.get(),
            ScrollOffset::new(
                f64::from((w - 200.0).max(0.0)),
                f64::from((h - 100.0).max(0.0))
            )
        );
        let stats = lemon::paint_pass(
            ui.retained().unwrap(),
            ui.layout_map(),
            &mut lemon::Scene::new(),
            1.0,
            None,
            false,
        );
        assert_eq!(
            stats.fills,
            fills + 1,
            "both scrollbars plus the parent background"
        );
    }
}

#[test]
fn both_offsets_move_hit_targets_together_with_the_content() {
    let offset = State::new(ScrollOffset::new(100.0, 100.0));
    let clicked = State::new(false);
    let target = clicked.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        let target = target.clone();
        ScrollView::with_offset(
            offset.clone(),
            View::new().width(600.0).height(400.0).child(
                View::new()
                    .position(Position::Absolute)
                    .left(250.0)
                    .top(180.0)
                    .width(40.0)
                    .height(20.0)
                    .action(move || target.set(true)),
            ),
        )
        .axis(ScrollAxis::Both)
        .width(200.0)
        .height(100.0)
        .into_element()
    })
    .unwrap();
    update(&mut ui);
    ui.dispatch_input(InputEvent::Click(LogicalPoint::new(160.0, 90.0)))
        .unwrap();
    assert!(
        clicked.get(),
        "hit testing must use both translated coordinates"
    );
}
