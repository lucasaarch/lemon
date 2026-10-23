use lemon::{
    input::{hit_test::LogicalPoint, InputEvent},
    prelude::*,
    ui::UiState,
    Viewport,
};
use std::{cell::RefCell, rc::Rc};
const SIZE: Viewport = Viewport {
    width: 300.0,
    height: 200.0,
};
fn update(ui: &mut UiState) {
    ui.update(SIZE, 1.0, ui.focused(), true).unwrap();
}
fn event(ui: &mut UiState, event: InputEvent) {
    ui.dispatch_input(event).unwrap();
}
fn point(x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(x, y)
}
#[test]
fn focus_notifies_once_on_click_tab_blur_and_unmount_and_flushes_callback_updates() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let events = log.clone();
    let visible = State::new(true);
    let show = visible.clone();
    let focused = State::new(false);
    let focused_state = focused.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        let a = events.clone();
        let b = events.clone();
        let state = focused_state.clone();
        ColumnView::new()
            .fill()
            .children(children![
                if show.get() {
                    View::new()
                        .key(1)
                        .width(80.0)
                        .height(40.0)
                        .focusable()
                        .on_focus(move |value| {
                            a.borrow_mut().push((1, value));
                            state.set(value);
                        })
                        .into_element()
                } else {
                    View::new().key(3).width(80.0).height(40.0).into_element()
                },
                View::new()
                    .key(2)
                    .width(80.0)
                    .height(40.0)
                    .focusable()
                    .on_focus(move |value| b.borrow_mut().push((2, value))),
                Text::new(format!("focused: {}", focused_state.get())),
            ])
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    event(&mut ui, InputEvent::Click(point(10.0, 10.0)));
    assert!(focused.get());
    // Callback mutations are rendered within the same dispatch, without a second event.
    fn has_text(node: &lemon::retained::RetainedNode) -> bool {
        node.text
            .as_ref()
            .is_some_and(|t| t.content == "focused: true")
            || node.children.iter().any(has_text)
    }
    assert!(has_text(ui.retained().unwrap().root().unwrap()));
    event(&mut ui, InputEvent::Click(point(10.0, 10.0))); // no duplicate
    event(
        &mut ui,
        InputEvent::Key(KeyEvent {
            key: LemonKey::Named(NamedKey::Tab),
            state: KeyState::Pressed,
            repeat: false,
            modifiers: Modifiers::default(),
        }),
    );
    event(&mut ui, InputEvent::Click(point(250.0, 150.0)));
    event(&mut ui, InputEvent::Click(point(10.0, 10.0)));
    visible.set(false);
    update(&mut ui);
    assert_eq!(
        *log.borrow(),
        vec![
            (1, true),
            (1, false),
            (2, true),
            (2, false),
            (1, true),
            (1, false)
        ]
    );
    assert!(!focused.get());
}
#[test]
fn hover_includes_descendants_and_exits_on_window_leave_hide_or_removal() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let events = log.clone();
    let mode = State::new(0);
    let shown = mode.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        let parent = events.clone();
        let child = events.clone();
        let mode = shown.get();
        View::new()
            .fill()
            .child(if mode == 2 {
                View::new().key(9).into_element()
            } else {
                View::new()
                    .key(1)
                    .width(100.0)
                    .height(100.0)
                    .interactive(mode == 0)
                    .on_hover(move |inside| parent.borrow_mut().push((1, inside)))
                    .child(
                        View::new()
                            .width(40.0)
                            .height(40.0)
                            .on_hover(move |inside| child.borrow_mut().push((2, inside))),
                    )
                    .into_element()
            })
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    event(&mut ui, InputEvent::Hover(point(80.0, 80.0)));
    event(&mut ui, InputEvent::Hover(point(10.0, 10.0)));
    event(&mut ui, InputEvent::Hover(point(20.0, 20.0)));
    event(&mut ui, InputEvent::PointerLeave);
    assert_eq!(
        *log.borrow(),
        vec![(1, true), (2, true), (2, false), (1, false)]
    );
    log.borrow_mut().clear();
    event(&mut ui, InputEvent::Hover(point(80.0, 80.0)));
    mode.set(1);
    update(&mut ui);
    assert_eq!(*log.borrow(), vec![(1, true), (1, false)]);
    mode.set(0);
    update(&mut ui);
    event(&mut ui, InputEvent::Hover(point(80.0, 80.0)));
    mode.set(2);
    update(&mut ui);
    assert_eq!(
        *log.borrow(),
        vec![(1, true), (1, false), (1, true), (1, false)]
    );
}
#[test]
fn filtered_and_opacity_groups_keep_local_layers_under_later_siblings() {
    for filter in [false, true] {
        let clicks = State::new(0);
        let value = clicks.clone();
        let mut ui = UiState::default();
        ui.mount(move |_| {
            let back = value.clone();
            let front = value.clone();
            let group = View::new().width(80.0).height(80.0).child(
                View::new()
                    .position(Position::Absolute)
                    .top(0.0)
                    .left(0.0)
                    .width(80.0)
                    .height(80.0)
                    .layer(100)
                    .action(move || back.set(1)),
            );
            let group = if filter {
                group.brightness(0.5)
            } else {
                group.opacity(0.5)
            };
            View::new()
                .fill()
                .child(group)
                .child(
                    View::new()
                        .position(Position::Absolute)
                        .top(0.0)
                        .left(0.0)
                        .width(80.0)
                        .height(80.0)
                        .action(move || front.set(2)),
                )
                .into_element()
        })
        .unwrap();
        update(&mut ui);
        event(&mut ui, InputEvent::Click(point(10.0, 10.0)));
        assert_eq!(
            clicks.get(),
            2,
            "later sibling must paint/hit above isolated group's descendants"
        );
    }
}
