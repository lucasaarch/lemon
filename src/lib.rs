//! Lemon — reactive native desktop UI in Rust.
//!
//! # Getting started
//!
//! Most apps only need [`run`], [`WindowConfig`], [`Cx`], and the layout widgets
//! ([`ColumnView`], [`RowView`], [`Text`], [`Button`], …). Import them via [`prelude`]:
//!
//! ```no_run
//! use lemon::prelude::*;
//!
//! fn app(cx: &Cx) -> Element {
//!     let count = cx.state(0i32);
//!     let label = count.clone();
//!     let inc = count.clone();
//!     ColumnView::new()
//!         .children(children![
//!             Text::new(move || label.get().to_string()),
//!             Button::new(cx, "+").action(move || inc.update(|n| *n += 1)),
//!         ])
//!         .into_element()
//! }
//!
//! fn main() {
//!     run(WindowConfig::default().title("Counter"), app);
//! }
//! ```
//!
//! # How an update flows
//!
//! 1. Your root function (`app`) returns an [`element::Element`] tree built with builders.
//! 2. [`Cx::state`] and dynamic [`Text::new`] closures subscribe to reactive state.
//! 3. When state changes, the runtime diffs the previous tree against the new one and emits patches.
//! 4. The UI pipeline applies retained patches, lays out with Taffy, and builds a Vello scene.
//! 5. The platform translates native input and presents that scene.
//!
//! # Keys and lists
//!
//! Give stable [`.key(id)`](ColumnView::key) values to siblings that are inserted, removed, or reordered
//! (see the `keys` example). Without keys, children are reconciled by index only.
//!
//! # Components
//!
//! [`Component::new`] wraps a sub-view with its own [`Cx`] hooks. It currently takes a **function
//! pointer** (`fn(&Cx) -> Element`), not a capturing closure. For list rows with per-item state,
//! prefer keyed [`RowView`] / [`ColumnView`] children instead of capturing [`Component`]s.
//!
//! # Lower-level API
//!
//! [`Runtime`], [`RetainedTree`], [`layout_pass`], and [`paint_pass`] are public for tests and
//! custom hosts; normal apps should use [`run`] and never touch those directly.

pub mod prelude;

mod host_requests;
mod macros;

pub mod animation;
pub mod asset;
pub mod debug;
pub mod diff;
pub mod element;
pub mod font;
pub mod host;
pub mod icons;
pub mod input;
pub mod layout;
pub mod paint;
pub mod retained;
pub mod runtime;
pub mod theme;
pub mod transition;
pub mod ui;
pub mod widget;
pub mod window;

pub mod platform;
pub mod reactive;

pub use animation::{AnimRegistry, AnimSlot, AnimationConfig, AnimationHandle, Easing};
pub use asset::ImageHandle;
pub use element::builders::{
    ColumnView, Component, Icon, Image, RowView, ScrollView, ScrollViewStyle, Text, View,
};
pub use font::{register_font_bytes, register_font_path, FontRegistrationError};
pub use icons::IconData;
pub use transition::{Transition, TransitionProperty};
pub use widget::{
    Button, ButtonStyle, Checkbox, Dialog, FileTree, FileTreeNode, FileTreeStyle, Popover,
    SecureField, Select, SelectStyle, Slider, SliderStyle, Switch, TextArea, TextField,
    TextFieldStyle, ToastId, Toaster,
};

pub use element::events::{
    Cursor, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey, ScrollAxis, ScrollDelta,
    ScrollOffset,
};
pub use element::style::{
    BoxShadow, Color, Distribute, LinearGradient, Overflow, Placement, Position, StyleProps,
};
pub use element::types::TextAlignment;
pub use layout::{
    layout_pass, layout_pass_if_dirty, measure_element_height,
    measure_element_height_with_font_context, scroll_content_max_offset, sync_scroll_layout_max,
    LayoutMap, LayoutRect, Viewport,
};
pub use reactive::State;
pub use window::{WindowChrome, WindowConfig};

pub use paint::{paint_pass, PaintStats};
pub use platform::{
    run, run_with_fonts, run_with_theme, run_with_theme_and_fonts, AppState, WindowState,
};
pub use retained::RetainedTree;
pub use runtime::cx::Cx;
pub use runtime::Runtime;
pub use theme::{
    current_theme, set_active_theme, ColorTokens, RadiusTokens, SpacingTokens, Theme, ThemePair,
    TypographyTokens, WidgetChromeTokens,
};

pub use debug::Category as DebugCategory;

pub use element::decoration::Decorate;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::Patch;
    use parley::FontContext;
    use std::thread::sleep;
    use std::time::{Duration, Instant};
    use vello::Scene;

    fn layout_pass(
        tree: &mut RetainedTree,
        viewport: Viewport,
    ) -> Result<LayoutMap, crate::retained::RetainedError> {
        let mut font_cx = FontContext::new();
        crate::layout::layout_pass(tree, &mut font_cx, viewport)
    }

    fn layout_pass_if_dirty(
        tree: &mut RetainedTree,
        viewport: Viewport,
    ) -> Result<Option<LayoutMap>, crate::retained::RetainedError> {
        let mut font_cx = FontContext::new();
        crate::layout::layout_pass_if_dirty(tree, &mut font_cx, viewport)
    }

    #[test]
    fn counter_increments_produce_update_text_patches() {
        let count = State::new(0i32);
        let c = count.clone();

        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let c2 = c.clone();
            ColumnView::new()
                .child(Text::new(move || format!("Count: {}", c2.get())))
                .into_element()
        });

        assert!(rt.take_patches().is_empty(), "no patches on first mount");

        count.set(1);
        rt.flush_effects();

        let patches = rt.take_patches();
        assert!(!patches.is_empty());
        let has_patch = patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateText { content, .. } if content == "Count: 1"));
        assert!(has_patch, "expected UpdateText 'Count: 1'");

        count.set(2);
        rt.flush_effects();
        let patches = rt.take_patches();
        let has_patch = patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateText { content, .. } if content == "Count: 2"));
        assert!(has_patch, "expected UpdateText 'Count: 2'");
    }

    #[test]
    fn conditional_child_produces_insert_and_remove_patches() {
        let show = State::new(false);
        let s = show.clone();

        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let visible = s.get();
            let mut col = ColumnView::new();
            if visible {
                col = col.child(Text::new("visible"));
            }
            col.into_element()
        });

        assert!(rt.take_patches().is_empty());

        show.set(true);
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(
            patches
                .iter()
                .any(|p| matches!(p, Patch::InsertChild { .. })),
            "showing child must produce InsertChild"
        );

        show.set(false);
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(
            patches
                .iter()
                .any(|p| matches!(p, Patch::RemoveChild { .. })),
            "hiding child must produce RemoveChild"
        );
    }

    #[test]
    fn multiple_signals_each_trigger_patch() {
        let name = State::new("Alice".to_owned());
        let age = State::new(30u32);
        let n = name.clone();
        let a = age.clone();

        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let n2 = n.clone();
            let a2 = a.clone();
            ColumnView::new()
                .child(Text::new(move || n2.get()))
                .child(Text::new(move || a2.get().to_string()))
                .into_element()
        });

        name.set("Bob".to_owned());
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateText { content, .. } if content == "Bob")));

        age.set(31);
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(patches
            .iter()
            .any(|p| matches!(p, Patch::UpdateText { content, .. } if content == "31")));
    }

    #[test]
    fn runtime_patches_drive_relayout_height_change() {
        let text = State::new("A".to_owned());
        let t = text.clone();

        let mut rt = Runtime::new();
        rt.mount(move |_cx| {
            let t2 = t.clone();
            ColumnView::new()
                .width(120.0)
                .child(Text::new(move || t2.get()).size(16.0))
                .into_element()
        });

        let initial = rt.root_element().expect("root element after mount");
        let mut tree = RetainedTree::mount(initial).unwrap();
        let viewport = Viewport {
            width: 400.0,
            height: 600.0,
        };

        let map_before = layout_pass(&mut tree, viewport).unwrap();
        let text_id = tree.root().unwrap().children[0].taffy_id.unwrap();
        let height_before = map_before.get(text_id).unwrap().height;

        text.set("Line one\nLine two\nLine three".to_owned());
        rt.flush_effects();
        let patches = rt.take_patches();
        assert!(
            patches
                .iter()
                .any(|p| matches!(p, Patch::UpdateText { .. })),
            "expected UpdateText patch, got {patches:?}"
        );
        tree.apply_patches(patches).unwrap();

        let updated_content = &tree.root().unwrap().children[0]
            .text
            .as_ref()
            .unwrap()
            .content;
        assert!(
            updated_content.contains("Line three"),
            "retained text not updated: {updated_content:?}"
        );
        assert!(
            tree.root().unwrap().children[0]
                .text
                .as_ref()
                .unwrap()
                .needs_layout
        );

        assert!(tree.layout_dirty());
        let map_after = layout_pass_if_dirty(&mut tree, viewport)
            .unwrap()
            .expect("layout should run after patches");
        let height_after = map_after.get(text_id).unwrap().height;

        assert!(
            height_after > height_before,
            "height should grow after longer text: before={height_before} after={height_after}"
        );
        assert!(!tree.layout_dirty());
        assert!(
            !tree.root().unwrap().children[0]
                .text
                .as_ref()
                .unwrap()
                .needs_layout
        );
    }

    #[test]
    fn counter_demo_paints_text_and_button_without_explicit_styles() {
        let mut tree = RetainedTree::mount(
            ColumnView::new()
                .spacing(12.0)
                .inset(24.0)
                .child(Text::new("0").size(24.0))
                .child(Button::new(&Cx::new(), "Incrementar"))
                .into_element(),
        )
        .unwrap();

        let layout = layout_pass(
            &mut tree,
            Viewport {
                width: 900.0,
                height: 600.0,
            },
        )
        .unwrap();

        let mut scene = Scene::new();
        let stats = paint_pass(&tree, &layout, &mut scene, 1.0, None, true);

        assert!(stats.glyph_runs > 0, "counter label should paint glyphs");
        assert!(stats.fills >= 1, "button should paint a background fill");
    }

    #[test]
    fn end_to_end_runtime_layout_paint_without_panic() {
        let count = State::new(0i32);
        let c = count.clone();

        let mut rt = Runtime::new();
        rt.mount(move |cx| {
            let c2 = c.clone();
            ColumnView::new()
                .width(200.0)
                .spacing(8.0)
                .child(Text::new(move || format!("Count: {}", c2.get())).size(16.0))
                .child(
                    Button::new(cx, "Increment")
                        .width(120.0)
                        .height(40.0)
                        .surface(Color::rgb8(50, 100, 150)),
                )
                .into_element()
        });

        let mut tree = RetainedTree::mount(rt.root_element().unwrap()).unwrap();
        let viewport = Viewport {
            width: 400.0,
            height: 600.0,
        };

        let mut scene = Scene::new();
        let layout = layout_pass(&mut tree, viewport).unwrap();
        let _stats = paint_pass(&tree, &layout, &mut scene, 1.0, None, true);

        count.set(1);
        rt.flush_effects();
        tree.apply_patches(rt.take_patches()).unwrap();
        let layout = layout_pass_if_dirty(&mut tree, viewport)
            .unwrap()
            .expect("layout after patch");
        let _stats = paint_pass(&tree, &layout, &mut scene, 2.0, None, true);
    }

    #[test]
    fn prelude_exports_common_app_types() {
        use crate::prelude::*;

        fn _app(cx: &Cx) -> Element {
            let n = cx.state(0);
            let n2 = n.clone();
            let anim: AnimationHandle =
                cx.motion(AnimationConfig::default().easing(Easing::Linear));
            anim.reset();
            ColumnView::new()
                .children(children![Text::new(move || n2.get().to_string())])
                .into_element()
        }
    }

    #[test]
    fn crate_root_re_exports_animation_types() {
        let registry = AnimRegistry::default();
        let handle = registry.register(AnimSlot::new(AnimationConfig::default()));
        handle.play();
        assert!(handle.is_playing());
    }

    #[test]
    fn animation_tick_advances_progress() {
        let registry = AnimRegistry::default();
        let handle = registry.register(AnimSlot::new(AnimationConfig::new(Duration::from_millis(
            100,
        ))));

        handle.play();
        let initial_progress = handle.raw_progress();
        assert_eq!(registry.active_slots(), 1);

        sleep(Duration::from_millis(20));
        let keep_redrawing = registry.tick_active(Instant::now());

        assert!(keep_redrawing);
        assert!(handle.raw_progress() > initial_progress);
        assert_eq!(registry.active_slots(), 1);

        sleep(Duration::from_millis(120));
        let keep_redrawing = registry.tick_active(Instant::now());

        assert!(!keep_redrawing);
        assert_eq!(handle.raw_progress(), 1.0);
        assert_eq!(registry.active_slots(), 0);
    }

    #[test]
    fn crate_root_re_exports_component_builder() {
        fn child(_cx: &Cx) -> element::Element {
            Text::new("child").into_element()
        }

        let element = Component::new(child).key(9).into_element();
        let element::Element::Component(component) = element else {
            panic!("expected component element");
        };

        assert_eq!(component.key(), Some(&element::types::Key(9)));
    }

    #[test]
    fn crate_root_re_exports_overflow() {
        let hidden = Overflow::Hidden;
        assert_eq!(hidden, element::style::Overflow::Hidden);
    }

    #[test]
    fn theme_default_light_has_sensible_values() {
        let theme = Theme::default_light();
        assert_eq!(theme.typography.font_size_md, 14.0);
        assert_eq!(theme.typography.font_family, "system-ui");
        assert_eq!(theme.spacing.lg, 16.0);
        assert_eq!(theme.radius.md, 8.0);
        assert!(theme.colors.accent.r > 0.0 || theme.colors.accent.b > 0.0);
    }
}
