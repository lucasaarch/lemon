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
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Lemon".to_owned(),
            width: 900.0,
            height: 600.0,
            resizable: true,
            chrome: WindowChrome::System,
        }
    }
}

impl WindowConfig {
    /// Selects native chrome; applies to primary and subsequently opened windows.
    pub fn chrome(mut self, chrome: WindowChrome) -> Self {
        self.chrome = chrome;
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
            .chrome(WindowChrome::Overlay);

        assert_eq!(config.title, "Counter");
        assert_eq!(config.width, 640.0);
        assert_eq!(config.height, 480.0);
        assert!(!config.resizable);
        assert_eq!(config.chrome, WindowChrome::Overlay);
    }
}
