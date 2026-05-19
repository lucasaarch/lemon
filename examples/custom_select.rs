//! Primitive-first customization demo: a select-like control built from `View`, `Column`, and
//! `Text` instead of the built-in `Select` widget.
//!
//! This is the escape hatch for app-specific visuals. Keep state in signals, give repeated rows
//! stable keys, and use dynamic paint closures when colors depend on interaction state.

use lemon::prelude::*;

#[derive(Clone, Copy, PartialEq)]
struct Accent {
    id: u64,
    label: &'static str,
    color: Color,
}

const ACCENTS: [Accent; 4] = [
    Accent {
        id: 1,
        label: "Blue",
        color: Color::rgb8(80, 145, 255),
    },
    Accent {
        id: 2,
        label: "Green",
        color: Color::rgb8(70, 180, 130),
    },
    Accent {
        id: 3,
        label: "Rose",
        color: Color::rgb8(235, 95, 135),
    },
    Accent {
        id: 4,
        label: "Gold",
        color: Color::rgb8(230, 175, 70),
    },
];

fn selected_label(selected: Signal<Accent>) -> impl Fn() -> String {
    move || format!("Accent: {}", selected.get().label)
}

fn app(cx: &Cx) -> Element {
    let selected = cx.use_signal(ACCENTS[0]);
    let open = cx.use_signal(false);
    let trigger_hovered = cx.use_signal(false);
    let trigger_pressed = cx.use_signal(false);

    let selected_for_swatch = selected.clone();
    let selected_for_label = selected.clone();
    let selected_for_preview = selected.clone();
    let open_for_trigger = open.clone();
    let open_for_outside = open.clone();
    let hovered_for_bg = trigger_hovered.clone();
    let pressed_for_bg = trigger_pressed.clone();
    let hovered_enter = trigger_hovered.clone();
    let hovered_leave = trigger_hovered.clone();
    let pressed_leave = trigger_pressed.clone();
    let pressed_down = trigger_pressed.clone();
    let pressed_up = trigger_pressed.clone();

    let mut root = Column::new()
        .padding(40.0)
        .gap(18.0)
        .width(360.0)
        .on_click_outside(move || open_for_outside.set(false))
        .child(Text::new("Primitive custom select").font_size(22.0))
        .child(
            Text::new("The trigger, listbox, option rows, and swatch are regular Lemon nodes.")
                .font_size(13.0)
                .color(Color::rgb8(140, 150, 170)),
        )
        .child(
            Row::new()
                .gap(12.0)
                .align_items(Align::Center)
                .child(
                    View::new()
                        .width(28.0)
                        .height(28.0)
                        .radius(8.0)
                        .background(move || selected_for_swatch.get().color),
                )
                .child(
                    View::new()
                        .width(220.0)
                        .height(38.0)
                        .padding_x(12.0)
                        .radius(8.0)
                        .background(move || {
                            if pressed_for_bg.get() {
                                Color::rgb8(38, 45, 60)
                            } else if hovered_for_bg.get() {
                                Color::rgb8(48, 57, 74)
                            } else {
                                Color::rgb8(31, 37, 50)
                            }
                        })
                        .border(Color::rgb8(73, 84, 108), 1.0)
                        .align_items(Align::Center)
                        .justify_content(Justify::Center)
                        .cursor(Cursor::Pointer)
                        .on_click(move || open_for_trigger.update(|value| *value = !*value))
                        .on_hover_enter(move || hovered_enter.set(true))
                        .on_hover_leave(move || {
                            hovered_leave.set(false);
                            pressed_leave.set(false);
                        })
                        .on_pointer_down(move |_, _| pressed_down.set(true))
                        .on_pointer_up(move |_, _| pressed_up.set(false))
                        .child(
                            Text::new(selected_label(selected_for_label))
                                .font_size(14.0)
                                .color(Color::rgb8(235, 240, 248)),
                        ),
                ),
        )
        .child(
            Text::new(move || {
                let accent = selected_for_preview.get();
                format!(
                    "Selected #{:02X}{:02X}{:02X}",
                    accent.color.to_rgb8().0,
                    accent.color.to_rgb8().1,
                    accent.color.to_rgb8().2
                )
            })
            .font_size(13.0)
            .color(Color::rgb8(175, 185, 205)),
        );

    if open.get() {
        let mut dropdown = Column::new()
            .key(100)
            .width(260.0)
            .padding(6.0)
            .gap(4.0)
            .radius(10.0)
            .background(Color::rgb8(24, 29, 40))
            .border(Color::rgb8(73, 84, 108), 1.0);

        for accent in ACCENTS {
            let selected_for_row = selected.clone();
            let open_for_row = open.clone();
            let is_current = selected.get().id == accent.id;
            dropdown = dropdown.child(
                Row::new()
                    .key(accent.id)
                    .height(34.0)
                    .padding_x(10.0)
                    .gap(10.0)
                    .radius(7.0)
                    .align_items(Align::Center)
                    .cursor(Cursor::Pointer)
                    .background(if is_current {
                        Color::rgb8(43, 52, 70)
                    } else {
                        Color::rgb8(24, 29, 40)
                    })
                    .on_click(move || {
                        selected_for_row.set(accent);
                        open_for_row.set(false);
                    })
                    .child(
                        View::new()
                            .width(14.0)
                            .height(14.0)
                            .radius(7.0)
                            .background(accent.color),
                    )
                    .child(
                        Text::new(accent.label)
                            .font_size(14.0)
                            .color(Color::rgb8(235, 240, 248)),
                    ),
            );
        }

        root = root.child(dropdown);
    }

    root.into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon - custom select")
            .size(460.0, 380.0),
        app,
    );
}
