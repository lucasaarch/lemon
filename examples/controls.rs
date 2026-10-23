//! Boolean controls, anchored panels, modal dialogs and timed notifications.
use lemon::prelude::*;

fn app(cx: &Cx) -> Element {
    let accepted = cx.state(false);
    let notifications = cx.state(true);
    let panel_open = cx.state(false);
    let dialog_open = cx.state(false);
    let toaster = Toaster::new(cx);
    let show_panel = panel_open.clone();
    let show_dialog = dialog_open.clone();
    let close_dialog = dialog_open.clone();
    let saved = toaster.clone();
    let theme = cx.theme();

    let options = Popover::new(
        cx,
        panel_open,
        Button::new(cx, "Options")
            .style(ButtonStyle::Bordered)
            .action(move || show_panel.update(|open| *open = !*open)),
        ColumnView::new()
            .spacing(12.0)
            .child(Text::new("Preferences").weight(600))
            .child(Switch::new(cx, notifications.clone()).label("Notifications")),
    );
    let dialog = Dialog::new(
        cx,
        dialog_open,
        ColumnView::new()
            .spacing(16.0)
            .child(Text::new("Settings").size(20.0).weight(600))
            .child(Text::new("Tab stays inside this dialog. Escape closes it."))
            .child(Checkbox::new(cx, accepted.clone()).label("Accept changes"))
            .child(Button::new(cx, "Done").action(move || close_dialog.set(false))),
    );

    ColumnView::new()
        .fill()
        .inset(24.0)
        .inset_top(48.0)
        .spacing(16.0)
        .align(Align::Start)
        .surface(theme.colors.background)
        .child(Text::new("Lemon controls").size(24.0).weight(600))
        .child(Checkbox::new(cx, accepted).label("Accept terms"))
        .child(Switch::new(cx, notifications).label("Notifications"))
        .child(options)
        .child(Button::new(cx, "Open dialog").action(move || show_dialog.set(true)))
        .child(Button::new(cx, "Show notification").action(move || {
            saved.show("Changes saved");
        }))
        .child(dialog)
        .child(toaster)
        .into_element()
}
fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — controls")
            .size(560.0, 520.0)
            .chrome(WindowChrome::Overlay),
        app,
    );
}
