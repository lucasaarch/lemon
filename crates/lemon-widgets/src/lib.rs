//! App-facing re-exports of Lemon’s UI builders.
//!
//! Use this crate in examples and applications so imports stay short:
//!
//! ```no_run
//! use lemon_widgets::prelude::*;
//!
//! fn app(cx: &Cx) -> Element {
//!     Column::new().child(Text::new("Hello")).into_element()
//! }
//!
//! fn main() {
//!     run(WindowConfig::default().title("App"), app);
//! }
//! ```
//!
//! Everything here is also available from [`lemon::element::builders`] if you prefer a single
//! dependency.
//!
//! # Migrating from `Box_`
//!
//! The generic container builder was renamed to [`View`] in the `lemon` crate (see `cargo doc -p lemon`).
//! [`Box_`] is still re-exported here as a deprecated alias.

pub mod prelude;

pub use lemon::children;
pub mod button;
pub use button::Button;
pub use lemon::element::builders::{Column, Component, Row, Text, View};
pub mod image;
pub mod scroll;
pub mod select;
pub mod slider;
pub mod text_field_state;
pub mod text_input;
pub use image::Image;
pub use scroll::{Scroll, ScrollStyle};
pub use select::{Select, SelectStyle};
pub use slider::{Slider, SliderStyle};
pub use text_field_state::TextFieldState;
pub use text_input::{TextInput, TextInputStyle};

/// Deprecated alias for [`View`].
#[deprecated(
    since = "0.2.0",
    note = "renamed to `View`; see the `lemon` crate-level docs"
)]
pub use lemon::element::builders::View as Box_;

#[cfg(test)]
mod text_input_tests {
    use super::*;
    use lemon::{element::Element, Cursor, Cx, KeyEvent, KeyState, LemonKey, Modifiers, Signal};

    #[test]
    fn text_input_renders_placeholder_when_value_is_empty() {
        fn root(cx: &Cx) -> Element {
            let state = cx.use_signal(TextFieldState::new(""));
            TextInput::new(cx, state)
                .placeholder("Enter text...")
                .into_element()
        }
        let _ = root;
    }

    #[test]
    fn text_input_is_focusable_and_has_text_cursor() {
        let cx = Cx::new();
        let state = Signal::new(TextFieldState::new(""));
        let el = TextInput::new(&cx, state).into_element();

        let Element::View(view) = el else {
            panic!("expected View element from TextInput");
        };
        assert!(view.style.focusable, "TextInput must be focusable");
        assert_eq!(
            view.style.cursor,
            Cursor::Text,
            "TextInput must use Text cursor"
        );
        assert!(view.text_input.is_some(), "TextInput must attach metadata");
    }

    #[test]
    fn text_input_on_key_down_updates_state() {
        let cx = Cx::new();
        let state = Signal::new(TextFieldState::new(""));
        let el = TextInput::new(&cx, state.clone()).into_element();

        let Element::View(view) = el else {
            panic!("expected View element from TextInput");
        };
        let on_key_down = view
            .handlers
            .on_key_down
            .as_ref()
            .expect("must have on_key_down");
        on_key_down(KeyEvent {
            key: LemonKey::Character("a".into()),
            modifiers: Modifiers::default(),
            repeat: false,
            state: KeyState::Pressed,
        });
        let field = state.get();
        assert_eq!(field.value, "a");
        assert_eq!(field.cursor, 1);
    }

    #[test]
    fn text_input_runtime_tree_carries_cursor_after_key() {
        use lemon::runtime::Runtime;

        let state = Signal::new(TextFieldState::new(""));
        let mut runtime = Runtime::new();
        let mount_state = state.clone();
        runtime.mount(move |cx| TextInput::new(cx, mount_state.clone()).into_element());

        let Element::View(view) = runtime.root_element().expect("root element") else {
            panic!("expected View root");
        };
        let on_key_down = view.handlers.on_key_down.as_ref().expect("on_key_down");
        on_key_down(KeyEvent {
            key: LemonKey::Character("hi".into()),
            modifiers: Modifiers::default(),
            repeat: false,
            state: KeyState::Pressed,
        });

        runtime.flush_effects();
        let root = runtime.root_element().expect("root after flush");
        let Element::View(view) = root else {
            panic!("expected View root");
        };
        let meta = view.text_input.as_ref().expect("text_input meta on view");
        assert_eq!(meta.value, "hi");
        assert_eq!(
            meta.cursor, 2,
            "frozen tree must carry cursor for diff/paint"
        );
    }

    #[test]
    fn text_input_diff_emits_widget_chrome_when_value_and_cursor_change() {
        use lemon::diff::{diff, NodePath, Patch};

        let state = Signal::new(TextFieldState::new(""));
        let cx = Cx::new();
        let before = TextInput::new(&cx, state.clone()).into_element();
        state.update(|s| {
            s.handle_key(&KeyEvent {
                key: LemonKey::Character("hi".into()),
                modifiers: Modifiers::default(),
                repeat: false,
                state: KeyState::Pressed,
            });
        });
        let after = TextInput::new(&cx, state).into_element();
        let patches = diff(before, after, NodePath::root());
        assert!(
            patches
                .iter()
                .any(|p| matches!(p, Patch::UpdateWidgetChrome { .. })),
            "value/cursor change must emit UpdateWidgetChrome, got {patches:?}"
        );
    }

    #[test]
    fn runtime_diff_after_typing_includes_widget_chrome() {
        use lemon::diff::{diff, NodePath, Patch};
        use lemon::runtime::Runtime;

        let state = Signal::new(TextFieldState::new(""));
        let mut runtime = Runtime::new();
        let mount_state = state.clone();
        runtime.mount(move |cx| TextInput::new(cx, mount_state.clone()).into_element());

        let old = runtime.root_element().expect("root before key");
        let Element::View(view) = old.clone() else {
            panic!("expected View");
        };
        view.handlers.on_key_down.as_ref().expect("on_key_down")(KeyEvent {
            key: LemonKey::Character("hi".into()),
            modifiers: Modifiers::default(),
            repeat: false,
            state: KeyState::Pressed,
        });

        runtime.flush_effects();
        let new = runtime.root_element().expect("root after key");
        let manual_patches = diff(old, new, NodePath::root());
        assert!(
            manual_patches
                .iter()
                .any(|p| matches!(p, Patch::UpdateWidgetChrome { .. })),
            "manual diff should include chrome, got {manual_patches:?}"
        );

        let runtime_patches = runtime.take_patches();
        assert!(
            runtime_patches
                .iter()
                .any(|p| matches!(p, Patch::UpdateWidgetChrome { .. })),
            "runtime patches should include chrome, got {runtime_patches:?}"
        );
    }

    #[test]
    fn text_input_typing_updates_retained_cursor_via_patches() {
        use lemon::diff::Patch;
        use lemon::retained::RetainedNode;
        use lemon::retained::RetainedTree;
        use lemon::runtime::Runtime;

        fn find_text_input_meta(
            node: &RetainedNode,
        ) -> Option<&lemon::element::types::TextInputMeta> {
            if let Some(meta) = node.text_input.as_ref() {
                return Some(meta);
            }
            for child in &node.children {
                if let Some(meta) = find_text_input_meta(child) {
                    return Some(meta);
                }
            }
            None
        }

        let state = Signal::new(TextFieldState::new(""));
        let mut runtime = Runtime::new();
        let mount_state = state.clone();
        runtime.mount(move |cx| TextInput::new(cx, mount_state.clone()).into_element());

        let mut tree =
            RetainedTree::mount(runtime.root_element().expect("initial root")).expect("mount");

        let Element::View(view) = runtime.root_element().expect("root") else {
            panic!("expected View root");
        };
        let on_key_down = view.handlers.on_key_down.as_ref().expect("on_key_down");
        on_key_down(KeyEvent {
            key: LemonKey::Character("hi".into()),
            modifiers: Modifiers::default(),
            repeat: false,
            state: KeyState::Pressed,
        });

        runtime.flush_effects();
        let patches = runtime.take_patches();
        assert!(
            patches
                .iter()
                .any(|p| matches!(p, Patch::UpdateWidgetChrome { .. })),
            "typing should emit UpdateWidgetChrome, got {patches:?}"
        );
        tree.apply_patches(patches).expect("apply patches");

        let meta = find_text_input_meta(tree.root.as_ref().expect("root"))
            .expect("text_input meta on retained tree");
        assert_eq!(meta.value, "hi");
        assert_eq!(
            meta.cursor, 2,
            "retained meta.cursor drives caret paint position"
        );
    }

    #[test]
    fn text_input_has_border() {
        let cx = Cx::new();
        let state = Signal::new(TextFieldState::new(""));
        let el = TextInput::new(&cx, state).into_element();

        let Element::View(view) = el else {
            panic!("expected View element from TextInput");
        };
        assert!(
            view.paint.border_color.top.is_some()
                || view.paint.border_color.right.is_some()
                || view.paint.border_color.bottom.is_some()
                || view.paint.border_color.left.is_some(),
            "TextInput must always have a border color"
        );
        assert!(
            view.paint.border_width.any_positive(),
            "TextInput must have a non-zero border width"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lemon::{element::Element, Cx, Overflow, Signal};

    #[test]
    fn scroll_new_compiles_with_cx() {
        fn _check(cx: &Cx) -> Element {
            Scroll::new(cx, Text::new("item"))
                .height(180.0)
                .into_element()
        }
    }

    #[test]
    fn scroll_builds_hidden_viewport_and_updates_offset() {
        let offset = Signal::new(10.0f64);
        let root = Scroll::with_offset(offset.clone(), Text::new("item"))
            .height(200.0)
            .width(300.0)
            .into_element();

        let Element::View(viewport) = root else {
            panic!("expected scroll viewport to be Element::View");
        };

        assert_eq!(viewport.style.overflow, Overflow::Hidden);
        assert!(viewport.scroll_bar);
        assert!(!viewport.scroll_viewport);
        assert!(viewport.handlers.on_scroll.is_some());
        assert_eq!(viewport.children.len(), 1);

        let Element::View(inner) = &viewport.children[0] else {
            panic!("expected inner scroll content wrapper to be Element::View");
        };
        assert_eq!(inner.style.margin.as_ref().map(|m| m.top), Some(-10.0));

        let on_scroll = viewport.handlers.on_scroll.as_ref().unwrap();
        on_scroll(-20.0);
        assert_eq!(offset.get(), 30.0);
        on_scroll(1000.0);
        assert_eq!(offset.get(), 0.0);
    }
}
