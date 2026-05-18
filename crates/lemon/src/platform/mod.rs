//! Platform layer: winit window, wgpu surface, Vello renderer, and frame loop.

mod hit_test;
mod window;

use std::num::NonZeroUsize;
use std::sync::Arc;

use parley::FontContext;
use vello::peniko::Color;
use vello::util::{RenderContext, RenderSurface};
use vello::{AaConfig, Renderer, RendererOptions, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent as WinitKeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey as WinitNamedKey};
use winit::window::{CursorIcon, Window, WindowId};

use crate::element::events::{Cursor, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey};
use crate::element::Element;
use crate::layout::{layout_pass, LayoutMap, Viewport};
use crate::paint::paint_pass;
use crate::retained::focus::FocusManager;
use crate::retained::RetainedTree;
use crate::runtime::{cx::Cx, Runtime};
use hit_test::{
    dispatch_click, find_node_by_taffy_id, hit_test_hover, hit_test_on_click, LogicalPoint,
};

pub use window::WindowConfig;

/// Root view function type used by [`AppState`].
pub type RootComponent = Arc<dyn Fn(&Cx) -> Element>;

/// Application state for the winit / wgpu / Vello shell (Camada 8).
pub struct AppState {
    pub window: Option<Arc<Window>>,
    pub render_cx: RenderContext,
    pub surface: Option<RenderSurface<'static>>,
    pub renderer: Option<Renderer>,
    pub scene: Scene,
    pub runtime: Runtime,
    pub retained: Option<RetainedTree>,
    pub layout_map: LayoutMap,
    pub font_cx: FontContext,
    pub layout_dirty: bool,
    pub paint_dirty: bool,
    pub focus_manager: FocusManager,
    pub hovered_node_id: Option<taffy::NodeId>,
    pub active_modifiers: Modifiers,
    window_config: WindowConfig,
    root_component: Option<RootComponent>,
    last_cursor: Option<(f32, f32)>,
    mounted: bool,
}

impl AppState {
    pub fn new(config: WindowConfig, root: impl Fn(&Cx) -> Element + 'static) -> Self {
        Self {
            window: None,
            render_cx: RenderContext::new(),
            surface: None,
            renderer: None,
            scene: Scene::new(),
            runtime: Runtime::new(),
            retained: None,
            layout_map: LayoutMap::default(),
            font_cx: FontContext::new(),
            layout_dirty: true,
            paint_dirty: true,
            focus_manager: FocusManager::new(),
            hovered_node_id: None,
            active_modifiers: Modifiers::default(),
            window_config: config,
            root_component: Some(Arc::new(root)),
            last_cursor: None,
            mounted: false,
        }
    }

    fn window_config(&self) -> &WindowConfig {
        &self.window_config
    }

    fn attach_window(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let surface = pollster::block_on(self.render_cx.create_surface(
            window.clone(),
            width,
            height,
            wgpu::PresentMode::AutoVsync,
        ))
        .expect("create wgpu surface");

        let device = &self.render_cx.devices[surface.dev_id].device;
        let renderer = Renderer::new(
            device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: NonZeroUsize::new(1),
                pipeline_cache: None,
            },
        )
        .expect("create vello renderer");

        self.window = Some(window);
        self.surface = Some(surface);
        self.renderer = Some(renderer);
        self.layout_dirty = true;
        self.paint_dirty = true;
    }

    fn ensure_mounted(&mut self) {
        if self.mounted {
            return;
        }
        let root = Arc::clone(self.root_component.as_ref().expect("root component"));
        self.runtime.mount(move |cx| root(cx));
        let element = self
            .runtime
            .root_element()
            .expect("root element after mount");
        self.retained = Some(RetainedTree::mount(element).expect("retained mount"));
        self.mounted = true;
        self.layout_dirty = true;
        self.paint_dirty = true;
    }

    fn viewport(&self) -> Viewport {
        let window = self.window.as_ref().expect("window");
        let scale = window.scale_factor() as f32;
        let size: LogicalSize<f32> = window.inner_size().to_logical(f64::from(scale));
        Viewport {
            width: size.width.max(1.0),
            height: size.height.max(1.0),
        }
    }

    fn scale_factor(&self) -> f32 {
        self.window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0)
    }

    fn update_frame(&mut self) {
        self.ensure_mounted();

        self.runtime.flush_effects();
        let patches = self.runtime.take_patches();
        if !patches.is_empty() {
            let tree = self.retained.as_mut().expect("retained tree");
            if let Err(err) = tree.apply_patches(patches) {
                eprintln!("apply_patches: {err:?}");
            }
            self.layout_dirty = true;
        }

        if self.layout_dirty {
            let viewport = self.viewport();
            let scale = self.scale_factor();
            let tree = self.retained.as_mut().expect("retained tree");
            match layout_pass(tree, viewport, scale) {
                Ok(map) => {
                    self.layout_map = map;
                    self.layout_dirty = false;
                    self.paint_dirty = true;
                }
                Err(err) => eprintln!("layout_pass: {err:?}"),
            }
        }

        if self.paint_dirty {
            self.scene.reset();
            let scale = self.scale_factor();
            if let Some(tree) = self.retained.as_ref() {
                paint_pass(tree, &self.layout_map, &mut self.scene, scale);
            }
            self.paint_dirty = false;
        }
    }

    fn resize_surface(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if let Some(surface) = self.surface.as_mut() {
            self.render_cx.resize_surface(surface, width, height);
            self.layout_dirty = true;
            self.paint_dirty = true;
        }
    }

    /// Route a pointer click through hit-test and dispatch `on_click` (logical coordinates).
    fn event_pass_click(&mut self, point: LogicalPoint) -> bool {
        let Some(root) = self.retained.as_ref().and_then(|t| t.root.as_ref()) else {
            return false;
        };
        let Some(node) = hit_test_on_click(root, &self.layout_map, point) else {
            return false;
        };
        let handled = dispatch_click(node);
        if handled {
            self.layout_dirty = true;
            self.paint_dirty = true;
        }
        handled
    }

    fn event_pass_keyboard(&mut self, key_event: WinitKeyEvent) {
        let lemon_key = winit_key_to_lemon(&key_event.logical_key);
        let state = if key_event.state == ElementState::Pressed {
            KeyState::Pressed
        } else {
            KeyState::Released
        };

        if state == KeyState::Pressed
            && !key_event.repeat
            && lemon_key == LemonKey::Named(NamedKey::Tab)
        {
            if let Some(tree) = self.retained.as_ref() {
                self.focus_manager.cycle(tree, !self.active_modifiers.shift);
            }
            self.paint_dirty = true;
            return;
        }

        let Some(focused_id) = self.focus_manager.focused else {
            return;
        };

        let handler = self
            .retained
            .as_ref()
            .and_then(|tree| tree.root.as_ref())
            .and_then(|root| find_node_by_taffy_id(root, focused_id))
            .and_then(|node| match state {
                KeyState::Pressed => node.handlers.on_key_down.clone(),
                KeyState::Released => node.handlers.on_key_up.clone(),
            });

        let Some(handler) = handler else {
            return;
        };

        handler(KeyEvent {
            key: lemon_key,
            modifiers: self.active_modifiers.clone(),
            repeat: key_event.repeat,
            state,
        });
        self.layout_dirty = true;
        self.paint_dirty = true;
    }

    fn event_pass_hover(&mut self, point: LogicalPoint) {
        let new_id = self
            .retained
            .as_ref()
            .and_then(|tree| tree.root.as_ref())
            .and_then(|root| hit_test_hover(root, &self.layout_map, point))
            .and_then(|node| node.taffy_id);

        if new_id == self.hovered_node_id {
            return;
        }

        let old_leave = self.hovered_node_id.and_then(|old_id| {
            self.retained
                .as_ref()
                .and_then(|tree| tree.root.as_ref())
                .and_then(|root| find_node_by_taffy_id(root, old_id))
                .and_then(|node| node.handlers.on_hover_leave.clone())
        });

        let (new_enter, new_cursor) = if let Some(id) = new_id {
            self.retained
                .as_ref()
                .and_then(|tree| tree.root.as_ref())
                .and_then(|root| find_node_by_taffy_id(root, id))
                .map(|node| {
                    (
                        node.handlers.on_hover_enter.clone(),
                        node.style.cursor.clone(),
                    )
                })
                .unwrap_or((None, Cursor::Default))
        } else {
            (None, Cursor::Default)
        };

        if let Some(handler) = old_leave {
            handler();
        }
        if let Some(handler) = new_enter {
            handler();
        }
        if let Some(window) = &self.window {
            window.set_cursor(lemon_cursor_to_winit(&new_cursor));
        }

        self.hovered_node_id = new_id;
        self.layout_dirty = true;
        self.paint_dirty = true;
    }

    fn cursor_logical(&self, physical_x: f64, physical_y: f64) -> LogicalPoint {
        hit_test::physical_to_logical(physical_x, physical_y, self.scale_factor())
    }

    fn present(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        let width = surface.config.width;
        let height = surface.config.height;
        let device_handle = &self.render_cx.devices[surface.dev_id];

        renderer
            .render_to_texture(
                &device_handle.device,
                &device_handle.queue,
                &self.scene,
                &surface.target_view,
                &vello::RenderParams {
                    base_color: Color::from_rgb8(18, 18, 22),
                    width,
                    height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .expect("render to texture");

        let surface_texture = match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.render_cx.configure_surface(surface);
                window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {
                window.request_redraw();
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

        let mut encoder =
            device_handle
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Surface Blit"),
                });
        surface.blitter.copy(
            &device_handle.device,
            &mut encoder,
            &surface.target_view,
            &surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        device_handle.queue.submit([encoder.finish()]);
        surface_texture.present();
        device_handle
            .device
            .poll(wgpu::PollType::Poll)
            .expect("poll wgpu device");
    }

    fn render_frame(&mut self) {
        self.update_frame();
        self.present();
        self.runtime.flush_deferred_effects();
    }

    fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn needs_redraw(&self) -> bool {
        self.layout_dirty || self.paint_dirty
    }
}

struct LemonApplication {
    state: Option<AppState>,
}

impl ApplicationHandler for LemonApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if state.window.is_some() {
            return;
        }

        let config = state.window_config().clone();
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(config.title.clone())
                        .with_inner_size(LogicalSize::new(config.width, config.height))
                        .with_resizable(config.resizable),
                )
                .expect("create window"),
        );

        state.attach_window(window);
        state.request_redraw();
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
                state.resize_surface(size.width, size.height);
                state.request_redraw();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let current = modifiers.state();
                state.active_modifiers = Modifiers {
                    shift: current.shift_key(),
                    ctrl: current.control_key(),
                    alt: current.alt_key(),
                    meta: current.super_key(),
                };
            }
            WindowEvent::KeyboardInput { event, .. } => {
                state.event_pass_keyboard(event);
                if state.needs_redraw() {
                    state.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let logical = state.cursor_logical(position.x, position.y);
                state.last_cursor = Some((logical.x, logical.y));
                state.event_pass_hover(logical);
                if state.needs_redraw() {
                    state.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let point = state
                    .last_cursor
                    .map(|(x, y)| LogicalPoint::new(x, y))
                    .unwrap_or(LogicalPoint::new(0.0, 0.0));
                if state.event_pass_click(point) {
                    state.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => state.render_frame(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            if state.needs_redraw() {
                state.request_redraw();
            }
        }
    }
}

/// Start the winit event loop with the given window configuration and root component.
pub fn run(config: WindowConfig, root: impl Fn(&Cx) -> Element + 'static) {
    let event_loop = EventLoop::new().expect("create event loop");
    let mut app = LemonApplication {
        state: Some(AppState::new(config, root)),
    };
    event_loop.run_app(&mut app).expect("run event loop");
}

fn winit_key_to_lemon(key: &Key) -> LemonKey {
    match key {
        Key::Character(s) => LemonKey::Character(s.to_string()),
        Key::Named(WinitNamedKey::Tab) => LemonKey::Named(NamedKey::Tab),
        Key::Named(WinitNamedKey::Enter) => LemonKey::Named(NamedKey::Enter),
        Key::Named(WinitNamedKey::Escape) => LemonKey::Named(NamedKey::Escape),
        Key::Named(WinitNamedKey::Space) => LemonKey::Named(NamedKey::Space),
        Key::Named(WinitNamedKey::Backspace) => LemonKey::Named(NamedKey::Backspace),
        Key::Named(WinitNamedKey::Delete) => LemonKey::Named(NamedKey::Delete),
        Key::Named(WinitNamedKey::ArrowLeft) => LemonKey::Named(NamedKey::ArrowLeft),
        Key::Named(WinitNamedKey::ArrowRight) => LemonKey::Named(NamedKey::ArrowRight),
        Key::Named(WinitNamedKey::ArrowUp) => LemonKey::Named(NamedKey::ArrowUp),
        Key::Named(WinitNamedKey::ArrowDown) => LemonKey::Named(NamedKey::ArrowDown),
        Key::Named(WinitNamedKey::Home) => LemonKey::Named(NamedKey::Home),
        Key::Named(WinitNamedKey::End) => LemonKey::Named(NamedKey::End),
        Key::Named(WinitNamedKey::PageUp) => LemonKey::Named(NamedKey::PageUp),
        Key::Named(WinitNamedKey::PageDown) => LemonKey::Named(NamedKey::PageDown),
        _ => LemonKey::Other,
    }
}

fn lemon_cursor_to_winit(cursor: &Cursor) -> CursorIcon {
    match cursor {
        Cursor::Default => CursorIcon::Default,
        Cursor::Pointer => CursorIcon::Pointer,
        Cursor::Text => CursorIcon::Text,
        Cursor::Grab => CursorIcon::Grab,
        Cursor::Grabbing => CursorIcon::Grabbing,
        Cursor::Wait => CursorIcon::Wait,
        Cursor::NotAllowed => CursorIcon::NotAllowed,
        Cursor::Move => CursorIcon::Move,
        Cursor::Crosshair => CursorIcon::Crosshair,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::builders::Text;

    #[test]
    fn app_state_starts_dirty_with_runtime() {
        let state = AppState::new(WindowConfig::default(), |_cx| {
            Text::new("hi").into_element()
        });

        assert!(state.layout_dirty);
        assert!(state.paint_dirty);
        assert!(state.window.is_none());
        assert!(state.retained.is_none());
        assert!(!state.mounted);
        assert!(state.focus_manager.focused.is_none());
        assert!(state.hovered_node_id.is_none());
    }
}
