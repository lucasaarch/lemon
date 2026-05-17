# Lemon Retained Tree Implementation Plan

> **Status (2026-05-17):** Implemented on `master`. Component wrapper patches (`MountComponent` / `UnmountComponent` / `UpdateComponent`) were added later by `2026-05-17-lemon-component-lifecycle.md`; treat that plan as the source of truth for component retained semantics.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Lemon Layer 5 for the currently supported concrete element types: a retained tree backed by Taffy nodes plus a patch-application engine that keeps retained state synchronized with the existing diff output.

**Architecture:** Keep the new retained tree independent from the runtime/frame loop first so it can be validated with ordinary unit tests. Model only concrete renderable/layoutable nodes in this slice (`Box_`, `Row`, `Column`, `Text`, `Button`, `Image`), translate `StyleProps` into Taffy `Style`, and apply existing patches against the retained tree while preserving child order, resolved paint data, text cache invalidation, and handler retention.

**Tech Stack:** Rust 2021; `taffy = 0.7`; existing Lemon `Element`, `Patch`, `StyleProps`, and `PaintData` types.

**Spec reference:** `docs/superpowers/specs/2026-05-17-lemon-architecture-design.md`

---

## File Structure

```text
src/
  lib.rs                    ← export retained module
  retained/
    mod.rs                  ← RetainedTree, RetainedNode, text cache, patch application
```

**Design notes locked for this slice:**
- `ComponentElement` lifecycle remains deferred; this plan does not implement `MountComponent` / `UnmountComponent`.
- `RetainedNode` will only represent concrete nodes that own a real `taffy::NodeId`.
- Button labels stay inline on retained button nodes for now rather than being normalized into implicit child text nodes. This preserves current behavior and avoids forcing a premature paint-model decision.

---

### Task 1: Add Retained Tree Skeleton And Style Translation

**Files:**
- Modify: `src/lib.rs`
- Create: `src/retained/mod.rs`
- Test: `src/retained/mod.rs`

- [x] **Step 1: Write the failing tests**

Add these tests to `src/retained/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::style::{Align, Color, Dimension, Edges, Justify, StyleProps};

    #[test]
    fn style_props_convert_to_taffy_style() {
        let style = StyleProps {
            width: Some(Dimension::Points(120.0)),
            height: Some(Dimension::Percent(0.5)),
            padding: Some(Edges::all(8.0)),
            margin: Some(Edges::all(4.0)),
            gap: Some(12.0),
            flex_grow: Some(1.0),
            flex_shrink: Some(0.0),
            align_items: Some(Align::Center),
            justify_content: Some(Justify::SpaceBetween),
        };

        let taffy_style = style.to_taffy_style();

        assert_eq!(taffy_style.size.width, taffy::style::Dimension::length(120.0));
        assert_eq!(taffy_style.size.height, taffy::style::Dimension::percent(0.5));
        assert_eq!(taffy_style.gap.width, taffy::style::LengthPercentage::length(12.0));
        assert_eq!(taffy_style.padding.left, taffy::style::LengthPercentage::length(8.0));
        assert_eq!(taffy_style.margin.left, taffy::style::LengthPercentageAuto::length(4.0));
        assert_eq!(taffy_style.flex_grow, 1.0);
        assert_eq!(taffy_style.flex_shrink, 0.0);
        assert_eq!(taffy_style.align_items, Some(taffy::style::AlignItems::Center));
        assert_eq!(
            taffy_style.justify_content,
            Some(taffy::style::JustifyContent::SpaceBetween)
        );
    }

    #[test]
    fn retained_node_helpers_expose_text_and_paint_state() {
        let node = RetainedNode::text(
            taffy::NodeId::from(7),
            "hello".to_owned(),
            Default::default(),
            Color::rgb8(255, 0, 0),
        );

        assert_eq!(node.text_content(), Some("hello"));
        assert_eq!(node.paint.background, None);
        assert_eq!(node.taffy_id, taffy::NodeId::from(7));
    }
}
```

- [x] **Step 2: Run the tests to verify red**

Run:

```bash
cargo test retained::tests -- --nocapture
```

Expected: compilation fails because `retained` module, `StyleProps::to_taffy_style`, and `RetainedNode` do not exist yet.

- [x] **Step 3: Implement the minimal retained skeleton**

Create `src/retained/mod.rs` with:

```rust
use std::rc::Rc;

use taffy::{NodeId, Style, TaffyTree};

use crate::element::style::{Color, CornerRadii, PaintData, StyleProps, TextStyle};

#[derive(Clone, Debug, PartialEq)]
pub struct TextCache {
    pub content: String,
    pub style: TextStyle,
    pub needs_layout: bool,
}

#[derive(Clone)]
pub struct EventHandlers {
    pub on_click: Option<Rc<dyn Fn()>>,
}

impl std::fmt::Debug for EventHandlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventHandlers")
            .field("on_click", &self.on_click.as_ref().map(|_| "Rc<dyn Fn()>"))
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RetainedKind {
    Box,
    Row,
    Column,
    Text,
    Button,
    Image { src: String },
}

#[derive(Clone, Debug)]
pub struct RetainedNode {
    pub kind: RetainedKind,
    pub taffy_id: NodeId,
    pub style: StyleProps,
    pub paint: PaintData,
    pub children: Vec<RetainedNode>,
    pub handlers: EventHandlers,
    pub text: Option<TextCache>,
}

impl RetainedNode {
    pub fn text(taffy_id: NodeId, content: String, style: TextStyle, color: Color) -> Self {
        Self {
            kind: RetainedKind::Text,
            taffy_id,
            style: StyleProps::default(),
            paint: PaintData {
                background: None,
                border_color: Some(color),
                border_width: 0.0,
                radius: CornerRadii::default(),
            },
            children: Vec::new(),
            handlers: EventHandlers { on_click: None },
            text: Some(TextCache {
                content,
                style,
                needs_layout: true,
            }),
        }
    }

    pub fn text_content(&self) -> Option<&str> {
        self.text.as_ref().map(|text| text.content.as_str())
    }
}

#[derive(Debug)]
pub struct RetainedTree {
    pub taffy: TaffyTree<()>,
    pub root: Option<RetainedNode>,
}

impl RetainedTree {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            root: None,
        }
    }
}

impl Default for RetainedTree {
    fn default() -> Self {
        Self::new()
    }
}
```

Update `src/lib.rs`:

```rust
pub mod retained;
```

Add to `src/retained/mod.rs`:

```rust
impl StyleProps {
    pub fn to_taffy_style(&self) -> Style {
        use taffy::prelude::{AlignItems, Dimension, JustifyContent, LengthPercentage, LengthPercentageAuto, Rect, Size};

        Style {
            size: Size {
                width: self.width.clone().map_or(Dimension::Auto, into_taffy_dimension),
                height: self.height.clone().map_or(Dimension::Auto, into_taffy_dimension),
            },
            padding: self.padding.as_ref().map_or(Rect::zero(), |edges| Rect {
                left: LengthPercentage::length(edges.left),
                right: LengthPercentage::length(edges.right),
                top: LengthPercentage::length(edges.top),
                bottom: LengthPercentage::length(edges.bottom),
            }),
            margin: self.margin.as_ref().map_or(Rect::zero(), |edges| Rect {
                left: LengthPercentageAuto::length(edges.left),
                right: LengthPercentageAuto::length(edges.right),
                top: LengthPercentageAuto::length(edges.top),
                bottom: LengthPercentageAuto::length(edges.bottom),
            }),
            gap: Size {
                width: self.gap.map_or(LengthPercentage::zero(), LengthPercentage::length),
                height: self.gap.map_or(LengthPercentage::zero(), LengthPercentage::length),
            },
            flex_grow: self.flex_grow.unwrap_or(0.0),
            flex_shrink: self.flex_shrink.unwrap_or(1.0),
            align_items: self.align_items.as_ref().map(|align| match align {
                crate::element::style::Align::Stretch => AlignItems::Stretch,
                crate::element::style::Align::Start => AlignItems::Start,
                crate::element::style::Align::End => AlignItems::End,
                crate::element::style::Align::Center => AlignItems::Center,
                crate::element::style::Align::Baseline => AlignItems::Baseline,
            }),
            justify_content: self.justify_content.as_ref().map(|justify| match justify {
                crate::element::style::Justify::Start => JustifyContent::Start,
                crate::element::style::Justify::End => JustifyContent::End,
                crate::element::style::Justify::Center => JustifyContent::Center,
                crate::element::style::Justify::SpaceBetween => JustifyContent::SpaceBetween,
                crate::element::style::Justify::SpaceAround => JustifyContent::SpaceAround,
                crate::element::style::Justify::SpaceEvenly => JustifyContent::SpaceEvenly,
            }),
            ..Default::default()
        }
    }
}

fn into_taffy_dimension(dimension: crate::element::style::Dimension) -> taffy::style::Dimension {
    match dimension {
        crate::element::style::Dimension::Auto => taffy::style::Dimension::Auto,
        crate::element::style::Dimension::Points(value) => taffy::style::Dimension::length(value),
        crate::element::style::Dimension::Percent(value) => taffy::style::Dimension::percent(value),
    }
}
```

- [x] **Step 4: Run the tests to verify green**

Run:

```bash
cargo test retained::tests -- --nocapture
```

Expected: `test result: ok. 2 passed`

- [x] **Step 5: Commit**

```bash
git add src/lib.rs src/retained/mod.rs
git commit -m "feat(retained): scaffold retained tree model"
```

---

### Task 2: Build A Retained Tree From Existing Element Trees

**Files:**
- Modify: `src/retained/mod.rs`
- Test: `src/retained/mod.rs`

- [x] **Step 1: Write the failing tests**

Add these tests:

```rust
#[test]
fn mount_builds_retained_tree_for_container_children_and_text() {
    use crate::element::builders::{Column, Text};

    let element = Column::new()
        .gap(6.0)
        .child(Text::new("hello"))
        .child(Text::new("world"))
        .into_element();

    let tree = RetainedTree::mount(element).unwrap();
    let root = tree.root.as_ref().unwrap();

    assert!(matches!(root.kind, RetainedKind::Column));
    assert_eq!(root.children.len(), 2);
    assert_eq!(root.children[0].text_content(), Some("hello"));
    assert_eq!(root.children[1].text_content(), Some("world"));

    let taffy_children = tree.taffy.children(root.taffy_id).unwrap();
    assert_eq!(taffy_children.len(), 2);
    assert_eq!(taffy_children[0], root.children[0].taffy_id);
    assert_eq!(taffy_children[1], root.children[1].taffy_id);
}

#[test]
fn mount_resolves_paint_and_handlers_for_button_nodes() {
    use crate::element::builders::Button;
    use crate::element::style::Color;
    use std::cell::Cell;

    let fired = Rc::new(Cell::new(false));
    let click = fired.clone();
    let element = Button::new("Press")
        .background(Color::rgb8(10, 20, 30))
        .on_click(move || click.set(true))
        .into_element();

    let tree = RetainedTree::mount(element).unwrap();
    let root = tree.root.as_ref().unwrap();

    assert!(matches!(root.kind, RetainedKind::Button));
    assert_eq!(root.paint.background, Some(Color::rgb8(10, 20, 30)));
    assert_eq!(root.text_content(), Some("Press"));

    let handler = root.handlers.on_click.as_ref().unwrap();
    handler();
    assert!(fired.get());
}
```

- [x] **Step 2: Run the tests to verify red**

Run:

```bash
cargo test retained::tests::mount_ -- --nocapture
```

Expected: compile error because `RetainedTree::mount` and element-to-retained conversion do not exist yet.

- [x] **Step 3: Implement mounting from `Element`**

Extend `src/retained/mod.rs` with:

```rust
use crate::element::{
    content::TextContent,
    style::PaintData,
    types::{BoxElement, ButtonElement, ImageElement, TextElement},
    Element,
};

impl RetainedTree {
    pub fn mount(element: Element) -> Result<Self, taffy::TaffyError> {
        let mut tree = Self::new();
        let root = tree.build_node(element)?;
        tree.root = Some(root);
        Ok(tree)
    }

    fn build_node(&mut self, element: Element) -> Result<RetainedNode, taffy::TaffyError> {
        match element {
            Element::Column(node) => self.build_box_node(RetainedKind::Column, node),
            Element::Row(node) => self.build_box_node(RetainedKind::Row, node),
            Element::Box_(node) => self.build_box_node(RetainedKind::Box, node),
            Element::Text(node) => self.build_text_node(node),
            Element::Button(node) => self.build_button_node(node),
            Element::Image(node) => self.build_image_node(node),
            Element::Fragment(children) => {
                let mut root = self.build_box_node(RetainedKind::Column, BoxElement::default())?;
                root.children = children
                    .into_iter()
                    .map(|child| self.build_node(child))
                    .collect::<Result<Vec<_>, _>>()?;
                for (index, child) in root.children.iter().enumerate() {
                    self.taffy
                        .insert_child_at_index(root.taffy_id, index, child.taffy_id)?;
                }
                Ok(root)
            }
            Element::Component(_) | Element::None => {
                let id = self.taffy.new_leaf(Default::default())?;
                Ok(RetainedNode {
                    kind: RetainedKind::Box,
                    taffy_id: id,
                    style: StyleProps::default(),
                    paint: PaintData::default(),
                    children: Vec::new(),
                    handlers: EventHandlers { on_click: None },
                    text: None,
                })
            }
        }
    }
}
```

Implement the `build_box_node`, `build_text_node`, `build_button_node`, and `build_image_node` helpers so they:
- create a Taffy leaf for text/image/button and a parent node for containers
- resolve `PaintProps` into `PaintData`
- preserve child order in both `children` and Taffy
- copy button click handlers into `EventHandlers`
- store current text content in `TextCache { needs_layout: true }`

- [x] **Step 4: Run the tests to verify green**

Run:

```bash
cargo test retained::tests::mount_ -- --nocapture
```

Expected: both `mount_...` tests pass.

- [x] **Step 5: Commit**

```bash
git add src/retained/mod.rs
git commit -m "feat(retained): mount retained trees from element trees"
```

---

### Task 3: Apply Existing Diff Patches To The Retained Tree

**Files:**
- Modify: `src/retained/mod.rs`
- Test: `src/retained/mod.rs`

- [x] **Step 1: Write the failing tests**

Add these tests:

```rust
#[test]
fn update_text_patch_replaces_content_and_invalidates_layout() {
    use crate::diff::{NodePath, Patch};
    use crate::element::builders::Text;

    let mut tree = RetainedTree::mount(Text::new("before").into_element()).unwrap();
    let root = tree.root.as_mut().unwrap();
    root.text.as_mut().unwrap().needs_layout = false;

    tree.apply_patch(Patch::UpdateText {
        node: NodePath::root(),
        content: "after".to_owned(),
    })
    .unwrap();

    let root = tree.root.as_ref().unwrap();
    assert_eq!(root.text_content(), Some("after"));
    assert!(root.text.as_ref().unwrap().needs_layout);
}

#[test]
fn insert_and_remove_child_patches_keep_taffy_and_retained_order_in_sync() {
    use crate::diff::{NodePath, Patch};
    use crate::element::builders::{Column, Text};

    let mut tree = RetainedTree::mount(Column::new().child(Text::new("a")).into_element()).unwrap();

    tree.apply_patch(Patch::InsertChild {
        parent: NodePath::root(),
        index: 1,
        element: Text::new("b").into_element(),
    })
    .unwrap();

    let root = tree.root.as_ref().unwrap();
    assert_eq!(root.children.len(), 2);
    assert_eq!(root.children[1].text_content(), Some("b"));
    assert_eq!(tree.taffy.children(root.taffy_id).unwrap()[1], root.children[1].taffy_id);

    tree.apply_patch(Patch::RemoveChild {
        parent: NodePath::root(),
        index: 0,
    })
    .unwrap();

    let root = tree.root.as_ref().unwrap();
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].text_content(), Some("b"));
    assert_eq!(tree.taffy.children(root.taffy_id).unwrap()[0], root.children[0].taffy_id);
}

#[test]
fn replace_node_patch_rebuilds_subtree_at_same_index() {
    use crate::diff::{NodePath, Patch};
    use crate::element::builders::{Column, Text};

    let mut tree = RetainedTree::mount(
        Column::new()
            .child(Text::new("old"))
            .child(Text::new("keep"))
            .into_element(),
    )
    .unwrap();
    let old_child_id = tree.root.as_ref().unwrap().children[0].taffy_id;

    tree.apply_patch(Patch::ReplaceNode {
        node: NodePath(vec![0]),
        new_element: Column::new().child(Text::new("new")).into_element(),
    })
    .unwrap();

    let root = tree.root.as_ref().unwrap();
    assert_ne!(root.children[0].taffy_id, old_child_id);
    assert!(matches!(root.children[0].kind, RetainedKind::Column));
    assert_eq!(root.children[0].children[0].text_content(), Some("new"));
    assert_eq!(root.children[1].text_content(), Some("keep"));
}
```

- [x] **Step 2: Run the tests to verify red**

Run:

```bash
cargo test retained::tests::update_text_patch retained::tests::insert_and_remove retained::tests::replace_node_patch -- --nocapture
```

Expected: compile error because `apply_patch` and tree traversal helpers do not exist yet.

- [x] **Step 3: Implement patch application**

Extend `src/retained/mod.rs` with:

```rust
use crate::diff::{NodePath, Patch};

impl RetainedTree {
    pub fn apply_patch(&mut self, patch: Patch) -> Result<(), taffy::TaffyError> {
        match patch {
            Patch::UpdateStyle { node, style } => {
                let retained = self.node_mut(&node).expect("node path must resolve");
                retained.style = style.clone();
                self.taffy.set_style(retained.taffy_id, style.to_taffy_style())?;
            }
            Patch::UpdatePaint { node, paint } => {
                self.node_mut(&node).expect("node path must resolve").paint = paint;
            }
            Patch::UpdateText { node, content } => {
                let retained = self.node_mut(&node).expect("node path must resolve");
                let text = retained.text.as_mut().expect("UpdateText requires a text cache");
                text.content = content;
                text.needs_layout = true;
            }
            Patch::InsertChild { parent, index, element } => {
                let child = self.build_node(element)?;
                let parent_node = self.node_mut(&parent).expect("parent path must resolve");
                self.taffy
                    .insert_child_at_index(parent_node.taffy_id, index, child.taffy_id)?;
                parent_node.children.insert(index, child);
            }
            Patch::RemoveChild { parent, index } => {
                let parent_node = self.node_mut(&parent).expect("parent path must resolve");
                let removed = parent_node.children.remove(index);
                self.taffy.remove_child_at_index(parent_node.taffy_id, index)?;
                self.remove_subtree_from_taffy(removed)?;
            }
            Patch::ReplaceNode { node, new_element } => {
                self.replace_node(node, new_element)?;
            }
            Patch::MoveChild { parent, from, to } => {
                self.move_child(parent, from, to)?;
            }
        }
        Ok(())
    }
}
```

Implement `node_mut`, `replace_node`, `move_child`, and `remove_subtree_from_taffy` so they:
- traverse `NodePath` safely
- preserve retained/Taffy ordering
- recursively remove descendants from Taffy when a subtree is deleted
- rebuild replacement subtrees with fresh `NodeId`s

- [x] **Step 4: Run the focused patch tests to verify green**

Run:

```bash
cargo test retained::tests::update_text_patch retained::tests::insert_and_remove retained::tests::replace_node_patch -- --nocapture
```

Expected: all three tests pass.

- [x] **Step 5: Run full project verification**

Run:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo check
cargo test
```

Expected:
- `cargo fmt --all` exits `0`
- `cargo clippy --all-targets --all-features -- -D warnings` exits `0`
- `cargo check` exits `0`
- `cargo test` exits `0`

- [x] **Step 6: Commit**

```bash
git add src/lib.rs src/retained/mod.rs
git commit -m "feat(retained): apply diff patches to retained tree"
```
