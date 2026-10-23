use lemon::{
    host::HostContext,
    input::{hit_test::LogicalPoint, InputEvent},
    prelude::*,
    retained::{focus::collect_focusable, RetainedNode},
    ui::UiState,
    Theme, Viewport,
};
use std::time::{Duration, Instant};
const SIZE: Viewport = Viewport {
    width: 640.0,
    height: 480.0,
};

fn update(ui: &mut UiState) {
    ui.update(SIZE, 1.0, ui.focused(), true).unwrap();
}

fn key(ui: &mut UiState, named: NamedKey) {
    ui.dispatch_input(InputEvent::Key(KeyEvent {
        key: LemonKey::Named(named),
        state: KeyState::Pressed,
        repeat: false,
        modifiers: Modifiers::default(),
    }))
    .unwrap();
    update(ui);
}

fn click(ui: &mut UiState, x: f32, y: f32) {
    for event in [
        InputEvent::PointerDown(LogicalPoint::new(x, y)),
        InputEvent::PointerUp(LogicalPoint::new(x, y)),
        InputEvent::Click(LogicalPoint::new(x, y)),
    ] {
        ui.dispatch_input(event).unwrap();
    }
    update(ui);
}

fn find_key(node: &RetainedNode, key: u64) -> Option<&RetainedNode> {
    let matches = if key == 2 {
        node.text_input.is_some()
    } else {
        node.style.focusable
            && node.children.iter().any(|child| {
                child.text.as_ref().is_some_and(|text| {
                    if key == 1 {
                        text.content == "Background" || text.content == "Anchor"
                    } else {
                        text.content == "Done"
                    }
                })
            })
    };
    if matches {
        return Some(node);
    }
    node.children.iter().find_map(|child| find_key(child, key))
}

fn point(ui: &UiState, key: u64) -> (f32, f32) {
    let node = find_key(ui.retained().unwrap().root().unwrap(), key).unwrap();
    let rect = ui.layout_map().get(node.taffy_id.unwrap()).unwrap();
    (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

#[test]
fn dialog_traps_input_restores_focus_and_escape_works_from_a_field() {
    for theme in [Theme::default_light(), Theme::default_dark()] {
        let host = HostContext::new(theme);
        let _scope = host.enter();
        let open = State::new(false);
        let visible = open.clone();
        let clicks = State::new(0);
        let count = clicks.clone();
        let mut ui = UiState::default();
        ui.mount(move |cx| {
            let count = count.clone();
            ColumnView::new()
                .fill()
                .child(
                    Button::new(cx, "Background")
                        .action(move || count.update(|v| *v += 1))
                        .decorate(|v| v.key(1)),
                )
                .child(Dialog::new(
                    cx,
                    visible.clone(),
                    ColumnView::new()
                        .spacing(8.0)
                        .child(TextField::new(cx, cx.state(String::new())).decorate(|v| v.key(2)))
                        .child(Button::new(cx, "Done").decorate(|v| v.key(3))),
                ))
                .into_element()
        })
        .unwrap();
        update(&mut ui);
        key(&mut ui, NamedKey::Tab);
        let background = ui.focused();
        assert!(background.is_some());
        open.set(true);
        update(&mut ui);
        assert_ne!(ui.focused(), background);
        let allowed = collect_focusable(ui.retained().unwrap());
        assert_eq!(allowed.len(), 2);
        for _ in 0..5 {
            key(&mut ui, NamedKey::Tab);
            assert!(allowed.contains(&ui.focused().unwrap()));
        }
        let (x, y) = point(&ui, 1);
        click(&mut ui, x, y);
        assert_eq!(clicks.get(), 0);
        // Backdrop closes without activating the background button.
        assert!(!open.get());
        assert_eq!(ui.focused(), background);
        open.set(true);
        update(&mut ui);
        let (x, y) = point(&ui, 2);
        click(&mut ui, x, y);
        key(&mut ui, NamedKey::Escape);
        assert!(!open.get());
        assert_eq!(ui.focused(), background);
        click(&mut ui, 20.0, 20.0);
        assert_eq!(clicks.get(), 1);
    }
}

#[test]
fn nested_dialogs_restore_the_previous_scope_and_disabled_backdrop_stays_open() {
    let first = State::new(true);
    let second = State::new(false);
    let a = first.clone();
    let b = second.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        View::new()
            .fill()
            .child(Dialog::new(cx, a.clone(), Button::new(cx, "First")))
            .child(Dialog::new(cx, b.clone(), Text::new("Second")).dismiss_on_backdrop(false))
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    let initial = ui.focused();
    assert!(initial.is_some());
    second.set(true);
    update(&mut ui);
    assert_ne!(initial, ui.focused());
    click(&mut ui, 5.0, 5.0);
    assert!(second.get());
    key(&mut ui, NamedKey::Escape);
    assert!(!second.get());
    assert!(first.get());
    assert_eq!(initial, ui.focused());
    key(&mut ui, NamedKey::Escape);
    assert!(!first.get());
}

#[test]
fn popover_is_anchored_dismisses_outside_and_escape_bubbles_from_child() {
    let open = State::new(true);
    let source = open.clone();
    let value = State::new(String::new());
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        ColumnView::new()
            .fill()
            .inset(20.0)
            .child(Popover::new(
                cx,
                source.clone(),
                Button::new(cx, "Anchor").decorate(|v| v.key(1)),
                TextField::new(cx, value.clone()).decorate(|v| v.key(2)),
            ))
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    let (_, anchor_y) = point(&ui, 1);
    let (x, y) = point(&ui, 2);
    assert!(y > anchor_y);
    click(&mut ui, x, y);
    assert!(open.get());
    key(&mut ui, NamedKey::Escape);
    assert!(!open.get());
    open.set(true);
    update(&mut ui);
    click(&mut ui, 600.0, 440.0);
    assert!(!open.get());
}

#[test]
fn toast_queue_updates_after_mount_expires_and_belongs_to_its_host() {
    let host = HostContext::new(Theme::default_light());
    let _scope = host.enter();
    let holder = std::rc::Rc::new(std::cell::RefCell::new(None));
    let output = holder.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        let toaster = Toaster::new(cx);
        *output.borrow_mut() = Some(toaster.clone());
        View::new().fill().child(toaster).into_element()
    })
    .unwrap();
    update(&mut ui);
    let toaster = holder.borrow().clone().unwrap();
    let id = toaster.show_for("Saved", Duration::from_secs(1));
    update(&mut ui);
    assert_eq!(collect_focusable(ui.retained().unwrap()).len(), 1);
    assert!(host.animations().active_slots() > 0);
    let other = HostContext::new(Theme::default_dark());
    assert_eq!(other.animations().active_slots(), 0);
    host.animations()
        .tick_active(Instant::now() + Duration::from_secs(2));
    host.animations().tick_frame();
    update(&mut ui);
    assert_eq!(collect_focusable(ui.retained().unwrap()).len(), 0);
    toaster.dismiss(id);
    toaster.show("One");
    toaster.show("Two");
    update(&mut ui);

    fn text_rect(ui: &UiState, content: &str) -> lemon::layout::LayoutRect {
        fn find<'a>(node: &'a RetainedNode, content: &str) -> Option<&'a RetainedNode> {
            if node
                .text
                .as_ref()
                .is_some_and(|text| text.content == content)
            {
                return Some(node);
            }
            node.children.iter().find_map(|child| find(child, content))
        }
        let node = find(ui.retained().unwrap().root().unwrap(), content).unwrap();
        *ui.layout_map().get(node.taffy_id.unwrap()).unwrap()
    }
    let one = text_rect(&ui, "One");
    let two = text_rect(&ui, "Two");
    assert!(
        one.y + one.height < two.y,
        "toasts must stack without overlapping"
    );

    assert_eq!(collect_focusable(ui.retained().unwrap()).len(), 2);
    key(&mut ui, NamedKey::Tab);
    key(&mut ui, NamedKey::Space);
    assert_eq!(collect_focusable(ui.retained().unwrap()).len(), 1);
    toaster.clear();
    update(&mut ui);
    assert_eq!(collect_focusable(ui.retained().unwrap()).len(), 0);
}

#[test]
fn popover_panel_blocks_background_click_focus_capture_hover_and_scroll() {
    let open = State::new(true);
    let visible = open.clone();
    let calls = State::new(0);
    let source = calls.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        let click = source.clone();
        let press = source.clone();
        let scroll = source.clone();
        let hover = source.clone();
        View::new()
            .fill()
            .child(
                View::new()
                    .position(Position::Absolute)
                    .top(0.0)
                    .left(0.0)
                    .width(400.0)
                    .height(300.0)
                    .focusable()
                    .action(move || click.update(|v| *v += 1))
                    .on_pointer_down(move |_, _| press.update(|v| *v += 1))
                    .on_scroll(move |_| scroll.update(|v| *v += 1))
                    .on_hover_enter(move || hover.update(|v| *v += 1)),
            )
            .child(Popover::new(
                cx,
                visible.clone(),
                View::new().height(40.0).width(100.0),
                View::new().height(100.0),
            ))
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    click(&mut ui, 100.0, 80.0);
    assert_eq!(calls.get(), 0);
    assert!(ui.focused().is_none());
    assert!(open.get());
    ui.dispatch_input(InputEvent::Hover(LogicalPoint::new(100.0, 80.0)))
        .unwrap();
    ui.dispatch_input(InputEvent::Scroll {
        point: LogicalPoint::new(100.0, 80.0),
        delta: lemon::ScrollDelta::new(0.0, -30.0),
    })
    .unwrap();
    assert_eq!(calls.get(), 0);
    open.set(false);
    update(&mut ui);
    click(&mut ui, 100.0, 80.0);
    assert!(calls.get() > 0);
}

#[test]
fn dialog_panel_shrinks_with_the_viewport() {
    let mut ui = UiState::default();
    ui.mount(|cx| {
        Dialog::new(
            cx,
            cx.state(true),
            Text::new("A dialog that wraps in a small window"),
        )
        .into_element()
    })
    .unwrap();
    ui.update(
        Viewport {
            width: 240.0,
            height: 300.0,
        },
        1.0,
        ui.focused(),
        true,
    )
    .unwrap();
    let root = ui.retained().unwrap().root().unwrap();
    let panel = &root.children[0];
    let rect = ui.layout_map().get(panel.taffy_id.unwrap()).unwrap();
    assert!(
        rect.width <= 192.01,
        "panel exceeds its viewport inset: {rect:?}"
    );
    assert!(rect.x >= 23.99);
}

#[test]
fn escape_dismisses_the_innermost_open_overlay_only() {
    let dialog = State::new(true);
    let d = dialog.clone();
    let popover = State::new(true);
    let p = popover.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        Dialog::new(
            cx,
            d.clone(),
            Popover::new(
                cx,
                p.clone(),
                Button::new(cx, "Anchor"),
                Text::new("Details"),
            ),
        )
        .into_element()
    })
    .unwrap();
    update(&mut ui);
    key(&mut ui, NamedKey::Escape);
    assert!(!popover.get());
    assert!(dialog.get());
    key(&mut ui, NamedKey::Escape);
    assert!(!dialog.get());
}
