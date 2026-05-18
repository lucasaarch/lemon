//! [`Cx::use_effect`]: side effects that run after mount and when subscribed signals change.

use lemon::{run, Color, Cx, WindowConfig};
use lemon_widgets::{children, Button, Column, Text};

fn app(cx: &Cx) -> lemon::element::Element {
    let trigger = cx.use_signal(0u32);
    let effect_runs = cx.use_signal(0u32);
    let last_trigger = cx.use_signal(0u32);

    let trigger_read = trigger.clone();
    let runs = effect_runs.clone();
    let last = last_trigger.clone();

    cx.use_effect(move || {
        let value = trigger_read.get();
        runs.update(|n| *n += 1);
        last.set(value);
    });

    let trigger_display = trigger.clone();
    let runs_display = effect_runs.clone();
    let last_display = last_trigger.clone();
    let bump = trigger.clone();

    Column::new()
        .padding(24.0)
        .gap(14.0)
        .children(children![
            Text::new("use_effect").font_size(18.0),
            Text::new(
                "Effects run once after the first frame, then again whenever a signal read inside the closure changes.",
            )
            .font_size(14.0)
            .color(Color::rgb8(140, 150, 170)),
            Text::new(move || format!("trigger signal: {}", trigger_display.get())).font_size(16.0),
            Text::new(move || format!("effect executed: {} times", runs_display.get()))
                .font_size(16.0)
                .color(Color::rgb8(120, 200, 160)),
            Text::new(move || format!("last trigger value seen: {}", last_display.get()))
                .font_size(15.0),
            Button::new("Bump trigger").on_click(move || bump.update(|n| *n += 1)),
            Text::new("Do not call hooks inside the effect body.")
                .font_size(13.0)
                .color(Color::rgb8(100, 110, 130)),
        ])
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — use_effect")
            .size(520.0, 420.0),
        app,
    );
}
