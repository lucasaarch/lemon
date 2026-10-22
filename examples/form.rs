//! Compose plain/secure fields, select, slider, and scroll widgets in a small form.
//! The optional password is cleared on submit and never added to the history.
//! Type a name, choose a role, and submit settings to the scrollable history.

use lemon::prelude::*;

fn app(cx: &Cx) -> Element {
    let name = cx.state(String::new());
    let password = cx.state(String::new());
    let role = cx.state(None::<String>);
    let volume = cx.state(0.5_f32);
    let history = cx.state(Vec::<String>::new());

    let mut entries = ColumnView::new().spacing(8.0);
    // History only appends, so existing indices remain stable IDs.
    for (id, entry) in history.get().into_iter().enumerate() {
        entries = entries.child(RowView::new().key(id as u64).child(Text::new(entry)));
    }

    let name_submit = name.clone();
    let password_submit = password.clone();
    let role_submit = role.clone();
    let volume_submit = volume.clone();
    let volume_label = volume.clone();
    ColumnView::new()
        .inset(24.0)
        .spacing(12.0)
        .children(children![
            TextField::new(cx, name).placeholder("Name").width(300.0),
            SecureField::new(cx, password)
                .placeholder("Password (not saved)")
                .width(300.0),
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
            Button::new(cx, "Submit").action(move || {
                let name = name_submit.get().trim().to_owned();
                let Some(role) = role_submit.get() else {
                    return;
                };
                if name.is_empty() {
                    return;
                }
                let entry = format!("{name} — {role} — {:.0}%", volume_submit.get() * 100.0);
                history.update(|entries| entries.push(entry));
                name_submit.update(|state| state.clear());
                password_submit.update(|state| state.clear());
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
            .size(420.0, 520.0),
        app,
    );
}

#[cfg(test)]
#[path = "../tests/examples/form.rs"]
mod tests;
