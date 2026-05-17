# Lemon Nested Component Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the missing component lifecycle slice from the Lemon spec: `ComponentElement` construction, component-aware diff patches, nested component preservation by `type_id + key`, and retained-tree support for transparent component wrapper nodes.

**Architecture:** Keep this plan focused on component semantics only. Extend the current pure runtime so nested components are real preserved instances rather than inert `Element::Component` leaves, and adjust the retained tree to represent component wrapper nodes as transparent runtime nodes that delegate layout ownership to their concrete child root. Layout, Parley, Vello, and the platform loop stay out of scope for this plan.

**Tech Stack:** Rust 2021; existing Lemon `Element`, `Patch`, `Runtime`, and `RetainedTree`; `std::any::TypeId`; `taffy = 0.7`.

**Spec reference:** `docs/superpowers/specs/2026-05-17-lemon-architecture-design.md`

---

## Scope Check

The remaining work in the spec is too broad for one safe executable plan. It spans at least four subsystems:

1. nested component lifecycle
2. layout + text measurement
3. paint
4. platform/frame loop

This plan intentionally covers only the first subsystem because it is a hard dependency for the others and can still be verified with `cargo test`.

---

## File Structure

```text
src/
  diff/mod.rs               ← Patch enum and component-aware diff semantics
  element/builders.rs       ← public Component builder with optional key support
  element/types.rs          ← ComponentElement helpers and key ergonomics
  retained/mod.rs           ← transparent retained component nodes with no own Taffy node
  runtime/mod.rs            ← nested component instances, lifecycle, patch queue integration
```

**Design decisions locked by this plan:**
- `RetainedKind::Component` is transparent and does **not** own a `taffy::NodeId`.
- `RetainedNode` must therefore stop assuming every node has a `NodeId`; concrete layoutable nodes keep one, component wrapper nodes do not.
- `ComponentElement` identity is exactly `type_id + key`; matching identity preserves the nested component instance, mismatching identity unmounts then mounts.

**Ambiguity called out before implementation:**
- The spec says all retained nodes have a `taffy_id`, then later says `Component` nodes do not. This plan resolves that contradiction explicitly by changing `RetainedNode` to carry `Option<NodeId>`.
- The spec does not define how props are updated when a preserved component keeps identity but receives a new captured closure. This plan treats the newest `ComponentElement.view` closure as authoritative and swaps it into the preserved runtime slot before re-rendering.

---

### Task 1: Public Component Builder And Diff Patch Semantics

**Files:**
- Modify: `src/element/builders.rs`
- Modify: `src/element/types.rs`
- Modify: `src/diff/mod.rs`
- Test: `src/element/builders.rs`
- Test: `src/diff/mod.rs`

- [x] **Step 1: Write the failing builder tests**

Add to `src/element/builders.rs`:

```rust
    #[test]
    fn component_builder_sets_type_id_and_key() {
        use crate::element::types::Key;

        fn child(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("child").into_element()
        }

        let element = Component::new(child).key(7).into_element();
        let Element::Component(component) = element else {
            panic!("expected component element");
        };

        assert_eq!(component.type_id, std::any::TypeId::of::<fn(&crate::runtime::cx::Cx) -> Element>());
        assert_eq!(component.key, Some(Key(7)));
    }
```

- [x] **Step 2: Run the builder test to verify red**

Run:

```bash
cargo test element::builders::tests::component_builder_sets_type_id_and_key -- --nocapture
```

Expected: compile error because `Component::new` does not exist.

- [x] **Step 3: Implement the minimal public `Component` builder**

Add to `src/element/builders.rs`:

```rust
pub struct Component {
    element: crate::element::types::ComponentElement,
}

impl Component {
    pub fn new(view: fn(&crate::runtime::cx::Cx) -> Element) -> Self {
        Self {
            element: crate::element::types::ComponentElement {
                view: Rc::new(move |cx| view(cx)),
                type_id: std::any::TypeId::of::<fn(&crate::runtime::cx::Cx) -> Element>(),
                key: None,
            },
        }
    }

    pub fn key(mut self, key: u64) -> Self {
        self.element.key = Some(crate::element::types::Key(key));
        self
    }

    pub fn into_element(self) -> Element {
        Element::Component(self.element)
    }
}

impl From<Component> for Element {
    fn from(component: Component) -> Self {
        component.into_element()
    }
}
```

If `TypeId::of::<fn(&Cx) -> Element>()` proves too coarse for later prop-capturing support, replace it in the same task with a new constructor in `src/element/types.rs`:

```rust
impl ComponentElement {
    pub fn new(
        type_id: std::any::TypeId,
        view: Rc<dyn Fn(&crate::runtime::cx::Cx) -> crate::element::Element>,
    ) -> Self {
        Self {
            view,
            type_id,
            key: None,
        }
    }
}
```

- [x] **Step 4: Run the builder test to verify green**

Run:

```bash
cargo test element::builders::tests::component_builder_sets_type_id_and_key -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Write the failing diff tests**

Add to `src/diff/mod.rs`:

```rust
    #[test]
    fn equal_component_identity_produces_no_patch() {
        fn child(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("child").into_element()
        }

        let old = crate::element::builders::Component::new(child)
            .key(1)
            .into_element();
        let new = crate::element::builders::Component::new(child)
            .key(1)
            .into_element();

        assert!(diff(old, new, NodePath::root()).is_empty());
    }

    #[test]
    fn changed_component_identity_unmounts_then_mounts() {
        fn child_a(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("a").into_element()
        }
        fn child_b(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("b").into_element()
        }

        let old = crate::element::builders::Component::new(child_a)
            .key(1)
            .into_element();
        let new = crate::element::builders::Component::new(child_b)
            .key(1)
            .into_element();

        let patches = diff(old, new, NodePath::root());

        assert!(matches!(patches.first(), Some(Patch::UnmountComponent { .. })));
        assert!(matches!(patches.get(1), Some(Patch::MountComponent { .. })));
    }
```

- [x] **Step 6: Run the diff tests to verify red**

Run:

```bash
cargo test diff::tests::equal_component_identity_produces_no_patch diff::tests::changed_component_identity_unmounts_then_mounts -- --nocapture
```

Expected: compile error because `Patch::MountComponent` and `Patch::UnmountComponent` do not exist.

- [x] **Step 7: Implement component diff semantics**

Update `src/diff/mod.rs`:

```rust
    MountComponent {
        node: NodePath,
        component: crate::element::types::ComponentElement,
    },
    UnmountComponent {
        node: NodePath,
    },
```

Add a component match arm:

```rust
        (Component(o), Component(n)) => {
            if o.type_id != n.type_id || o.key != n.key {
                patches.push(Patch::UnmountComponent { node: path.clone() });
                patches.push(Patch::MountComponent {
                    node: path,
                    component: n,
                });
            }
        }
```

Keep the fallback `ReplaceNode` for non-component type changes untouched.

- [x] **Step 8: Run the diff tests to verify green**

Run:

```bash
cargo test diff::tests::equal_component_identity_produces_no_patch diff::tests::changed_component_identity_unmounts_then_mounts -- --nocapture
```

Expected: PASS.

- [x] **Step 9: Commit**

```bash
git add src/element/builders.rs src/element/types.rs src/diff/mod.rs
git commit -m "feat(component): add public component builder and diff lifecycle patches"
```

---

### Task 2: Preserve Nested Runtime Component Instances By `type_id + key`

**Files:**
- Modify: `src/runtime/mod.rs`
- Test: `src/runtime/mod.rs`

- [x] **Step 1: Write the failing runtime tests**

Add to `src/runtime/mod.rs`:

```rust
    #[test]
    fn nested_component_state_survives_parent_rerender_when_identity_matches() {
        use crate::element::builders::{Column, Component, Text};

        fn child(cx: &Cx) -> Element {
            let count = cx.use_signal(0u32);
            count.update(|value| *value += 1);
            Text::new(move || count.get().to_string()).into_element()
        }

        let trigger = Signal::new(0u32);
        let t2 = trigger.clone();
        let mut runtime = Runtime::new();
        runtime.mount(move |_cx| {
            t2.get();
            Column::new()
                .child(Component::new(child).key(1))
                .into_element()
        });

        trigger.set(1);
        runtime.flush_effects();
        let patches = runtime.take_patches();

        assert!(patches.iter().any(
            |patch| matches!(patch, Patch::UpdateText { content, .. } if content == "2")
        ));
    }

    #[test]
    fn nested_component_identity_change_unmounts_and_mounts() {
        use crate::element::builders::{Column, Component, Text};

        fn child_a(_cx: &Cx) -> Element {
            Text::new("a").into_element()
        }

        fn child_b(_cx: &Cx) -> Element {
            Text::new("b").into_element()
        }

        let swap = Signal::new(false);
        let s2 = swap.clone();
        let mut runtime = Runtime::new();
        runtime.mount(move |_cx| {
            let child = if s2.get() { child_b } else { child_a };
            Column::new()
                .child(Component::new(child).key(1))
                .into_element()
        });

        swap.set(true);
        runtime.flush_effects();
        let patches = runtime.take_patches();

        assert!(patches.iter().any(|patch| matches!(patch, Patch::UnmountComponent { .. })));
        assert!(patches.iter().any(|patch| matches!(patch, Patch::MountComponent { .. })));
    }
```

- [x] **Step 2: Run the runtime tests to verify red**

Run:

```bash
cargo test runtime::tests::nested_component_state_survives_parent_rerender_when_identity_matches runtime::tests::nested_component_identity_change_unmounts_and_mounts -- --nocapture
```

Expected: FAIL because nested components are currently treated as inert `Element::Component` values.

- [x] **Step 3: Implement nested component slots**

Replace the flat slot model in `src/runtime/mod.rs` with a recursive one:

```rust
struct ComponentSlot {
    path: NodePath,
    previous: Option<Element>,
    pending: Rc<RefCell<Option<Element>>>,
    view: ComponentFn,
    cx: Rc<RefCell<Cx>>,
    children: Vec<ComponentSlot>,
    effect: Effect,
}
```

Add helpers:

```rust
fn same_component_identity(
    old: &crate::element::types::ComponentElement,
    new: &crate::element::types::ComponentElement,
) -> bool {
    old.type_id == new.type_id && old.key == new.key
}

fn render_slot(slot: &mut ComponentSlot, patches: &mut Vec<Patch>) {
    if let Some(new_tree) = slot.pending.borrow_mut().take() {
        let next_previous = new_tree.clone();
        if let Some(old_tree) = slot.previous.take() {
            diff_with_nested_components(&mut slot.children, old_tree, new_tree, slot.path.clone(), patches);
        }
        slot.previous = Some(next_previous);
    }
}
```

Implement `diff_with_nested_components(...)` in the same file so that:
- matching `ComponentElement` identity reuses the child slot, replaces its `view` closure with the newest closure, resets hook index, and renders the child
- changed identity drops the old slot, queues `Patch::UnmountComponent`, creates a new slot, queues `Patch::MountComponent`, and renders it
- non-component branches continue using `crate::diff::diff(...)`

- [x] **Step 4: Run the runtime tests to verify green**

Run:

```bash
cargo test runtime::tests::nested_component_state_survives_parent_rerender_when_identity_matches runtime::tests::nested_component_identity_change_unmounts_and_mounts -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src/runtime/mod.rs
git commit -m "feat(runtime): preserve nested component instances by identity"
```

---

### Task 3: Transparent Retained Component Wrapper Nodes

**Files:**
- Modify: `src/retained/mod.rs`
- Test: `src/retained/mod.rs`

- [x] **Step 1: Write the failing retained-tree tests**

Add to `src/retained/mod.rs`:

```rust
    #[test]
    fn mount_component_patch_creates_transparent_wrapper_without_taffy_node() {
        use crate::element::builders::{Component, Text};

        fn child(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("child").into_element()
        }

        let mut tree = RetainedTree::mount(Text::new("root").into_element()).unwrap();
        tree.apply_patch(Patch::MountComponent {
            node: NodePath::root(),
            component: crate::element::builders::Component::new(child)
                .key(5)
                .into_component_element(),
        })
        .unwrap();

        let root = tree.root.as_ref().unwrap();
        assert!(matches!(root.kind, RetainedKind::Component { .. }));
        assert!(root.taffy_id.is_none());
    }

    #[test]
    fn component_wrapper_is_transparent_to_child_lookup() {
        let wrapper = RetainedNode {
            kind: RetainedKind::Component {
                type_id: std::any::TypeId::of::<u32>(),
                key: Some(crate::element::types::Key(1)),
            },
            taffy_id: None,
            style: Default::default(),
            paint: Default::default(),
            children: vec![RetainedNode::text(
                taffy::NodeId::from(9_u64),
                "child".to_owned(),
                Default::default(),
                Default::default(),
            )],
            handlers: Default::default(),
            text: None,
        };

        assert_eq!(wrapper.children[0].text_content(), Some("child"));
    }
```

- [x] **Step 2: Run the retained-tree tests to verify red**

Run:

```bash
cargo test retained::tests::mount_component_patch_creates_transparent_wrapper_without_taffy_node retained::tests::component_wrapper_is_transparent_to_child_lookup -- --nocapture
```

Expected: compile error because retained component nodes do not exist and `taffy_id` is not optional.

- [x] **Step 3: Refactor retained nodes for transparent components**

Change `src/retained/mod.rs`:

```rust
pub enum RetainedKind {
    Box,
    Row,
    Column,
    Text,
    Button,
    Image { src: String },
    Component {
        type_id: std::any::TypeId,
        key: Option<crate::element::types::Key>,
    },
}

pub struct RetainedNode {
    pub kind: RetainedKind,
    pub taffy_id: Option<NodeId>,
    pub style: StyleProps,
    pub paint: PaintData,
    pub children: Vec<RetainedNode>,
    pub handlers: EventHandlers,
    pub text: Option<TextCache>,
}
```

Add a helper:

```rust
impl RetainedNode {
    pub fn layout_node_id(&self) -> Option<NodeId> {
        self.taffy_id
    }
}
```

Update all concrete-node constructors so they wrap the old `NodeId` in `Some(...)`.

- [x] **Step 4: Implement component patch application**

Add retained-tree patch arms:

```rust
            Patch::MountComponent { node, component } => {
                let retained = RetainedNode {
                    kind: RetainedKind::Component {
                        type_id: component.type_id,
                        key: component.key.clone(),
                    },
                    taffy_id: None,
                    style: StyleProps::default(),
                    paint: PaintData::default(),
                    children: Vec::new(),
                    handlers: EventHandlers::default(),
                    text: None,
                };
                self.replace_with_wrapper(node, retained)?;
            }
            Patch::UnmountComponent { node } => {
                self.unwrap_component(node)?;
            }
```

Implement `replace_with_wrapper(...)` and `unwrap_component(...)` so they:
- never allocate a Taffy node for the wrapper
- preserve the concrete child subtree under the wrapper
- leave Taffy ownership attached only to concrete descendants

- [x] **Step 5: Run the retained-tree tests to verify green**

Run:

```bash
cargo test retained::tests::mount_component_patch_creates_transparent_wrapper_without_taffy_node retained::tests::component_wrapper_is_transparent_to_child_lookup -- --nocapture
```

Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add src/retained/mod.rs
git commit -m "feat(retained): add transparent component wrapper nodes"
```

---

### Task 4: Full Verification And Spec Alignment Sweep

**Files:**
- Modify: `docs/superpowers/plans/2026-05-17-lemon-component-lifecycle.md`

- [x] **Step 1: Re-read spec component requirements**

Check these exact requirements against the implementation:

```text
- montagem dentro de Effect
- atualização via signal -> rerender -> diff
- desmontagem dropa hooks do cx
- estabilidade por type_id + key
- Component wrapper opaco para retained/layout/paint
```

**Spec alignment (verified):**

| Requirement | Code / test |
|-------------|-------------|
| Montagem dentro de `Effect` | `create_component_slot` + `Effect::new` / `Effect::new_lazy` in `src/runtime/mod.rs` |
| Signal → rerender → diff | `signal_change_produces_update_text_patch`, `nested_component_state_survives_parent_rerender_when_identity_matches` |
| Desmontagem dropa hooks do `Cx` | `unmount_component_slot` drops slot; `unmounting_component_slot_drops_cx_hooks` |
| Estabilidade por identidade + key | `same_component_identity` + diff/runtime tests |
| Wrapper transparente (retained/layout) | `RetainedKind::Component`, `taffy_id: None`, `resolve_transparent_mut`, retained tests |

Identity uses function-pointer address (`ComponentElement::identity`) in addition to `TypeId`.

- [x] **Step 2: Run formatting**

Run:

```bash
cargo fmt --all
```

Expected: exit code `0`.

- [x] **Step 3: Run clippy**

Run:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: exit code `0`.

- [x] **Step 4: Run type-check**

Run:

```bash
cargo check
```

Expected: exit code `0`.

- [x] **Step 5: Run the full test suite**

Run:

```bash
cargo test
```

Expected: exit code `0`.

- [x] **Step 6: Commit**

```bash
git add src/diff/mod.rs src/element/builders.rs src/element/types.rs src/retained/mod.rs src/runtime/mod.rs docs/superpowers/plans/2026-05-17-lemon-component-lifecycle.md
git commit -m "feat(component): implement nested component lifecycle"
```

---

## Self-Review

**Spec coverage:** This plan covers the remaining component-specific gaps from Layers 2, 4, and 5: public component construction, diff lifecycle patches, runtime preservation by `type_id + key`, and transparent retained component wrappers. It does **not** cover layout, Parley measurement, paint, winit/wgpu integration, keyed child diffing, deferred `use_effect`, or equality-aware `Derived<T>` propagation.

**Placeholder scan:** No `TODO`, `TBD`, or “similar to above” instructions remain. Each task includes explicit files, tests, commands, and code.

**Type consistency:** The plan consistently uses `Patch::MountComponent`, `Patch::UnmountComponent`, `RetainedKind::Component`, `ComponentElement.type_id`, and `Key(u64)` across all tasks.
