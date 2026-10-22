use crate::window::{RootComponent, WindowConfig};

/// Request to the host to create an independent native window.
///
/// The host drains its [`HostContext`](crate::host::HostContext) queue and
/// interprets these parameters; the component runtime never creates OS windows.
pub struct OpenWindowRequest {
    /// Native title and initial dimensions.
    pub config: WindowConfig,
    /// Root view to mount in the new window.
    pub root: RootComponent,
}
