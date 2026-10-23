use lemon::{
    host::HostContext,
    input::{
        hit_test::{hit_test_on_click, LogicalPoint},
        InputEvent,
    },
    prelude::*,
    ui::UiState,
    Theme, Viewport,
};
use std::{cell::Cell, rc::Rc};

fn root(element: Element) -> lemon::element::types::BoxElement {
    let Element::View(view) = element else {
        panic!("view")
    };
    view
}

fn key(key: NamedKey) -> InputEvent {
    InputEvent::Key(KeyEvent {
        key: LemonKey::Named(key),
        modifiers: Modifiers::default(),
        state: KeyState::Pressed,
        repeat: false,
    })
}
const VIEWPORT: Viewport = Viewport {
    width: 500.0,
    height: 400.0,
};

#[test]
fn presets_follow_both_themes_and_keep_field_behavior_and_explicit_overrides() {
    for theme in [Theme::default_light(), Theme::default_dark()] {
        let host = HostContext::new(theme.clone());
        let _scope = host.enter();
        for style in [
            TextFieldStyle::Clean,
            TextFieldStyle::Bordered,
            TextFieldStyle::Filled,
        ] {
            let cx = Cx::new();
            let plain = root(
                TextField::new(&cx, State::new("hello".into()))
                    .style(style)
                    .into_element(),
            );
            let secret = root(
                SecureField::new(&cx, State::new("hello".into()))
                    .style(style)
                    .into_element(),
            );
            let paint = plain.paint.resolve();
            assert_eq!(paint, secret.paint.resolve());
            assert_eq!(
                paint.border_width.top,
                if style == TextFieldStyle::Bordered {
                    1.5
                } else {
                    0.0
                }
            );
            assert_eq!(
                paint.background.unwrap().a,
                if style == TextFieldStyle::Clean {
                    0.0
                } else {
                    1.0
                }
            );
            assert!(plain.style.focusable && secret.style.focusable);
            assert_eq!(plain.text_input.unwrap().value, "hello");
            assert_eq!(secret.text_input.unwrap().value, "•••••");
            let overridden = root(
                TextField::new(&cx, State::new(String::new()))
                    .rounding(13.0)
                    .border_color(theme.colors.error)
                    .style(style)
                    .into_element(),
            );
            assert_eq!(overridden.paint.resolve().radius.top_left, 13.0);
            assert_eq!(
                overridden.paint.resolve().border_color.top,
                Some(theme.colors.error)
            );
        }
        for style in [
            ButtonStyle::Clean,
            ButtonStyle::Bordered,
            ButtonStyle::Filled,
        ] {
            let cx = Cx::new();
            let button = root(Button::new(&cx, "Save").style(style).into_element());
            let paint = button.paint.resolve();
            assert_eq!(
                paint.border_width.top,
                if style == ButtonStyle::Bordered {
                    1.0
                } else {
                    0.0
                }
            );
            assert_eq!(
                paint.background.unwrap().a,
                if style == ButtonStyle::Filled {
                    1.0
                } else {
                    0.0
                }
            );
            assert!(button.style.focusable);
            button.handlers.on_hover_enter.as_ref().unwrap()();
            assert!(button.paint.resolve().background.unwrap().a > 0.0);
            let disabled = root(
                Button::new(&cx, "Save")
                    .style(style)
                    .disabled(true)
                    .into_element(),
            );
            assert!(!disabled.style.focusable);
            assert!(
                disabled.handlers.on_click.is_none() && disabled.handlers.on_key_down.is_none()
            );
        }
    }
}

#[test]
fn switching_a_field_preset_keeps_the_value_caret_and_geometry() {
    let value = State::new("hello".to_owned());
    let preset = State::new(TextFieldStyle::Bordered);
    let source = value.clone();
    let style = preset.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        TextField::new(cx, source.clone())
            .style(style.get())
            .width(220.0)
            .into_element()
    })
    .unwrap();
    ui.update(VIEWPORT, 1.0, None, true).unwrap();
    ui.dispatch_input(InputEvent::Click(LogicalPoint::new(5.0, 5.0)))
        .unwrap();
    ui.dispatch_input(key(NamedKey::End)).unwrap();
    ui.update(VIEWPORT, 1.0, ui.focused(), true).unwrap();
    let geometry = ui.layout_map().clone();
    for style in [
        TextFieldStyle::Clean,
        TextFieldStyle::Filled,
        TextFieldStyle::Bordered,
    ] {
        preset.set(style);
        ui.update(VIEWPORT, 1.0, ui.focused(), true).unwrap();
        let field = ui.retained().unwrap().root().unwrap();
        assert_eq!(field.text_input.as_ref().unwrap().cursor, 5);
        assert_eq!(value.get(), "hello");
        assert_eq!(ui.layout_map(), &geometry);
    }
}

#[test]
fn inactive_subtrees_reject_input_and_release_focus_including_layered_children() {
    let enabled = State::new(true);
    let source = enabled.clone();
    let moves = Rc::new(Cell::new(0));
    let movement = moves.clone();
    let clicks = Rc::new(Cell::new(0));
    let count = clicks.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        View::new()
            .interactive(source.get())
            .child(
                View::new()
                    .width(100.0)
                    .height(40.0)
                    .layer(10)
                    .focusable()
                    .on_pointer_down(|_, _| {})
                    .on_pointer_move({
                        let movement = movement.clone();
                        move |_, _| movement.set(movement.get() + 1)
                    })
                    .action({
                        let count = count.clone();
                        move || count.set(count.get() + 1)
                    }),
            )
            .into_element()
    })
    .unwrap();
    ui.update(VIEWPORT, 1.0, None, true).unwrap();
    let point = LogicalPoint::new(5.0, 5.0);
    ui.dispatch_input(InputEvent::Click(point)).unwrap();
    assert!(ui.focused().is_some());
    assert!(
        ui.dispatch_input(InputEvent::PointerDown(point))
            .unwrap()
            .handled
    );
    assert!(
        ui.dispatch_input(InputEvent::PointerMove(point))
            .unwrap()
            .handled
    );
    assert_eq!(moves.get(), 1);
    enabled.set(false);
    ui.apply_updates().unwrap();
    assert!(ui.focused().is_none());
    assert!(
        !ui.dispatch_input(InputEvent::PointerMove(point))
            .unwrap()
            .handled
    );
    assert_eq!(moves.get(), 1, "inactive subtrees release pointer capture");
    assert!(!ui.layout_dirty());
    assert!(hit_test_on_click(
        ui.retained().unwrap().root().unwrap(),
        ui.layout_map(),
        point
    )
    .is_none());
    ui.dispatch_input(key(NamedKey::Tab)).unwrap();
    assert!(ui.focused().is_none());
    ui.dispatch_input(InputEvent::Click(point)).unwrap();
    assert_eq!(clicks.get(), 1);
}

#[test]
fn select_keyboard_navigation_and_button_activation_use_native_focus_dispatch() {
    let value = State::new(None::<u8>);
    let source = value.clone();
    let open = State::new(false);
    let opened = open.clone();
    let clicks = Rc::new(Cell::new(0));
    let count = clicks.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        ColumnView::new()
            .child(
                Select::with_open(
                    cx,
                    source.clone(),
                    opened.clone(),
                    vec![(1, "One".into()), (2, "Two".into())],
                )
                .width(200.0),
            )
            .child(
                Button::new(cx, "Save")
                    .style(ButtonStyle::Bordered)
                    .action({
                        let count = count.clone();
                        move || count.set(count.get() + 1)
                    }),
            )
            .into_element()
    })
    .unwrap();
    ui.update(VIEWPORT, 1.0, None, true).unwrap();
    ui.dispatch_input(key(NamedKey::Tab)).unwrap();
    ui.dispatch_input(key(NamedKey::ArrowDown)).unwrap();
    assert!(open.get());
    ui.dispatch_input(key(NamedKey::ArrowDown)).unwrap();
    ui.dispatch_input(key(NamedKey::Enter)).unwrap();
    assert_eq!(value.get(), Some(2));
    assert!(!open.get());
    ui.dispatch_input(key(NamedKey::Space)).unwrap();
    assert!(open.get());
    ui.dispatch_input(key(NamedKey::Escape)).unwrap();
    assert!(!open.get());
    ui.dispatch_input(key(NamedKey::Tab)).unwrap();
    ui.dispatch_input(key(NamedKey::Enter)).unwrap();
    ui.dispatch_input(key(NamedKey::Space)).unwrap();
    assert_eq!(clicks.get(), 2);
}
