use lemon::{
    host::HostContext,
    input::{hit_test::LogicalPoint, InputEvent},
    prelude::*,
    ui::UiState,
    AnimSlot, ImageHandle, Theme, Viewport,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

fn viewport() -> Viewport {
    Viewport {
        width: 600.0,
        height: 500.0,
    }
}
fn finish(ui: &mut UiState) {
    ui.update(viewport(), 1.0, ui.focused(), true).unwrap();
}
fn key(key: LemonKey, modifiers: Modifiers) -> InputEvent {
    InputEvent::Key(KeyEvent {
        key,
        modifiers,
        repeat: false,
        state: KeyState::Pressed,
    })
}
fn box_node(element: &Element) -> &lemon::element::types::BoxElement {
    match element {
        Element::View(node) | Element::Column(node) | Element::Row(node) => node,
        _ => panic!("expected container"),
    }
}
fn panel(view: View) -> View {
    view.width(240.0)
        .height(48.0)
        .inset(7.0)
        .rounding(11.0)
        .surface(Color::rgb8(140, 70, 180))
}

#[test]
fn one_decoration_styles_every_builder_without_replacing_widget_contracts() {
    fn component(_: &Cx) -> Element {
        Text::new("component").into_element()
    }
    let cx = Cx::new();
    let image = ImageHandle::from_arc(std::sync::Arc::new(lemon::asset::image_handle::ImageData {
        width: 1,
        height: 1,
        pixels: vec![255; 4],
    }));
    let elements = [
        ColumnView::new().key(4).decorate(panel),
        RowView::new().decorate(panel),
        View::new().decorate(panel),
        Text::new("leaf").key(9).decorate(panel),
        Component::new(component).key(7).decorate(panel),
        Button::new(&cx, "Action").disabled(true).decorate(panel),
        TextField::new(&cx, State::new(TextFieldState::new("text"))).decorate(panel),
        SecureField::new(&cx, State::new(TextFieldState::new("secret"))).decorate(panel),
        Slider::new(&cx, State::new(0.5)).decorate(panel),
        Scroll::new(&cx, Text::new("content")).decorate(panel),
        Select::new(&cx, State::new(Some(1)), vec![(1, "One".into())]).decorate(panel),
        Image::new(image).decorate(panel),
    ];
    for element in &elements {
        let root = box_node(element);
        assert_eq!(
            root.style.width,
            Some(lemon::element::style::Dimension::Points(240.0))
        );
        assert_eq!(root.style.padding.unwrap().top, 7.0);
        assert_eq!(root.paint.resolve().radius.top_left, 11.0);
        assert_eq!(
            root.paint.resolve().background,
            Some(Color::rgb8(140, 70, 180))
        );
    }
    assert!(matches!(elements[0], Element::Column(_)));
    assert!(matches!(elements[1], Element::Row(_)));
    assert_eq!(box_node(&elements[0]).key.as_ref().unwrap().0, 4);
    assert_eq!(box_node(&elements[3]).key.as_ref().unwrap().0, 9);
    assert_eq!(box_node(&elements[4]).key.as_ref().unwrap().0, 7);
    assert!(box_node(&elements[5]).handlers.on_click.is_none());
    assert!(box_node(&elements[6]).style.focusable);
    assert!(box_node(&elements[6]).handlers.on_key_down.is_some());
    assert_eq!(
        box_node(&elements[7]).text_input.as_ref().unwrap().value,
        "••••••"
    );
    assert!(box_node(&elements[8]).handlers.on_pointer_down.is_some());
    assert!(box_node(&elements[9]).handlers.on_scroll.is_some());
}

#[test]
fn decorations_preserve_axis_and_apply_explicit_overrides_in_order() {
    let decorated = ColumnView::new()
        .inset(99.0)
        .child(Text::new("child"))
        .decorate(|view| panel(view).place(Placement::BottomLeft))
        .decorate(|view| view.width(300.0));
    let expected = ColumnView::new()
        .width(300.0)
        .place(Placement::BottomLeft)
        .into_element();
    assert_eq!(
        box_node(&decorated).style.align_items,
        box_node(&expected).style.align_items
    );
    assert_eq!(
        box_node(&decorated).style.justify_content,
        box_node(&expected).style.justify_content
    );
    assert_eq!(
        box_node(&decorated).style.width,
        box_node(&expected).style.width
    );
    assert_eq!(box_node(&decorated).children.len(), 1);
    assert_eq!(box_node(&decorated).style.padding.unwrap().top, 7.0);
}

#[test]
fn pointer_focus_keyboard_capture_hover_and_scroll_work_without_desktop() {
    let value = State::new(TextFieldState::new(""));
    let source = value.clone();
    let volume = State::new(0.0);
    let source_volume = volume.clone();
    let offset = State::new(0.0);
    let source_offset = offset.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        ColumnView::new()
            .spacing(12.0)
            .child(TextField::new(cx, source.clone()).decorate(panel))
            .child(Slider::new(cx, source_volume.clone()).width(240.0))
            .child(
                Scroll::with_offset(source_offset.clone(), View::new().height(600.0))
                    .width(240.0)
                    .height(80.0),
            )
            .child(View::new().width(100.0).height(20.0).focusable())
            .into_element()
    })
    .unwrap();
    finish(&mut ui);
    let field = ui.retained().unwrap().root().unwrap().children[0]
        .taffy_id
        .unwrap();
    let at_field = LogicalPoint::new(10.0, 10.0);
    ui.dispatch_input(InputEvent::Click(at_field)).unwrap();
    assert_eq!(ui.focused(), Some(field));
    let hover = ui.dispatch_input(InputEvent::Hover(at_field)).unwrap();
    assert_eq!(hover.cursor, Cursor::Text);
    assert!(!ui.layout_dirty(), "hover paint must not force layout");
    ui.dispatch_input(key(LemonKey::Character("Olá".into()), Modifiers::default()))
        .unwrap();
    assert_eq!(value.get().value, "Olá");
    finish(&mut ui);
    let root = ui.retained().unwrap().root().unwrap();
    let slider = *ui
        .layout_map()
        .get(root.children[1].taffy_id.unwrap())
        .unwrap();
    let scroll = *ui
        .layout_map()
        .get(root.children[2].taffy_id.unwrap())
        .unwrap();
    ui.dispatch_input(InputEvent::PointerDown(LogicalPoint::new(
        slider.x + 1.0,
        slider.y + slider.height / 2.0,
    )))
    .unwrap();
    ui.dispatch_input(InputEvent::PointerMove(LogicalPoint::new(599.0, slider.y)))
        .unwrap();
    assert_eq!(volume.get(), 1.0, "captured drag clamps outside bounds");
    ui.dispatch_input(InputEvent::PointerUp(LogicalPoint::new(599.0, slider.y)))
        .unwrap();
    ui.dispatch_input(InputEvent::PointerMove(LogicalPoint::new(0.0, slider.y)))
        .unwrap();
    assert_eq!(volume.get(), 1.0, "release ends capture");
    ui.dispatch_input(InputEvent::Scroll {
        point: LogicalPoint::new(scroll.x + 10.0, scroll.y + 10.0),
        delta_y: -30.0,
    })
    .unwrap();
    assert!(offset.get() > 0.0);
    ui.dispatch_input(key(LemonKey::Named(NamedKey::Tab), Modifiers::default()))
        .unwrap();
    assert_ne!(ui.focused(), Some(field));
}

#[test]
fn callback_only_updates_replace_captured_props_without_layout() {
    let label = State::new(1);
    let source = label.clone();
    let clicked = State::new(0);
    let output = clicked.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        let next = source.get();
        let clicked = output.clone();
        View::new()
            .width(100.0)
            .height(40.0)
            .action(move || clicked.set(next))
            .into_element()
    })
    .unwrap();
    finish(&mut ui);
    label.set(2);
    ui.apply_updates().unwrap();
    assert!(!ui.layout_dirty());
    ui.dispatch_input(InputEvent::Click(LogicalPoint::new(5.0, 5.0)))
        .unwrap();
    assert_eq!(clicked.get(), 2, "click must use the newly captured props");
}

#[test]
fn clipboard_and_animation_requests_stay_with_originating_host() {
    let clipboard = Rc::new(RefCell::new(String::from("first host")));
    let read = clipboard.clone();
    let write = clipboard.clone();
    let first = HostContext::new(Theme::default_light()).with_clipboard(
        move || Some(read.borrow().clone()),
        move |text| *write.borrow_mut() = text.into(),
    );
    let second = HostContext::new(Theme::default_dark());
    let value = State::new(TextFieldState::new(""));
    let source = value.clone();
    let mut ui = UiState::default();
    let animation = {
        let _scope = first.enter();
        ui.mount(move |cx| TextField::new(cx, source.clone()).decorate(panel))
            .unwrap();
        first
            .animations()
            .register(AnimSlot::new(AnimationConfig::new(Duration::from_secs(10))))
    };
    let _other_host = second.enter();
    finish(&mut ui);
    ui.dispatch_input(InputEvent::Click(LogicalPoint::new(5.0, 5.0)))
        .unwrap();
    ui.dispatch_input(key(
        LemonKey::Character("v".into()),
        Modifiers {
            meta: true,
            ..Default::default()
        },
    ))
    .unwrap();
    assert_eq!(value.get().value, "first host");
    animation.play();
    animation.progress();
    assert!(first.animations().take_pending());
    assert!(!second.animations().take_pending());
    assert_eq!(second.animations().active_slots(), 0);
    assert_eq!(first.animations().active_slots(), 1);
    first.animations().tick_frame();
    assert_eq!(first.animations().frame_signal().get(), 1);
    assert_eq!(second.animations().frame_signal().get(), 0);
    animation.reset();
    assert!(first.animations().take_pending());
    assert!(!second.animations().take_pending());
}

#[test]
fn removing_a_focused_control_clears_its_input_identity() {
    let visible = State::new(true);
    let source = visible.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        View::new()
            .child(if source.get() {
                View::new()
                    .width(100.0)
                    .height(30.0)
                    .focusable()
                    .into_element()
            } else {
                Element::None
            })
            .into_element()
    })
    .unwrap();
    finish(&mut ui);
    ui.dispatch_input(InputEvent::Click(LogicalPoint::new(5.0, 5.0)))
        .unwrap();
    assert!(ui.focused().is_some());
    visible.set(false);
    ui.apply_updates().unwrap();
    assert!(ui.focused().is_none());
}
