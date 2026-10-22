//! Desktop host: native windows, GPU presentation, and frame scheduling.

mod window;

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc as StdArc;
use std::sync::Arc;
use std::time::Instant;

use parley::FontContext;
use vello::peniko::Color;
use vello::util::{RenderContext, RenderSurface};
use vello::{AaConfig, Renderer, RendererOptions};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{
    ElementState, KeyEvent as WinitKeyEvent, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey as WinitNamedKey};
use winit::window::{CursorIcon, Theme as NativeTheme, Window, WindowId};

use crate::animation::AnimRegistry;
use crate::element::events::{Cursor, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey};
use crate::element::Element;
use crate::font::{register_font_data, FontRegistrationError};
use crate::host::{HostContext, HostScope};
use crate::input::hit_test::{self, LogicalPoint};
use crate::layout::Viewport;
use crate::runtime::cx::Cx;
use crate::theme::{Theme, ThemePair};
use crate::ui::UiState;

pub use window::WindowConfig;

/// Root view function type used by [`AppState`] and [`WindowState`].
pub type RootComponent = Arc<dyn Fn(&Cx) -> Element>;

/// Per-window platform state owned by [`AppState`].
///
/// `WindowState` contains the native window, GPU surface, renderer, runtime, retained tree,
/// layout cache, and input state for exactly one `winit` [`WindowId`]. Most apps should call
/// [`run`] or [`run_with_theme`] and let Lemon construct this internally.
pub struct WindowState {
    /// Native `winit` window handle, attached after the event loop resumes.
    pub window: Option<Arc<Window>>,
    /// GPU surface associated with [`window`](Self::window).
    pub surface: Option<RenderSurface<'static>>,
    /// Vello renderer used for this window's surface.
    pub renderer: Option<Renderer>,
    /// Headless UI processing state; inspect via its getters and dispatch translated input here.
    pub ui: UiState,
    /// Currently active keyboard modifiers for events delivered to this window.
    pub active_modifiers: Modifiers,
    root_component: Option<RootComponent>,
    host: HostContext,
    last_cursor: Option<(f32, f32)>,
    mounted: bool,
}

impl WindowState {
    fn new(host: HostContext, root_component: RootComponent) -> Self {
        Self {
            window: None,
            surface: None,
            renderer: None,
            ui: UiState::default(),
            active_modifiers: Modifiers::default(),
            host,
            root_component: Some(root_component),
            last_cursor: None,
            mounted: false,
        }
    }

    fn activate_theme(&self) -> HostScope {
        self.host.enter()
    }

    fn attach_window(&mut self, render_cx: &mut RenderContext, window: Arc<Window>) {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let surface = pollster::block_on(render_cx.create_surface(
            window.clone(),
            width,
            height,
            wgpu::PresentMode::AutoVsync,
        ))
        .expect("create wgpu surface");

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
        .expect("create vello renderer");

        self.window = Some(window);
        self.surface = Some(surface);
        self.renderer = Some(renderer);
        self.ui.invalidate_layout();
    }

    fn ensure_mounted(&mut self) {
        if self.mounted {
            return;
        }
        let _scope = self.activate_theme();
        let root = Arc::clone(self.root_component.as_ref().expect("root component"));
        self.ui.mount(move |cx| root(cx)).expect("UI mount");
        self.mounted = true;
        self.ui.invalidate_layout();
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
        let _scope = self.activate_theme();
        self.ui.tick_caret_blink();

        let frame = crate::debug::next_frame();
        crate::lemon_trace!(Runtime, "frame {frame} update_frame: flush");

        let viewport = self.viewport();
        let scale = self.scale_factor();
        let focused = self.ui.focused();
        let caret_visible = self.ui.caret_visible_now();
        if let Err(err) = self.ui.update(viewport, scale, focused, caret_visible) {
            eprintln!("UI update: {err:?}");
        }
    }

    fn resize_surface(&mut self, render_cx: &mut RenderContext, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if let Some(surface) = self.surface.as_mut() {
            render_cx.resize_surface(surface, width, height);
            self.ui.invalidate_layout();
        }
    }

    fn dispatch_input(&mut self, event: crate::input::InputEvent) -> bool {
        let _scope = self.activate_theme();
        match self.ui.dispatch_input(event) {
            Ok(result) => {
                if let Some(window) = &self.window {
                    window.set_cursor(lemon_cursor_to_winit(&result.cursor));
                }
                result.handled
            }
            Err(error) => {
                eprintln!("input: {error:?}");
                false
            }
        }
    }

    fn event_pass_click(&mut self, point: LogicalPoint) -> bool {
        self.dispatch_input(crate::input::InputEvent::Click(point))
    }
    fn event_pass_pointer_down(&mut self, point: LogicalPoint) -> bool {
        self.dispatch_input(crate::input::InputEvent::PointerDown(point))
    }
    fn event_pass_pointer_move(&mut self, point: LogicalPoint) -> bool {
        self.dispatch_input(crate::input::InputEvent::PointerMove(point))
    }
    fn event_pass_pointer_up(&mut self, point: LogicalPoint) -> bool {
        self.dispatch_input(crate::input::InputEvent::PointerUp(point))
    }
    fn event_pass_hover(&mut self, point: LogicalPoint) {
        self.dispatch_input(crate::input::InputEvent::Hover(point));
    }
    fn event_pass_scroll(&mut self, point: LogicalPoint, delta_y: f64) -> bool {
        self.dispatch_input(crate::input::InputEvent::Scroll { point, delta_y })
    }
    fn event_pass_keyboard(&mut self, event: WinitKeyEvent) {
        self.dispatch_input(crate::input::InputEvent::Key(KeyEvent {
            key: winit_key_to_lemon(&event.logical_key),
            modifiers: self.active_modifiers.clone(),
            repeat: event.repeat,
            state: if event.state == ElementState::Pressed {
                KeyState::Pressed
            } else {
                KeyState::Released
            },
        }));
    }

    fn cursor_logical(&self, physical_x: f64, physical_y: f64) -> LogicalPoint {
        hit_test::physical_to_logical(physical_x, physical_y, self.scale_factor())
    }

    fn present(&mut self, render_cx: &mut RenderContext) {
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
        let device_handle = &render_cx.devices[surface.dev_id];

        renderer
            .render_to_texture(
                &device_handle.device,
                &device_handle.queue,
                self.ui.scene(),
                &surface.target_view,
                &vello::RenderParams {
                    base_color: theme_background_peniko(self.host.theme().colors.background),
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
                render_cx.configure_surface(surface);
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

    fn render_frame(&mut self, render_cx: &mut RenderContext) {
        self.update_frame();
        self.present(render_cx);
        self.ui.finish_frame();
    }

    fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn needs_redraw(&self) -> bool {
        self.ui.layout_dirty() || self.ui.paint_dirty()
    }
}

/// Top-level platform state for a Lemon application.
///
/// `AppState` owns the shared GPU [`RenderContext`] and the map of live windows keyed by
/// `winit` [`WindowId`]. The default [`run`] and [`run_with_theme`] entry points create one
/// window today, but the state is structured so event dispatch and teardown are window-id based.
pub struct AppState {
    /// Shared WGPU/Vello render context used to create and resize all window surfaces.
    pub render_cx: RenderContext,
    /// Live per-window states keyed by the `WindowId` delivered by `winit` events.
    pub windows: HashMap<WindowId, WindowState>,
    window_config: WindowConfig,
    themes: ThemePair,
    root_component: RootComponent,
    animation_registry: AnimRegistry,
    registered_fonts: Vec<RegisteredFont>,
    host: HostContext,
}

#[derive(Clone)]
struct RegisteredFont {
    family_name: String,
    bytes: StdArc<Vec<u8>>,
}

impl AppState {
    /// Creates application state for a single-window Lemon app.
    ///
    /// The native window is not created until the event loop is resumed. After resume, the first
    /// window is inserted into [`windows`](Self::windows) under its `winit` [`WindowId`].
    /// Its appearance follows the system using [`ThemePair::default`], with dark fallback
    /// when native appearance is unavailable.
    pub fn new(config: WindowConfig, root: impl Fn(&Cx) -> Element + 'static) -> Self {
        Self::with_theme(config, ThemePair::default(), root)
    }

    fn with_theme(
        config: WindowConfig,
        themes: impl Into<ThemePair>,
        root: impl Fn(&Cx) -> Element + 'static,
    ) -> Self {
        let themes = themes.into();
        let host = HostContext::new(resolve_native_theme(&themes, None)).with_clipboard(
            || arboard::Clipboard::new().ok()?.get_text().ok(),
            |text| {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(text);
                }
            },
        );
        Self {
            render_cx: RenderContext::new(),
            windows: HashMap::new(),
            window_config: config,
            animation_registry: host.animations(),
            host,
            themes,
            root_component: Arc::new(root),
            registered_fonts: Vec::new(),
        }
    }

    fn apply_system_theme(&mut self, appearance: Option<NativeTheme>) -> bool {
        let theme = resolve_native_theme(&self.themes, appearance);
        if !self.host.set_theme(theme) {
            return false;
        }
        for window in self.windows.values_mut() {
            // Reactive views have queued patches. Chrome and the window clear color
            // also need repainting when no explicit element property changed.
            window.ui.invalidate_paint();
            window.request_redraw();
        }
        true
    }

    fn window_config(&self) -> &WindowConfig {
        &self.window_config
    }

    fn make_window_state(&self) -> WindowState {
        let mut state = WindowState::new(self.host.clone(), Arc::clone(&self.root_component));
        for registered in &self.registered_fonts {
            state
                .ui
                .edit_fonts(|fonts| {
                    register_font_data(
                        fonts,
                        registered.family_name.clone(),
                        StdArc::clone(&registered.bytes),
                    )
                })
                .expect("font registration was validated before storing");
        }
        state
    }

    /// Registers an in-memory font under `family_name` for all windows in this app.
    ///
    /// Registered fonts are applied to existing windows immediately and to future windows when
    /// they are created. Use the same family in text styles with
    /// [`.font(...)`](crate::Text::font).
    pub fn register_font_bytes(
        &mut self,
        family_name: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<(), FontRegistrationError> {
        let family_name = family_name.into();
        let bytes = StdArc::new(bytes.into());

        let mut probe = FontContext::new();
        register_font_data(&mut probe, family_name.clone(), StdArc::clone(&bytes))?;

        for window in self.windows.values_mut() {
            window.ui.edit_fonts(|fonts| {
                register_font_data(fonts, family_name.clone(), StdArc::clone(&bytes))
            })?;
            window.ui.invalidate_layout();
        }

        self.registered_fonts
            .push(RegisteredFont { family_name, bytes });
        Ok(())
    }

    /// Loads a font from `path` and registers it under `family_name`.
    pub fn register_font_path(
        &mut self,
        family_name: impl Into<String>,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), FontRegistrationError> {
        let bytes = std::fs::read(path.as_ref())?;
        self.register_font_bytes(family_name, bytes)
    }

    fn remove_window(&mut self, window_id: WindowId) -> bool {
        self.windows.remove(&window_id);
        self.windows.is_empty()
    }

    fn tick_animations(&mut self) -> bool {
        let requested = self.animation_registry.take_pending();
        let active = self.animation_registry.tick_active(Instant::now());
        if requested || active {
            self.animation_registry.tick_frame();
        }
        requested || active
    }

    /// Opens a new native window from a queued [`OpenWindowRequest`](crate::host::OpenWindowRequest).
    ///
    /// Creates the OS window, attaches the GPU surface and renderer, and inserts the new
    /// [`WindowState`] into [`windows`](Self::windows) under its `winit` [`WindowId`].
    fn spawn_window(&mut self, event_loop: &ActiveEventLoop, req: crate::host::OpenWindowRequest) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(req.params.title.clone())
                        .with_inner_size(LogicalSize::new(req.params.width, req.params.height))
                        .with_resizable(req.params.resizable),
                )
                .expect("create window"),
        );
        let window_id = window.id();
        self.apply_system_theme(window.theme());
        let mut window_state = WindowState::new(self.host.clone(), req.root);
        window_state.attach_window(&mut self.render_cx, window);
        window_state.request_redraw();
        self.windows.insert(window_id, window_state);
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
        if !state.windows.is_empty() {
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

        let window_id = window.id();
        state.apply_system_theme(window.theme());
        let mut window_state = state.make_window_state();
        window_state.attach_window(&mut state.render_cx, window);
        window_state.request_redraw();
        state.windows.insert(window_id, window_state);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        let _scope = state
            .windows
            .get(&window_id)
            .map(|window| window.host.enter());
        match event {
            WindowEvent::CloseRequested if state.remove_window(window_id) => {
                event_loop.exit();
            }
            WindowEvent::CloseRequested => {}
            WindowEvent::ThemeChanged(appearance) => {
                if state.windows.contains_key(&window_id) {
                    state.apply_system_theme(Some(appearance));
                }
            }
            WindowEvent::Resized(size) => {
                let Some(window_state) = state.windows.get_mut(&window_id) else {
                    return;
                };
                window_state.resize_surface(&mut state.render_cx, size.width, size.height);
                window_state.request_redraw();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let Some(window_state) = state.windows.get_mut(&window_id) else {
                    return;
                };
                let current = modifiers.state();
                window_state.active_modifiers = Modifiers {
                    shift: current.shift_key(),
                    ctrl: current.control_key(),
                    alt: current.alt_key(),
                    meta: current.super_key(),
                };
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let Some(window_state) = state.windows.get_mut(&window_id) else {
                    return;
                };
                window_state.event_pass_keyboard(event);
                if window_state.needs_redraw() {
                    window_state.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let Some(window_state) = state.windows.get_mut(&window_id) else {
                    return;
                };
                let logical = window_state.cursor_logical(position.x, position.y);
                window_state.last_cursor = Some((logical.x, logical.y));
                window_state.event_pass_hover(logical);
                window_state.event_pass_pointer_move(logical);
                if window_state.needs_redraw() {
                    window_state.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let Some(window_state) = state.windows.get_mut(&window_id) else {
                    return;
                };
                let point = window_state
                    .last_cursor
                    .map(|(x, y)| LogicalPoint::new(x, y))
                    .unwrap_or(LogicalPoint::new(0.0, 0.0));
                let clicked = window_state.event_pass_click(point);
                let pointer_handled = window_state.event_pass_pointer_down(point);
                if clicked || pointer_handled {
                    window_state.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                let Some(window_state) = state.windows.get_mut(&window_id) else {
                    return;
                };
                let point = window_state
                    .last_cursor
                    .map(|(x, y)| LogicalPoint::new(x, y))
                    .unwrap_or(LogicalPoint::new(0.0, 0.0));
                let pointer_released = window_state.event_pass_pointer_up(point);
                if pointer_released || window_state.needs_redraw() {
                    window_state.request_redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let Some(window_state) = state.windows.get_mut(&window_id) else {
                    return;
                };
                // Normalize line-based scroll to pixel delta. One line ≈ 20 logical pixels,
                // matching common conventions on platforms that report in line units.
                let delta_y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => f64::from(y) * 20.0,
                    MouseScrollDelta::PixelDelta(pos) => pos.y,
                };
                let point = window_state
                    .last_cursor
                    .map(|(x, y)| LogicalPoint::new(x, y))
                    .unwrap_or(LogicalPoint::new(0.0, 0.0));
                if window_state.event_pass_scroll(point, delta_y) {
                    window_state.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(window_state) = state.windows.get_mut(&window_id) else {
                    return;
                };
                window_state.render_frame(&mut state.render_cx);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Spawn any windows requested by `cx.open_window(...)` during the last event batch.
        if let Some(state) = self.state.as_mut() {
            let requests = state.host.take_window_requests();
            for req in requests {
                state.spawn_window(event_loop, req);
            }
        }

        if let Some(state) = self.state.as_mut() {
            let animation_redraw = state.tick_animations();
            for window_state in state.windows.values_mut() {
                window_state.ui.tick_caret_blink();
                if window_state.ui.focused_text_input()
                    || window_state.needs_redraw()
                    || animation_redraw
                {
                    window_state.request_redraw();
                }
            }
        }
    }
}

fn resolve_native_theme(themes: &ThemePair, appearance: Option<NativeTheme>) -> Theme {
    match appearance {
        Some(NativeTheme::Light) => themes.light.clone(),
        Some(NativeTheme::Dark) | None => themes.dark.clone(),
    }
}

fn theme_background_peniko(color: crate::element::style::Color) -> Color {
    let (r, g, b) = color.to_rgb8();
    Color::from_rgb8(r, g, b)
}

/// Opens a native window and runs `root` as the UI until the window closes.
///
/// Follows the native system light/dark appearance, including changes while running,
/// using [`ThemePair::default`]. Native detection and change events are available on
/// macOS and Windows through winit; unavailable appearance falls back to the dark palette.
///
/// `root` is called on the first frame and again whenever reactive state read during the last
/// render changes (signals, dynamic text closures, etc.). Build widgets with [`Column`](crate::ColumnView),
/// [`Text`](crate::Text), button widgets, and related builders, then finish with
/// [`.into_element()`](crate::ColumnView::into_element).
///
/// # Example
///
/// See the `counter` example in the repository or the crate-level docs in [`crate`].
pub fn run(config: WindowConfig, root: impl Fn(&Cx) -> Element + 'static) {
    run_with_theme(config, ThemePair::default(), root);
}

/// Runs `root` with a custom fixed palette or an adaptive light/dark pair.
///
/// Passing a [`Theme`] keeps that palette fixed. Passing a [`ThemePair`] follows the
/// system appearance with the supplied palettes, preserving component state when it
/// changes. Explicit widget style overrides remain in effect. Unknown native appearance
/// selects the pair's dark palette. Native decorations retain the system appearance.
///
/// The theme is scoped to the originating host during rendering, layout, and painting.
///
/// ```no_run
/// use lemon_desktop::{run_with_theme, Theme, ThemePair, WindowConfig};
/// use lemon_core::prelude::*;
///
/// fn app(cx: &Cx) -> Element {
///     let theme = cx.theme();
///     Text::new("Hello").color(theme.colors.foreground).into_element()
/// }
///
/// let mut light = Theme::default_light();
/// let mut dark = Theme::default_dark();
/// light.radius.md = 10.0;
/// dark.radius.md = 10.0;
/// run_with_theme(WindowConfig::default(), ThemePair::new(light, dark), app);
/// ```
pub fn run_with_theme(
    config: WindowConfig,
    theme: impl Into<ThemePair>,
    root: impl Fn(&Cx) -> Element + 'static,
) {
    run_with_theme_and_fonts(config, theme, root, |_| {});
}

/// Runs `root` with system appearance, registering fonts before the first frame.
///
/// Uses the same adaptive palettes and platform fallback as [`run`].
///
/// # Example
///
/// ```no_run
/// use lemon_core::prelude::*;
/// use lemon_desktop::{run_with_fonts, WindowConfig};
///
/// fn app(_cx: &Cx) -> Element {
///     Text::new("Brand font").font("MyApp Sans").into_element()
/// }
///
/// fn main() {
///     let bundled = std::fs::read("assets/MyAppSans.ttf").unwrap();
///     run_with_fonts(WindowConfig::default(), app, move |state| {
///         state
///             .register_font_bytes("MyApp Sans", bundled)
///             .expect("register bundled font");
///     });
/// }
/// ```
pub fn run_with_fonts(
    config: WindowConfig,
    root: impl Fn(&Cx) -> Element + 'static,
    register_fonts: impl FnOnce(&mut AppState),
) {
    run_with_theme_and_fonts(config, ThemePair::default(), root, register_fonts);
}

/// Runs `root` with a fixed [`Theme`] or adaptive [`ThemePair`], registering fonts first.
///
/// `register_fonts` runs before the event loop starts. Theme selection and platform
/// fallback match [`run_with_theme`].
pub fn run_with_theme_and_fonts(
    config: WindowConfig,
    theme: impl Into<ThemePair>,
    root: impl Fn(&Cx) -> Element + 'static,
    register_fonts: impl FnOnce(&mut AppState),
) {
    crate::debug::configure_from_env();
    let event_loop = EventLoop::new().expect("create event loop");
    let mut state = AppState::with_theme(config, theme, root);
    register_fonts(&mut state);
    let mut app = LemonApplication { state: Some(state) };
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
    use std::path::PathBuf;

    fn test_font_path() -> PathBuf {
        let candidates = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "C:/Windows/Fonts/arial.ttf",
        ];
        candidates
            .iter()
            .map(PathBuf::from)
            .find(|path| path.is_file())
            .expect("expected at least one system TTF font for tests")
    }

    #[test]
    fn app_state_starts_without_windows() {
        let state = AppState::new(WindowConfig::default(), |_cx| {
            Text::new("hi").into_element()
        });

        assert!(state.windows.is_empty());
        assert_eq!(state.window_config().title, WindowConfig::default().title);
    }

    #[test]
    fn new_window_state_starts_dirty_with_runtime() {
        let state = AppState::new(WindowConfig::default(), |_cx| {
            Text::new("hi").into_element()
        });
        let window_state = state.make_window_state();

        assert!(window_state.ui.layout_dirty());
        assert!(window_state.ui.paint_dirty());
        assert!(window_state.window.is_none());
        assert!(window_state.ui.retained().is_none());
        assert!(!window_state.mounted);
        assert!(window_state.ui.focused().is_none());
    }

    #[test]
    fn removing_last_window_reports_empty_app() {
        let mut state = AppState::new(WindowConfig::default(), |_cx| {
            Text::new("hi").into_element()
        });
        let id = WindowId::dummy();

        state.windows.insert(id, state.make_window_state());

        assert!(state.remove_window(id));
        assert!(state.windows.is_empty());
    }

    #[test]
    fn register_font_bytes_applies_to_new_window_font_context() {
        let mut state = AppState::new(WindowConfig::default(), |_cx| {
            Text::new("hi").into_element()
        });
        let bytes = std::fs::read(test_font_path()).expect("read font bytes");
        state
            .register_font_bytes("PlatformFontBytes", bytes)
            .expect("register font bytes");

        let mut window_state = state.make_window_state();
        assert!(window_state.ui.edit_fonts(|fonts| fonts
            .collection
            .family_by_name("PlatformFontBytes")
            .is_some()));
    }

    #[test]
    fn register_font_path_applies_to_new_window_font_context() {
        let mut state = AppState::new(WindowConfig::default(), |_cx| {
            Text::new("hi").into_element()
        });
        state
            .register_font_path("PlatformFontPath", test_font_path())
            .expect("register font path");

        let mut window_state = state.make_window_state();
        assert!(window_state.ui.edit_fonts(|fonts| fonts
            .collection
            .family_by_name("PlatformFontPath")
            .is_some()));
    }

    #[test]
    fn system_appearance_selects_default_palette_for_current_and_future_windows() {
        let mut app = AppState::new(WindowConfig::default(), |_| {
            Text::new("Hello").into_element()
        });
        assert_eq!(
            app.host.theme(),
            Theme::default_dark(),
            "unknown appearance uses dark fallback"
        );
        assert!(app.apply_system_theme(Some(NativeTheme::Light)));
        let mut window = app.make_window_state();
        window.ensure_mounted();
        assert_eq!(window.host.theme(), Theme::default_light());
        let id = WindowId::dummy();
        app.windows.insert(id, window);
        app.windows
            .get_mut(&id)
            .unwrap()
            .ui
            .update(
                Viewport {
                    width: 300.0,
                    height: 200.0,
                },
                1.0,
                None,
                true,
            )
            .unwrap();
        assert!(app.apply_system_theme(Some(NativeTheme::Dark)));
        assert_eq!(app.windows[&id].host.theme(), Theme::default_dark());
        assert!(app.windows[&id].ui.paint_dirty());
        assert_eq!(app.make_window_state().host.theme(), Theme::default_dark());
        app.windows
            .get_mut(&id)
            .unwrap()
            .ui
            .update(
                Viewport {
                    width: 300.0,
                    height: 200.0,
                },
                1.0,
                None,
                true,
            )
            .unwrap();
        assert!(!app.apply_system_theme(Some(NativeTheme::Dark)));
        assert!(
            !app.windows[&id].ui.paint_dirty(),
            "duplicate native events need no repaint"
        );
    }

    #[test]
    fn fixed_custom_theme_ignores_system_changes_and_adaptive_pair_uses_custom_palettes() {
        let mut light = Theme::default_light();
        light.colors.accent = crate::Color::rgb8(160, 40, 80);
        let mut dark = Theme::default_dark();
        dark.colors.accent = crate::Color::rgb8(90, 180, 130);
        let mut fixed = AppState::with_theme(WindowConfig::default(), light.clone(), |_| {
            Text::new("Fixed").into_element()
        });
        assert!(!fixed.apply_system_theme(Some(NativeTheme::Dark)));
        assert_eq!(fixed.host.theme(), light);
        let mut adaptive = AppState::with_theme(
            WindowConfig::default(),
            ThemePair::new(light.clone(), dark.clone()),
            |_| Text::new("Adaptive").into_element(),
        );
        assert_eq!(adaptive.host.theme(), dark);
        adaptive.apply_system_theme(Some(NativeTheme::Light));
        assert_eq!(adaptive.host.theme(), light);
        adaptive.apply_system_theme(Some(NativeTheme::Dark));
        assert_eq!(adaptive.host.theme(), dark);
    }

    #[test]
    fn system_theme_change_preserves_mounted_component_state_and_updates_background() {
        use crate::element::builders::{Button, ColumnView};
        fn app(cx: &Cx) -> Element {
            let count = cx.state(0);
            let label = count.clone();
            ColumnView::new()
                .child(Text::new(move || format!("Count: {}", label.get())))
                .child(Button::new("Increment").action(move || count.update(|n| *n += 1)))
                .into_element()
        }
        let mut state = AppState::new(WindowConfig::default(), app);
        let mut window = state.make_window_state();
        window.ensure_mounted();
        let Element::Column(root) = window.ui.runtime().root_element().unwrap() else {
            panic!("column")
        };
        let Element::Button(button) = &root.children[1] else {
            panic!("button")
        };
        button.on_click.as_ref().unwrap()();
        let id = WindowId::dummy();
        state.windows.insert(id, window);
        state.apply_system_theme(Some(NativeTheme::Light));
        let window = state.windows.get_mut(&id).unwrap();
        let _scope = window.activate_theme();
        window
            .ui
            .update(
                Viewport {
                    width: 400.0,
                    height: 300.0,
                },
                1.0,
                None,
                true,
            )
            .unwrap();
        let root = window.ui.retained().unwrap().root().unwrap();
        let label = root.children[0].text.as_ref().unwrap();
        assert_eq!(label.content, "Count: 1");
        assert_eq!(
            label.style.color,
            Some(Theme::default_light().colors.foreground)
        );
        assert_eq!(
            window.host.theme().colors.background,
            Theme::default_light().colors.background
        );
    }
}
