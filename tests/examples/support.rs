use lemon::{
    element::Element,
    input::{hit_test::LogicalPoint, InputEvent},
    retained::RetainedNode,
    ui::UiState,
    Cx, Viewport,
};
pub fn mount(root: impl Fn(&Cx) -> Element + 'static) -> UiState {
    let mut ui = UiState::default();
    ui.mount(root).unwrap();
    update(&mut ui);
    ui
}
pub fn update(ui: &mut UiState) {
    ui.update(
        Viewport {
            width: 1000.0,
            height: 850.0,
        },
        1.0,
        ui.focused(),
        true,
    )
    .unwrap();
}
pub fn text(node: &RetainedNode) -> Vec<String> {
    node.text
        .iter()
        .map(|text| text.content.clone())
        .chain(node.children.iter().flat_map(text))
        .collect()
}
pub fn click(ui: &mut UiState, label: &str) {
    fn find<'a>(node: &'a RetainedNode, label: &str) -> Option<&'a RetainedNode> {
        if !node.style.interactive {
            return None;
        }
        if node.handlers.on_click.is_some() && text(node).first().is_some_and(|text| text == label)
        {
            return Some(node);
        }
        node.children.iter().find_map(|child| find(child, label))
    }
    update(ui);
    let node = find(ui.retained().unwrap().root().unwrap(), label).expect("clickable control");
    let rect = ui.layout_map().get(node.taffy_id.unwrap()).unwrap();
    ui.dispatch_input(InputEvent::Click(LogicalPoint::new(
        rect.x + rect.width / 2.0,
        rect.y + rect.height / 2.0,
    )))
    .unwrap();
    update(ui);
}
