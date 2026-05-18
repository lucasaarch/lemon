//! Widgets: [`TextInput`], [`Scroll`], native focus, caret, and scrollbar (see `lemon-widgets`).

use lemon::{run, Cx, WindowConfig};
use lemon_widgets::{children, Button, Column, Row, Scroll, Text, TextFieldState, TextInput};

#[derive(Clone)]
struct Entry {
    name: String,
    email: String,
}

fn app(cx: &Cx) -> lemon::element::Element {
    let name_state = cx.use_signal(TextFieldState::new(""));
    let email_state = cx.use_signal(TextFieldState::new(""));
    let entries = cx.use_signal(Vec::<Entry>::new());

    let theme = cx.use_theme();

    let list_content = if entries.get().is_empty() {
        Column::new().child(
            Text::new("No entries submitted yet.")
                .color(theme.colors.foreground_secondary)
                .font_size(14.0),
        )
    } else {
        let mut list = Column::new().gap(8.0);
        for (idx, entry) in entries.get().into_iter().enumerate() {
            list = list.child(Row::new().key(idx as u64).gap(8.0).children(children![
                    Text::new(entry.name)
                        .font_size(14.0)
                        .color(theme.colors.foreground),
                    Text::new(format!("<{}>", entry.email))
                        .font_size(14.0)
                        .color(theme.colors.foreground_secondary),
                ]));
        }
        list
    };

    let entries_submit = entries.clone();
    let name_submit = name_state.clone();
    let email_submit = email_state.clone();

    Column::new()
        .padding(24.0)
        .gap(12.0)
        .children(children![
            Text::new("Widgets — form")
                .font_size(18.0)
                .color(theme.colors.foreground),
            Text::new("TextInput + Scroll from lemon-widgets; combines signals, keys, and layout.")
                .font_size(14.0)
                .color(theme.colors.foreground_secondary),
            TextInput::new(cx, name_state)
                .placeholder("Name")
                .width(360.0),
            TextInput::new(cx, email_state)
                .placeholder("Email")
                .width(360.0),
            Button::new(cx, "Submit")
                .on_click(move || {
                    let name = name_submit.get().value.trim().to_string();
                    let email = email_submit.get().value.trim().to_string();

                    if name.is_empty() || email.is_empty() {
                        return;
                    }

                    entries_submit.update(move |items| {
                        items.push(Entry { name, email });
                    });

                    name_submit.update(|s| s.clear());
                    email_submit.update(|s| s.clear());
                })
                .width(120.0),
            Text::new(move || format!("Submitted entries ({})", entries.get().len()))
                .font_size(16.0)
                .color(theme.colors.foreground),
            Scroll::new(cx, list_content).height(240.0).width(520.0),
        ])
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — form (widgets)")
            .size(900.0, 700.0),
        app,
    );
}
