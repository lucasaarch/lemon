//! A compact file explorer with Lucide icons, indentation guides and keyboard navigation.
//! Click folders to expand them; use Up/Down, Left/Right and Enter while focused.

use lemon::prelude::*;
use std::collections::BTreeSet;

fn app(cx: &Cx) -> Element {
    let theme = cx.theme();
    let selected = cx.state(None);
    let expanded = cx.state(BTreeSet::from([1]));
    let nodes = vec![FileTreeNode::folder(
        1,
        "lemon",
        vec![
            FileTreeNode::folder(
                2,
                ".github/workflows",
                vec![FileTreeNode::file(3, "ci.yml")],
            ),
            FileTreeNode::folder(4, "examples", vec![FileTreeNode::file(5, "file_tree.rs")]),
            FileTreeNode::folder(
                6,
                "src",
                vec![
                    FileTreeNode::folder(7, "widget", vec![FileTreeNode::file(8, "file_tree.rs")]),
                    FileTreeNode::file(9, "lib.rs"),
                ],
            ),
            FileTreeNode::folder(10, "target", vec![]),
            FileTreeNode::folder(11, "tests", vec![FileTreeNode::file(12, "file_tree.rs")]),
            FileTreeNode::file(13, ".gitignore"),
            FileTreeNode::file(14, "AGENTS.md"),
            FileTreeNode::file(15, "Cargo.lock"),
            FileTreeNode::file(16, "Cargo.toml"),
            FileTreeNode::file(17, "CLAUDE.md").icon(lucide::EXTERNAL_LINK),
            FileTreeNode::file(18, "LICENSE"),
            FileTreeNode::file(19, "Makefile").icon(lucide::LIST_FILTER),
            FileTreeNode::file(20, "README.md"),
        ],
    )];
    let tree = FileTree::new(cx, nodes)
        .selection(selected)
        .expanded(expanded);
    View::new()
        .fill()
        .inset(8.0)
        .surface(theme.colors.surface)
        .child(Scroll::new(cx, tree).width(288.0).height(480.0))
        .into_element()
}

fn main() {
    run(
        WindowConfig::default()
            .title("Lemon — files")
            .size(304.0, 500.0),
        app,
    );
}
