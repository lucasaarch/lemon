//! Slider widget demo: interactive RGB color mixer.
//!
//! Three `Slider` widgets control the R, G, B channels of a color swatch.
//! The hex code and the swatch update live as you drag.

use lemon::{run, Color, Cx, WindowConfig};
use lemon_widgets::{children, Column, Row, Slider, Text, View};

fn app(cx: &Cx) -> lemon::element::Element {
    let r = cx.use_signal(0.5_f32);
    let g = cx.use_signal(0.3_f32);
    let b = cx.use_signal(0.8_f32);

    let r_val = (r.get() * 255.0) as u8;
    let g_val = (g.get() * 255.0) as u8;
    let b_val = (b.get() * 255.0) as u8;

    let hex_r = r.clone();
    let hex_g = g.clone();
    let hex_b = b.clone();

    Column::new()
        .padding(40.0)
        .gap(20.0)
        .children(children![
            Text::new("Slider").font_size(22.0),
            Text::new("Drag the sliders to blend an RGB color.")
                .font_size(13.0)
                .color(Color::rgb8(140, 150, 170)),
            View::new()
                .width(300.0)
                .height(90.0)
                .radius(10.0)
                .background(Color::rgb8(r_val, g_val, b_val)),
            Text::new(move || {
                format!(
                    "#{:02X}{:02X}{:02X}",
                    (hex_r.get() * 255.0) as u8,
                    (hex_g.get() * 255.0) as u8,
                    (hex_b.get() * 255.0) as u8,
                )
            })
            .font_size(15.0)
            .color(Color::rgb8(180, 190, 210)),
            Row::new().gap(12.0).children(children![
                Text::new("R").font_size(14.0),
                Slider::new(r).width(240.0),
            ]),
            Row::new().gap(12.0).children(children![
                Text::new("G").font_size(14.0),
                Slider::new(g).width(240.0),
            ]),
            Row::new().gap(12.0).children(children![
                Text::new("B").font_size(14.0),
                Slider::new(b).width(240.0),
            ]),
        ])
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — slider")
            .size(480.0, 440.0),
        app,
    );
}
