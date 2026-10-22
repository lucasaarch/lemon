//! A themed view animates its width and opacity. Click Play to start or restart it.

use std::time::Duration;

use lemon::prelude::*;
use lemon::{run_with_theme, ThemePair};

fn app(cx: &Cx) -> Element {
    let theme = cx.theme();
    let animation =
        cx.motion(AnimationConfig::new(Duration::from_millis(1200)).easing(Easing::EaseInOut));
    let progress = animation.progress();

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
                .surface(theme.colors.accent),
            Button::new(cx, "Play").action(move || {
                animation.reset();
                animation.play();
            }),
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
            .size(340.0, 240.0),
        themes,
        app,
    );
}
