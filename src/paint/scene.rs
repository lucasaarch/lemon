//! Scene ownership and GPU composition for subtree filters.
use std::{cell::RefCell, sync::Arc};
use vello::{
    kurbo::{Affine, Rect},
    peniko::{Blob, ImageAlphaType, ImageData, ImageFormat},
};

/// A Lemon scene, including intermediate layers required by blur and brightness.
/// Use [`Renderer`] to render it; a raw Vello renderer cannot resolve filter layers.
/// Created by [`super::paint_pass`] or [`crate::ui::UiState::scene`].
#[derive(Default)]
pub struct Scene {
    pub(super) raw: vello::Scene,
    pub(super) layers: RefCell<Vec<FilterLayer>>,
    pub(super) identity: Arc<()>,
}

impl Scene {
    /// Creates an empty scene.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears draw commands and filter layers, invalidating cached GPU results.
    pub fn reset(&mut self) {
        self.raw.reset();
        self.layers.get_mut().clear();
        self.identity = Arc::new(());
    }

    /// Returns the number of isolated filter layers, including nested layers.
    pub fn effect_layer_count(&self) -> usize {
        self.layers.borrow().len()
    }
}

pub(super) struct FilterLayer {
    scene: vello::Scene,
    image: ImageData,
    sigma: f32,
    brightness: f32,
}

pub(super) fn filter_layer(
    layers: &RefCell<Vec<FilterLayer>>,
    scene: &mut vello::Scene,
    content: vello::Scene,
    viewport: Rect,
    sigma: f64,
    brightness: f32,
) {
    // Render with enough transparent padding to retain the Gaussian's visible tail.
    // Downsample strong blurs to keep convolution bounded, also respecting texture limits.
    let padding = (sigma * 3.0).ceil();
    let width = viewport.width() + padding * 2.0;
    let height = viewport.height() + padding * 2.0;
    let scale = if sigma > 0.0 {
        (8.0 / sigma).min(1.0)
    } else {
        1.0
    }
    .min(4096.0 / width.max(height).max(1.0));
    let width = (width * scale).ceil().max(1.0) as u32;
    let height = (height * scale).ceil().max(1.0) as u32;
    let image = ImageData {
        data: Blob::new(Arc::new(&[] as &[u8])),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::Alpha,
        width,
        height,
    };
    let mut source = vello::Scene::new();
    source.append(
        &content,
        Some(Affine::scale(scale) * Affine::translate((padding, padding))),
    );
    scene.draw_image(
        &image,
        Affine::translate((-padding, -padding)) * Affine::scale(1.0 / scale),
    );
    layers.borrow_mut().push(FilterLayer {
        scene: source,
        image,
        sigma: (sigma * scale) as f32,
        brightness,
    });
}

/// GPU renderer for Lemon scenes, including Gaussian blur and brightness layers.
/// Own one per device/window. Intermediate results are reused while the scene is unchanged.
/// All intermediate textures remain owned by this renderer; there is no global cache.
///
/// ```no_run
/// use lemon::{Renderer, Scene};
/// fn render(device:&wgpu::Device, queue:&wgpu::Queue, target:&wgpu::TextureView, scene:&Scene) {
///     let mut renderer=Renderer::new(device,vello::RendererOptions::default()).unwrap();
///     renderer.render_to_texture(device,queue,scene,target,&vello::RenderParams {
///         base_color:vello::peniko::Color::from_rgba8(0,0,0,0), width:400,height:300,
///         antialiasing_method:vello::AaConfig::Area,
///     }).unwrap();
/// }
/// ```
pub struct Renderer {
    raw: vello::Renderer,
    filters: Option<FilterPipeline>,
    identity: std::sync::Weak<()>,
    images: Vec<ImageData>,
    antialiasing: Option<vello::AaConfig>,
}

impl Renderer {
    /// Creates a renderer for `device`, using Vello's antialiasing and resource options.
    /// Returns Vello's initialization error when the device/options are unsupported.
    pub fn new(
        device: &wgpu::Device,
        options: vello::RendererOptions,
    ) -> Result<Self, vello::Error> {
        Ok(Self {
            raw: vello::Renderer::new(device, options)?,
            filters: None,
            identity: Default::default(),
            images: Vec::new(),
            antialiasing: None,
        })
    }

    /// Renders a scene, processing nested filters before their enclosing layers.
    /// The device and queue must match those used at construction; output dimensions
    /// and texture requirements follow Vello's `RenderParams`. Returns rendering errors.
    pub fn render_to_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        target: &wgpu::TextureView,
        params: &vello::RenderParams,
    ) -> Result<(), vello::Error> {
        if !self.identity.ptr_eq(&Arc::downgrade(&scene.identity))
            || self.antialiasing != Some(params.antialiasing_method)
        {
            for image in self.images.drain(..) {
                self.raw.unregister_texture(image);
            }
            for layer in scene.layers.borrow().iter() {
                let filters = self
                    .filters
                    .get_or_insert_with(|| FilterPipeline::new(device));
                let source = texture(
                    device,
                    layer.image.width,
                    layer.image.height,
                    wgpu::TextureFormat::Rgba8Unorm,
                    wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                );
                self.raw.render_to_texture(
                    device,
                    queue,
                    &layer.scene,
                    &source.create_view(&Default::default()),
                    &vello::RenderParams {
                        base_color: vello::peniko::Color::from_rgba8(0, 0, 0, 0),
                        width: layer.image.width,
                        height: layer.image.height,
                        antialiasing_method: params.antialiasing_method,
                    },
                )?;
                let output = filters.apply(device, queue, &source, layer.sigma, layer.brightness);
                self.raw.override_image(
                    &layer.image,
                    Some(wgpu::TexelCopyTextureInfoBase {
                        texture: output,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    }),
                );
                self.images.push(layer.image.clone());
            }
            self.identity = Arc::downgrade(&scene.identity);
            self.antialiasing = Some(params.antialiasing_method);
        }
        self.raw
            .render_to_texture(device, queue, &scene.raw, target, params)
    }
}

fn texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("lemon filter layer"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

struct FilterPipeline {
    horizontal: wgpu::ComputePipeline,
    vertical: wgpu::ComputePipeline,
}

impl FilterPipeline {
    fn new(device: &wgpu::Device) -> Self {
        fn pipeline(device: &wgpu::Device, vertical: bool) -> wgpu::ComputePipeline {
            let source = include_str!("filter.wgsl")
                .replace(
                    "OUTPUT_FORMAT",
                    if vertical {
                        "rgba8unorm"
                    } else {
                        "rgba16float"
                    },
                )
                .replace("VERTICAL_PASS", if vertical { "true" } else { "false" });
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("lemon filters"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("lemon filters"),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            })
        }
        Self {
            horizontal: pipeline(device, false),
            vertical: pipeline(device, true),
        }
    }

    fn apply(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &wgpu::Texture,
        sigma: f32,
        brightness: f32,
    ) -> wgpu::Texture {
        let (width, height) = (source.width(), source.height());
        let middle = texture(
            device,
            width,
            height,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let output = texture(
            device,
            width,
            height,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        );
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lemon filter parameters"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bytes: Vec<u8> = [
            sigma.to_le_bytes(),
            brightness.to_le_bytes(),
            width.to_le_bytes(),
            height.to_le_bytes(),
        ]
        .into_iter()
        .flatten()
        .collect();
        queue.write_buffer(&uniform, 0, &bytes);
        let mut encoder = device.create_command_encoder(&Default::default());
        for (pipeline, input, output) in [
            (&self.horizontal, source, &middle),
            (&self.vertical, &middle, &output),
        ] {
            let input = input.create_view(&Default::default());
            let output = output.create_view(&Default::default());
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("lemon filter inputs"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&input),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&output),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform.as_entire_binding(),
                    },
                ],
            });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
        queue.submit([encoder.finish()]);
        output
    }
}
