//! Headless processing of a window's UI, independent of native presentation.

use parley::FontContext;
use taffy::NodeId;
use vello::Scene;

use crate::element::Element;
use crate::retained::RetainedError;
use crate::{layout_pass, sync_scroll_layout_max, Cx, LayoutMap, RetainedTree, Runtime, Viewport};

/// UI processing state shared by desktop hosts and headless tests.
///
/// Hosts supply viewport changes, translated input events, frame scheduling, and presentation.
/// Apply reactive updates before layout, then rebuild the scene only when dirty.
///
/// ```
/// use lemon::{Text, Viewport, ui::UiState};
/// let mut ui = UiState::default();
/// ui.mount(|_| Text::new("Hello").into_element()).unwrap();
/// ui.update(Viewport { width: 400.0, height: 300.0 }, 1.0, None, true).unwrap();
/// assert!(!ui.layout_dirty());
/// assert!(!ui.paint_dirty());
/// ```
///
/// Internals are read-only; update through this pipeline to preserve caches.
/// ```compile_fail
/// let mut ui = lemon::ui::UiState::default();
/// ui.layout_dirty = false;
/// ```
pub struct UiState {
    pub(crate) transitions: crate::transition::Transitions,
    pub(crate) transitions_active: bool,
    pub(crate) input: crate::input::InputState,
    pub(crate) host: crate::host::HostContext,
    pub(crate) viewport: Option<Viewport>,
    last_paint: Option<(f32, Option<NodeId>, bool)>,
    /// Reactive component runtime.
    pub(crate) runtime: Runtime,
    /// Retained state, absent until mounted.
    pub(crate) retained: Option<RetainedTree>,
    /// Latest layout geometry in logical points.
    pub(crate) layout_map: LayoutMap,
    /// Registered fonts and text measurement context.
    pub(crate) font_cx: FontContext,
    /// Scene buffer rebuilt after visual changes.
    pub(crate) scene: Scene,
    /// Host or tree geometry changes require layout before painting.
    pub(crate) layout_dirty: bool,
    /// Visual changes require rebuilding the scene before presentation.
    pub(crate) paint_dirty: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            transitions: crate::transition::Transitions::default(),
            transitions_active: false,
            input: crate::input::InputState::default(),
            host: crate::host::HostContext::current(),
            viewport: None,
            last_paint: None,
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
    /// Returns the component runtime for read-only inspection.
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }
    /// Returns retained state after mounting; mutations belong to this pipeline.
    pub fn retained(&self) -> Option<&RetainedTree> {
        self.retained.as_ref()
    }
    /// Returns the latest computed layout rectangles.
    pub fn layout_map(&self) -> &LayoutMap {
        &self.layout_map
    }
    /// Returns the scene most recently built by `update`.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    /// Returns whether geometry needs recomputing.
    pub fn layout_dirty(&self) -> bool {
        self.layout_dirty
    }
    /// Returns whether the scene needs rebuilding.
    pub fn paint_dirty(&self) -> bool {
        self.paint_dirty
    }
    /// Invalidates geometry and paint after viewport or external layout changes.
    pub fn invalidate_layout(&mut self) {
        self.layout_dirty = true;
        self.paint_dirty = true;
    }
    /// Invalidates paint after scale, focus chrome, or host appearance changes.
    pub fn invalidate_paint(&mut self) {
        self.paint_dirty = true;
    }

    /// Edits registered fonts, invalidating text measurement and geometry afterwards.
    ///
    /// The callback can use Lemon's `register_font_bytes` or Parley's font APIs.
    /// Its result is returned unchanged; caches are invalidated even if it returns
    /// an error or unwinds after partially changing the collection.
    ///
    /// ```no_run
    /// use lemon::{ui::UiState, register_font_bytes};
    /// let mut ui = UiState::default();
    /// ui.edit_fonts(|fonts| register_font_bytes(fonts, "Body", std::fs::read("Body.ttf").unwrap())).unwrap();
    /// ```
    pub fn edit_fonts<R>(&mut self, edit: impl FnOnce(&mut FontContext) -> R) -> R {
        // Invalidate first so partial writes during unwinding cannot leave clean caches.
        self.invalidate_layout();
        if let Some(tree) = &mut self.retained {
            tree.invalidate_text();
        }
        edit(&mut self.font_cx)
    }

    /// Mounts the root view and its nested components once.
    ///
    /// Returns an error if already mounted or if retained construction fails.
    pub fn mount(&mut self, root: impl Fn(&Cx) -> Element + 'static) -> Result<(), RetainedError> {
        if self.retained.is_some() {
            return Err(RetainedError::UnsupportedElement("UI already mounted"));
        }
        self.host = crate::host::HostContext::current();
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
        self.apply_patches(patches)
    }

    /// Applies an ordered patch batch, accumulating geometry and paint invalidation.
    ///
    /// Prefer `apply_updates` for reactive views. Integrations that generate patches
    /// directly use this method instead of mutating retained state or dirty flags.
    /// Earlier patches remain applied if a later patch fails.
    pub fn apply_patches(
        &mut self,
        patches: impl IntoIterator<Item = crate::diff::Patch>,
    ) -> Result<(), RetainedError> {
        let tree = self
            .retained
            .as_mut()
            .ok_or(RetainedError::UnsupportedElement("UI not mounted"))?;
        for patch in patches {
            self.paint_dirty = true;
            let result = tree.apply_patch(patch);
            self.layout_dirty |= tree.layout_dirty;
            result?;
        }
        self.reconcile_input();
        Ok(())
    }

    /// Runs deferred user effects after the host has presented this UI's scene.
    pub fn finish_frame(&mut self) {
        self.runtime.flush_deferred_effects();
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
        self.viewport = Some(viewport);
        Ok(())
    }

    /// Applies updates, lays out dirty geometry, and rebuilds a dirty scene.
    ///
    /// Viewport changes invalidate geometry; scale, focus and caret changes invalidate
    /// paint automatically. No GPU or native window is needed.
    pub fn update(
        &mut self,
        viewport: Viewport,
        scale: f32,
        focused: Option<NodeId>,
        caret_visible: bool,
    ) -> Result<(), RetainedError> {
        self.update_at(
            viewport,
            scale,
            focused,
            caret_visible,
            std::time::Instant::now(),
        )
    }

    /// Updates UI processing using an explicit monotonic transition timestamp.
    ///
    /// Custom hosts and tests can drive transitions deterministically. Pass increasing
    /// timestamps; an earlier timestamp is clamped to the start of each transition.
    /// Like [`Self::update`], this applies pending changes and refreshes dirty layout.
    ///
    /// ```
    /// use lemon::{View, Viewport, ui::UiState};
    /// let mut ui = UiState::default();
    /// ui.mount(|_| View::new().into_element()).unwrap();
    /// ui.update_at(Viewport { width: 400.0, height: 300.0 }, 1.0, None, true,
    ///     std::time::Instant::now()).unwrap();
    /// ```
    pub fn update_at(
        &mut self,
        viewport: Viewport,
        scale: f32,
        focused: Option<NodeId>,
        caret_visible: bool,
        now: std::time::Instant,
    ) -> Result<(), RetainedError> {
        let _scope = self.host.enter();
        self.apply_updates()?;
        if self.viewport != Some(viewport) {
            self.invalidate_layout();
        }
        let paint_context = (scale, focused, caret_visible);
        if self.last_paint != Some(paint_context) {
            self.invalidate_paint();
        }
        if self.layout_dirty || self.retained.as_ref().is_some_and(|tree| tree.layout_dirty) {
            self.layout(viewport)?;
        }
        if let Some(root) = self.retained.as_ref().and_then(|tree| tree.root.as_ref()) {
            let active = self.transitions.update(root, focused, now);
            self.paint_dirty |= active || self.transitions_active;
            self.transitions_active = active;
            if active {
                self.host.animations().request_frame();
            }
        }
        if self.paint_dirty {
            let tree = self
                .retained
                .as_ref()
                .ok_or(RetainedError::UnsupportedElement("UI not mounted"))?;
            self.scene.reset();
            crate::paint::paint_pass_with_visuals(
                tree,
                &self.layout_map,
                &mut self.scene,
                scale,
                focused,
                caret_visible,
                Some(&self.transitions.visuals),
            );
            self.paint_dirty = false;
            self.last_paint = Some(paint_context);
        }
        Ok(())
    }
}
