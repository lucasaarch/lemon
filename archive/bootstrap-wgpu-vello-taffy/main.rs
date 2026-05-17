mod layout;
mod ui;

use std::num::NonZeroUsize;
use std::sync::Arc;

use vello::peniko::Color;
use vello::util::{RenderContext, RenderSurface};
use vello::{AaConfig, Renderer, RendererOptions, Scene};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

struct State {
    render_cx: RenderContext,
    surface: RenderSurface<'static>,
    renderer: Renderer,
    scene: Scene,
    ui: ui::UiState,
    window: Arc<Window>,
}

impl State {
    fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let mut render_cx = RenderContext::new();
        let surface = pollster::block_on(render_cx.create_surface(
            window.clone(),
            width,
            height,
            wgpu::PresentMode::AutoVsync,
        ))
        .expect("failed to create render surface");

        let device = &render_cx.devices[surface.dev_id].device;
        let renderer = Renderer::new(
            device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: NonZeroUsize::new(1),
                pipeline_cache: None,
            },
        )
        .expect("failed to create vello renderer");

        Self {
            render_cx,
            surface,
            renderer,
            scene: Scene::new(),
            ui: ui::UiState::new(),
            window,
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.render_cx
                .resize_surface(&mut self.surface, width, height);
        }
    }

    fn render(&mut self) {
        let width = self.surface.config.width;
        let height = self.surface.config.height;
        let device_handle = &self.render_cx.devices[self.surface.dev_id];

        self.scene.reset();
        let scale_factor = self.window.scale_factor() as f32;
        self.ui.draw(
            &mut self.scene,
            width as f32,
            height as f32,
            scale_factor,
        );

        self.renderer
            .render_to_texture(
                &device_handle.device,
                &device_handle.queue,
                &self.scene,
                &self.surface.target_view,
                &vello::RenderParams {
                    base_color: Color::from_rgb8(18, 18, 22),
                    width,
                    height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .expect("failed to render scene");

        let surface_texture = match self.surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.render_cx.configure_surface(&self.surface);
                self.window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {
                self.window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                eprintln!("surface lost");
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("surface validation error");
                return;
            }
        };

        let mut encoder = device_handle.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Surface Blit"),
            },
        );
        self.surface.blitter.copy(
            &device_handle.device,
            &mut encoder,
            &self.surface.target_view,
            &surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        device_handle.queue.submit([encoder.finish()]);
        surface_texture.present();
        device_handle
            .device
            .poll(wgpu::PollType::Poll)
            .expect("failed to poll device");
    }
}

struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::WindowAttributes::default()
                        .with_title("lemon — taffy + vello")
                        .with_inner_size(winit::dpi::LogicalSize::new(900.0, 560.0)),
                )
                .unwrap(),
        );

        self.state = Some(State::new(window));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => state.render(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App { state: None };
    event_loop.run_app(&mut app).unwrap();
}
