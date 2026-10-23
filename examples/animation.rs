//! Explicit motion controls size; implicit transitions handle colors, hover and focus.

use std::time::Duration;

use lemon::prelude::*;
use lemon::{run_with_theme, ThemePair};

fn app(cx: &Cx) -> Element {
    let theme = cx.theme();
    let animation =
        cx.motion(AnimationConfig::new(Duration::from_millis(1200)).easing(Easing::EaseInOut));
    let progress = animation.progress();
    let value = cx.state(String::new());
    let alternate = cx.state(false);
    let hovered = cx.state(false);
    let focused = cx.state(false);
    let hover = hovered.clone();
    let focus = focused.clone();
    let color = if alternate.get() {
        theme.colors.surface_hover
    } else {
        theme.colors.accent
    };
    let transition = Transition::new(Duration::from_millis(180)).easing(Easing::EaseOut);

    ColumnView::new()
        .inset(theme.spacing.lg)
        .spacing(theme.spacing.md)
        .surface(theme.colors.background)
        .children(children![
            Text::new(format!("Progress: {:.0}%", progress * 100.0)).color(theme.colors.foreground),
            View::new()
                .width(40.0 + progress * 220.0)
                .height(48.0)
                .rounding(theme.radius.md)
                .opacity(0.3 + progress * 0.7)
                .gradient(RadialGradient::new(
                    (0.25, 0.25),
                    1.0,
                    color,
                    theme.colors.surface
                ))
                .blur((1.0 - progress) * 3.0)
                .brightness(if hovered.get() { 1.3 } else { 1.0 })
                .on_hover(move |inside| hover.set(inside))
                .transition(transition.properties([TransitionProperty::Surface])),
            TextField::new(cx, value)
                .placeholder("Click to focus, then click outside")
                .width(280.0)
                .decorate(|view| view
                    .transition(transition)
                    .on_focus(move |active| focus.set(active))),
            Text::new(if focused.get() {
                "Field focused"
            } else {
                "Field unfocused"
            }),
            Button::new(cx, "Play")
                .action(move || {
                    animation.reset();
                    animation.play();
                    alternate.update(|value| *value = !*value);
                })
                .decorate(|view| view.transition(transition)),
        ])
        .into_element()
}

fn main() {
    let mut themes = ThemePair::default();
    themes.light.radius.md = 12.0;
    themes.dark.radius.md = 12.0;
    run_with_theme(
        WindowConfig::default()
            .title("Lemon — animation")
            .size(360.0, 320.0)
            .on_open(|| println!("Window opened"))
            .on_focus(|active| println!("Window focused: {active}"))
            .on_close(|| println!("Window closed")),
        themes,
        app,
    );
}
