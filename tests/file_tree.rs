use lemon::{
    host::HostContext,
    input::{hit_test::LogicalPoint, InputEvent},
    prelude::*,
    retained::RetainedNode,
    ui::UiState,
    Theme, Viewport,
};
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

const VIEWPORT: Viewport = Viewport {
    width: 320.0,
    height: 500.0,
};
fn nodes() -> Vec<FileTreeNode> {
    vec![
        FileTreeNode::folder(
            1,
            "lemon",
            vec![
                FileTreeNode::folder(2, "src", vec![FileTreeNode::file(3, "lib.rs")]),
                FileTreeNode::file(4, "README.md"),
            ],
        ),
        FileTreeNode::folder(5, "empty", vec![]),
    ]
}
fn labels(node: &RetainedNode) -> Vec<&str> {
    let mut result = Vec::new();
    if let Some(text) = node.text_content() {
        result.push(text);
    }
    for child in &node.children {
        result.extend(labels(child));
    }
    result
}
fn press(key: NamedKey) -> InputEvent {
    InputEvent::Key(KeyEvent {
        key: LemonKey::Named(key),
        modifiers: Modifiers::default(),
        repeat: false,
        state: KeyState::Pressed,
    })
}
fn update(ui: &mut UiState) {
    ui.update(VIEWPORT, 1.0, ui.focused(), true).unwrap();
}
fn click_row(ui: &mut UiState, index: usize) {
    let root = ui.retained().unwrap().root().unwrap();
    let rect = ui
        .layout_map()
        .get(root.children[index].taffy_id.unwrap())
        .unwrap();
    let point = LogicalPoint::new(rect.x + 8.0, rect.y + rect.height / 2.0);
    ui.dispatch_input(InputEvent::Click(point)).unwrap();
    update(ui);
}
#[test]
fn folders_expand_select_and_preserve_disclosure_with_stable_rows_in_both_themes() {
    for theme in [Theme::default_light(), Theme::default_dark()] {
        let host = HostContext::new(theme);
        let _scope = host.enter();
        let selected = State::new(None);
        let expanded = State::new(BTreeSet::new());
        let selection = selected.clone();
        let open = expanded.clone();
        let mut ui = UiState::default();
        ui.mount(move |cx| {
            FileTree::new(cx, nodes())
                .selection(selection.clone())
                .expanded(open.clone())
                .into_element()
        })
        .unwrap();
        update(&mut ui);
        assert_eq!(
            labels(ui.retained().unwrap().root().unwrap()),
            ["lemon", "empty"]
        );
        click_row(&mut ui, 0);
        assert_eq!(selected.peek(), Some(1));
        assert_eq!(
            labels(ui.retained().unwrap().root().unwrap()),
            ["lemon", "src", "README.md", "empty"]
        );
        let root_id = ui.retained().unwrap().root().unwrap().children[0].taffy_id;
        click_row(&mut ui, 1);
        assert_eq!(
            labels(ui.retained().unwrap().root().unwrap()),
            ["lemon", "src", "lib.rs", "README.md", "empty"]
        );
        let leaf_id = ui.retained().unwrap().root().unwrap().children[2].taffy_id;
        click_row(&mut ui, 2);
        assert_eq!(selected.peek(), Some(3));
        assert_eq!(
            ui.retained().unwrap().root().unwrap().children[2].taffy_id,
            leaf_id
        );
        click_row(&mut ui, 0);
        assert_eq!(
            labels(ui.retained().unwrap().root().unwrap()),
            ["lemon", "empty"]
        );
        assert!(expanded.peek().contains(&2));
        click_row(&mut ui, 0);
        assert!(labels(ui.retained().unwrap().root().unwrap()).contains(&"lib.rs"));
        assert_eq!(
            ui.retained().unwrap().root().unwrap().children[0].taffy_id,
            root_id
        );
        click_row(&mut ui, 4);
        assert!(expanded.peek().contains(&5));
    }
}
#[test]
fn keyboard_navigates_visible_rows_and_parent_child_relationships() {
    let selected = State::new(None);
    let selection = selected.clone();
    let open = State::new(BTreeSet::new());
    let expanded = open.clone();
    let events = Rc::new(RefCell::new(Vec::new()));
    let callback = events.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        FileTree::new(cx, nodes())
            .selection(selection.clone())
            .expanded(expanded.clone())
            .on_select({
                let callback = callback.clone();
                move |id| callback.borrow_mut().push(id)
            })
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    for (key, expected) in [
        (NamedKey::Tab, None),
        (NamedKey::Home, Some(1)),
        (NamedKey::ArrowRight, Some(1)),
        (NamedKey::ArrowRight, Some(2)),
        (NamedKey::ArrowRight, Some(2)),
        (NamedKey::ArrowDown, Some(3)),
        (NamedKey::ArrowLeft, Some(2)),
        (NamedKey::ArrowLeft, Some(2)),
        (NamedKey::End, Some(5)),
        (NamedKey::ArrowUp, Some(4)),
        (NamedKey::Enter, Some(4)),
    ] {
        ui.dispatch_input(press(key)).unwrap();
        update(&mut ui);
        assert_eq!(selected.peek(), expected);
    }
    assert!(!open.peek().contains(&2));
    assert_eq!(events.borrow().last(), Some(&4));
    ui.dispatch_input(press(NamedKey::Tab)).unwrap();
    assert_eq!(
        ui.focused(),
        ui.retained().unwrap().root().unwrap().taffy_id,
        "Tab wraps to the single tree stop"
    );
}
#[test]
fn row_geometry_icons_and_theme_changes_flow_through_the_pipeline() {
    let host = HostContext::new(Theme::default_light());
    let _scope = host.enter();
    let mut ui = UiState::default();
    ui.mount(|cx| {
        FileTree::new(cx, nodes())
            .expanded(State::new(BTreeSet::from([1, 2])))
            .style(
                FileTreeStyle::default()
                    .row_height(30.0)
                    .indent(20.0)
                    .icon_size(18.0),
            )
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    let root = ui.retained().unwrap().root().unwrap();
    let icon = &root.children[2].children[1].children[0];
    let id = icon.taffy_id.unwrap();
    let rect = ui.layout_map().get(id).unwrap();
    assert_eq!(rect.width, 18.0);
    assert_eq!(rect.height, 18.0);
    assert_eq!(rect.x, 46.0);
    assert_eq!(icon.paint.icon.as_ref().unwrap().data.name(), "file-code");
    let before = icon.paint.icon.as_ref().unwrap().color;
    // A host theme update must propagate to vector paint without replacing the row.
    host.set_theme(Theme::default_dark());
    update(&mut ui);
    let root = ui.retained().unwrap().root().unwrap();
    let icon = &root.children[2].children[1].children[0];
    assert_eq!(icon.taffy_id, Some(id));
    assert_ne!(icon.paint.icon.as_ref().unwrap().color, before);
}
#[test]
#[should_panic(expected = "FileTree node IDs must be unique")]
fn duplicate_ids_are_rejected_even_in_closed_folders() {
    FileTree::new(
        &Cx::new(),
        vec![FileTreeNode::folder(
            1,
            "root",
            vec![FileTreeNode::file(1, "hidden")],
        )],
    )
    .into_element();
}

#[test]
fn inserting_and_reordering_nodes_preserves_selection_and_row_identity() {
    let data = State::new(vec![
        FileTreeNode::file(1, "a.rs"),
        FileTreeNode::file(2, "b.rs"),
    ]);
    let source = data.clone();
    let selected = State::new(Some(2));
    let selection = selected.clone();
    let mut ui = UiState::default();
    ui.mount(move |cx| {
        FileTree::new(cx, source.get())
            .selection(selection.clone())
            .into_element()
    })
    .unwrap();
    update(&mut ui);
    let original = ui.retained().unwrap().root().unwrap().children[1].taffy_id;
    data.set(vec![
        FileTreeNode::file(2, "renamed.rs"),
        FileTreeNode::file(3, "new.rs"),
        FileTreeNode::file(1, "a.rs"),
    ]);
    update(&mut ui);
    let root = ui.retained().unwrap().root().unwrap();
    assert_eq!(labels(root), ["renamed.rs", "new.rs", "a.rs"]);
    assert_eq!(root.children[0].taffy_id, original);
    assert_eq!(selected.peek(), Some(2));
}
