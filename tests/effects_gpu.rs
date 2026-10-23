//! Actual GPU pixel tests. Run explicitly: cargo test --test effects_gpu -- --ignored.
use lemon::{prelude::*, ui::UiState, Renderer, Viewport};
struct Gpu {
    context: vello::util::RenderContext,
    id: usize,
    renderer: Renderer,
}
impl Gpu {
    fn new() -> Self {
        let mut context = vello::util::RenderContext::new();
        let id = pollster::block_on(context.device(None)).expect("this test requires a GPU");
        let renderer = Renderer::new(&context.devices[id].device, Default::default()).unwrap();
        Self {
            context,
            id,
            renderer,
        }
    }
    fn render(&mut self, ui: &UiState, size: u32) -> Vec<u8> {
        let handle = &self.context.devices[self.id];
        let device = &handle.device;
        let queue = &handle.queue;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.renderer
            .render_to_texture(
                device,
                queue,
                ui.scene(),
                &texture.create_view(&Default::default()),
                &vello::RenderParams {
                    base_color: vello::peniko::Color::from_rgba8(0, 0, 0, 0),
                    width: size,
                    height: size,
                    antialiasing_method: vello::AaConfig::Area,
                },
            )
            .unwrap();
        let stride = (size * 4).div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(stride * size),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(size),
                },
            },
            wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let (send, recv) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| send.send(r).unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        recv.recv().unwrap().unwrap();
        let mapped = buffer.slice(..).get_mapped_range();
        mapped
            .chunks(stride as usize)
            .flat_map(|row| row[..(size * 4) as usize].iter().copied())
            .collect()
    }
}
fn pixel(data: &[u8], size: usize, x: usize, y: usize) -> [u8; 4] {
    data[(y * size + x) * 4..(y * size + x) * 4 + 4]
        .try_into()
        .unwrap()
}
fn ui(root: impl Fn(&Cx) -> Element + 'static, scale: f32) -> UiState {
    let mut ui = UiState::default();
    ui.mount(root).unwrap();
    ui.update(
        Viewport {
            width: 64.0,
            height: 64.0,
        },
        scale,
        None,
        false,
    )
    .unwrap();
    ui
}
#[test]
#[ignore = "requires a GPU; run with --ignored"]
fn filters_and_gradients_produce_real_pixels_including_nested_alpha_and_dpi() {
    let mut gpu = Gpu::new();
    let mut scene = ui(
        |_| {
            View::new()
                .width(32.0)
                .height(32.0)
                .surface(Color::rgb8(255, 0, 0))
                .brightness(0.5)
                .opacity(0.5)
                .into_element()
        },
        1.0,
    );
    let output = gpu.render(&scene, 64);
    let p = pixel(&output, 64, 10, 10);
    assert!(
        (124..=132).contains(&p[0]) && (124..=132).contains(&p[3]),
        "straight-alpha brightness/opacity: {p:?}"
    );
    assert_eq!(
        output,
        gpu.render(&scene, 64),
        "cached filters must remain stable on redraw"
    );
    scene = ui(
        |_| {
            View::new()
                .width(32.0)
                .height(32.0)
                .brightness(0.5)
                .child(
                    View::new()
                        .width(24.0)
                        .height(24.0)
                        .brightness(0.5)
                        .surface(Color::rgb8(255, 0, 0)),
                )
                .into_element()
        },
        1.0,
    );
    assert_eq!(scene.scene().effect_layer_count(), 2);
    let output = gpu.render(&scene, 64);
    let p = pixel(&output, 64, 10, 10);
    assert!(
        (60..=70).contains(&p[0]) && p[3] > 250,
        "nested brightness: {p:?}"
    );
    for scale in [1.0, 2.0] {
        scene = ui(
            |_| {
                View::new()
                    .width(20.0)
                    .height(20.0)
                    .surface(Color::rgb8(255, 0, 0))
                    .blur(3.0)
                    .into_element()
            },
            scale,
        );
        let size = (64.0 * scale) as usize;
        let output = gpu.render(&scene, size as u32);
        let edge = pixel(
            &output,
            size,
            (22.0 * scale) as usize,
            (10.0 * scale) as usize,
        );
        let center = pixel(
            &output,
            size,
            (10.0 * scale) as usize,
            (10.0 * scale) as usize,
        );
        assert!(
            edge[3] > 10 && edge[3] < 120,
            "blur must spread alpha outside original bounds: {edge:?}"
        );
        assert!(
            center[3] > 240,
            "blur center should remain opaque: {center:?}"
        );
        assert_eq!(
            pixel(
                &output,
                size,
                (40.0 * scale) as usize,
                (40.0 * scale) as usize
            ),
            [0, 0, 0, 0]
        );
    }
    scene = ui(
        |_| {
            View::new()
                .width(32.0)
                .height(32.0)
                .gradient(RadialGradient::new(
                    (0.5, 0.5),
                    0.5,
                    Color::rgb8(255, 255, 255),
                    Color::rgb8(0, 0, 0),
                ))
                .into_element()
        },
        1.0,
    );
    let output = gpu.render(&scene, 64);
    assert!(pixel(&output, 64, 16, 16)[0] > 230);
    assert!(pixel(&output, 64, 31, 16)[0] < 30);
    scene = ui(
        |_| {
            View::new()
                .width(32.0)
                .height(32.0)
                .gradient(
                    LinearGradient::new((0.0, 0.0), (1.0, 0.0), Color::default(), Color::default())
                        .stops([
                            GradientStop::new(0.0, Color::rgb8(255, 0, 0)),
                            GradientStop::new(0.5, Color::rgb8(0, 255, 0)),
                            GradientStop::new(1.0, Color::rgb8(0, 0, 255)),
                        ]),
                )
                .into_element()
        },
        1.0,
    );
    let output = gpu.render(&scene, 64);
    assert!(pixel(&output, 64, 0, 16)[0] > 230);
    assert!(pixel(&output, 64, 16, 16)[1] > 230);
    assert!(pixel(&output, 64, 31, 16)[2] > 230);
    let image =
        lemon::ImageHandle::from_arc(std::sync::Arc::new(lemon::asset::image_handle::ImageData {
            width: 1,
            height: 1,
            pixels: vec![200, 100, 50, 128],
        }));
    scene = ui(
        move |_| {
            Image::new(image.clone())
                .width(16.0)
                .height(16.0)
                .decorate(|view| view.brightness(0.5))
        },
        1.0,
    );
    let output = gpu.render(&scene, 64);
    let p = pixel(&output, 64, 8, 8);
    for (actual, expected) in p.into_iter().zip([100, 50, 25, 128]) {
        assert!(
            (i32::from(actual) - expected).abs() <= 3,
            "image brightness must preserve alpha and RGB: {p:?}"
        );
    }
}
