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
