//! Compose text input, select, slider, and scroll widgets in a small form.
//! Type a name, choose a role, and submit settings to the scrollable history.

use lemon::prelude::*;

fn app(cx: &Cx) -> Element {
    let name = cx.use_signal(TextFieldState::new(""));
    let role = cx.use_signal(None::<String>);
    let volume = cx.use_signal(0.5_f32);
    let history = cx.use_signal(Vec::<String>::new());

    let mut entries = Column::new().gap(8.0);
    // History only appends, so existing indices remain stable IDs.
    for (id, entry) in history.get().into_iter().enumerate() {
        entries = entries.child(Row::new().key(id as u64).child(Text::new(entry)));
    }

    let name_submit = name.clone();
    let role_submit = role.clone();
    let volume_submit = volume.clone();
    let volume_label = volume.clone();
    Column::new()
        .padding(24.0)
        .gap(12.0)
        .children(children![
            TextInput::new(cx, name).placeholder("Name").width(300.0),
            Select::new(
                cx,
                role,
                vec![
                    ("Reader".into(), "Reader".into()),
                    ("Editor".into(), "Editor".into()),
                ]
            )
            .placeholder("Choose a role")
            .width(300.0),
            Text::new(move || format!("Volume: {:.0}%", volume_label.get() * 100.0)),
            Slider::new(cx, volume).width(300.0),
            Button::new(cx, "Submit").on_click(move || {
                let name = name_submit.get().value.trim().to_owned();
                let Some(role) = role_submit.get() else {
                    return;
                };
                if name.is_empty() {
                    return;
                }
                let entry = format!("{name} — {role} — {:.0}%", volume_submit.get() * 100.0);
                history.update(|entries| entries.push(entry));
                name_submit.update(|state| state.clear());
            }),
            Text::new("Submitted settings"),
            Scroll::new(cx, entries).width(360.0).height(140.0),
        ])
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — form")
            .size(420.0, 460.0),
        app,
    );
}

#[cfg(test)]
#[path = "../tests/examples/form.rs"]
mod tests;
