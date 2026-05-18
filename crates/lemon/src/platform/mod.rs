//! Platform layer: winit window, wgpu surface, Vello renderer, and frame loop.

mod hit_test;
mod window;

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Instant;

use parley::FontContext;
use vello::peniko::Color;
use vello::util::{RenderContext, RenderSurface};
use vello::{AaConfig, Renderer, RendererOptions, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{
    ElementState, KeyEvent as WinitKeyEvent, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey as WinitNamedKey};
use winit::window::{CursorIcon, Window, WindowId};

use crate::diff::Patch;
use crate::element::events::{Cursor, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey};
use crate::element::Element;
use crate::layout::{layout_pass, sync_scroll_layout_max, LayoutMap, Viewport};
use crate::paint::paint_pass;
use crate::retained::focus::FocusManager;
use crate::retained::RetainedTree;
use crate::runtime::{cx::Cx, Runtime};
use hit_test::{
    dispatch_click, dispatch_outside_clicks, find_node_by_taffy_id, hit_test_focusable,
    hit_test_hover, hit_test_on_click, hit_test_pointer_down, hit_test_scroll, normalize_coords,
    LogicalPoint,
};

pub use window::WindowConfig;

fn patches_need_layout(patches: &[Patch]) -> bool {
    patches.iter().any(|patch| {
        !matches!(
            patch,
            Patch::UpdateWidgetChrome { .. } | Patch::UpdateComponent { .. }
        )
    })
}

/// Root view function type used by [`AppState`].
pub type RootComponent = Arc<dyn Fn(&Cx) -> Element>;

/// Owns the window, GPU surface, runtime, and retained tree for one Lemon app.
///
/// Constructed internally by [`run`]. Advanced integrations can build an [`AppState`] manually
/// and drive [`Runtime::flush_effects`](crate::Runtime::flush_effects) / layout / paint in a
/// custom event loop; most apps should call [`run`] instead.
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
    /// The node currently capturing pointer events (set on pointer-down, cleared on pointer-up).
    pub mouse_capture_node: Option<taffy::NodeId>,
    /// True while the left mouse button is held down.
    pub mouse_button_down: bool,
    pub active_modifiers: Modifiers,
    window_config: WindowConfig,
    root_component: Option<RootComponent>,
    last_cursor: Option<(f32, f32)>,
    mounted: bool,
    last_caret_activity: Instant,
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
            mouse_capture_node: None,
            mouse_button_down: false,
            active_modifiers: Modifiers::default(),
            window_config: config,
            root_component: Some(Arc::new(root)),
            last_cursor: None,
            mounted: false,
            last_caret_activity: Instant::now(),
        }
    }

    fn caret_visible_now(&self) -> bool {
        let elapsed = self.last_caret_activity.elapsed().as_millis();
        if elapsed < 150 {
            return true;
        }
        (elapsed / 530).is_multiple_of(2)
    }

    fn focused_text_input(&self) -> bool {
        let Some(focused_id) = self.focus_manager.focused else {
            return false;
        };
        let Some(root) = self.retained.as_ref().and_then(|t| t.root.as_ref()) else {
            return false;
        };
        find_node_by_taffy_id(root, focused_id).is_some_and(|node| node.text_input.is_some())
    }

    fn tick_caret_blink(&mut self) {
        if self.focused_text_input() {
            self.paint_dirty = true;
        }
    }

    fn apply_runtime_patches(&mut self) {
        crate::lemon_trace!(
            Runtime,
            "{} apply_runtime_patches: flush",
            crate::debug::frame_tag()
        );
        self.runtime.flush_effects();
        let patches = self.runtime.take_patches();
        crate::debug::trace_patches("apply_runtime_patches", &patches);
        if patches.is_empty() {
            return;
        }
        let needs_layout = patches_need_layout(&patches);
        crate::lemon_trace!(
            Runtime,
            "{} needs_layout={needs_layout}",
            crate::debug::frame_tag()
        );
        if let Some(tree) = self.retained.as_mut() {
            if let Err(err) = tree.apply_patches(patches) {
                eprintln!("apply_patches: {err:?}");
            }
        }
        if needs_layout {
            self.run_layout_pass(crate::debug::frame_tag());
        }
        self.paint_dirty = true;
    }

    fn run_layout_pass(&mut self, context: impl AsRef<str>) {
        self.layout_dirty = true;
        let viewport = self.viewport();
        let scale = self.scale_factor();
        let context = context.as_ref();
        crate::lemon_trace!(
            Layout,
            "{context} layout_pass {}x{} scale={scale}",
            viewport.width,
            viewport.height
        );
        let tree = self.retained.as_mut().expect("retained tree");
        match layout_pass(tree, viewport) {
            Ok(map) => {
                self.layout_map = map;
                if let Some(root) = tree.root.as_ref() {
                    sync_scroll_layout_max(root, &self.layout_map);
                }
                self.layout_dirty = false;
                if tree.text_needs_reflow() {
                    crate::lemon_trace!(
                        Layout,
                        "{context} layout_pass ok but text still needs reflow"
                    );
                } else {
                    crate::lemon_trace!(Layout, "{context} layout_pass ok");
                }
            }
            Err(err) => eprintln!("layout_pass: {err:?}"),
        }
    }

    fn needs_layout_before_paint(&self) -> bool {
        self.layout_dirty
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
        let bootstrap_patches = self.runtime.take_patches();
        let mut tree = RetainedTree::mount(element).expect("retained mount");
        if !bootstrap_patches.is_empty() {
            tree.apply_patches(bootstrap_patches)
                .expect("apply bootstrap patches");
        }
        self.retained = Some(tree);
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
        self.tick_caret_blink();

        let frame = crate::debug::next_frame();
        crate::lemon_trace!(Runtime, "frame {frame} update_frame: flush");

        self.runtime.flush_effects();
        let patches = self.runtime.take_patches();
        crate::debug::trace_patches("update_frame", &patches);
        if !patches.is_empty() {
            let needs_layout = patches_need_layout(&patches);
            let tree = self.retained.as_mut().expect("retained tree");
            if let Err(err) = tree.apply_patches(patches) {
                eprintln!("apply_patches: {err:?}");
            }
            if needs_layout {
                self.layout_dirty = true;
            }
            self.paint_dirty = true;
        }

        if self.needs_layout_before_paint() {
            self.run_layout_pass(format!("frame {frame}"));
        }

        if self.paint_dirty {
            self.scene.reset();
            let scale = self.scale_factor();
            let caret_visible = self.caret_visible_now();
            if let Some(tree) = self.retained.as_ref() {
                let stats = paint_pass(
                    tree,
                    &self.layout_map,
                    &mut self.scene,
                    scale,
                    self.focus_manager.focused,
                    caret_visible,
                );
                crate::lemon_trace!(
                    Paint,
                    "frame {frame} paint_pass fills={} strokes={} glyphs={}",
                    stats.fills,
                    stats.strokes,
                    stats.glyph_runs
                );
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
        let Some(tree) = self.retained.as_ref() else {
            return false;
        };
        let Some(root) = tree.root.as_ref() else {
            return false;
        };

        if let Some(node) = hit_test_focusable(root, &self.layout_map, point) {
            if let Some(id) = node.taffy_id {
                self.focus_manager.focused = Some(id);
                self.last_caret_activity = Instant::now();
                self.paint_dirty = true;
            }
        }

        let Some(node) = hit_test_on_click(root, &self.layout_map, point) else {
            return false;
        };
        let handled = dispatch_click(node);
        crate::lemon_trace!(
            Input,
            "click at ({:.1},{:.1}) taffy_id={:?} handled={handled}",
            point.x,
            point.y,
            node.taffy_id
        );
        if handled {
            self.apply_runtime_patches();
        }
        handled
    }

    /// Dispatch `on_pointer_down` to the deepest node under `point` with that handler, set
    /// mouse capture, and dispatch `on_click_outside` to all qualifying nodes.
    fn event_pass_pointer_down(&mut self, point: LogicalPoint) -> bool {
        let Some(root) = self.retained.as_ref().and_then(|t| t.root.as_ref()) else {
            return false;
        };

        dispatch_outside_clicks(root, &self.layout_map, point);

        let hit = hit_test_pointer_down(root, &self.layout_map, point);
        let Some((node, (nx, ny))) = hit else {
            self.mouse_capture_node = None;
            self.apply_runtime_patches();
            return false;
        };

        let capture_id = node.taffy_id;
        if let Some(handler) = node.handlers.on_pointer_down.clone() {
            handler(nx, ny);
        }
        self.mouse_capture_node = capture_id;
        self.apply_runtime_patches();
        true
    }

    /// Dispatch `on_pointer_move` to the currently captured node (if any) with normalized coords.
    fn event_pass_pointer_move(&mut self, point: LogicalPoint) -> bool {
        if !self.mouse_button_down {
            return false;
        }
        let Some(capture_id) = self.mouse_capture_node else {
            return false;
        };

        let (handler, rect) = match self
            .retained
            .as_ref()
            .and_then(|t| t.root.as_ref())
            .and_then(|root| find_node_by_taffy_id(root, capture_id))
        {
            Some(node) => {
                let h = node.handlers.on_pointer_move.clone();
                let r = node
                    .taffy_id
                    .and_then(|id| self.layout_map.get(id))
                    .copied();
                (h, r)
            }
            None => return false,
        };

        if let (Some(handler), Some(rect)) = (handler, rect) {
            let (nx, ny) = normalize_coords(point, &rect);
            handler(nx, ny);
            self.apply_runtime_patches();
            return true;
        }
        false
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
            self.last_caret_activity = Instant::now();
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
        self.last_caret_activity = Instant::now();
        self.apply_runtime_patches();
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

    /// Route a scroll event through hit-test and dispatch `on_scroll` (logical coordinates).
    fn event_pass_scroll(&mut self, point: LogicalPoint, delta_y: f64) -> bool {
        let Some(root) = self.retained.as_ref().and_then(|t| t.root.as_ref()) else {
            return false;
        };
        let Some(node) = hit_test_scroll(root, &self.layout_map, point) else {
            return false;
        };
        if let Some(handler) = node.handlers.on_scroll.clone() {
            handler(delta_y);
            self.apply_runtime_patches();
            true
        } else {
            false
        }
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
                state.event_pass_pointer_move(logical);
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
                state.mouse_button_down = true;
                let clicked = state.event_pass_click(point);
                let pointer_handled = state.event_pass_pointer_down(point);
                if clicked || pointer_handled {
                    state.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                state.mouse_button_down = false;
                state.mouse_capture_node = None;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // Normalize line-based scroll to pixel delta. One line ≈ 20 logical pixels,
                // matching common conventions on platforms that report in line units.
                let delta_y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => f64::from(y) * 20.0,
                    MouseScrollDelta::PixelDelta(pos) => pos.y,
                };
                let point = state
                    .last_cursor
                    .map(|(x, y)| LogicalPoint::new(x, y))
                    .unwrap_or(LogicalPoint::new(0.0, 0.0));
                if state.event_pass_scroll(point, delta_y) {
                    state.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => state.render_frame(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_mut() {
            state.tick_caret_blink();
            if state.focused_text_input() || state.needs_redraw() {
                state.request_redraw();
            }
        }
    }
}

/// Opens a native window and runs `root` as the UI until the window closes.
///
/// `root` is called on the first frame and again whenever reactive state read during the last
/// render changes (signals, dynamic text closures, etc.). Build widgets with [`Column`](crate::Column),
/// [`Text`](crate::Text), [`Button`](crate::Button), and related builders, then finish with
/// [`.into_element()`](crate::Column::into_element).
///
/// # Example
///
/// See the `counter` example in the repository or the crate-level docs in [`crate`].
pub fn run(config: WindowConfig, root: impl Fn(&Cx) -> Element + 'static) {
    crate::debug::configure_from_env();
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
        assert!(state.mouse_capture_node.is_none());
        assert!(!state.mouse_button_down);
    }
}
