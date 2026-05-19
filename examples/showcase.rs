//! V0.5 showcase: theme tokens, widget defaults, animation progress, opacity, and a theme toggle.

use std::time::Duration;

use lemon::prelude::*;
use lemon::{run_with_theme, set_active_theme, Theme};

#[derive(Clone, PartialEq)]
enum Focus {
    Overview,
    Motion,
    Polish,
}

fn theme_name(light_mode: Signal<bool>) -> impl Fn() -> String {
    move || {
        if light_mode.get() {
            "Theme: light".to_string()
        } else {
            "Theme: dark".to_string()
        }
    }
}

fn progress_label(progress: f32) -> String {
    format!(
        "Animation progress: {:>3}%",
        (progress * 100.0).round() as u32
    )
}

fn app(cx: &Cx) -> Element {
    let base_theme = cx.use_theme();
    let light_mode = cx.use_signal(false);
    let selected = cx.use_signal(Some(Focus::Overview));
    let density = cx.use_signal(0.58_f32);
    let glow = cx
        .use_animation(AnimationConfig::new(Duration::from_millis(1800)).easing(Easing::EaseInOut));

    let theme = if light_mode.get() {
        Theme::default_light()
    } else {
        base_theme
    };
    set_active_theme(theme.clone());

    let progress = glow.progress();
    if !glow.is_playing() {
        glow.play();
    }

    let panel_opacity = 0.45 + progress * 0.45;
    let pulse_width = 48.0 + progress * 188.0;
    let light_for_toggle = light_mode.clone();
    let light_for_label = light_mode.clone();
    let density_for_text = density.clone();

    Column::new()
        .padding(theme.spacing.xl)
        .gap(theme.spacing.lg)
        .background(theme.colors.background)
        .children(children![
            Row::new()
                .gap(theme.spacing.lg)
                .align_items(Align::Center)
                .children(children![
                    Column::new().gap(theme.spacing.xs).children(children![
                        Text::new("Lemon V0.5 showcase")
                            .font_size(theme.typography.font_size_xl)
                            .weight(700)
                            .color(theme.colors.foreground),
                        Text::new("Theme tokens, animated paint, opacity, and widget chrome together.")
                            .font_size(theme.typography.font_size_md)
                            .color(theme.colors.foreground_secondary),
                    ]),
                    Button::new(cx, theme_name(light_for_label))
                        .width(132.0)
                        .height(40.0)
                        .on_click(move || light_for_toggle.update(|value| *value = !*value)),
                ]),
            View::new()
                .padding(theme.spacing.lg)
                .radius(theme.radius.lg)
                .opacity(panel_opacity)
                .background(theme.colors.surface)
                .border(theme.colors.border, 1.0)
                .children(children![
                    Text::new("Animated opacity panel")
                        .font_size(theme.typography.font_size_lg)
                        .weight(600)
                        .color(theme.colors.foreground),
                    Text::new(progress_label(progress))
                        .font_size(theme.typography.font_size_md)
                        .color(theme.colors.foreground_secondary),
                    View::new()
                        .width(236.0)
                        .height(12.0)
                        .radius(6.0)
                        .background(theme.colors.border)
                        .child(
                            View::new()
                                .width(pulse_width)
                                .height(12.0)
                                .radius(6.0)
                                .background(theme.colors.accent),
                        ),
                ]),
            Row::new().gap(theme.spacing.lg).children(children![
                Column::new()
                    .width(260.0)
                    .padding(theme.spacing.lg)
                    .gap(theme.spacing.md)
                    .radius(theme.radius.lg)
                    .background(theme.colors.surface)
                    .border(theme.colors.border, 1.0)
                    .children(children![
                        Text::new("Theme-aware widgets")
                            .font_size(theme.typography.font_size_lg)
                            .weight(600)
                            .color(theme.colors.foreground),
                        Text::new("Select and Slider read their defaults from the active theme.")
                            .font_size(theme.typography.font_size_sm)
                            .color(theme.colors.foreground_secondary),
                        Select::new(
                            cx,
                            selected,
                            vec![
                                (Focus::Overview, "Overview".to_string()),
                                (Focus::Motion, "Motion".to_string()),
                                (Focus::Polish, "Polish".to_string()),
                            ],
                        )
                        .placeholder("Choose focus")
                        .width(190.0),
                        Slider::new(cx, density).width(190.0),
                        Text::new(move || format!("Density: {:.0}%", density_for_text.get() * 100.0))
                            .font_size(theme.typography.font_size_sm)
                            .color(theme.colors.foreground_secondary),
                    ]),
                Column::new()
                    .width(300.0)
                    .padding(theme.spacing.lg)
                    .gap(theme.spacing.md)
                    .radius(theme.radius.lg)
                    .background(theme.colors.surface)
                    .border(theme.colors.border, 1.0)
                    .children(children![
                        Text::new("Platform polish")
                            .font_size(theme.typography.font_size_lg)
                            .weight(600)
                            .color(theme.colors.foreground),
                        Text::new("Hover the controls, drag the slider, and leave this window idle while the animation keeps redrawing.")
                            .font_size(theme.typography.font_size_sm)
                            .color(theme.colors.foreground_secondary),
                        Row::new().gap(theme.spacing.sm).children(children![
                            View::new()
                                .width(72.0)
                                .height(72.0)
                                .radius(theme.radius.lg)
                                .opacity(0.35 + progress * 0.65)
                                .background(theme.colors.accent),
                            View::new()
                                .width(72.0)
                                .height(72.0)
                                .radius(theme.radius.lg)
                                .opacity(0.9 - progress * 0.45)
                                .background(theme.colors.error),
                            View::new()
                                .width(72.0)
                                .height(72.0)
                                .radius(theme.radius.lg)
                                .opacity(0.55 + progress * 0.3)
                                .background(theme.chrome.focus_ring),
                        ]),
                    ]),
            ]),
        ])
        .into_element()
}

fn main() {
    run_with_theme(
        WindowConfig::default()
            .title("Lemon - V0.5 showcase")
            .size(760.0, 560.0),
        Theme::default_dark(),
        app,
    );
}
