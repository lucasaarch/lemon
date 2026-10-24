//! App-facing widgets built on Lemon’s element and layout primitives.
//!
//! Import from the crate root or the facade's `lemon::prelude`:
//!
//! ```no_run
//! use lemon::prelude::*;
//!
//! fn app(cx: &Cx) -> Element {
//!     ColumnView::new().child(Text::new("Hello")).into_element()
//! }
//! ```

/// Primitive containers, text and component construction share this entry point
/// with composed widgets.
pub use crate::element::builders::{
    ColumnView, Component, Icon, Image, RowView, ScrollView, ScrollViewStyle, Text, View,
};
pub use crate::element::decoration::Decorate;

pub mod button;

pub mod field;

pub mod file_tree;

pub mod overlay;

pub mod select;

pub mod table;

pub mod resizable_view;

pub mod slider;

pub mod toaster;

pub mod toggle;
pub use overlay::{Dialog, Popover};
pub use toaster::{ToastId, ToastKind, Toaster};
pub use toggle::{Checkbox, Switch};

pub use button::{Button, ButtonStyle};
pub use field::{SecureField, TextArea, TextField, TextFieldStyle};
pub use file_tree::{FileTree, FileTreeNode, FileTreeStyle, Tree, TreeNode, TreeStyle};
pub use resizable_view::{ResizableView, ResizeEdge};
pub use select::{Select, SelectStyle};
pub use slider::{Slider, SliderStyle};
pub use table::Table;

// Shared timing for built-in controls; root decoration can override it.
fn interaction_transition() -> crate::Transition {
    crate::Transition::new(std::time::Duration::from_millis(140)).easing(crate::Easing::EaseOut)
}

#[cfg(test)]
mod text_field_tests {
    use super::*;
    use crate::element::Element;
    use crate::{Cursor, Cx, KeyEvent, KeyState, LemonKey, Modifiers, State};

    #[test]
    fn text_field_renders_placeholder_when_value_is_empty() {
        fn root(cx: &Cx) -> Element {
            let state = cx.state("".to_owned());
            TextField::new(cx, state)
                .placeholder("Enter text...")
                .into_element()
        }
        let Element::View(field) = root(&Cx::new()) else {
            panic!("expected field")
        };
        let Element::Row(row) = &field.children[0] else {
            panic!("expected text wrapper")
        };
        let Element::Text(text) = &row.children[0] else {
            panic!("expected placeholder")
        };
        assert_eq!(text.content.resolve(), "Enter text...");
        assert_eq!(field.text_input.as_ref().unwrap().value, "");
    }

    #[test]
    fn text_field_is_focusable_and_has_text_cursor() {
        let cx = Cx::new();
        let state = State::new("".to_owned());
        let el = TextField::new(&cx, state).into_element();

        let Element::View(view) = el else {
            panic!("expected View element from TextField");
        };
        assert!(view.style.focusable, "TextField must be focusable");
        assert_eq!(
            view.style.cursor,
            Cursor::Text,
            "TextField must use Text cursor"
        );
        assert!(view.text_input.is_some(), "TextField must attach metadata");
    }

    #[test]
    fn text_field_on_key_down_updates_state() {
        let cx = Cx::new();
        let state = State::new("".to_owned());
        let el = TextField::new(&cx, state.clone()).into_element();

        let Element::View(view) = el else {
            panic!("expected View element from TextField");
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
        assert_eq!(field, "a");
    }

    #[test]
    fn text_field_runtime_tree_carries_cursor_after_key() {
        use crate::runtime::Runtime;

        let state = State::new("".to_owned());
        let mut runtime = Runtime::new();
        let mount_state = state.clone();
        runtime.mount(move |cx| TextField::new(cx, mount_state.clone()).into_element());

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
    fn text_field_diff_emits_widget_chrome_when_value_and_cursor_change() {
        use crate::diff::{diff, NodePath, Patch};

        let state = State::new("".to_owned());
        let cx = Cx::new();
        let before = TextField::new(&cx, state.clone()).into_element();
        let Element::View(field) = &before else {
            panic!("field")
        };
        field.handlers.on_key_down.as_ref().unwrap()(KeyEvent {
            key: LemonKey::Character("hi".into()),
            modifiers: Modifiers::default(),
            repeat: false,
            state: KeyState::Pressed,
        });
        cx.reset_hooks();
        let after = TextField::new(&cx, state).into_element();
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
        use crate::diff::{diff, NodePath, Patch};
        use crate::runtime::Runtime;

        let state = State::new("".to_owned());
        let mut runtime = Runtime::new();
        let mount_state = state.clone();
        runtime.mount(move |cx| TextField::new(cx, mount_state.clone()).into_element());

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
    fn text_field_typing_updates_retained_cursor_via_patches() {
        use crate::diff::Patch;
        use crate::retained::RetainedNode;
        use crate::retained::RetainedTree;
        use crate::runtime::Runtime;

        fn find_text_input_meta(
            node: &RetainedNode,
        ) -> Option<&crate::element::types::TextInputMeta> {
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

        let state = State::new("".to_owned());
        let mut runtime = Runtime::new();
        let mount_state = state.clone();
        runtime.mount(move |cx| TextField::new(cx, mount_state.clone()).into_element());

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
    fn text_field_has_border() {
        let cx = Cx::new();
        let state = State::new("".to_owned());
        let el = TextField::new(&cx, state).into_element();

        let Element::View(view) = el else {
            panic!("expected View element from TextField");
        };
        assert!(
            view.paint.border_color.top.is_some()
                || view.paint.border_color.right.is_some()
                || view.paint.border_color.bottom.is_some()
                || view.paint.border_color.left.is_some(),
            "TextField must always have a border color"
        );
        assert!(
            view.paint.border_width.any_positive(),
            "TextField must have a non-zero border width"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Element;
    use crate::{Cx, Overflow, State};

    #[test]
    fn scroll_new_compiles_with_cx() {
        use crate::element::builders::Text;

        fn _check(cx: &Cx) -> Element {
            ScrollView::new(cx, Text::new("item"))
                .height(180.0)
                .into_element()
        }
    }

    #[test]
    fn scroll_builds_hidden_viewport_and_updates_offset() {
        use crate::element::builders::Text;

        let offset = State::new(crate::ScrollOffset::new(0.0, 10.0));
        let root = ScrollView::with_offset(offset.clone(), Text::new("item"))
            .height(200.0)
            .width(300.0)
            .into_element();

        let Element::View(viewport) = root else {
            panic!("expected scroll viewport to be Element::View");
        };

        assert_eq!(viewport.style.overflow, Overflow::Hidden);
        assert_eq!(viewport.scroll_viewport, Some(crate::ScrollAxis::Vertical));
        assert!(viewport.handlers.on_scroll.is_some());
        assert_eq!(viewport.children.len(), 1);

        let Element::View(inner) = &viewport.children[0] else {
            panic!("expected inner scroll content wrapper to be Element::View");
        };
        assert_eq!(inner.style.margin.as_ref().map(|m| m.top), Some(-10.0));

        let on_scroll = viewport.handlers.on_scroll.as_ref().unwrap();
        on_scroll(crate::ScrollDelta::new(0.0, -20.0));
        assert_eq!(offset.get(), crate::ScrollOffset::new(0.0, 30.0));
        on_scroll(crate::ScrollDelta::new(0.0, 1000.0));
        assert_eq!(offset.get(), crate::ScrollOffset::default());
    }
}
