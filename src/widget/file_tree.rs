use std::{collections::BTreeSet, rc::Rc};

use crate::{
    element::{style::Align, Element},
    icons::{lucide, IconData},
    Color, ColumnView, Cursor, Cx, Icon, KeyEvent, KeyState, LemonKey, NamedKey, RowView, State,
    Text, View,
};

/// One item in a [`Tree`], independent of the local filesystem.
///
/// IDs must be stable and unique across the entire tree, including hidden nodes.
/// Empty branches remain expandable. A label may contain a compact path such as
/// `.github/workflows`; it is displayed as supplied.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeNode {
    id: u64,
    label: String,
    children: Option<Rc<[TreeNode]>>,
    icon: Option<IconData>,
    expanded_icon: Option<IconData>,
    color: Option<Color>,
    icon_color: Option<Color>,
}

impl TreeNode {
    /// Creates a leaf node with a default document icon.
    pub fn leaf(id: u64, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            children: None,
            icon: None,
            expanded_icon: None,
            color: None,
            icon_color: None,
        }
    }

    /// Creates a branch with ordered children, initially collapsed unless its ID
    /// is present in the tree's expanded state.
    pub fn branch(id: u64, label: impl Into<String>, children: Vec<Self>) -> Self {
        Self {
            children: Some(children.into()),
            ..Self::leaf(id, label)
        }
    }

    /// Overrides this node's icon. For branches this is used when closed and as the
    /// expanded icon unless [`expanded_icon`](Self::expanded_icon) is also set.
    pub fn icon(mut self, icon: IconData) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Overrides the icon shown while this branch is expanded.
    pub fn expanded_icon(mut self, icon: IconData) -> Self {
        self.expanded_icon = Some(icon);
        self
    }

    /// Turns this node into a branch with ordered children.
    pub fn with_children(mut self, children: Vec<Self>) -> Self {
        self.children = Some(children.into());
        self
    }

    /// Overrides this node's label and icon color (for example a modified file).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Overrides the icon color, taking precedence over [`color`](Self::color) and
    /// leaving the row label unchanged.
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = Some(color);
        self
    }

    /// Returns the stable node identifier.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the display label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns children; `None` identifies a leaf, `Some(&[])` an empty branch.
    pub fn children(&self) -> Option<&[Self]> {
        self.children.as_deref()
    }
}

/// Geometry and color overrides for [`Tree`].
///
/// Defaults: 26-point rows, 16-point indentation/icons, subtle hierarchy guides,
/// and theme-accent selection and hover fills. Theme colors are resolved each render.
///
/// ```
/// use lemon::TreeStyle;
/// let style = TreeStyle::default().row_height(28.0).indent(18.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct TreeStyle {
    row_height: f32,
    indent: f32,
    icon_size: f32,
    guide_color: Option<Color>,
    selection_color: Option<Color>,
    hover_color: Option<Color>,
}

impl Default for TreeStyle {
    fn default() -> Self {
        Self {
            row_height: 26.0,
            indent: 16.0,
            icon_size: 16.0,
            guide_color: None,
            selection_color: None,
            hover_color: None,
        }
    }
}

impl TreeStyle {
    /// Sets row height in logical points. Panics unless finite and positive.
    pub fn row_height(mut self, height: f32) -> Self {
        assert!(
            height.is_finite() && height > 0.0,
            "row height must be finite and positive"
        );
        self.row_height = height;
        self
    }

    /// Sets indentation per level in logical points. Panics unless finite and nonnegative.
    pub fn indent(mut self, indent: f32) -> Self {
        assert!(
            indent.is_finite() && indent >= 0.0,
            "indent must be finite and nonnegative"
        );
        self.indent = indent;
        self
    }

    /// Sets square icon size in logical points. Panics unless finite and nonnegative.
    pub fn icon_size(mut self, size: f32) -> Self {
        assert!(
            size.is_finite() && size >= 0.0,
            "icon size must be finite and nonnegative"
        );
        self.icon_size = size;
        self
    }

    /// Overrides hierarchy guide color. Use a transparent color to hide guides.
    pub fn guide_color(mut self, color: Color) -> Self {
        self.guide_color = Some(color);
        self
    }

    /// Overrides the selected row's fill, which remains visible when the tree loses focus.
    pub fn selection_color(mut self, color: Color) -> Self {
        self.selection_color = Some(color);
        self
    }

    /// Overrides the hovered row's fill; selection takes precedence.
    pub fn hover_color(mut self, color: Color) -> Self {
        self.hover_color = Some(color);
        self
    }
}

/// An accessible, themed, collapsible tree with selectable nodes and custom icons.
///
/// Click a row to select it and toggle a branch. Up/Down and Home/End navigate
/// visible rows; Right opens a branch or enters it, Left closes it or selects its
/// parent, and Enter/Space choose the current row (toggling branches).
/// Branches start expanded. The tree is one Tab stop; collapsing a branch preserves
/// its descendants' open state.
///
/// Supply stable unique IDs. Collapsing a branch preserves its descendants' open state.
/// This widget consumes three hook slots; construct it in a stable order. Rows use
/// no hooks, so nodes can be freely inserted/reordered with stable unique IDs.
/// Data is supplied by the application; this widget never reads the filesystem.
/// Wrap it in [`super::ScrollView`] for scrolling and use `.decorate(...)` for its root.
///
/// ```
/// use lemon::prelude::*;
/// let cx = Cx::new();
/// let tree = Tree::new(&cx, vec![TreeNode::branch(1, "src", vec![
///     TreeNode::leaf(2, "main.rs").icon(lucide::FILE_CODE),
/// ])]).on_select(|id| println!("Selected {id}"));
/// ```
pub struct Tree {
    nodes: Vec<TreeNode>,
    selected: State<Option<u64>>,
    expanded: State<BTreeSet<u64>>,
    hovered: State<Option<u64>>,
    on_select: Option<Rc<dyn Fn(u64)>>,
    on_hover: Option<Rc<dyn Fn(Option<u64>)>>,
    collapsible: bool,
    style: TreeStyle,
    theme: crate::Theme,
    id_error_name: &'static str,
}

impl Tree {
    /// Creates a tree with no selection and all branches expanded.
    ///
    /// # Panics
    /// Building the element panics if node IDs are duplicated anywhere in the tree.
    pub fn new(cx: &Cx, nodes: Vec<TreeNode>) -> Self {
        Self::with_default_expansion(cx, nodes, true)
    }

    fn with_default_expansion(cx: &Cx, nodes: Vec<TreeNode>, expand_all: bool) -> Self {
        let selected = cx.state(None);
        let initial_expansion = if expand_all {
            expanded_ids(&nodes)
        } else {
            BTreeSet::new()
        };
        let expanded = cx.state(initial_expansion);
        let hovered = cx.state(None);
        Self {
            nodes,
            theme: cx.theme(),
            selected,
            expanded,
            hovered,
            on_select: None,
            on_hover: None,
            collapsible: true,
            style: TreeStyle::default(),
            id_error_name: "Tree",
        }
    }

    /// Uses app-owned selection, allowing the selected node to be observed or changed.
    /// A missing or hidden ID is preserved but does not highlight a visible row.
    pub fn selection(mut self, selected: State<Option<u64>>) -> Self {
        self.selected = selected;
        self
    }

    /// Uses app-owned expanded branch IDs. IDs absent from the data are ignored.
    ///
    /// ```
    /// use lemon::prelude::*;
    /// use std::collections::BTreeSet;
    /// let cx = Cx::new();
    /// let open = State::new(BTreeSet::from([1]));
    /// let tree = Tree::new(&cx, vec![TreeNode::branch(1, "src", vec![])])
    ///     .expanded(open);
    /// ```
    pub fn expanded(mut self, expanded: State<BTreeSet<u64>>) -> Self {
        self.expanded = expanded;
        self
    }

    /// Calls `action` with the chosen ID on clicks, keyboard navigation and Enter/Space.
    /// Repeated activation of the same row also calls it. Leaves and branches both report IDs.
    pub fn on_select(mut self, action: impl Fn(u64) + 'static) -> Self {
        self.on_select = Some(Rc::new(action));
        self
    }

    /// Calls `action` with the hovered node ID, or `None` when the pointer leaves a row.
    pub fn on_hover(mut self, action: impl Fn(Option<u64>) + 'static) -> Self {
        self.on_hover = Some(Rc::new(action));
        self
    }

    #[allow(dead_code)] // Used by the debug-only inspector.
    pub(crate) fn non_collapsible(mut self) -> Self {
        self.collapsible = false;
        self
    }

    /// Configures row geometry and colors; root decoration applies afterward.
    pub fn style(mut self, style: TreeStyle) -> Self {
        self.style = style;
        self
    }

    /// Builds keyed rows. Panics if any IDs are duplicated, including in closed branches.
    pub fn into_element(self) -> Element {
        validate_ids(&self.nodes, &mut BTreeSet::new(), self.id_error_name);
        let expanded = if self.collapsible {
            self.expanded.get()
        } else {
            expanded_ids(&self.nodes)
        };
        let selected = self.selected.get();
        let hovered = self.hovered.get();
        let theme = self.theme;
        let rows = visible_rows(&self.nodes, &expanded);
        let collapsible = self.collapsible;
        let mut root = ColumnView::new()
            .fill_width()
            .clip()
            .focusable()
            .transition(super::interaction_transition());
        let guide = self
            .style
            .guide_color
            .unwrap_or(theme.colors.foreground_secondary.with_alpha(0.18));
        for entry in &rows {
            let node = &entry.node;
            let id = node.id;
            let is_open = expanded.contains(&id);
            let color = node.color.unwrap_or(theme.colors.foreground);
            let icon_color = node
                .icon_color
                .or(node.color)
                .unwrap_or(theme.colors.foreground_secondary);
            let icon = if node.children.is_some() {
                if is_open {
                    node.expanded_icon
                        .or(node.icon)
                        .or(Some(lucide::FOLDER_OPEN))
                } else {
                    node.icon.or(Some(lucide::FOLDER))
                }
            } else {
                node.icon.or(Some(lucide::FILE_TEXT))
            };
            let mut guides = RowView::new()
                .width(entry.depth as f32 * self.style.indent)
                .height(self.style.row_height);
            for _ in 0..entry.depth {
                guides = guides.child(
                    View::new()
                        .width(self.style.indent)
                        .height(self.style.row_height)
                        .center()
                        .child(
                            View::new()
                                .width(1.0)
                                .height(self.style.row_height)
                                .surface(guide),
                        ),
                );
            }
            let fill = if selected == Some(id) {
                self.style
                    .selection_color
                    .unwrap_or(theme.colors.accent.with_alpha(0.18))
            } else if hovered == Some(id) {
                self.style
                    .hover_color
                    .unwrap_or(theme.colors.accent.with_alpha(0.08))
            } else {
                Color::default()
            };
            let selection = self.selected.clone();
            let open = self.expanded.clone();
            let callback = self.on_select.clone();
            let is_folder = node.children.is_some();
            let enter = self.hovered.clone();
            let leave = self.hovered.clone();
            let on_enter = self.on_hover.clone();
            let on_leave = self.on_hover.clone();
            let row = RowView::new()
                .key(id)
                .fill_width()
                .height(self.style.row_height)
                .inset_horizontal(6.0)
                .surface(fill)
                .cursor(Cursor::Pointer)
                .clip()
                .child(guides)
                .child(
                    RowView::new()
                        .grow(1.0)
                        .height(self.style.row_height)
                        .spacing(6.0)
                        .align(Align::Center)
                        .child(if let Some(icon) = icon {
                            Icon::new(icon)
                                .size(self.style.icon_size)
                                .color(icon_color)
                                .stroke_width(1.5)
                                .into_element()
                        } else {
                            View::new()
                                .width(self.style.icon_size)
                                .height(self.style.icon_size)
                                .into_element()
                        })
                        .child(Text::new(node.label.clone()).color(color)),
                )
                .on_hover_enter(move || {
                    enter.set(Some(id));
                    if let Some(callback) = &on_enter {
                        callback(Some(id));
                    }
                })
                .on_hover_leave(move || {
                    if leave.peek() == Some(id) {
                        leave.set(None);
                        if let Some(callback) = &on_leave {
                            callback(None);
                        }
                    }
                })
                .action(move || {
                    choose(id, &selection, &callback);
                    if is_folder && collapsible {
                        toggle(id, &open);
                    }
                });
            root = root.child(row);
        }
        let selection = self.selected;
        let expanded = self.expanded;
        let callback = self.on_select;
        root.on_key_down(move |event| {
            navigate(event, &rows, &selection, &expanded, &callback, collapsible)
        })
        .into_element()
    }
}

impl From<Tree> for Element {
    fn from(tree: Tree) -> Self {
        tree.into_element()
    }
}

/// File or directory data accepted by the file-oriented [`FileTree`] convenience widget.
#[derive(Clone, Debug, PartialEq)]
pub struct FileTreeNode {
    id: u64,
    label: String,
    children: Option<Rc<[FileTreeNode]>>,
    icon: Option<IconData>,
    color: Option<Color>,
}

impl FileTreeNode {
    /// Creates a file and infers its icon from its label.
    pub fn file(id: u64, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            children: None,
            icon: None,
            color: None,
        }
    }

    /// Creates a directory with ordered children.
    pub fn folder(id: u64, label: impl Into<String>, children: Vec<Self>) -> Self {
        Self {
            children: Some(children.into()),
            ..Self::file(id, label)
        }
    }

    /// Overrides the inferred file icon or the default folder icons.
    pub fn icon(mut self, icon: IconData) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Sets this node's label and icon color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Returns the stable identifier.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the displayed label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns ordered directory children.
    pub fn children(&self) -> Option<&[Self]> {
        self.children.as_deref()
    }
}

/// File-oriented geometry and color settings. Use [`TreeStyle`] for application trees.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileTreeStyle(TreeStyle);

impl FileTreeStyle {
    /// Sets row height in logical points. Panics unless finite and positive.
    pub fn row_height(mut self, height: f32) -> Self {
        self.0 = self.0.row_height(height);
        self
    }
    /// Sets indentation per level in logical points. Panics unless finite and nonnegative.
    pub fn indent(mut self, indent: f32) -> Self {
        self.0 = self.0.indent(indent);
        self
    }
    /// Sets square icon size in logical points. Panics unless finite and nonnegative.
    pub fn icon_size(mut self, size: f32) -> Self {
        self.0 = self.0.icon_size(size);
        self
    }
    /// Overrides hierarchy guide color. Use a transparent color to hide guides.
    pub fn guide_color(mut self, color: Color) -> Self {
        self.0 = self.0.guide_color(color);
        self
    }
    /// Overrides the selected row's fill, which remains visible when the tree loses focus.
    pub fn selection_color(mut self, color: Color) -> Self {
        self.0 = self.0.selection_color(color);
        self
    }
    /// Overrides the hovered row's fill; selection takes precedence.
    pub fn hover_color(mut self, color: Color) -> Self {
        self.0 = self.0.hover_color(color);
        self
    }
}

/// File explorer using the shared [`Tree`] interaction and painting implementation.
///
/// This convenience form preserves automatic file-extension icons. Use [`Tree`] and
/// [`TreeNode`] when the data represents components, syntax or another hierarchy.
pub struct FileTree(Tree);

impl FileTree {
    /// Creates a file tree with collapsed directories.
    pub fn new(cx: &Cx, nodes: Vec<FileTreeNode>) -> Self {
        let mut tree =
            Tree::with_default_expansion(cx, nodes.iter().map(file_node).collect(), false);
        tree.id_error_name = "FileTree";
        Self(tree)
    }

    /// Uses app-owned selected item state.
    pub fn selection(mut self, selected: State<Option<u64>>) -> Self {
        self.0 = self.0.selection(selected);
        self
    }
    /// Uses app-owned expanded directory state.
    pub fn expanded(mut self, expanded: State<BTreeSet<u64>>) -> Self {
        self.0 = self.0.expanded(expanded);
        self
    }
    /// Reports activated file or directory IDs.
    pub fn on_select(mut self, action: impl Fn(u64) + 'static) -> Self {
        self.0 = self.0.on_select(action);
        self
    }
    /// Configures geometry and color.
    pub fn style(mut self, style: FileTreeStyle) -> Self {
        self.0 = self.0.style(style.0);
        self
    }
    /// Builds the shared tree renderer.
    pub fn into_element(self) -> Element {
        self.0.into_element()
    }
}

impl From<FileTree> for Element {
    fn from(tree: FileTree) -> Self {
        tree.into_element()
    }
}

fn file_node(node: &FileTreeNode) -> TreeNode {
    let children = node
        .children
        .as_ref()
        .map(|children| children.iter().map(file_node).collect());
    let mut tree = match children {
        Some(children) => TreeNode::branch(node.id, node.label.clone(), children),
        None => TreeNode::leaf(node.id, node.label.clone())
            .icon(node.icon.unwrap_or_else(|| file_icon(&node.label))),
    };
    if let Some(icon) = node.icon.filter(|_| node.children.is_some()) {
        tree = tree.icon(icon);
    }
    if let Some(color) = node.color {
        tree = tree.color(color);
    }
    tree
}

#[derive(Clone)]
struct VisibleRow {
    node: TreeNode,
    depth: usize,
    parent: Option<u64>,
}

fn validate_ids(nodes: &[TreeNode], ids: &mut BTreeSet<u64>, type_name: &str) {
    for node in nodes {
        assert!(
            ids.insert(node.id),
            "{} node IDs must be unique: {}",
            type_name,
            node.id
        );
        if let Some(children) = &node.children {
            validate_ids(children, ids, type_name);
        }
    }
}

fn expanded_ids(nodes: &[TreeNode]) -> BTreeSet<u64> {
    fn collect(nodes: &[TreeNode], output: &mut BTreeSet<u64>) {
        for node in nodes {
            if let Some(children) = &node.children {
                output.insert(node.id);
                collect(children, output);
            }
        }
    }
    let mut output = BTreeSet::new();
    collect(nodes, &mut output);
    output
}

fn visible_rows(nodes: &[TreeNode], expanded: &BTreeSet<u64>) -> Vec<VisibleRow> {
    fn collect(
        nodes: &[TreeNode],
        expanded: &BTreeSet<u64>,
        depth: usize,
        parent: Option<u64>,
        rows: &mut Vec<VisibleRow>,
    ) {
        for node in nodes {
            rows.push(VisibleRow {
                node: node.clone(),
                depth,
                parent,
            });
            if expanded.contains(&node.id) {
                if let Some(children) = &node.children {
                    collect(children, expanded, depth + 1, Some(node.id), rows);
                }
            }
        }
    }
    let mut rows = Vec::new();
    collect(nodes, expanded, 0, None, &mut rows);
    rows
}

fn choose(id: u64, selected: &State<Option<u64>>, callback: &Option<Rc<dyn Fn(u64)>>) {
    selected.set(Some(id));
    if let Some(callback) = callback {
        callback(id);
    }
}

fn toggle(id: u64, expanded: &State<BTreeSet<u64>>) {
    expanded.update(|open| {
        if !open.remove(&id) {
            open.insert(id);
        }
    });
}

fn navigate(
    event: KeyEvent,
    rows: &[VisibleRow],
    selected: &State<Option<u64>>,
    expanded: &State<BTreeSet<u64>>,
    callback: &Option<Rc<dyn Fn(u64)>>,
    collapsible: bool,
) {
    if event.state != KeyState::Pressed
        || rows.is_empty()
        || event.modifiers.meta
        || event.modifiers.ctrl
        || event.modifiers.alt
    {
        return;
    }
    let current = rows
        .iter()
        .position(|row| Some(row.node.id) == selected.peek());
    let key = match event.key {
        LemonKey::Named(key) => key,
        _ => return,
    };
    let index = match key {
        NamedKey::Home => Some(0),
        NamedKey::End => Some(rows.len() - 1),
        NamedKey::ArrowDown => Some(current.map_or(0, |i| (i + 1).min(rows.len() - 1))),
        NamedKey::ArrowUp => Some(current.map_or(0, |i| i.saturating_sub(1))),
        _ => None,
    };
    if let Some(index) = index {
        choose(rows[index].node.id, selected, callback);
        return;
    }
    let Some(index) = current else {
        return;
    };
    let row = &rows[index];
    match key {
        NamedKey::ArrowRight if row.node.children.is_some() => {
            if !expanded.peek().contains(&row.node.id) {
                expanded.update(|open| {
                    open.insert(row.node.id);
                });
            } else if rows
                .get(index + 1)
                .is_some_and(|next| next.parent == Some(row.node.id))
            {
                choose(rows[index + 1].node.id, selected, callback);
            }
        }
        NamedKey::ArrowLeft => {
            if collapsible && row.node.children.is_some() && expanded.peek().contains(&row.node.id)
            {
                expanded.update(|open| {
                    open.remove(&row.node.id);
                });
            } else if let Some(parent) = row.parent {
                choose(parent, selected, callback);
            }
        }
        NamedKey::Enter | NamedKey::Space if !event.repeat => {
            choose(row.node.id, selected, callback);
            if collapsible && row.node.children.is_some() {
                toggle(row.node.id, expanded);
            }
        }
        _ => {}
    }
}

fn file_icon(label: &str) -> IconData {
    let label = label.to_ascii_lowercase();
    if label.ends_with(".lock") {
        lucide::LOCK_KEYHOLE
    } else if label.starts_with(".git") {
        lucide::GIT_BRANCH
    } else if label == "license" || label.starts_with("license.") {
        lucide::BADGE_CHECK
    } else if label.ends_with(".md") {
        lucide::BOOK_OPEN
    } else if label.ends_with(".toml")
        || label.ends_with(".yaml")
        || label.ends_with(".yml")
        || label.ends_with(".json")
    {
        lucide::FILE_COG
    } else if label.ends_with(".rs")
        || label.ends_with(".js")
        || label.ends_with(".ts")
        || label.ends_with(".py")
        || label.ends_with(".go")
    {
        lucide::FILE_CODE
    } else if label.ends_with(".png") || label.ends_with(".jpg") || label.ends_with(".svg") {
        lucide::FILE_IMAGE
    } else {
        lucide::FILE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_collapsible_tree_keeps_branches_visible_when_selected() {
        let cx = Cx::new();
        let selected = State::new(None);
        let expanded = State::new(BTreeSet::new());
        let tree = Tree::new(
            &cx,
            vec![TreeNode::branch(
                1,
                "root",
                vec![TreeNode::leaf(2, "child")],
            )],
        )
        .selection(selected.clone())
        .expanded(expanded.clone())
        .non_collapsible()
        .into_element();
        let Element::Column(root) = tree else {
            panic!("Tree must render a column")
        };
        assert_eq!(root.children.len(), 2);
        let Element::Row(row) = &root.children[0] else {
            panic!("Tree item must render a row")
        };
        row.handlers.on_click.as_ref().unwrap()();
        assert_eq!(selected.peek(), Some(1));
        assert!(expanded.peek().is_empty());
    }
}
