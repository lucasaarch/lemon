use lemon::{
    host::HostContext,
    input::{hit_test::LogicalPoint, InputEvent},
    prelude::*,
    retained::RetainedNode,
    ui::UiState,
    Theme, Viewport,
};
const VIEWPORT: Viewport = Viewport {
    width: 400.0,
    height: 500.0,
};
fn update(ui: &mut UiState) {
    ui.update(VIEWPORT, 1.0, ui.focused(), true).unwrap();
}
fn key(ui: &mut UiState, key: LemonKey, shift: bool, meta: bool) {
    ui.dispatch_input(InputEvent::Key(KeyEvent {
        key,
        modifiers: Modifiers {
            shift,
            meta,
            ..Default::default()
        },
        repeat: false,
        state: KeyState::Pressed,
    }))
    .unwrap();
    update(ui);
}
fn named(ui: &mut UiState, key_name: NamedKey) {
    key(ui, LemonKey::Named(key_name), false, false);
}
fn find(node: &RetainedNode, predicate: fn(&RetainedNode) -> bool) -> Option<&RetainedNode> {
    if predicate(node) {
        return Some(node);
    }
    node.children.iter().find_map(|node| find(node, predicate))
}
fn root(ui: &UiState) -> &RetainedNode {
    ui.retained().unwrap().root().unwrap()
}

#[test]
fn multiline_editing_wrapping_selection_and_external_writes_share_field_state() {
    for theme in [Theme::default_light(), Theme::default_dark()] {
        let host = HostContext::new(theme);
        let _scope = host.enter();
        let value = State::new(String::new());
        let source = value.clone();
        let mut ui = UiState::default();
        ui.mount(move |cx| {
            TextArea::new(cx, source.clone())
                .width(140.0)
                .height(100.0)
                .resizable(false)
                .into_element()
        })
        .unwrap();
        update(&mut ui);
        named(&mut ui, NamedKey::Tab);
        key(
            &mut ui,
            LemonKey::Character("Olá 世界".into()),
            false,
            false,
        );
        named(&mut ui, NamedKey::Enter);
        key(&mut ui, LemonKey::Character("next".into()), false, false);
        assert_eq!(value.peek(), "Olá 世界\nnext");
        assert_eq!(
            root(&ui).text_input.as_ref().unwrap().cursor,
            value.peek().len()
        );
        key(&mut ui, LemonKey::Named(NamedKey::Home), true, false);
        assert!(root(&ui).text_input.as_ref().unwrap().selection.is_some());
        named(&mut ui, NamedKey::Backspace);
        assert_eq!(value.peek(), "Olá 世界\n");
        key(&mut ui, LemonKey::Character("a".into()), false, true);
        named(&mut ui, NamedKey::Delete);
        assert_eq!(value.peek(), "");
        let id = root(&ui).taffy_id;
        value.set(
            "a long line that wraps onto several visual lines inside this narrow field".into(),
        );
        update(&mut ui);
        named(&mut ui, NamedKey::ArrowDown);
        assert!(
            root(&ui).text_input.as_ref().unwrap().cursor > 0,
            "Down must follow a soft-wrapped visual line"
        );
        assert_eq!(root(&ui).taffy_id, id);
    }
}

#[test]
fn resize_capture_changes_only_height_clamps_minimum_and_can_be_disabled() {
    let resizable = State::new(true);
    let resize = resizable.clone();
    let minimum = State::new(70.0);
    let min = minimum.clone();
    let disabled = State::new(false);
    let disable = disabled.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        TextArea::new(cx, State::new("keep".into()))
            .width(240.0)
            .height(100.0)
            .min_height(min.get())
            .resizable(resize.get())
            .disabled(disable.get())
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    let handle = find(root(&ui), |node| node.handlers.on_drag.is_some()).unwrap();
    let handle_id = handle.taffy_id;
    let rect = ui.layout_map().get(handle_id.unwrap()).unwrap();
    let start = LogicalPoint::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    ui.dispatch_input(InputEvent::PointerDown(start)).unwrap();
    for (dy, height) in [(40.0, 140.0), (60.0, 160.0), (-500.0, 70.0)] {
        ui.dispatch_input(InputEvent::PointerMove(LogicalPoint::new(
            start.x + 300.0,
            start.y + dy,
        )))
        .unwrap();
        update(&mut ui);
        let rect = ui.layout_map().get(root(&ui).taffy_id.unwrap()).unwrap();
        assert_eq!(rect.width, 240.0);
        assert_eq!(rect.height, height);
        let viewport = find(root(&ui), |node| node.scroll_viewport.is_some()).unwrap();
        assert_eq!(
            ui.layout_map()
                .get(viewport.taffy_id.unwrap())
                .unwrap()
                .height,
            height - 30.0
        );
        assert_eq!(
            find(root(&ui), |node| node.handlers.on_drag.is_some())
                .unwrap()
                .taffy_id,
            handle_id
        );
    }
    ui.dispatch_input(InputEvent::PointerUp(start)).unwrap();
    resizable.set(false);
    update(&mut ui);
    assert!(find(root(&ui), |node| node.handlers.on_drag.is_some()).is_none());
    assert_eq!(
        ui.layout_map()
            .get(root(&ui).taffy_id.unwrap())
            .unwrap()
            .height,
        70.0
    );
    minimum.set(120.0);
    update(&mut ui);
    assert_eq!(
        ui.layout_map()
            .get(root(&ui).taffy_id.unwrap())
            .unwrap()
            .height,
        120.0
    );
    resizable.set(true);
    disabled.set(true);
    update(&mut ui);
    assert!(!root(&ui).style.focusable);
    assert!(find(root(&ui), |node| node.handlers.on_drag.is_some()).is_none());
}

#[test]
fn typing_reveals_the_caret_but_manual_scrolling_is_preserved_and_clear_clamps_offset() {
    let value = State::new(String::new());
    let source = value.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        TextArea::new(cx, source.clone())
            .width(240.0)
            .height(80.0)
            .resizable(false)
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    named(&mut ui, NamedKey::Tab);
    key(
        &mut ui,
        LemonKey::Character("a line\n".repeat(20)),
        false,
        false,
    );
    let viewport = find(root(&ui), |node| node.scroll_viewport.is_some()).unwrap();
    let text = find(root(&ui), |node| node.text.is_some()).unwrap();
    let view_rect = ui.layout_map().get(viewport.taffy_id.unwrap()).unwrap();
    let text_rect = ui.layout_map().get(text.taffy_id.unwrap()).unwrap();
    let cache = text.text.as_ref().unwrap();
    let caret = lemon::layout::caret_geometry_in_layout(
        cache.parley_layout.as_ref().unwrap(),
        &cache.content,
        value.peek().len(),
        1.0,
        root(&ui).text_input.as_ref().unwrap().affinity,
    );
    assert!(text_rect.y + caret.y1 as f32 <= view_rect.y + view_rect.height + 0.1,
        "text={text_rect:?}, viewport={view_rect:?}, caret={caret:?}, extent={:?}, inner={:?}, lines={}, layout_h={}",
        lemon::layout::scroll_content_bounds(viewport,ui.layout_map()),viewport.children[0].style.margin,
        cache.parley_layout.as_ref().unwrap().len(), cache.parley_layout.as_ref().unwrap().height());
    let offset = -viewport.children[0].style.margin.unwrap_or_default().top;
    assert!(offset > 0.0);
    let point = LogicalPoint::new(view_rect.x + 10.0, view_rect.y + 10.0);
    ui.dispatch_input(InputEvent::Scroll {
        point,
        delta: ScrollDelta::new(0.0, 30.0),
    })
    .unwrap();
    update(&mut ui);
    let viewport = find(root(&ui), |node| node.scroll_viewport.is_some()).unwrap();
    assert!(
        (-viewport.children[0].style.margin.unwrap_or_default().top - (offset - 30.0)).abs() < 0.1
    );
    value.set(String::new());
    update(&mut ui);
    let viewport = find(root(&ui), |node| node.scroll_viewport.is_some()).unwrap();
    assert_eq!(
        viewport.children[0].style.margin.unwrap_or_default().top,
        0.0
    );
}

#[test]
fn dragging_selects_text_and_the_release_click_keeps_the_selection() {
    let value = State::new("first line\nsecond line".to_owned());
    let source = value.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        TextArea::new(cx, source.clone())
            .width(240.0)
            .resizable(false)
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    let text = find(root(&ui), |node| node.text.is_some()).unwrap();
    let rect = ui.layout_map().get(text.taffy_id.unwrap()).unwrap();
    let start = LogicalPoint::new(rect.x + 0.5, rect.y + 10.0);
    let end = LogicalPoint::new(rect.x + 30.0, rect.y + 10.0);
    ui.dispatch_input(InputEvent::PointerDown(start)).unwrap();
    ui.dispatch_input(InputEvent::PointerMove(end)).unwrap();
    ui.dispatch_input(InputEvent::PointerUp(end)).unwrap();
    ui.dispatch_input(InputEvent::Click(end)).unwrap();
    update(&mut ui);
    assert!(root(&ui).text_input.as_ref().unwrap().selection.is_some());
    key(&mut ui, LemonKey::Character("X".into()), false, false);
    assert!(value.peek().starts_with('X'));
}

#[test]
fn end_keeps_the_caret_on_the_current_soft_wrapped_line() {
    let mut ui = UiState::default();
    ui.mount(|cx| {
        TextArea::new(
            cx,
            State::new("A long sentence that wraps across multiple lines of the editor".into()),
        )
        .width(140.0)
        .height(150.0)
        .resizable(false)
        .into_element()
    })
    .unwrap();
    update(&mut ui);
    named(&mut ui, NamedKey::Tab);
    named(&mut ui, NamedKey::Home);
    let text = find(root(&ui), |node| node.text.is_some())
        .unwrap()
        .text
        .as_ref()
        .unwrap();
    assert!(
        text.parley_layout.as_ref().unwrap().len() > 1,
        "the area must actually wrap long text"
    );
    let start = lemon::layout::caret_geometry_in_layout(
        text.parley_layout.as_ref().unwrap(),
        &text.content,
        0,
        1.0,
        root(&ui).text_input.as_ref().unwrap().affinity,
    );
    named(&mut ui, NamedKey::End);
    let text = find(root(&ui), |node| node.text.is_some())
        .unwrap()
        .text
        .as_ref()
        .unwrap();
    let cursor = root(&ui).text_input.as_ref().unwrap().cursor;
    let end = lemon::layout::caret_geometry_in_layout(
        text.parley_layout.as_ref().unwrap(),
        &text.content,
        cursor,
        1.0,
        root(&ui).text_input.as_ref().unwrap().affinity,
    );
    assert_eq!(
        start.y0, end.y0,
        "End must stay at the visual end, not jump to the next wrapped line"
    );
    assert!(end.x0 > start.x0);
}

#[test]
fn releasing_a_resize_over_another_control_does_not_activate_it() {
    let clicks = State::new(0);
    let count = clicks.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        RowView::new()
            .child(
                TextArea::new(cx, State::new(String::new()))
                    .width(240.0)
                    .height(100.0),
            )
            .child(
                Button::new(cx, "Action")
                    .width(120.0)
                    .height(100.0)
                    .action({
                        let count = count.clone();
                        move || count.update(|count| *count += 1)
                    }),
            )
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    let handle = find(root(&ui), |node| node.handlers.on_drag.is_some()).unwrap();
    let rect = ui.layout_map().get(handle.taffy_id.unwrap()).unwrap();
    let start = LogicalPoint::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    let end = LogicalPoint::new(270.0, 50.0);
    ui.dispatch_input(InputEvent::PointerDown(start)).unwrap();
    ui.dispatch_input(InputEvent::PointerMove(end)).unwrap();
    ui.dispatch_input(InputEvent::PointerUp(end)).unwrap();
    ui.dispatch_input(InputEvent::Click(end)).unwrap();
    assert_eq!(clicks.peek(), 0);
    ui.dispatch_input(InputEvent::Click(end)).unwrap();
    assert_eq!(
        clicks.peek(),
        1,
        "a subsequent ordinary click still activates the control"
    );
}
