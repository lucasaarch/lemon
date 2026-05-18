use lemon::{run, Color, Cx, WindowConfig};
use lemon_widgets::{Button, Column, Row, Scroll, Text, TextFieldState, TextInput};

#[derive(Clone)]
struct Entry {
    name: String,
    email: String,
}

fn app(cx: &Cx) -> lemon::element::Element {
    let name_state = cx.use_signal(TextFieldState::new(""));
    let email_state = cx.use_signal(TextFieldState::new(""));
    let name_focused = cx.use_signal(false);
    let email_focused = cx.use_signal(false);
    let entries = cx.use_signal(Vec::<Entry>::new());
    let scroll_offset = cx.use_signal(0.0f64);

    let mut list_content = Column::new().gap(8.0);
    let entries_snapshot = entries.get();
    if entries_snapshot.is_empty() {
        list_content = list_content.child(
            Text::new("No entries submitted yet.")
                .color(Color::rgb8(140, 140, 160))
                .font_size(14.0),
        );
    } else {
        for entry in entries_snapshot {
            list_content = list_content.child(
                Row::new()
                    .gap(8.0)
                    .child(Text::new(entry.name).font_size(14.0))
                    .child(
                        Text::new(format!("<{}>", entry.email))
                            .font_size(14.0)
                            .color(Color::rgb8(150, 160, 180)),
                    ),
            );
        }
    }

    let entries_submit = entries.clone();
    let name_submit = name_state.clone();
    let email_submit = email_state.clone();
    let name_focus_submit = name_focused.clone();
    let email_focus_submit = email_focused.clone();
    let scroll_offset_submit = scroll_offset.clone();

    Column::new()
        .padding(24.0)
        .gap(12.0)
        .child(Text::new("Form").font_size(22.0))
        .child(
            TextInput::new(name_state, name_focused)
                .placeholder("Name")
                .width(360.0),
        )
        .child(
            TextInput::new(email_state, email_focused)
                .placeholder("Email")
                .width(360.0),
        )
        .child(
            Button::new("Submit")
                .on_click(move || {
                    let name = name_submit.get().value.trim().to_string();
                    let email = email_submit.get().value.trim().to_string();

                    if name.is_empty() || email.is_empty() {
                        return;
                    }

                    entries_submit.update(|items| {
                        items.push(Entry {
                            name: name.clone(),
                            email: email.clone(),
                        });
                    });

                    name_submit.update(|s| {
                        s.value.clear();
                        s.cursor = 0;
                    });
                    email_submit.update(|s| {
                        s.value.clear();
                        s.cursor = 0;
                    });

                    name_focus_submit.set(false);
                    email_focus_submit.set(false);
                    scroll_offset_submit.set(0.0);
                })
                .width(120.0),
        )
        .child(
            Text::new(move || format!("Submitted entries ({})", entries.get().len()))
                .font_size(16.0)
                .color(Color::rgb8(200, 200, 220)),
        )
        .child(
            Scroll::new(list_content, scroll_offset)
                .height(240.0)
                .width(520.0),
        )
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon Form")
            .size(900.0, 700.0),
        app,
    );
}
