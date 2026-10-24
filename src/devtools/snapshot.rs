use super::NodeInfo;
use crate::diff::NodePath;
use crate::retained::{RetainedKind, RetainedNode};
use crate::ui::UiState;
use crate::widget::file_tree::TreeNode;

pub(super) fn tree_nodes(nodes: &[NodeInfo], parent: Option<&[usize]>) -> Vec<TreeNode> {
    nodes
        .iter()
        .filter(|node| match parent {
            None => node.path.0.is_empty(),
            Some(parent) => {
                node.path.0.len() == parent.len() + 1 && node.path.0.starts_with(parent)
            }
        })
        .map(|node| {
            let children = tree_nodes(nodes, Some(&node.path.0));
            let mut item = if children.is_empty() {
                TreeNode::leaf(node.tree_id, node.label.clone())
            } else {
                TreeNode::branch(node.tree_id, node.label.clone(), children)
            };
            item = item.icon(node.icon);
            item = item.icon_color(node.icon_color);
            item
        })
        .collect()
}

pub(super) fn collect_nodes(
    node: &RetainedNode,
    ui: &UiState,
    path: &NodePath,
    _depth: usize,
    output: &mut Vec<NodeInfo>,
) {
    let (kind, icon, icon_color) = match node.kind {
        RetainedKind::View => (
            "View",
            crate::icons::lucide::BOX,
            crate::Color::rgb8(93, 190, 171),
        ),
        RetainedKind::Row => (
            "Row",
            crate::icons::lucide::LAYOUT_ARROW_RIGHT,
            crate::Color::rgb8(110, 168, 255),
        ),
        RetainedKind::Column => (
            "Column",
            crate::icons::lucide::LAYOUT_ARROW_DOWN,
            crate::Color::rgb8(190, 145, 255),
        ),
        RetainedKind::Text => (
            "Text",
            crate::icons::lucide::TYPE,
            crate::Color::rgb8(255, 190, 105),
        ),
        RetainedKind::Component { .. } => (
            "Component",
            crate::icons::lucide::COMPONENT,
            crate::Color::rgb8(255, 125, 145),
        ),
    };
    let rect = node
        .taffy_id
        .and_then(|id| ui.layout_map().get(id).copied());
    let mut properties = vec![
        ("layout.width".into(), format!("{:?}", node.style.width)),
        ("layout.height".into(), format!("{:?}", node.style.height)),
        (
            "layout.min width".into(),
            format!("{:?}", node.style.min_width),
        ),
        ("layout.padding".into(), format!("{:?}", node.style.padding)),
        ("layout.margin".into(), format!("{:?}", node.style.margin)),
        ("layout.gap".into(), format!("{:?}", node.style.gap)),
        (
            "layout.position".into(),
            format!("{:?}", node.style.position),
        ),
        (
            "paint.background".into(),
            format!("{:?}", node.paint.background),
        ),
        (
            "paint.border width".into(),
            format!("{:?}", node.paint.border_width),
        ),
        (
            "paint.border color".into(),
            format!("{:?}", node.paint.border_color),
        ),
        ("paint.rounding".into(), format!("{:?}", node.paint.radius)),
        ("paint.opacity".into(), format!("{}", node.style.opacity)),
        ("paint.blur".into(), format!("{}", node.style.blur)),
        (
            "paint.brightness".into(),
            format!("{}", node.style.brightness),
        ),
        (
            "paint.shadow".into(),
            format!("{:?}", node.paint.box_shadow),
        ),
        (
            "paint.gradient".into(),
            node.paint.background_gradient.is_some().to_string(),
        ),
        ("paint.image".into(), node.paint.image.is_some().to_string()),
        ("paint.icon".into(), node.paint.icon.is_some().to_string()),
        (
            "content.scroll viewport".into(),
            format!("{:?}", node.scroll_viewport),
        ),
    ];
    if let Some(id) = &node.debug_id {
        properties.push(("identity.id".into(), id.clone()));
    }
    if let Some(text) = &node.text {
        properties.push((
            "content.characters".into(),
            text.content.chars().count().to_string(),
        ));
        properties.push(("content.font".into(), text.style.font_family.clone()));
        properties.push((
            "content.font size".into(),
            format!("{}", text.style.font_size),
        ));
    }
    output.push(NodeInfo {
        id: node.taffy_id,
        tree_id: node.taffy_id.map(u64::from).unwrap_or(
            path.0
                .iter()
                .fold(0, |id, part| id.wrapping_mul(31).wrapping_add(*part as u64)),
        ),
        path: path.clone(),
        label: node.debug_id.clone().unwrap_or_else(|| {
            format!(
                "{kind}{}",
                if node.text_input.is_some() {
                    " · TextInput"
                } else {
                    ""
                }
            )
        }),
        kind: kind.to_owned(),
        debug_id: node.debug_id.clone(),
        rect,
        icon,
        icon_color,
        properties,
    });
    for (index, child) in node.children.iter().enumerate() {
        let mut child_path = path.clone();
        child_path.0.push(index);
        collect_nodes(child, ui, &child_path, _depth + 1, output);
    }
}
