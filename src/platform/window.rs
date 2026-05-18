/// Window size and title passed to [`crate::platform::run`].
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
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Lemon".to_owned(),
            width: 900.0,
            height: 600.0,
            resizable: true,
        }
    }
}

impl WindowConfig {
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
            .resizable(false);

        assert_eq!(config.title, "Counter");
        assert_eq!(config.width, 640.0);
        assert_eq!(config.height, 480.0);
        assert!(!config.resizable);
    }
}
