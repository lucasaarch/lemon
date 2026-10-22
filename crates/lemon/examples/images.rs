//! Create an RGBA image in memory and display it at two sizes. No external files needed.

use std::sync::Arc;

use lemon::prelude::*;
use lemon::{asset::image_handle::ImageData, ImageHandle};

fn gradient() -> ImageHandle {
    let mut pixels = Vec::with_capacity(64 * 64 * 4);
    for y in 0..64 {
        for x in 0..64 {
            pixels.extend_from_slice(&[x * 4, y * 4, 180, 255]);
        }
    }
    ImageHandle::from_arc(Arc::new(ImageData {
        width: 64,
        height: 64,
        pixels,
    }))
}

fn app(cx: &Cx) -> Element {
    // Keep the allocation stable across renders; cloned handles share the pixels.
    let image = cx.use_memo(gradient).get();
    Row::new()
        .padding(24.0)
        .gap(16.0)
        .child(Image::new(image.clone()).width(64.0).height(64.0))
        .child(Image::new(image).width(192.0).height(192.0))
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — images")
            .size(340.0, 260.0),
        app,
    );
}
