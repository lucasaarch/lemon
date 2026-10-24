use super::*;
use crate::element::Element;
use crate::input::{hit_test::LogicalPoint, InputEvent};
use crate::prelude::*;
use crate::retained::RetainedNode;
use crate::ui::UiState;
use crate::{
    AnimationConfig, Component, KeyEvent, KeyState, LemonKey, Modifiers, NamedKey, Viewport,
};

fn update(ui: &mut UiState) {
    ui.update(
        Viewport {
            width: 1100.0,
            height: 760.0,
        },
        1.0,
        ui.focused(),
        true,
    )
    .unwrap();
}

fn target() -> UiState {
    let mut ui = UiState::default();
    ui.inspection = Some(Capture::default());
    ui.mount(|cx| {
        let state = cx.state(7u32);
        let source = state.clone();
        let derived = cx.derive(move || source.get() * 2);
        cx.watch(move || {
            derived.get();
        });
        cx.on_mount(|| || {});
        let _animation = cx.motion(AnimationConfig::default());
        ColumnView::new()
            .fill()
            .child(Text::new("private text"))
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    ui
}

fn text(node: &RetainedNode) -> String {
    std::iter::once(node.text_content().unwrap_or_default().to_owned())
        .chain(node.children.iter().map(text))
        .collect::<Vec<_>>()
        .join("\n")
}

fn control<'a>(node: &'a RetainedNode, label: &str) -> Option<&'a RetainedNode> {
    if node.handlers.on_click.is_some() && text(node).trim() == label {
        return Some(node);
    }
    node.children.iter().find_map(|child| control(child, label))
}

fn scroll_view(node: &RetainedNode, axis: crate::ScrollAxis) -> Option<&RetainedNode> {
    if node.scroll_viewport == Some(axis) {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| scroll_view(child, axis))
}

fn click(ui: &mut UiState, label: &str) {
    let node = control(ui.retained().unwrap().root().unwrap(), label).unwrap_or_else(|| {
        panic!(
            "clickable row {label:?}; tree text: {}",
            text(ui.retained().unwrap().root().unwrap())
        )
    });
    let rect = *ui.layout_map().get(node.taffy_id.unwrap()).unwrap();
    assert!(rect.width > 0.0 && rect.height > 0.0);
    assert!(
        rect.y >= 0.0 && rect.y + rect.height <= 760.0,
        "control outside viewport: {label}: {rect:?}"
    );
    ui.dispatch_input(InputEvent::Click(LogicalPoint::new(
        rect.x + rect.width / 2.0,
        rect.y + rect.height / 2.0,
    )))
    .unwrap();
    update(ui);
}

#[test]
fn captures_layout_paint_and_hook_metadata_without_values() {
    let mut target = target();
    let session = Session::new();
    session.publish(&mut target);
    let snapshot = session.snapshot.peek();
    assert_eq!(snapshot.nodes.len(), 2);
    assert!(snapshot.nodes.iter().all(|node| node.rect.is_some()));
    assert!(snapshot.nodes.iter().all(|node| node
        .properties
        .iter()
        .all(|(_, value)| value != "private text")));
    let component = &snapshot.components[0];
    assert_eq!(component.render.renders, 1);
    assert_eq!(
        component
            .render
            .hooks
            .iter()
            .map(|hook| hook.kind)
            .collect::<Vec<_>>(),
        ["state", "derive", "watch", "on_mount", "motion"]
    );
    assert!(component.render.hooks.iter().all(|hook| hook.calls == 1));
    assert!(component.render.hooks[0].value_type.contains("u32"));
    assert_eq!(snapshot.frames[0].layouts, 1);
    assert_eq!(snapshot.frames[0].paints, 1);
    assert!(snapshot.frames[0].glyph_runs > 0);
    assert_eq!(target.inspection.as_ref().unwrap().pending.paints, 0);
}

#[test]
fn named_nodes_and_colored_icons_are_captured_for_the_tree() {
    let mut target = UiState::default();
    target.inspection = Some(Capture::default());
    target
        .mount(|_| {
            View::new()
                .id("app_root")
                .child(
                    ColumnView::new()
                        .id("toolbar")
                        .child(Text::new("Save").id("save_button")),
                )
                .into_element()
        })
        .unwrap();
    update(&mut target);
    let session = Session::new();
    session.publish(&mut target);
    let nodes = &session.snapshot.peek().nodes;
    assert_eq!(nodes[0].label, "app_root");
    assert_eq!(nodes[0].kind, "View");
    assert_eq!(nodes[1].label, "toolbar");
    assert_eq!(nodes[1].kind, "Column");
    assert_eq!(nodes[2].label, "save_button");
    assert_eq!(nodes[2].kind, "Text");
    assert_ne!(nodes[1].icon_color, nodes[2].icon_color);
}

#[test]
fn hovering_an_inspector_row_tracks_a_temporary_target_highlight() {
    let mut target = target();
    let session = Session::new();
    session.publish(&mut target);
    let view = session.clone();
    let mut inspector = UiState::default();
    inspector.mount(move |cx| view.view(cx)).unwrap();
    update(&mut inspector);

    let row =
        control(inspector.retained().unwrap().root().unwrap(), "Text").expect("text element row");
    let rect = *inspector.layout_map().get(row.taffy_id.unwrap()).unwrap();
    inspector
        .dispatch_input(InputEvent::Hover(LogicalPoint::new(
            rect.x + rect.width / 2.0,
            rect.y + rect.height / 2.0,
        )))
        .unwrap();
    assert_eq!(
        session.hovered.peek(),
        target.retained().unwrap().root().unwrap().children[0].taffy_id
    );
    assert_eq!(session.selected.peek(), None);

    inspector
        .dispatch_input(InputEvent::Hover(LogicalPoint::new(-10.0, -10.0)))
        .unwrap();
    assert_eq!(session.hovered.peek(), None);
}

#[test]
fn tree_container_icons_match_their_layout_direction() {
    let mut ui = UiState::default();
    ui.mount(|_| {
        ColumnView::new()
            .child(RowView::new().child(Text::new("child")))
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    let mut nodes = Vec::new();
    collect_nodes(
        ui.retained().unwrap().root().unwrap(),
        &ui,
        &NodePath::root(),
        0,
        &mut nodes,
    );
    assert_eq!(nodes[0].icon.name(), "layout-arrow-down");
    assert_eq!(nodes[1].icon.name(), "layout-arrow-right");
}

#[test]
fn caps_history_discards_paused_samples_and_isolates_sessions() {
    let mut target = target();
    let session = Session::new();
    for _ in 0..HISTORY + 3 {
        session.publish(&mut target);
    }
    assert_eq!(session.snapshot.peek().frames.len(), HISTORY);
    assert_eq!(session.snapshot.peek().frames[0].number, 4);
    session.paused.set(true);
    target.inspection.as_mut().unwrap().pending.patches = 42;
    session.publish(&mut target);
    assert_eq!(target.inspection.as_ref().unwrap().pending.patches, 0);
    session.paused.set(false);
    session.publish(&mut target);
    assert_eq!(session.snapshot.peek().frames.last().unwrap().patches, 0);
    assert!(Session::new().snapshot.peek().nodes.is_empty());
}

#[test]
fn ui_tabs_selection_pause_and_keyboard_activation_work_through_input() {
    let mut target = target();
    let session = Session::new();
    session.publish(&mut target);
    let view = session.clone();
    let mut inspector = UiState::default();
    inspector.mount(move |cx| view.view(cx)).unwrap();
    update(&mut inspector);
    assert!(inspector.inspection.is_none());
    let root = inspector.retained().unwrap().root().unwrap();
    let tree_row = control(root, "Column").expect("element tree row");
    let row_width = inspector
        .layout_map()
        .get(tree_row.taffy_id.unwrap())
        .unwrap()
        .width;
    let list_view = scroll_view(root, ScrollAxis::Vertical).expect("vertical element list");
    let list_width = inspector
        .layout_map()
        .get(list_view.taffy_id.unwrap())
        .unwrap()
        .width;
    assert!(
        row_width >= list_width * 0.9,
        "element tree row ({row_width}) should fill its parent ({list_width})"
    );
    let text_row =
        control(inspector.retained().unwrap().root().unwrap(), "Text").expect("text tree row");
    assert_eq!(
        text_row.children[1].children[0]
            .paint
            .icon
            .as_ref()
            .unwrap()
            .data
            .name(),
        "type"
    );
    click(&mut inspector, "Column");
    assert!(
        control(inspector.retained().unwrap().root().unwrap(), "Text").is_some(),
        "selecting a branch must leave its children visible in the inspector tree"
    );
    assert_eq!(
        session.selected.peek(),
        target.retained().unwrap().root().unwrap().taffy_id
    );
    assert!(text(inspector.retained().unwrap().root().unwrap()).contains("Layout bounds"));
    click(&mut inspector, "Clear highlight");
    assert_eq!(session.selected.peek(), None);
    click(&mut inspector, "Pause capture");
    assert!(session.paused.peek());
    click(&mut inspector, "Resume capture");
    assert!(!session.paused.peek());
    click(&mut inspector, "Components / hooks");
    click(&mut inspector, "[]  key=None  renders=1");
    assert!(text(inspector.retained().unwrap().root().unwrap()).contains("#0  state"));
    click(&mut inspector, "Profiler");
    let root = inspector.retained().unwrap().root().unwrap();
    assert!(text(root).contains("patches"));
    assert!(text(root).contains("scene"));
    // Click focuses a tab; Enter activates that same primitive through logical input.
    click(&mut inspector, "Elements");
    inspector
        .dispatch_input(InputEvent::Key(KeyEvent {
            key: LemonKey::Named(NamedKey::Enter),
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
            repeat: false,
        }))
        .unwrap();
    update(&mut inspector);
    assert!(text(inspector.retained().unwrap().root().unwrap()).contains("Select a node"));
    assert!(inspector.inspection.is_none());
}

#[test]
fn removed_node_clears_selection_and_capture_preserves_dirty_semantics() {
    let show = State::new(true);
    let source = show.clone();
    let mut target = UiState::default();
    target.inspection = Some(Capture::default());
    target
        .mount(move |_| {
            ColumnView::new()
                .children(if source.get() {
                    vec![Text::new("temporary").into_element()]
                } else {
                    vec![]
                })
                .into_element()
        })
        .unwrap();
    update(&mut target);
    let session = Session::new();
    session.publish(&mut target);
    let id = target.retained().unwrap().root().unwrap().children[0].taffy_id;
    session.selected.set(id);
    let before = target.layout_map().clone();
    target.inspection.as_mut().unwrap().highlighted = id;
    target.invalidate_paint();
    update(&mut target);
    assert_eq!(&before, target.layout_map());
    assert!(!target.layout_dirty() && !target.paint_dirty());
    assert_eq!(target.inspection.as_ref().unwrap().pending.layouts, 0);
    show.set(false);
    target.apply_updates().unwrap();
    assert!(target.layout_dirty());
    update(&mut target);
    session.publish(&mut target);
    assert_eq!(session.selected.peek(), None);
    assert!(session.snapshot.peek().frames.last().unwrap().patches > 0);
}

#[test]
fn keyed_components_keep_hook_metadata_when_reordered() {
    fn child(cx: &Cx, value: &u32) -> Element {
        let state = cx.state(*value);
        Text::new(state.get().to_string()).into_element()
    }
    let order = State::new(vec![1u32, 2]);
    let source = order.clone();
    let mut target = UiState::default();
    target.inspection = Some(Capture::default());
    target
        .mount(move |_| {
            ColumnView::new()
                .children(source.get().into_iter().map(|id| {
                    Component::new_with_props(child, id)
                        .key(id.into())
                        .into_element()
                }))
                .into_element()
        })
        .unwrap();
    update(&mut target);
    let before = target.runtime().inspect_components();
    assert_eq!(before.len(), 3);
    order.set(vec![2, 1]);
    update(&mut target);
    let after = target.runtime().inspect_components();
    for old in &before[1..] {
        let new = after
            .iter()
            .find(|component| component.key == old.key)
            .unwrap();
        assert_eq!(old.id, new.id);
        assert_eq!(new.render.hooks[0].kind, "state");
        assert!(new.render.hooks[0].calls >= old.render.hooks[0].calls);
    }
}
