//! Image widget demo: three programmatic RGBA8 patterns.
//!
//! Images are synthesised at startup — no external files required.
//! Demonstrates `ImageHandle::from_arc` and `Image` sizing.

use lemon::{asset::image_handle::ImageData, run, Color, Cx, ImageHandle, WindowConfig};
use lemon_widgets::{children, Column, Image, Row, Text};
use std::sync::Arc;

fn make_image(width: u32, height: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> ImageHandle {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(&f(x, y));
        }
    }
    ImageHandle::from_arc(Arc::new(ImageData {
        width,
        height,
        pixels,
    }))
}

fn grayscale_gradient() -> ImageHandle {
    make_image(200, 120, |x, _| {
        let v = (x * 255 / 199) as u8;
        [v, v, v, 255]
    })
}

fn checkerboard() -> ImageHandle {
    const CELL: u32 = 15;
    make_image(120, 120, |x, y| {
        let bright = (x / CELL + y / CELL).is_multiple_of(2);
        let v = if bright { 220u8 } else { 55u8 };
        [v, v, v, 255]
    })
}

fn color_gradient() -> ImageHandle {
    make_image(200, 120, |x, y| {
        let r = (x * 255 / 199) as u8;
        let g = (y * 255 / 119) as u8;
        [r, g, 180, 255]
    })
}

fn app(_cx: &Cx) -> lemon::element::Element {
    Column::new()
        .padding(40.0)
        .gap(24.0)
        .children(children![
            Text::new("Image").font_size(22.0),
            Text::new("Programmatic RGBA8 images rendered by Lemon's paint pipeline.")
                .font_size(13.0)
                .color(Color::rgb8(140, 150, 170)),
            Row::new().gap(28.0).children(children![
                Column::new().gap(8.0).children(children![
                    Image::new(grayscale_gradient()).width(200.0).height(120.0),
                    Text::new("Grayscale gradient")
                        .font_size(13.0)
                        .color(Color::rgb8(160, 170, 190)),
                ]),
                Column::new().gap(8.0).children(children![
                    Image::new(checkerboard()).width(120.0).height(120.0),
                    Text::new("Checkerboard")
                        .font_size(13.0)
                        .color(Color::rgb8(160, 170, 190)),
                ]),
                Column::new().gap(8.0).children(children![
                    Image::new(color_gradient()).width(200.0).height(120.0),
                    Text::new("Color gradient")
                        .font_size(13.0)
                        .color(Color::rgb8(160, 170, 190)),
                ]),
            ]),
        ])
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — image")
            .size(780.0, 340.0),
        app,
    );
}
