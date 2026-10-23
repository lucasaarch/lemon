use super::*;
mod support;
#[test]
fn task_rows_keep_identity_through_removal_and_completion() {
    let mut runtime = support::mount(app);
    support::click(&mut runtime, "Remove");
    let text = support::text(runtime.retained().unwrap().root().unwrap()).join("\n");
    assert!(!text.contains("Plan the next release"));
    assert!(text.contains("Review the documentation"));
    support::click(&mut runtime, "Review the documentation");
    assert!(support::text(runtime.retained().unwrap().root().unwrap())
        .iter()
        .any(|s| s == "0 remaining · 2 completed"));
    support::click(&mut runtime, "Clear finished");
    support::click(&mut runtime, "Clear completed");
    let text = support::text(runtime.retained().unwrap().root().unwrap()).join("\n");
    assert!(text.contains("Nothing here"));
    assert!(!text.contains("Share a working prototype"));
}

#[test]
fn adding_and_filtering_use_the_bound_field_and_select() {
    use lemon::{
        input::{hit_test::LogicalPoint, InputEvent},
        retained::RetainedNode,
    };
    fn field(node: &RetainedNode) -> Option<&RetainedNode> {
        if node.text_input.is_some() {
            return Some(node);
        }
        node.children.iter().find_map(field)
    }
    let mut ui = support::mount(app);
    let node = field(ui.retained().unwrap().root().unwrap()).unwrap();
    let rect = *ui.layout_map().get(node.taffy_id.unwrap()).unwrap();
    ui.dispatch_input(InputEvent::Click(LogicalPoint::new(
        rect.x + 10.0,
        rect.y + 10.0,
    )))
    .unwrap();
    ui.dispatch_input(InputEvent::Key(KeyEvent {
        key: LemonKey::Character("Write release notes".into()),
        state: KeyState::Pressed,
        modifiers: Modifiers::default(),
        repeat: false,
    }))
    .unwrap();
    support::click(&mut ui, "Add task");
    assert!(support::text(ui.retained().unwrap().root().unwrap())
        .iter()
        .any(|text| text == "Write release notes"));
    support::click(&mut ui, "All tasks");
    support::click(&mut ui, "Completed");
    let text = support::text(ui.retained().unwrap().root().unwrap()).join("\n");
    assert!(text.contains("Share a working prototype"));
    assert!(!text.contains("Write release notes"));
}
