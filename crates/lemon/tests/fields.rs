use lemon::prelude::*;

#[test]
fn facade_and_prelude_export_plain_and_secure_fields_with_shared_styles() {
    let cx = Cx::new();
    let plain = State::new(TextFieldState::new("name"));
    let secret = State::new(TextFieldState::new("password"));
    let style: lemon::TextFieldStyle = TextFieldStyle::default().rounding(8.0);
    let field: lemon::TextField = TextField::new(&cx, plain).style(style.clone());
    let secure: lemon::SecureField = SecureField::new(&cx, secret).style(style);
    let Element::Column(root) = ColumnView::new().child(field).child(secure).into_element() else {
        panic!("expected column")
    };
    assert_eq!(root.children.len(), 2);
}

#[test]
#[allow(deprecated)]
fn deprecated_names_and_module_path_remain_compatible() {
    let cx = Cx::new();
    let state = State::new(TextFieldState::new("plain"));
    let style: TextFieldStyle = lemon::TextInputStyle::default();
    let _: TextField = lemon::TextInput::new(&cx, state.clone()).style(style);
    let _: TextField = lemon::prelude::TextInput::new(&cx, state.clone());
    let _: TextField = lemon::widget::text_input::TextInput::new(&cx, state);
    let _: TextFieldStyle = lemon::widget::text_input::TextInputStyle::default();
}
