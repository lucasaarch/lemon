//! Rich example: image, Select, Slider, and Scroll with scrollbar visuals (V0.4 showcase).

use lemon::{asset::image_handle::ImageData, run, Color, Cx, ImageHandle, WindowConfig};
use lemon_widgets::{children, Column, Image, Row, Scroll, Select, Slider, Text};
use std::sync::Arc;

#[derive(Clone, PartialEq)]
enum Quality {
    Sd,
    Hd,
    UltraHd,
}

fn gradient_image() -> ImageHandle {
    const SIZE: u32 = 128;
    let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let r = (x * 255 / (SIZE - 1)) as u8;
            let g = (y * 255 / (SIZE - 1)) as u8;
            pixels.extend_from_slice(&[r, g, 180, 255]);
        }
    }
    ImageHandle::from_arc(Arc::new(ImageData {
        width: SIZE,
        height: SIZE,
        pixels,
    }))
}

fn app(cx: &Cx) -> lemon::element::Element {
    let quality = cx.use_signal(None::<Quality>);
    let volume = cx.use_signal(0.7_f32);

    let tracks = [
        "01 — Sunrise Prelude",
        "02 — Neon Drift",
        "03 — Crystal Oscillator",
        "04 — Velo Cascade",
        "05 — Midnight Circuit",
        "06 — Quantum Fade",
        "07 — Signal Bloom",
        "08 — Arctic Haze",
        "09 — Kinetic Pulse",
        "10 — Echoing Void",
        "11 — Parallel Light",
        "12 — The Return",
    ];
    let track_content_h = tracks.len() as f32 * 30.0;

    let mut track_list = Column::new().gap(4.0);
    for (idx, name) in tracks.iter().enumerate() {
        track_list = track_list.child(
            Row::new().key(idx as u64).padding(4.0).child(
                Text::new(*name)
                    .font_size(14.0)
                    .color(Color::rgb8(200, 210, 230)),
            ),
        );
    }

    Column::new()
        .padding(24.0)
        .gap(20.0)
        .children(children![
            Text::new("Rich — V0.4 showcase").font_size(20.0),
            Text::new("Image · Select · Slider · Scroll")
                .font_size(13.0)
                .color(Color::rgb8(140, 150, 170)),
            Row::new().gap(24.0).children(children![
                Column::new().gap(8.0).children(children![
                    Text::new("Album art")
                        .font_size(13.0)
                        .color(Color::rgb8(160, 170, 190)),
                    Image::new(gradient_image()).width(160.0).height(160.0),
                ]),
                Column::new().gap(12.0).children(children![
                    Text::new("Quality")
                        .font_size(13.0)
                        .color(Color::rgb8(160, 170, 190)),
                    Select::new(
                        cx,
                        quality,
                        vec![
                            (Quality::Sd, "SD — 480p".to_string()),
                            (Quality::Hd, "HD — 1080p".to_string()),
                            (Quality::UltraHd, "4K Ultra HD".to_string()),
                        ],
                    )
                    .placeholder("Choose quality")
                    .width(180.0),
                    Text::new("Volume")
                        .font_size(13.0)
                        .color(Color::rgb8(160, 170, 190)),
                    Slider::new(volume).width(180.0),
                ]),
            ]),
            Text::new("Tracklist")
                .font_size(16.0)
                .color(Color::rgb8(200, 210, 230)),
            Scroll::new(cx, track_list)
                .height(180.0)
                .width(480.0)
                .content_height(track_content_h),
        ])
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — rich (V0.4)")
            .size(700.0, 600.0),
        app,
    );
}
