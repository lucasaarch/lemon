//! Headless processing of a window's UI, independent of native presentation.

use parley::FontContext;
use taffy::NodeId;
use vello::Scene;

use crate::element::Element;
use crate::retained::RetainedError;
use crate::{
    layout_pass, paint_pass, sync_scroll_layout_max, Cx, LayoutMap, RetainedTree, Runtime, Viewport,
};

/// UI processing state shared by desktop hosts and headless tests.
///
/// Hosts own viewport changes, focus/input, frame scheduling, and presentation.
/// Apply reactive updates before layout, then rebuild the scene only when dirty.
///
/// ```
/// use lemon_core::{Text, Viewport, ui::UiState};
/// let mut ui = UiState::default();
/// ui.mount(|_| Text::new("Hello").into_element()).unwrap();
/// ui.update(Viewport { width: 400.0, height: 300.0 }, 1.0, None, true).unwrap();
/// assert!(!ui.layout_dirty);
/// assert!(!ui.paint_dirty);
/// ```
pub struct UiState {
    /// Reactive component runtime.
    pub runtime: Runtime,
    /// Retained state, absent until mounted.
    pub retained: Option<RetainedTree>,
    /// Latest layout geometry in logical points.
    pub layout_map: LayoutMap,
    /// Registered fonts and text measurement context.
    pub font_cx: FontContext,
    /// Scene buffer rebuilt after visual changes.
    pub scene: Scene,
    /// Host or tree geometry changes require layout before painting.
    pub layout_dirty: bool,
    /// Visual changes require rebuilding the scene before presentation.
    pub paint_dirty: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            runtime: Runtime::new(),
            retained: None,
            layout_map: LayoutMap::default(),
            font_cx: FontContext::new(),
            scene: Scene::new(),
            layout_dirty: true,
            paint_dirty: true,
        }
    }
}

impl UiState {
    /// Mounts the root view and its nested components once.
    ///
    /// Returns an error if already mounted or if retained construction fails.
    pub fn mount(&mut self, root: impl Fn(&Cx) -> Element + 'static) -> Result<(), RetainedError> {
        if self.retained.is_some() {
            return Err(RetainedError::UnsupportedElement("UI already mounted"));
        }
        self.runtime.mount(root);
        let element = self
            .runtime
            .root_element()
            .ok_or(RetainedError::UnsupportedElement("missing root element"))?;
        let mut tree = RetainedTree::mount(element)?;
        tree.apply_patches(self.runtime.take_patches())?;
        self.retained = Some(tree);
        self.layout_dirty = true;
        self.paint_dirty = true;
        Ok(())
    }

    /// Applies queued reactive changes without eagerly running layout.
    ///
    /// Dirty flags accumulate across batches. Earlier mutations remain applied
    /// if a later patch fails; errors are propagated to the host.
    pub fn apply_updates(&mut self) -> Result<(), RetainedError> {
        self.runtime.flush_effects();
        let patches = self.runtime.take_patches();
        if patches.is_empty() {
            return Ok(());
        }
        let tree = self
            .retained
            .as_mut()
            .ok_or(RetainedError::UnsupportedElement("UI not mounted"))?;
        self.paint_dirty = true;
        let result = tree.apply_patches(patches);
        self.layout_dirty |= tree.layout_dirty;
        result
    }

    /// Computes geometry and scroll limits for `viewport`, preserving dirtiness on failure.
    ///
    /// Hosts call this explicitly after a resize or before geometry-dependent input.
    pub fn layout(&mut self, viewport: Viewport) -> Result<(), RetainedError> {
        self.layout_dirty = true;
        self.paint_dirty = true;
        let tree = self
            .retained
            .as_mut()
            .ok_or(RetainedError::UnsupportedElement("UI not mounted"))?;
        self.layout_map = layout_pass(tree, &mut self.font_cx, viewport)?;
        if let Some(root) = &tree.root {
            sync_scroll_layout_max(root, &self.layout_map);
        }
        self.layout_dirty = false;
        Ok(())
    }

    /// Applies updates, lays out dirty geometry, and rebuilds a dirty scene.
    ///
    /// The host must invalidate layout on viewport changes and paint on changes
    /// to `scale`, `focused`, or `caret_visible`. No GPU or native window is needed.
    pub fn update(
        &mut self,
        viewport: Viewport,
        scale: f32,
        focused: Option<NodeId>,
        caret_visible: bool,
    ) -> Result<(), RetainedError> {
        self.apply_updates()?;
        if self.layout_dirty || self.retained.as_ref().is_some_and(|tree| tree.layout_dirty) {
            self.layout(viewport)?;
        }
        if self.paint_dirty {
            let tree = self
                .retained
                .as_ref()
                .ok_or(RetainedError::UnsupportedElement("UI not mounted"))?;
            self.scene.reset();
            paint_pass(
                tree,
                &self.layout_map,
                &mut self.scene,
                scale,
                focused,
                caret_visible,
            );
            self.paint_dirty = false;
        }
        Ok(())
    }
}
