//! Desktop host: native windows, GPU presentation, and frame scheduling.

#[cfg(target_os = "macos")]
mod macos;

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Instant;

use crate::paint::Renderer;
use parley::FontContext;
use vello::peniko::Color;
use vello::util::{RenderContext, RenderSurface};
use vello::{AaConfig, RendererOptions};
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

use crate::window::{RootComponent, WindowConfig};

/// Per-window platform state owned by [`AppCx`].
///
/// `WindowState` contains the native window, GPU surface, renderer, runtime, retained tree,
/// layout cache, and input state for exactly one `winit` [`WindowId`]. Most apps should call
/// [`run`] and let Lemon construct this internally.
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
    traffic_lights_position: Option<(f32, f32)>,
    events: crate::window::WindowEvents,
    opened: bool,
    #[cfg(debug_assertions)]
    devtools: Option<crate::devtools::Session>,
    #[cfg(debug_assertions)]
    inspected_target: Option<WindowId>,
    last_window_focus: Option<bool>,
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
            traffic_lights_position: None,
            events: crate::window::WindowEvents::default(),
            opened: false,
            #[cfg(debug_assertions)]
            devtools: None,
            #[cfg(debug_assertions)]
            inspected_target: None,
            last_window_focus: None,
        }
    }

    fn activate_theme(&self) -> HostScope {
        self.host.enter()
    }

    fn attach_window(
        &mut self,
        render_cx: &mut RenderContext,
        window: Arc<Window>,
        config: &WindowConfig,
    ) {
        self.events = config.events.clone();
        self.traffic_lights_position = if config.chrome == crate::WindowChrome::Overlay {
            config.traffic_lights_position
        } else {
            None
        };
        apply_traffic_lights_position(&window, self.traffic_lights_position);
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

    fn notify_open(&mut self) {
        if !std::mem::replace(&mut self.opened, true) {
            let _scope = self.host.enter();
            if let Some(callback) = &self.events.open {
                callback();
            }
        }
    }

    fn notify_window_focus(&mut self, focused: bool) {
        let _scope = self.host.enter();
        if self.last_window_focus.replace(focused) != Some(focused) {
            if let Some(callback) = &self.events.focus {
                callback(focused);
            }
        }
    }

    fn allows_close(&self) -> bool {
        let _scope = self.host.enter();
        self.events
            .close_requested
            .as_ref()
            .is_none_or(|callback| callback())
    }

    fn update_native_chrome(&self) {
        if let Some(window) = &self.window {
            apply_traffic_lights_position(window, self.traffic_lights_position);
        }
    }

    fn ensure_mounted(&mut self) {
        if self.ui.retained().is_some() {
            return;
        }
        let _scope = self.activate_theme();
        let root = self.root_component.take().expect("root component");
        self.ui.mount(move |cx| root(cx)).expect("UI mount");
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
        self.dispatch_input_with_mode(event, false)
    }

    fn dispatch_input_deferred(&mut self, event: crate::input::InputEvent) -> bool {
        self.dispatch_input_with_mode(event, true)
    }

    fn dispatch_input_with_mode(
        &mut self,
        event: crate::input::InputEvent,
        defer_updates: bool,
    ) -> bool {
        let _scope = self.activate_theme();
        let result = if defer_updates {
            self.ui.dispatch_input_deferred(event)
        } else {
            self.ui.dispatch_input(event)
        };
        match result {
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
        self.dispatch_input_deferred(crate::input::InputEvent::PointerMove(point))
    }

    fn event_pass_pointer_up(&mut self, point: LogicalPoint) -> bool {
        self.dispatch_input(crate::input::InputEvent::PointerUp(point))
    }

    fn event_pass_hover(&mut self, point: LogicalPoint) {
        self.dispatch_input_deferred(crate::input::InputEvent::Hover(point));
    }

    fn event_pass_scroll(&mut self, point: LogicalPoint, delta: crate::ScrollDelta) -> bool {
        self.dispatch_input_deferred(crate::input::InputEvent::Scroll { point, delta })
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
        self.update_native_chrome();
        self.update_frame();
        self.present(render_cx);
        self.ui.finish_frame();
        #[cfg(debug_assertions)]
        if let Some(session) = &self.devtools {
            session.publish(&mut self.ui);
        }
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

/// Application initialization context passed to [`run`].
///
/// Loads shared fonts and named themes and queues windows before the native event
/// loop starts. This is separate from the component [`Cx`], which owns view hooks.
/// Each application owns its resources; newly opened windows inherit them.
///
/// ```no_run
/// use lemon::{run, ThemePair, WindowConfig, Text};
/// run(|cx| {
///     cx.load_theme("brand", ThemePair::default());
///     cx.set_theme("brand").unwrap();
///     cx.open_window(WindowConfig::default(), |_| Text::new("Hello").into_element());
/// });
/// ```
pub struct AppCx {
    render_cx: RenderContext,
    windows: HashMap<WindowId, WindowState>,
    themes: HashMap<String, ThemePair>,
    selected_theme: String,
    appearance: Option<NativeTheme>,
    animation_registry: AnimRegistry,
    registered_fonts: Vec<RegisteredFont>,
    host: HostContext,
}

/// A theme name passed to [`AppCx::set_theme`] has not been loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeNotFound {
    /// The unknown theme name. Selection is unchanged when this error is returned.
    pub name: String,
}

impl std::fmt::Display for ThemeNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "theme {:?} has not been loaded", self.name)
    }
}

impl std::error::Error for ThemeNotFound {}

#[derive(Clone)]
struct RegisteredFont {
    family_name: String,
    bytes: Arc<Vec<u8>>,
}

impl Default for AppCx {
    fn default() -> Self {
        Self::new()
    }
}

impl AppCx {
    /// Creates an application context with no windows and Lemon's adaptive theme.
    /// [`run`] creates this context automatically. Custom hosts can create one for
    /// configuration without starting a native event loop.
    pub fn new() -> Self {
        let themes = ThemePair::default();
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
            themes: HashMap::from([("lemon".to_owned(), themes)]),
            selected_theme: "lemon".to_owned(),
            appearance: None,
            animation_registry: host.animations(),
            host,
            registered_fonts: Vec::new(),
        }
    }

    /// Loads or replaces a named theme. Loading does not select it unless it is
    /// already active. The initial active name is `"lemon"`.
    /// A [`ThemePair`] follows system appearance; a single [`Theme`] stays fixed.
    /// Replacing the active theme updates current and future windows without
    /// resetting component state. Names are local to this application.
    ///
    /// ```
    /// use lemon::{AppCx, ThemePair};
    /// let mut cx=AppCx::new();
    /// cx.load_theme("brand",ThemePair::default());
    /// cx.set_theme("brand").unwrap();
    /// ```
    pub fn load_theme(&mut self, name: impl Into<String>, theme: impl Into<ThemePair>) {
        let name = name.into();
        let active = name == self.selected_theme;
        self.themes.insert(name, theme.into());
        if active {
            self.apply_system_theme(self.appearance);
        }
    }

    /// Selects a loaded theme for all windows. Returns [`ThemeNotFound`] without
    /// changing selection when the name is unknown. Select `"lemon"` to restore
    /// the built-in adaptive pair, unless it has been replaced with `load_theme`.
    ///
    /// ```
    /// use lemon::{AppCx, Theme};
    /// let mut cx=AppCx::new();
    /// cx.load_theme("dark",Theme::default_dark());
    /// cx.set_theme("dark").unwrap();
    /// assert!(cx.set_theme("missing").is_err());
    /// ```
    pub fn set_theme(&mut self, name: &str) -> Result<(), ThemeNotFound> {
        if !self.themes.contains_key(name) {
            return Err(ThemeNotFound {
                name: name.to_owned(),
            });
        }
        self.selected_theme = name.to_owned();
        self.apply_system_theme(self.appearance);
        Ok(())
    }

    /// Queues an independent native window, inheriting this application's fonts
    /// and selected theme. Call once per initial window inside [`run`]. The root
    /// may capture shared state and runs on each reactive render, using component [`Cx`].
    /// Later windows can be requested with [`Cx::open_window`].
    ///
    /// ```no_run
    /// use lemon::{run, Text, WindowConfig};
    /// run(|cx| {
    ///     cx.open_window(WindowConfig::default().title("Main"), |_| Text::new("Main").into_element());
    ///     cx.open_window(WindowConfig::default().title("Tools"), |_| Text::new("Tools").into_element());
    /// });
    /// ```
    pub fn open_window(&self, config: WindowConfig, root: impl Fn(&Cx) -> Element + 'static) {
        self.host.request_window(crate::host::OpenWindowRequest {
            config,
            root: Arc::new(root),
        });
    }

    fn apply_system_theme(&mut self, appearance: Option<NativeTheme>) -> bool {
        self.appearance = appearance;
        let theme = resolve_native_theme(&self.themes[&self.selected_theme], appearance);
        if !self.host.set_theme(theme) {
            return false;
        }
        for window in self.windows.values_mut() {
            window.ui.invalidate_paint();
            window.request_redraw();
        }
        true
    }

    fn make_window_state(&self, root: RootComponent) -> WindowState {
        let mut state = WindowState::new(self.host.clone(), root);
        for registered in &self.registered_fonts {
            state
                .ui
                .edit_fonts(|fonts| {
                    register_font_data(
                        fonts,
                        registered.family_name.clone(),
                        Arc::clone(&registered.bytes),
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
    /// [`.font(...)`](crate::Text::font). Returns `FontRegistrationError` if bytes
    /// contain no readable font face; invalid fonts are not stored.
    ///
    /// ```no_run
    /// use lemon::{run, Text, WindowConfig};
    /// let bytes=std::fs::read("assets/Brand.ttf").unwrap();
    /// run(move |cx| {
    ///     cx.load_font("Brand",bytes).unwrap();
    ///     cx.open_window(WindowConfig::default(), |_| Text::new("Hello").font("Brand").into_element());
    /// });
    /// ```
    pub fn load_font(
        &mut self,
        family_name: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<(), FontRegistrationError> {
        let family_name = family_name.into();
        let bytes = Arc::new(bytes.into());

        let mut probe = FontContext::new();
        register_font_data(&mut probe, family_name.clone(), Arc::clone(&bytes))?;

        for window in self.windows.values_mut() {
            window.ui.edit_fonts(|fonts| {
                register_font_data(fonts, family_name.clone(), Arc::clone(&bytes))
            })?;
            window.ui.invalidate_layout();
        }

        self.registered_fonts
            .push(RegisteredFont { family_name, bytes });
        Ok(())
    }

    /// Loads a font from `path` for all current and future windows under `family_name`.
    /// Returns `FontRegistrationError` on I/O failure or unreadable font data.
    ///
    /// ```no_run
    /// use lemon::run;
    /// run(|cx| { cx.load_font_file("Brand", "assets/Brand.ttf").unwrap(); });
    /// ```
    pub fn load_font_file(
        &mut self,
        family_name: impl Into<String>,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), FontRegistrationError> {
        let bytes = std::fs::read(path.as_ref())?;
        self.load_font(family_name, bytes)
    }

    fn remove_window(&mut self, window_id: WindowId) -> bool {
        #[cfg(debug_assertions)]
        {
            // Closing an inspector releases capture and removes its overlay; closing its
            // target also closes the owned inspector, without affecting other windows.
            let target = self
                .windows
                .get(&window_id)
                .and_then(|window| window.inspected_target);
            if let Some(target) = target.and_then(|id| self.windows.get_mut(&id)) {
                target.devtools = None;
                target.ui.inspection = None;
                target.ui.invalidate_paint();
                target.request_redraw();
            }
            let inspectors: Vec<_> = self
                .windows
                .iter()
                .filter(|(_, window)| window.inspected_target == Some(window_id))
                .map(|(id, _)| *id)
                .collect();
            for inspector in inspectors {
                self.remove_window(inspector);
            }
        }
        if let Some(window) = self.windows.remove(&window_id) {
            let _scope = window.host.enter();
            if let Some(callback) = &window.events.close {
                callback();
            }
        }
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

    #[cfg(debug_assertions)]
    fn open_devtools(&mut self, event_loop: &ActiveEventLoop, target_id: WindowId) {
        if let Some(inspector) = self
            .windows
            .values()
            .find(|window| window.inspected_target == Some(target_id))
        {
            if let Some(window) = &inspector.window {
                window.focus_window();
            }
            return;
        }
        let Some(target) = self.windows.get_mut(&target_id) else {
            return;
        };
        if target.inspected_target.is_some() {
            return;
        }
        let session = crate::devtools::Session::new();
        target.ui.inspection = Some(crate::devtools::Capture::default());
        target.devtools = Some(session.clone());
        target.ui.invalidate_paint();
        target.request_redraw();
        // Window roots use Arc by API contract, and run exclusively on the event-loop thread.
        #[allow(clippy::arc_with_non_send_sync)]
        let root: RootComponent = Arc::new(move |cx| session.view(cx));
        let inspector_id = self.spawn_window(
            event_loop,
            crate::host::OpenWindowRequest {
                config: WindowConfig::default()
                    .title("Lemon DevTools")
                    .size(1100.0, 760.0),
                root,
            },
        );
        self.windows
            .get_mut(&inspector_id)
            .unwrap()
            .inspected_target = Some(target_id);
    }

    /// Opens a new native window from a queued [`OpenWindowRequest`](crate::host::OpenWindowRequest).
    ///
    /// Creates the OS window, attaches the GPU surface and renderer, and inserts the new
    /// [`WindowState`] into [`windows`](Self::windows) under its `winit` [`WindowId`].
    fn spawn_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        req: crate::host::OpenWindowRequest,
    ) -> WindowId {
        let window = Arc::new(
            event_loop
                .create_window(native_window_attributes(&req.config))
                .expect("create window"),
        );
        let window_id = window.id();
        self.apply_system_theme(window.theme());
        let mut window_state = self.make_window_state(req.root);
        window_state.attach_window(&mut self.render_cx, window, &req.config);
        window_state.request_redraw();
        self.windows.insert(window_id, window_state);
        self.windows.get_mut(&window_id).unwrap().notify_open();
        window_id
    }
}

impl ApplicationHandler for AppCx {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let state = self;
        if !state.windows.is_empty() {
            return;
        }

        for request in state.host.take_window_requests() {
            state.spawn_window(event_loop, request);
        }
        // No implicit primary window: an empty startup returns immediately.
        if state.windows.is_empty() {
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let state = self;

        let _scope = state
            .windows
            .get(&window_id)
            .map(|window| window.host.enter());
        match event {
            WindowEvent::CloseRequested => {
                if state
                    .windows
                    .get(&window_id)
                    .is_some_and(|window| window.allows_close())
                    && state.remove_window(window_id)
                {
                    event_loop.exit();
                }
            }
            WindowEvent::Destroyed => {
                if state.remove_window(window_id) {
                    event_loop.exit();
                }
            }
            WindowEvent::Focused(focused) => {
                if let Some(window) = state.windows.get_mut(&window_id) {
                    window.notify_window_focus(focused);
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if let Some(window) = state.windows.get_mut(&window_id) {
                    window.dispatch_input(crate::input::InputEvent::PointerLeave);
                }
            }
            WindowEvent::Moved(position) => {
                if let Some(window) = state.windows.get(&window_id) {
                    if let (Some(native), Some(callback)) = (&window.window, &window.events.moved) {
                        let position: winit::dpi::LogicalPosition<f32> =
                            position.to_logical(native.scale_factor());
                        callback(position.x, position.y);
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = state.windows.get_mut(&window_id) {
                    window.ui.invalidate_layout();
                    window.request_redraw();
                    if let (Some(native), Some(callback)) = (&window.window, &window.events.resize)
                    {
                        let size: LogicalSize<f32> =
                            native.inner_size().to_logical(native.scale_factor());
                        callback(size.width, size.height);
                    }
                }
            }
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
                if let (Some(native), Some(callback)) =
                    (&window_state.window, &window_state.events.resize)
                {
                    let logical: LogicalSize<f32> = size.to_logical(native.scale_factor());
                    callback(logical.width, logical.height);
                }
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
                #[cfg(debug_assertions)]
                if event.state == ElementState::Pressed
                    && !event.repeat
                    && event.logical_key == Key::Named(WinitNamedKey::F12)
                {
                    state.open_devtools(event_loop, window_id);
                    return;
                }
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
                let delta = scroll_delta(
                    delta,
                    window_state.active_modifiers.shift,
                    window_state.scale_factor() as f64,
                );
                let point = window_state
                    .last_cursor
                    .map(|(x, y)| LogicalPoint::new(x, y))
                    .unwrap_or(LogicalPoint::new(0.0, 0.0));
                if window_state.event_pass_scroll(point, delta) {
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
        // Spawn windows requested by component callbacks during the last event batch.
        for request in self.host.take_window_requests() {
            self.spawn_window(event_loop, request);
        }
        let animation_redraw = self.tick_animations();
        for window_state in self.windows.values_mut() {
            let _scope = window_state.activate_theme();
            #[cfg(debug_assertions)]
            if let (Some(session), Some(capture)) =
                (&window_state.devtools, &mut window_state.ui.inspection)
            {
                let selected = session.selected.peek();
                let hovered = session.hovered.peek();
                if capture.highlighted != selected || capture.hovered != hovered {
                    capture.highlighted = selected;
                    capture.hovered = hovered;
                    window_state.ui.invalidate_paint();
                }
            }
            if let Err(error) = window_state.ui.apply_updates() {
                eprintln!("UI update: {error:?}");
            }
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

fn apply_traffic_lights_position(window: &Window, position: Option<(f32, f32)>) {
    #[cfg(target_os = "macos")]
    macos::position_traffic_lights(window, position);
    #[cfg(not(target_os = "macos"))]
    let _ = (window, position);
}

fn native_window_attributes(config: &WindowConfig) -> winit::window::WindowAttributes {
    let attributes = Window::default_attributes()
        .with_title(config.title.clone())
        .with_inner_size(LogicalSize::new(config.width, config.height))
        .with_resizable(config.resizable)
        .with_decorations(config.chrome != crate::WindowChrome::Borderless);
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::WindowAttributesExtMacOS;
        if config.chrome == crate::WindowChrome::Overlay {
            return attributes
                .with_titlebar_transparent(true)
                .with_title_hidden(true)
                .with_fullsize_content_view(true);
        }
    }
    #[cfg(not(target_os = "macos"))]
    let attributes = attributes.with_decorations(config.chrome == crate::WindowChrome::System);
    attributes
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

/// Configures an application once, then runs its native windows until the last closes.
///
/// Use the application context to load fonts/themes and call `open_window` for
/// each initial window. Without an explicit theme, Lemon follows the system's
/// light/dark appearance, falling back to dark when unavailable. No windows are
/// created implicitly; returning without opening any window exits immediately.
/// The closure may borrow local configuration because it finishes before the loop.
/// Component roots remain `'static` and receive the component [`Cx`].
///
/// # Panics
/// Panics if native event-loop creation or execution fails. Run on the main thread;
/// native platforms generally permit only one event loop per process.
///
/// ```no_run
/// use lemon::{run, ThemePair, Text, WindowConfig};
/// run(|cx| {
///     cx.load_theme("brand",ThemePair::default());
///     cx.set_theme("brand").unwrap();
///     // cx.load_font("Brand", std::fs::read("Brand.ttf").unwrap()).unwrap();
///     cx.open_window(WindowConfig::default().title("App"), |_| Text::new("Hello").into_element());
/// });
/// ```
pub fn run(initialize: impl FnOnce(&mut AppCx)) {
    crate::debug::configure_from_env();
    let event_loop = EventLoop::new().expect("create event loop");
    let mut cx = AppCx::new();
    {
        let _scope = cx.host.enter();
        initialize(&mut cx);
    }
    event_loop.run_app(&mut cx).expect("run event loop");
}

fn scroll_delta(delta: MouseScrollDelta, shift: bool, scale: f64) -> crate::ScrollDelta {
    let mut delta = match delta {
        MouseScrollDelta::LineDelta(x, y) => {
            crate::ScrollDelta::new(x as f64 * 20.0, y as f64 * 20.0)
        }
        MouseScrollDelta::PixelDelta(position) => {
            crate::ScrollDelta::new(position.x / scale, position.y / scale)
        }
    };
    if shift && delta.x == 0.0 {
        delta = crate::ScrollDelta::new(delta.y, 0.0);
    }
    delta
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
        Cursor::ResizeVertical => CursorIcon::NsResize,
        Cursor::ResizeHorizontal => CursorIcon::EwResize,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::builders::Text;
    use std::path::PathBuf;

    fn setup(config: WindowConfig, root: impl Fn(&Cx) -> Element + 'static) -> AppCx {
        let cx = AppCx::new();
        cx.open_window(config, root);
        cx
    }

    fn themed_setup(
        config: WindowConfig,
        theme: impl Into<ThemePair>,
        root: impl Fn(&Cx) -> Element + 'static,
    ) -> AppCx {
        let mut cx = setup(config, root);
        cx.load_theme("test", theme);
        cx.set_theme("test").unwrap();
        cx
    }

    fn queued_window(cx: &AppCx) -> WindowState {
        let root = cx.host.take_window_requests().remove(0).root;
        cx.make_window_state(root)
    }

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

    #[cfg(debug_assertions)]
    #[test]
    fn devtools_close_releases_capture_and_target_close_removes_only_its_inspector() {
        let mut app = AppCx::new();
        let target_id = WindowId::from(101);
        let inspector_id = WindowId::from(102);
        let other_id = WindowId::from(103);
        let make_window =
            || app.make_window_state(Arc::new(|_| Text::new("window").into_element()));
        let mut target = make_window();
        target.devtools = Some(crate::devtools::Session::new());
        target.ui.inspection = Some(crate::devtools::Capture::default());
        let mut inspector = make_window();
        inspector.inspected_target = Some(target_id);
        let other = make_window();
        app.windows.insert(target_id, target);
        app.windows.insert(inspector_id, inspector);
        app.windows.insert(other_id, other);
        assert!(!app.remove_window(inspector_id));
        let target = app.windows.get(&target_id).unwrap();
        assert!(target.devtools.is_none());
        assert!(target.ui.inspection.is_none());
        assert!(target.ui.paint_dirty());
        // A fresh inspector may be attached after the old one closes.
        let mut inspector =
            app.make_window_state(Arc::new(|_| Text::new("inspector").into_element()));
        inspector.inspected_target = Some(target_id);
        app.windows.insert(inspector_id, inspector);
        assert!(!app.remove_window(target_id));
        assert!(!app.windows.contains_key(&inspector_id));
        assert!(app.windows.contains_key(&other_id));
        assert_eq!(app.windows.len(), 1);
    }

    #[test]
    fn startup_has_no_implicit_window_and_can_queue_multiple_roots() {
        let cx = AppCx::new();
        assert!(cx.host.take_window_requests().is_empty());
        cx.open_window(WindowConfig::default().title("Main"), |_| {
            Text::new("Main").into_element()
        });
        cx.open_window(WindowConfig::default().title("Tools"), |_| {
            Text::new("Tools").into_element()
        });
        let requests = cx.host.take_window_requests();
        assert_eq!(requests.len(), 2);
        for (request, title) in requests.into_iter().zip(["Main", "Tools"]) {
            assert_eq!(request.config.title, title);
            let mut window = cx.make_window_state(request.root);
            window.ensure_mounted();
            assert_eq!(
                window.ui.retained().unwrap().root().unwrap().text_content(),
                Some(title)
            );
        }
    }

    #[test]
    fn named_themes_are_isolated_replaceable_and_unknown_selection_is_atomic() {
        let mut cx = AppCx::new();
        let other = AppCx::new();
        cx.apply_system_theme(Some(NativeTheme::Light));
        let mut pair = ThemePair::default();
        pair.light.radius.md = 42.0;
        pair.dark.radius.md = 43.0;
        cx.load_theme("brand", pair.clone());
        assert_eq!(
            cx.host.theme(),
            Theme::default_light(),
            "loading must not activate"
        );
        cx.set_theme("brand").unwrap();
        assert_eq!(cx.host.theme(), pair.light);
        cx.apply_system_theme(Some(NativeTheme::Dark));
        assert_eq!(cx.host.theme(), pair.dark);
        let err = cx.set_theme("missing").unwrap_err();
        assert_eq!(err.name, "missing");
        assert_eq!(cx.host.theme(), pair.dark);
        assert_eq!(cx.selected_theme, "brand");
        cx.load_theme("brand", Theme::default_light());
        assert_eq!(
            cx.host.theme(),
            Theme::default_light(),
            "replacing active theme applies immediately"
        );
        assert!(
            !cx.apply_system_theme(Some(NativeTheme::Light)),
            "single theme stays fixed"
        );
        assert_eq!(other.host.theme(), Theme::default_dark());
        cx.set_theme("lemon").unwrap();
        assert_eq!(cx.host.theme(), Theme::default_light());
    }

    #[test]
    fn fonts_load_into_current_and_future_windows_and_reject_invalid_data() {
        let mut cx = AppCx::new();
        let mut window = cx.make_window_state(Arc::new(|_| Text::new("Text").into_element()));
        window.ensure_mounted();
        window
            .ui
            .update(
                Viewport {
                    width: 300.0,
                    height: 200.0,
                },
                1.0,
                None,
                false,
            )
            .unwrap();
        let id = WindowId::dummy();
        cx.windows.insert(id, window);
        let bytes = std::fs::read(test_font_path()).unwrap();
        cx.load_font("Shared", bytes).unwrap();
        assert!(cx.windows[&id].ui.layout_dirty());
        assert!(cx
            .windows
            .get_mut(&id)
            .unwrap()
            .ui
            .edit_fonts(|fonts| fonts.collection.family_by_name("Shared").is_some()));
        let mut future = cx.make_window_state(Arc::new(|_| Text::new("Future").into_element()));
        assert!(future
            .ui
            .edit_fonts(|fonts| fonts.collection.family_by_name("Shared").is_some()));
        let count = cx.registered_fonts.len();
        assert!(cx.load_font("Bad", vec![1, 2, 3]).is_err());
        assert_eq!(cx.registered_fonts.len(), count);
    }

    #[test]
    fn chrome_configuration_maps_to_native_decorations() {
        use crate::WindowChrome;
        for chrome in [
            WindowChrome::System,
            WindowChrome::Overlay,
            WindowChrome::Borderless,
        ] {
            let config = WindowConfig::default().chrome(chrome);
            let attributes = native_window_attributes(&config);
            let decorated = chrome == WindowChrome::System
                || (cfg!(target_os = "macos") && chrome == WindowChrome::Overlay);
            assert_eq!(attributes.decorations, decorated);
        }
    }

    #[test]
    fn window_observers_are_per_window_deduplicated_and_close_can_be_vetoed() {
        use std::{
            cell::{Cell, RefCell},
            rc::Rc,
        };
        let log = Rc::new(RefCell::new(Vec::new()));
        let allow = Rc::new(Cell::new(false));
        let opened = log.clone();
        let focused = log.clone();
        let closed = log.clone();
        let accepted = allow.clone();
        let config = WindowConfig::default()
            .on_open(move || opened.borrow_mut().push("open"))
            .on_focus(move |value| {
                focused
                    .borrow_mut()
                    .push(if value { "focus" } else { "blur" })
            })
            .on_close_requested(move || accepted.get())
            .on_close(move || closed.borrow_mut().push("close"));
        assert_eq!(config, config.clone());
        assert_ne!(config, WindowConfig::default());
        let mut app = setup(config.clone(), |_| Text::new("hi").into_element());
        let mut window = queued_window(&app);
        // attach_window copies these observers for both primary and requested windows.
        window.events = config.events;
        window.notify_open();
        window.notify_open();
        window.notify_window_focus(true);
        window.notify_window_focus(true);
        window.notify_window_focus(false);
        assert!(!window.allows_close());
        assert_eq!(*log.borrow(), vec!["open", "focus", "blur"]);
        allow.set(true);
        assert!(window.allows_close());
        let id = WindowId::dummy();
        app.windows.insert(id, window);
        assert!(app.remove_window(id));
        assert!(app.remove_window(id));
        assert_eq!(*log.borrow(), vec!["open", "focus", "blur", "close"]);
    }

    #[test]
    fn app_state_starts_without_windows() {
        let state = setup(WindowConfig::default(), |_cx| {
            Text::new("hi").into_element()
        });

        assert!(state.windows.is_empty());
        assert_eq!(state.host.take_window_requests().len(), 1);
    }

    #[test]
    fn new_window_state_starts_dirty_with_runtime() {
        let state = setup(WindowConfig::default(), |_cx| {
            Text::new("hi").into_element()
        });
        let window_state = queued_window(&state);

        assert!(window_state.ui.layout_dirty());
        assert!(window_state.ui.paint_dirty());
        assert!(window_state.window.is_none());
        assert!(window_state.ui.retained().is_none());
        assert!(window_state.ui.focused().is_none());
    }

    #[test]
    fn removing_last_window_reports_empty_app() {
        let mut state = setup(WindowConfig::default(), |_cx| {
            Text::new("hi").into_element()
        });
        let id = WindowId::dummy();

        state.windows.insert(id, queued_window(&state));

        assert!(state.remove_window(id));
        assert!(state.windows.is_empty());
    }

    #[test]
    fn register_font_bytes_applies_to_new_window_font_context() {
        let mut state = setup(WindowConfig::default(), |_cx| {
            Text::new("hi").into_element()
        });
        let bytes = std::fs::read(test_font_path()).expect("read font bytes");
        state
            .load_font("PlatformFontBytes", bytes)
            .expect("register font bytes");

        let mut window_state = queued_window(&state);
        assert!(window_state.ui.edit_fonts(|fonts| fonts
            .collection
            .family_by_name("PlatformFontBytes")
            .is_some()));
    }

    #[test]
    fn requested_windows_receive_registered_fonts_and_their_own_root() {
        let mut app = setup(WindowConfig::default(), |_| {
            Text::new("Main").into_element()
        });
        app.load_font_file("SettingsFont", test_font_path())
            .unwrap();
        let host = app.host.clone();
        let _scope = host.enter();
        let config = WindowConfig::default()
            .title("Settings")
            .size(480.0, 360.0)
            .resizable(false)
            .chrome(crate::WindowChrome::Overlay)
            .traffic_lights_position(18.0, 22.0);
        Cx::new().open_window(config.clone(), |_| Text::new("Settings").into_element());
        let request = host.take_window_requests().pop().unwrap();
        assert_eq!(request.config, config);
        assert_eq!(request.config.traffic_lights_position, Some((18.0, 22.0)));
        let attributes = native_window_attributes(&request.config);
        assert_eq!(attributes.title, "Settings");
        assert!(!attributes.resizable);
        // This is the same initialization path used by the native spawn handler.
        let mut window = app.make_window_state(request.root);
        assert!(window
            .ui
            .edit_fonts(|fonts| fonts.collection.family_by_name("SettingsFont").is_some()));
        window.ensure_mounted();
        assert_eq!(
            window.ui.retained().unwrap().root().unwrap().text_content(),
            Some("Settings")
        );
    }

    #[test]
    fn register_font_path_applies_to_new_window_font_context() {
        let mut state = setup(WindowConfig::default(), |_cx| {
            Text::new("hi").into_element()
        });
        state
            .load_font_file("PlatformFontPath", test_font_path())
            .expect("register font path");

        let mut window_state = queued_window(&state);
        assert!(window_state.ui.edit_fonts(|fonts| fonts
            .collection
            .family_by_name("PlatformFontPath")
            .is_some()));
    }

    #[test]
    fn system_appearance_selects_default_palette_for_current_and_future_windows() {
        let mut app = setup(WindowConfig::default(), |_| {
            Text::new("Hello").into_element()
        });
        assert_eq!(
            app.host.theme(),
            Theme::default_dark(),
            "unknown appearance uses dark fallback"
        );
        assert!(app.apply_system_theme(Some(NativeTheme::Light)));
        let mut window = queued_window(&app);
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
        assert_eq!(
            app.make_window_state(Arc::new(|_| Text::new("Future").into_element()))
                .host
                .theme(),
            Theme::default_dark()
        );
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
        let mut fixed = themed_setup(WindowConfig::default(), light.clone(), |_| {
            Text::new("Fixed").into_element()
        });
        assert!(!fixed.apply_system_theme(Some(NativeTheme::Dark)));
        assert_eq!(fixed.host.theme(), light);
        let mut adaptive = themed_setup(
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
        use crate::element::builders::ColumnView;

        fn app(cx: &Cx) -> Element {
            let count = cx.state(0);
            let label = count.clone();
            ColumnView::new()
                .child(Text::new(move || format!("Count: {}", label.get())))
                .child(
                    crate::Button::new(cx, "Increment").action(move || count.update(|n| *n += 1)),
                )
                .into_element()
        }
        let mut state = setup(WindowConfig::default(), app);
        let mut window = queued_window(&state);
        window.ensure_mounted();
        let Element::Column(root) = window.ui.runtime().root_element().unwrap() else {
            panic!("column")
        };
        let Element::View(button) = &root.children[1] else {
            panic!("button")
        };
        button.handlers.on_click.as_ref().unwrap()();
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

#[cfg(test)]
mod scroll_delta_tests {
    use super::*;

    #[test]
    fn native_wheel_preserves_axes_scales_pixels_and_maps_shift_once() {
        assert_eq!(
            scroll_delta(MouseScrollDelta::LineDelta(1.0, -2.0), false, 2.0),
            crate::ScrollDelta::new(20.0, -40.0)
        );
        assert_eq!(
            scroll_delta(
                MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(12.0, 20.0)),
                false,
                2.0
            ),
            crate::ScrollDelta::new(6.0, 10.0)
        );
        assert_eq!(
            scroll_delta(MouseScrollDelta::LineDelta(0.0, -2.0), true, 1.0),
            crate::ScrollDelta::new(-40.0, 0.0)
        );
        assert_eq!(
            scroll_delta(MouseScrollDelta::LineDelta(-2.0, 0.0), true, 1.0),
            crate::ScrollDelta::new(-40.0, 0.0)
        );
    }
}
