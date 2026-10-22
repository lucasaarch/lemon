use std::sync::Arc;

use crate::{element::Element, Cx};

/// Configuration for a window requested through [`Cx::open_window`].
///
/// Defaults to a resizable 900 × 600 logical-point window titled "Lemon".
///
/// ```
/// use lemon::OpenWindowParams;
/// let params = OpenWindowParams::default().title("Settings").size(640.0, 480.0);
/// assert!(params.resizable);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct OpenWindowParams {
    /// Native title bar text.
    pub title: String,
    /// Initial client width in logical points.
    pub width: f32,
    /// Initial client height in logical points.
    pub height: f32,
    /// Whether the user can resize the window.
    pub resizable: bool,
}

impl Default for OpenWindowParams {
    fn default() -> Self {
        Self {
            title: "Lemon".to_owned(),
            width: 900.0,
            height: 600.0,
            resizable: true,
        }
    }
}

impl OpenWindowParams {
    /// Sets the native title bar text.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets the initial client size in logical points.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Enables or disables resizing by the user.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
}

/// Request to the host to create an independent native window.
///
/// The host drains its [`HostContext`](crate::host::HostContext) queue and
/// interprets these parameters; the component runtime never creates OS windows.
pub struct OpenWindowRequest {
    /// Native title and initial dimensions.
    pub params: OpenWindowParams,
    /// Root view to mount in the new window.
    pub root: Arc<dyn Fn(&Cx) -> Element>,
}
