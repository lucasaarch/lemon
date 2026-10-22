use lemon::prelude::*;

fn field(view: View) -> View {
    view.width(300.0).rounding(10.0)
}

fn app(cx: &Cx) -> Element {
    let name = cx.state(TextFieldState::new(""));
    let password = cx.state(TextFieldState::new(""));

    View::new()
        .fill()
        .center()
        .child(
            ColumnView::new()
                .spacing(12.0)
                .child(TextField::new(cx, name).placeholder("Name").decorate(field))
                .child(
                    SecureField::new(cx, password)
                        .placeholder("Password")
                        .decorate(field),
                ),
        )
        .into_element()
}

fn main() {
    run(WindowConfig::default(), app)
}

#[cfg(test)]
#[path = "../tests/examples/custom.rs"]
mod tests;
