//! Implicit transitions between visual states, driven by the owning host's frames.
//!
//! Native windows apply these through [`crate::ui::UiState::update`]. Custom hosts
//! use the same pipeline; the one-shot [`crate::paint_pass`] paints target values.

use crate::{element::style::PaintData, retained::RetainedNode, AnimationConfig, Color, Easing};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use taffy::NodeId;

/// A visual property animated by [`Transition`]. Geometry changes remain immediate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionProperty {
    /// Solid background color, including appearance and removal.
    Surface,
    /// Border colors, independently for each side.
    BorderColor,
    /// Text-field focus ring color and appearance/disappearance.
    FocusRing,
    /// Text foreground color.
    TextColor,
    /// Container opacity.
    Opacity,
    /// The four corner radii.
    Rounding,
    /// The four border widths (paint only).
    BorderWidth,
}

/// Animates subsequent visual changes in a view and its descendants.
///
/// Initial mounting paints immediately. Changes retarget from the current visual
/// value, including interrupted transitions. Descendant declarations override the
/// inherited transition. With no declaration, or with zero duration, changes
/// apply immediately. Layout, gradients, images and shadows are not interpolated.
/// All selected properties share the duration and restart together when retargeted.
///
/// ```
/// use std::time::Duration;
/// use lemon::{View, Transition, TransitionProperty, Easing};
/// let panel = View::new().transition(Transition::new(Duration::from_millis(180))
///     .easing(Easing::EaseOut)
///     .properties([TransitionProperty::Surface, TransitionProperty::BorderColor]));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    config: AnimationConfig,
    properties: u8,
}
impl Transition {
    /// Transitions all supported visual properties over `duration`, with linear easing.
    /// Zero duration applies changes immediately.
    pub fn new(duration: Duration) -> Self {
        Self {
            config: AnimationConfig::new(duration),
            properties: u8::MAX,
        }
    }
    /// Sets the easing curve for all selected properties.
    pub fn easing(mut self, easing: Easing) -> Self {
        self.config = self.config.easing(easing);
        self
    }
    /// Replaces the property selection. An empty selection animates nothing.
    pub fn properties(mut self, properties: impl IntoIterator<Item = TransitionProperty>) -> Self {
        self.properties = properties
            .into_iter()
            .fold(0, |mask, property| mask | (1 << property as u8));
        self
    }
    fn includes(self, property: TransitionProperty) -> bool {
        self.properties & (1 << property as u8) != 0
    }
}
impl Default for Transition {
    /// All visual properties, 150 milliseconds, linear easing.
    fn default() -> Self {
        Self::new(Duration::from_millis(150))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Visual {
    pub paint: PaintData,
    pub opacity: f32,
    pub text_color: Option<Color>,
}
impl Visual {
    fn target(node: &RetainedNode, focused: Option<NodeId>) -> Self {
        let mut paint = node.paint.clone();
        if node.text_input.is_some() {
            let mut color = paint
                .focus_ring_color
                .unwrap_or(crate::theme::current_theme().chrome.focus_ring);
            if node.taffy_id != focused {
                color.a = 0.0;
            }
            paint.focus_ring_color = Some(color);
        }
        Self {
            paint,
            opacity: node.style.opacity,
            text_color: node
                .text
                .as_ref()
                .map(|text| crate::element::style::resolved_text_color(&text.style)),
        }
    }
    fn interpolate(&self, target: &Self, t: f32, transition: Transition) -> Self {
        use TransitionProperty::*;
        let mut result = target.clone();
        if transition.includes(Surface) {
            result.paint.background =
                mix_optional(self.paint.background, target.paint.background, t);
        }
        if transition.includes(BorderColor) {
            result.paint.border_color.top = mix_optional(
                self.paint.border_color.top,
                target.paint.border_color.top,
                t,
            );
            result.paint.border_color.right = mix_optional(
                self.paint.border_color.right,
                target.paint.border_color.right,
                t,
            );
            result.paint.border_color.bottom = mix_optional(
                self.paint.border_color.bottom,
                target.paint.border_color.bottom,
                t,
            );
            result.paint.border_color.left = mix_optional(
                self.paint.border_color.left,
                target.paint.border_color.left,
                t,
            );
        }
        if transition.includes(FocusRing) {
            result.paint.focus_ring_color = mix_optional(
                self.paint.focus_ring_color,
                target.paint.focus_ring_color,
                t,
            );
        }
        if transition.includes(TextColor) {
            result.text_color = mix_optional(self.text_color, target.text_color, t);
        }
        if transition.includes(Opacity) {
            result.opacity = mix(self.opacity, target.opacity, t);
        }
        if transition.includes(Rounding) {
            result.paint.radius.top_left =
                mix(self.paint.radius.top_left, target.paint.radius.top_left, t);
            result.paint.radius.top_right = mix(
                self.paint.radius.top_right,
                target.paint.radius.top_right,
                t,
            );
            result.paint.radius.bottom_left = mix(
                self.paint.radius.bottom_left,
                target.paint.radius.bottom_left,
                t,
            );
            result.paint.radius.bottom_right = mix(
                self.paint.radius.bottom_right,
                target.paint.radius.bottom_right,
                t,
            );
        }
        if transition.includes(BorderWidth) {
            result.paint.border_width.top = mix(
                self.paint.border_width.top,
                target.paint.border_width.top,
                t,
            );
            result.paint.border_width.right = mix(
                self.paint.border_width.right,
                target.paint.border_width.right,
                t,
            );
            result.paint.border_width.bottom = mix(
                self.paint.border_width.bottom,
                target.paint.border_width.bottom,
                t,
            );
            result.paint.border_width.left = mix(
                self.paint.border_width.left,
                target.paint.border_width.left,
                t,
            );
        }
        result
    }
}
fn mix(a: f32, b: f32, t: f32) -> f32 {
    if t >= 1.0 {
        b
    } else {
        a + (b - a) * t
    }
}
fn mix_optional(a: Option<Color>, b: Option<Color>, t: f32) -> Option<Color> {
    if a == b || t >= 1.0 {
        return b;
    }
    let (a, b) = match (a, b) {
        (None, None) => return None,
        (Some(a), Some(b)) => (a, b),
        (Some(a), None) => (a, Color { a: 0.0, ..a }),
        (None, Some(b)) => (Color { a: 0.0, ..b }, b),
    };
    let alpha = mix(a.a, b.a, t);
    let channel = |a: f32, b: f32| {
        if alpha > 0.0 {
            mix(a, b, t) / alpha
        } else {
            0.0
        }
    };
    Some(Color {
        r: channel(a.r * a.a, b.r * b.a),
        g: channel(a.g * a.a, b.g * b.a),
        b: channel(a.b * a.a, b.b * b.a),
        a: alpha,
    })
}
struct Entry {
    from: Visual,
    target: Visual,
    started: Instant,
    transition: Transition,
}
impl Entry {
    fn sample(&self, now: Instant) -> Visual {
        let progress = if self.transition.config.duration().is_zero() {
            1.0
        } else {
            (now.saturating_duration_since(self.started).as_secs_f32()
                / self.transition.config.duration().as_secs_f32())
            .min(1.0)
        };
        self.from.interpolate(
            &self.target,
            self.transition.config.easing_curve().sample(progress),
            self.transition,
        )
    }
}
#[derive(Default)]
pub(crate) struct Transitions {
    entries: HashMap<NodeId, Entry>,
    pub visuals: HashMap<NodeId, Visual>,
}
impl Transitions {
    /// Reconciles targets without changing retained state or geometry.
    pub fn update(&mut self, root: &RetainedNode, focused: Option<NodeId>, now: Instant) -> bool {
        let mut live = HashSet::new();
        let mut active = false;
        self.visit(root, None, focused, now, &mut live, &mut active);
        self.entries.retain(|id, _| live.contains(id));
        self.visuals.retain(|id, _| live.contains(id));
        active
    }
    fn visit(
        &mut self,
        node: &RetainedNode,
        inherited: Option<Transition>,
        focused: Option<NodeId>,
        now: Instant,
        live: &mut HashSet<NodeId>,
        active: &mut bool,
    ) {
        let transition = node.style.transition.or(inherited);
        if let (Some(id), Some(transition)) = (node.taffy_id, transition) {
            live.insert(id);
            let target = Visual::target(node, focused);
            let entry = self.entries.entry(id).or_insert_with(|| Entry {
                from: target.clone(),
                target: target.clone(),
                started: now,
                transition,
            });
            let current = entry.sample(now);
            if entry.target != target || entry.transition != transition {
                *entry = Entry {
                    from: current,
                    target,
                    started: now,
                    transition,
                };
            }
            let current = entry.sample(now);
            *active |= current != entry.target;
            self.visuals.insert(id, current);
        }
        for child in &node.children {
            self.visit(child, transition, focused, now, live, active);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        host::HostContext,
        input::{hit_test::LogicalPoint, InputEvent},
        prelude::*,
        ui::UiState,
        Theme, Viewport,
    };
    fn viewport() -> Viewport {
        Viewport {
            width: 400.0,
            height: 300.0,
        }
    }
    fn update(ui: &mut UiState, now: Instant) {
        ui.update_at(viewport(), 1.0, ui.focused(), true, now)
            .unwrap();
    }
    fn visual(ui: &UiState, index: usize) -> Visual {
        let root = ui.retained().unwrap().root().unwrap();
        let node = if index == usize::MAX {
            root
        } else {
            &root.children[index]
        };
        ui.transitions.visuals[&node.taffy_id.unwrap()].clone()
    }
    #[test]
    fn reactive_colors_easing_interruption_and_completion_use_host_frames_without_layout() {
        let host = HostContext::new(Theme::default_dark());
        let _scope = host.enter();
        let color = State::new(Color::rgb8(255, 0, 0));
        let source = color.clone();
        let mut ui = UiState::default();
        ui.mount(move |_| {
            View::new()
                .width(200.0)
                .height(80.0)
                .surface(source.get())
                .border(source.get(), 1.0)
                .transition(Transition::new(Duration::from_secs(1)).easing(Easing::EaseIn))
                .into_element()
        })
        .unwrap();
        let start = Instant::now();
        update(&mut ui, start);
        assert!(!host.animations().take_pending(), "mount does not animate");
        let initial = visual(&ui, usize::MAX);
        color.set(Color::rgb8(0, 0, 255));
        update(&mut ui, start);
        assert_eq!(visual(&ui, usize::MAX), initial);
        assert!(host.animations().take_pending());
        assert!(!ui.layout_dirty());
        let mid = start + Duration::from_millis(500);
        update(&mut ui, mid);
        let current = visual(&ui, usize::MAX).paint.background.unwrap();
        assert_eq!((current.r, current.b), (0.75, 0.25));
        // Retained targets remain untouched by the presentation layer.
        assert_eq!(
            ui.retained().unwrap().root().unwrap().paint.background,
            Some(Color::rgb8(0, 0, 255))
        );
        color.set(Color::rgb8(0, 255, 0));
        update(&mut ui, mid);
        assert_eq!(visual(&ui, usize::MAX).paint.background, Some(current));
        update(&mut ui, mid + Duration::from_secs(1));
        assert_eq!(
            visual(&ui, usize::MAX).paint.background,
            Some(Color::rgb8(0, 255, 0))
        );
        host.animations().take_pending();
        update(&mut ui, mid + Duration::from_secs(2));
        assert!(
            !host.animations().take_pending(),
            "completed transitions stop scheduling frames"
        );
    }
    #[test]
    fn focus_ring_transitions_in_and_out_while_keyboard_focus_changes_immediately() {
        let mut ui = UiState::default();
        ui.mount(|cx| {
            TextField::new(cx, State::new(String::new()))
                .width(200.0)
                .decorate(|view| view.transition(Transition::new(Duration::from_secs(1))))
        })
        .unwrap();
        let start = Instant::now();
        update(&mut ui, start);
        assert_eq!(
            visual(&ui, usize::MAX).paint.focus_ring_color.unwrap().a,
            0.0
        );
        assert!(!ui.transitions_active);
        ui.dispatch_input(InputEvent::Click(LogicalPoint::new(5.0, 5.0)))
            .unwrap();
        assert!(ui.focused().is_some());
        update(&mut ui, start);
        update(&mut ui, start + Duration::from_millis(500));
        assert_eq!(
            visual(&ui, usize::MAX).paint.focus_ring_color.unwrap().a,
            0.5
        );
        ui.dispatch_input(InputEvent::Click(LogicalPoint::new(350.0, 250.0)))
            .unwrap();
        assert!(ui.focused().is_none());
        update(&mut ui, start + Duration::from_millis(500));
        update(&mut ui, start + Duration::from_millis(1000));
        assert_eq!(
            visual(&ui, usize::MAX).paint.focus_ring_color.unwrap().a,
            0.25
        );
        // The presentation ring still paints during blur even though focus is gone.
        let mut scene = vello::Scene::new();
        let stats = crate::paint::paint_pass_with_visuals(
            ui.retained().unwrap(),
            ui.layout_map(),
            &mut scene,
            1.0,
            None,
            false,
            Some(&ui.transitions.visuals),
        );
        assert_eq!(stats.strokes, 2, "border plus fading focus ring");

        update(&mut ui, start + Duration::from_millis(1500));
        assert_eq!(
            visual(&ui, usize::MAX).paint.focus_ring_color.unwrap().a,
            0.0
        );
        assert!(!ui.transitions_active);
    }
    #[test]
    fn property_selection_and_zero_duration_apply_other_properties_immediately() {
        let a = Visual {
            paint: PaintData {
                background: Some(Color::rgb8(255, 0, 0)),
                border_color: crate::element::style::Edges::all(Some(Color::rgb8(255, 0, 0))),
                ..Default::default()
            },
            opacity: 0.0,
            text_color: None,
        };
        let mut b = a.clone();
        b.paint.background = Some(Color::rgb8(0, 0, 255));
        b.paint.border_color = crate::element::style::Edges::all(Some(Color::rgb8(0, 255, 0)));
        b.opacity = 1.0;
        let transition =
            Transition::new(Duration::from_secs(1)).properties([TransitionProperty::BorderColor]);
        let current = a.interpolate(&b, 0.5, transition);
        assert_eq!(current.paint.background, b.paint.background);
        assert_eq!(current.opacity, 1.0);
        assert_eq!(
            current.paint.border_color.top.unwrap().to_rgb8(),
            (128, 128, 0)
        );
        let entry = Entry {
            from: a,
            target: b.clone(),
            started: Instant::now(),
            transition: Transition::new(Duration::ZERO),
        };
        assert_eq!(entry.sample(entry.started), b);
    }
    #[test]
    fn transparency_preserves_hue_and_removal_finishes_as_none() {
        let blue = Color::rgb8(0, 0, 255);
        assert_eq!(
            mix_optional(None, Some(blue), 0.5),
            Some(blue.with_alpha(0.5))
        );
        assert_eq!(
            mix_optional(Some(blue), None, 0.5),
            Some(blue.with_alpha(0.5))
        );
        assert_eq!(mix_optional(Some(blue), None, 1.0), None);
    }
    #[test]
    fn inherited_text_opacity_rounding_and_border_width_interpolate_without_moving_content() {
        let changed = State::new(false);
        let source = changed.clone();
        let mut ui = UiState::default();
        ui.mount(move |_| {
            let changed = source.get();
            let color = if changed {
                Color::rgb8(0, 0, 255)
            } else {
                Color::rgb8(255, 0, 0)
            };
            View::new()
                .width(200.0)
                .height(100.0)
                .opacity(if changed { 0.8 } else { 0.2 })
                .rounding(if changed { 12.0 } else { 4.0 })
                .border(color, if changed { 5.0 } else { 1.0 })
                .transition(Transition::new(Duration::from_secs(1)))
                .child(Text::new("Label").color(color))
                .into_element()
        })
        .unwrap();
        let start = Instant::now();
        update(&mut ui, start);
        let geometry = ui.layout_map().clone();
        changed.set(true);
        update(&mut ui, start);
        update(&mut ui, start + Duration::from_millis(500));
        let root = visual(&ui, usize::MAX);
        assert!((root.opacity - 0.5).abs() < 0.001);
        assert_eq!(root.paint.radius.top_left, 8.0);
        assert_eq!(root.paint.border_width.top, 3.0);
        assert_eq!(visual(&ui, 0).text_color.unwrap().to_rgb8(), (128, 0, 128));
        assert_eq!(ui.layout_map(), &geometry);
        assert!(!ui.layout_dirty());
    }

    #[test]
    fn removing_a_transition_or_node_cleans_up_its_presentation() {
        let enabled = State::new(true);
        let shown = State::new(true);
        let source = enabled.clone();
        let visible = shown.clone();
        let mut ui = UiState::default();
        ui.mount(move |_| {
            let view = View::new().surface(Color::rgb8(20, 30, 40));
            let view = if source.get() {
                view.transition(Transition::default())
            } else {
                view
            };
            view.child(if visible.get() {
                Text::new("Child").into_element()
            } else {
                Element::None
            })
            .into_element()
        })
        .unwrap();
        let start = Instant::now();
        update(&mut ui, start);
        assert_eq!(ui.transitions.entries.len(), 2);
        shown.set(false);
        update(&mut ui, start);
        assert_eq!(ui.transitions.entries.len(), 1);
        enabled.set(false);
        update(&mut ui, start);
        assert!(ui.transitions.entries.is_empty());
        assert!(ui.transitions.visuals.is_empty());
        assert!(!ui.transitions_active);
    }
    #[test]
    fn keyed_reordering_preserves_in_flight_presentation_and_removed_nodes_are_pruned() {
        let rows = State::new(vec![1, 2]);
        let changed = State::new(false);
        let order = rows.clone();
        let source = changed.clone();
        let mut ui = UiState::default();
        ui.mount(move |_| {
            ColumnView::new()
                .transition(Transition::new(Duration::from_secs(1)))
                .children(order.get().into_iter().map(|key| {
                    View::new()
                        .key(key)
                        .width(50.0)
                        .height(20.0)
                        .surface(if source.get() {
                            Color::rgb8(0, 0, 255)
                        } else {
                            Color::rgb8(255, 0, 0)
                        })
                }))
                .into_element()
        })
        .unwrap();
        let start = Instant::now();
        update(&mut ui, start);
        let id = ui.retained().unwrap().root().unwrap().children[0]
            .taffy_id
            .unwrap();
        changed.set(true);
        update(&mut ui, start);
        let mid = start + Duration::from_millis(500);
        update(&mut ui, mid);
        let current = ui.transitions.visuals[&id].clone();
        rows.set(vec![2, 1]);
        update(&mut ui, mid);
        assert_eq!(
            ui.retained().unwrap().root().unwrap().children[1].taffy_id,
            Some(id)
        );
        assert_eq!(ui.transitions.visuals[&id], current);
        rows.set(vec![2]);
        update(&mut ui, mid);
        assert!(!ui.transitions.visuals.contains_key(&id));
        assert_eq!(ui.transitions.entries.len(), 2);
    }
}
