use lemon::prelude::*;

#[test]
fn facade_and_prelude_export_plain_and_secure_fields_with_shared_styles() {
    let cx = Cx::new();
    let plain = State::new("name".to_owned());
    let secret = State::new("password".to_owned());
    let style: lemon::TextFieldStyle = TextFieldStyle::Bordered;
    let field: lemon::TextField = TextField::new(&cx, plain).style(style).rounding(8.0);
    let secure: lemon::SecureField = SecureField::new(&cx, secret).style(style).rounding(8.0);
    let Element::Column(root) = ColumnView::new().child(field).child(secure).into_element() else {
        panic!("expected column")
    };
    assert_eq!(root.children.len(), 2);
}

fn press(key: LemonKey) -> KeyEvent {
    KeyEvent {
        key,
        modifiers: Modifiers::default(),
        repeat: false,
        state: KeyState::Pressed,
    }
}

#[test]
fn string_state_edits_and_external_unicode_changes_keep_internal_caret_valid() {
    let cx = Cx::new();
    let value = State::new("abcd".to_owned());
    let Element::View(field) = TextField::new(&cx, value.clone()).into_element() else {
        panic!()
    };
    let handler = field.handlers.on_key_down.unwrap();
    handler(press(LemonKey::Named(NamedKey::End)));
    value.set("é".to_owned());
    handler(press(LemonKey::Character("!".to_owned())));
    assert_eq!(value.get(), "é!");
    value.set(String::new());
    handler(press(LemonKey::Character("ok".to_owned())));
    assert_eq!(value.get(), "ok");
    cx.reset_hooks();
    let Element::View(field) = TextField::new(&cx, value).into_element() else {
        panic!()
    };
    assert_eq!(field.text_input.unwrap().cursor, 2);
}

#[test]
fn typing_replaces_selection_and_external_writes_discard_old_selection() {
    let cx = Cx::new();
    let value = State::new("abcd".to_owned());
    let Element::View(field) = TextField::new(&cx, value.clone()).into_element() else {
        panic!()
    };
    let handler = field.handlers.on_key_down.unwrap();
    let mut select = press(LemonKey::Character("a".to_owned()));
    select.modifiers.ctrl = true;
    handler(select.clone());
    handler(press(LemonKey::Character("x".to_owned())));
    assert_eq!(value.get(), "x");
    handler(select);
    value.set("new".to_owned());
    handler(press(LemonKey::Character("!".to_owned())));
    assert_eq!(value.get(), "n!ew");
}
