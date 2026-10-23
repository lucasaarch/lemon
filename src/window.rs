//! Backend-independent window configuration and root content.

/// Root content shared by window requests and native window initialization.
///
/// The function runs on the UI thread and may capture shared [`crate::State`] values.
///
/// ```
/// use lemon::{window::RootComponent, Text};
/// let root: RootComponent = std::sync::Arc::new(|_| Text::new("Window").into_element());
/// ```
pub type RootComponent = std::sync::Arc<dyn Fn(&crate::Cx) -> crate::element::Element>;

/// Native window chrome. Defaults to the operating system title bar and borders.
///
/// ```
/// use lemon::{WindowChrome, WindowConfig};
/// let window = WindowConfig::default().chrome(WindowChrome::Overlay);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowChrome {
    /// Use the operating system title bar, borders and window buttons.
    #[default]
    System,
    /// Extend content behind a transparent title bar on macOS, keeping traffic lights.
    /// Reserve space for the buttons in your content. On Windows and Linux this
    /// removes decorations, including native buttons; draw your own controls.
    Overlay,
    /// Remove the title bar and native buttons on all supported desktop platforms.
    /// Borderless resizing and shadows depend on the window manager.
    Borderless,
}

/// Native window configuration used by [`crate::run`] and [`crate::Cx::open_window`].
///
/// Defaults to a resizable 900 × 600 logical-point window titled "Lemon".
///
/// Sizes are in **logical points** (winit logical pixels), not physical pixels.
///
/// ```
/// use lemon::WindowConfig;
///
/// let config = WindowConfig::default()
///     .title("My App")
///     .size(800.0, 600.0)
///     .resizable(true);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct WindowConfig {
    /// Window title shown in the title bar.
    pub title: String,
    /// Initial client width in logical points.
    pub width: f32,
    /// Initial client height in logical points.
    pub height: f32,
    /// Whether the user can resize the window.
    pub resizable: bool,
    /// Native title bar and border appearance (default: system decorations).
    pub chrome: WindowChrome,
    /// Optional macOS traffic-light group position, in logical points from the
    /// client area's top-left to the close button's top-left. Defaults to the OS position.
    /// Only used with [`WindowChrome::Overlay`]; ignored on other platforms.
    /// Coordinates must be finite and nonnegative; invalid direct field values are ignored.
    pub traffic_lights_position: Option<(f32, f32)>,
    /// Native lifecycle observers; empty by default. Configure using the `on_*` methods.
    pub events: WindowEvents,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Lemon".to_owned(),
            width: 900.0,
            height: 600.0,
            resizable: true,
            chrome: WindowChrome::System,
            traffic_lights_position: None,
            events: WindowEvents::default(),
        }
    }
}

impl WindowConfig {
    /// Selects native chrome; applies to primary and subsequently opened windows.
    pub fn chrome(mut self, chrome: WindowChrome) -> Self {
        self.chrome = chrome;
        self
    }

    /// Positions macOS traffic lights as a group, preserving their native spacing and actions.
    ///
    /// `x` and `y` are logical points from the client area's top-left to the close
    /// button's top-left. Requires [`WindowChrome::Overlay`]; other chrome modes and
    /// platforms ignore it. The host keeps the group inside the available window
    /// space when resized and leaves native full-screen controls alone.
    /// Reserve room for the native buttons in your content; no layout inset is added.
    /// Without this setting the operating system chooses the position.
    ///
    /// # Panics
    /// Panics if either coordinate is negative or non-finite.
    ///
    /// ```
    /// use lemon::{WindowChrome, WindowConfig};
    /// let window = WindowConfig::default()
    ///     .chrome(WindowChrome::Overlay)
    ///     .traffic_lights_position(20.0, 20.0);
    /// ```
    pub fn traffic_lights_position(mut self, x: f32, y: f32) -> Self {
        assert!(
            x.is_finite() && y.is_finite() && x >= 0.0 && y >= 0.0,
            "traffic light coordinates must be finite and nonnegative"
        );
        self.traffic_lights_position = Some((x, y));
        self
    }

    /// Sets the window title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets the initial window size (`width`, `height`) in logical points.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Enables or disables user resizing.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_overrides_defaults() {
        let config = WindowConfig::default()
            .title("Counter")
            .size(640.0, 480.0)
            .resizable(false)
            .chrome(WindowChrome::Overlay)
            .traffic_lights_position(20.0, 24.0);

        assert_eq!(config.title, "Counter");
        assert_eq!(config.width, 640.0);
        assert_eq!(config.height, 480.0);
        assert!(!config.resizable);
        assert_eq!(config.chrome, WindowChrome::Overlay);
        assert_eq!(config.traffic_lights_position, Some((20.0, 24.0)));
    }
    #[test]
    fn traffic_light_position_defaults_to_native_and_rejects_invalid_coordinates() {
        assert_eq!(WindowConfig::default().traffic_lights_position, None);
        for value in [-1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(std::panic::catch_unwind(
                || WindowConfig::default().traffic_lights_position(value, 0.0)
            )
            .is_err());
            assert!(std::panic::catch_unwind(
                || WindowConfig::default().traffic_lights_position(0.0, value)
            )
            .is_err());
        }
        assert_eq!(
            WindowConfig::default()
                .traffic_lights_position(0.0, 0.0)
                .traffic_lights_position,
            Some((0.0, 0.0))
        );
    }
}

/// Window lifecycle callbacks, configured through [`WindowConfig`] builder methods.
/// Clones share callbacks; equality compares callback identity, not closure contents.
#[derive(Clone, Default)]
pub struct WindowEvents {
    pub(crate) open: Option<std::rc::Rc<dyn Fn()>>,
    pub(crate) close: Option<std::rc::Rc<dyn Fn()>>,
    pub(crate) close_requested: Option<std::rc::Rc<dyn Fn() -> bool>>,
    pub(crate) focus: Option<std::rc::Rc<dyn Fn(bool)>>,
    pub(crate) resize: Option<std::rc::Rc<dyn Fn(f32, f32)>>,
    pub(crate) moved: Option<std::rc::Rc<dyn Fn(f32, f32)>>,
}
impl std::fmt::Debug for WindowEvents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowEvents")
            .field("open", &self.open.is_some())
            .field("close", &self.close.is_some())
            .field("close_requested", &self.close_requested.is_some())
            .field("focus", &self.focus.is_some())
            .field("resize", &self.resize.is_some())
            .field("moved", &self.moved.is_some())
            .finish()
    }
}
impl PartialEq for WindowEvents {
    fn eq(&self, other: &Self) -> bool {
        fn same<T: ?Sized>(a: &Option<std::rc::Rc<T>>, b: &Option<std::rc::Rc<T>>) -> bool {
            match (a, b) {
                (Some(a), Some(b)) => std::rc::Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
        }
        same(&self.open, &other.open)
            && same(&self.close, &other.close)
            && same(&self.close_requested, &other.close_requested)
            && same(&self.focus, &other.focus)
            && same(&self.resize, &other.resize)
            && same(&self.moved, &other.moved)
    }
}
impl WindowConfig {
    /// Called once after the native window and renderer are attached to the host.
    /// Applies independently to main and `Cx::open_window` windows. No callback by default.
    ///
    /// ```no_run
    /// use lemon::WindowConfig;
    /// let config = WindowConfig::default().on_open(|| println!("opened"));
    /// ```
    pub fn on_open(mut self, callback: impl Fn() + 'static) -> Self {
        self.events.open = Some(std::rc::Rc::new(callback));
        self
    }
    /// Called once when the host removes the window after an accepted close.
    /// Runs before native resources are dropped; does not run when close is canceled.
    /// No callback by default.
    ///
    /// ```
    /// use lemon::WindowConfig;
    /// let config = WindowConfig::default().on_close(|| println!("closed"));
    /// ```
    pub fn on_close(mut self, callback: impl Fn() + 'static) -> Self {
        self.events.close = Some(std::rc::Rc::new(callback));
        self
    }
    /// Intercepts a user's close request. Return `true` to close or `false` to keep open.
    /// With no callback, close requests are accepted. Forced OS destruction cannot be vetoed.
    ///
    /// ```no_run
    /// use lemon::{State, WindowConfig};
    /// let saved = State::new(true);
    /// let config = WindowConfig::default().on_close_requested(move || saved.get());
    /// ```
    pub fn on_close_requested(mut self, callback: impl Fn() -> bool + 'static) -> Self {
        self.events.close_requested = Some(std::rc::Rc::new(callback));
        self
    }
    /// Observes native window activation (`true`) and deactivation (`false`).
    /// Independent of which Lemon element has keyboard focus. No callback by default.
    ///
    /// ```
    /// use lemon::WindowConfig;
    /// let config = WindowConfig::default().on_focus(|active| println!("active: {active}"));
    /// ```
    pub fn on_focus(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.events.focus = Some(std::rc::Rc::new(callback));
        self
    }
    /// Observes client width/height changes in logical points, including DPI changes.
    /// No callback by default; initial dimensions are available in this configuration.
    ///
    /// ```
    /// use lemon::WindowConfig;
    /// let config = WindowConfig::default().on_resize(|width, height| println!("{width} x {height}"));
    /// ```
    pub fn on_resize(mut self, callback: impl Fn(f32, f32) + 'static) -> Self {
        self.events.resize = Some(std::rc::Rc::new(callback));
        self
    }
    /// Observes native outer-window `(x,y)` movement in logical screen coordinates.
    /// Not reported by platforms/window managers that do not expose window positions.
    /// No callback by default.
    ///
    /// ```
    /// use lemon::WindowConfig;
    /// let config = WindowConfig::default().on_move(|x, y| println!("position: {x}, {y}"));
    /// ```
    pub fn on_move(mut self, callback: impl Fn(f32, f32) + 'static) -> Self {
        self.events.moved = Some(std::rc::Rc::new(callback));
        self
    }
}
