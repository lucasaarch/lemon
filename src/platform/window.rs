/// Window creation options for [`crate::platform::AppState`].
#[derive(Clone, Debug, PartialEq)]
pub struct WindowConfig {
    pub title: String,
    pub width: f32,
    pub height: f32,
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
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

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
