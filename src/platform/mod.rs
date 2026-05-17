//! Platform layer: winit window, wgpu surface, and frame loop (wired in later tasks).

mod window;

use std::sync::Arc;

use parley::FontContext;
use vello::util::{RenderContext, RenderSurface};
use vello::{Renderer, Scene};
use winit::window::Window;

use crate::element::Element;
use crate::layout::LayoutMap;
use crate::retained::RetainedTree;
use crate::runtime::{cx::Cx, Runtime};

pub use window::WindowConfig;

/// Root view function type used by [`AppState`].
pub type RootComponent = Box<dyn Fn(&Cx) -> Element>;

/// Application state for the winit / wgpu / Vello shell (Camada 8).
///
/// GPU resources and the window are populated on first `resumed` (Task 2+).
#[allow(dead_code)]
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
    window_config: WindowConfig,
    root_component: Option<RootComponent>,
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
            window_config: config,
            root_component: Some(Box::new(root)),
        }
    }

    pub fn window_config(&self) -> &WindowConfig {
        &self.window_config
    }

    pub fn root_component(&self) -> Option<&RootComponent> {
        self.root_component.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::builders::Text;

    #[test]
    fn app_state_starts_dirty_with_runtime() {
        let state = AppState::new(WindowConfig::default(), |_cx| Text::new("hi").into_element());

        assert!(state.layout_dirty);
        assert!(state.paint_dirty);
        assert!(state.window.is_none());
        assert!(state.retained.is_none());
        assert!(state.root_component().is_some());
    }
}
