use lemon::{
    input::{hit_test::LogicalPoint, InputEvent},
    prelude::*,
    ui::UiState,
    Viewport,
};
#[test]
fn checkbox_and_switch_share_boolean_keyboard_and_disabled_behavior() {
    let a = State::new(false);
    let b = State::new(false);
    let disabled = State::new(false);
    let av = a.clone();
    let bv = b.clone();
    let dv = disabled.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        ColumnView::new()
            .child(Checkbox::new(cx, av.clone()).label("Checkbox"))
            .child(Switch::new(cx, bv.clone()).label("Switch"))
            .child(Switch::new(cx, dv.clone()).label("Disabled").disabled(true))
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
    let key = |ui: &mut UiState, name, repeat| {
        ui.dispatch_input(InputEvent::Key(KeyEvent {
            key: LemonKey::Named(name),
            state: KeyState::Pressed,
            repeat,
            modifiers: Modifiers::default(),
        }))
        .unwrap();
    };
    key(&mut ui, NamedKey::Tab, false);
    key(&mut ui, NamedKey::Space, false);
    assert!(a.get());
    key(&mut ui, NamedKey::Space, true);
    assert!(a.get());
    key(&mut ui, NamedKey::Tab, false);
    key(&mut ui, NamedKey::Enter, false);
    assert!(b.get());
    key(&mut ui, NamedKey::Tab, false);
    key(&mut ui, NamedKey::Space, false);
    assert!(!a.get());
    assert!(!disabled.get());
    ui.dispatch_input(InputEvent::Click(LogicalPoint::new(75.0, 10.0)))
        .unwrap();
    assert!(a.get());
    a.set(false);
    ui.apply_updates().unwrap();
    key(&mut ui, NamedKey::Space, false);
    assert!(a.get());
}
