use std::{collections::BTreeSet, rc::Rc};

use crate::{
    element::{style::Align, Element},
    icons::{lucide, IconData},
    Color, ColumnView, Cursor, Cx, Icon, KeyEvent, KeyState, LemonKey, NamedKey, RowView, State,
    Text, View,
};

/// One file or directory in a [`FileTree`], independent of the local filesystem.
///
/// IDs must be stable and unique across the entire tree, including hidden nodes.
/// Empty folders remain folders and can be expanded. A label may contain a compact
/// path such as `.github/workflows`; it is displayed as supplied.
#[derive(Clone, Debug, PartialEq)]
pub struct FileTreeNode {
    id: u64,
    label: String,
    children: Option<Rc<[FileTreeNode]>>,
    icon: Option<IconData>,
    color: Option<Color>,
}

impl FileTreeNode {
    /// Creates a leaf file. The default icon is inferred from the label/extension.
    pub fn file(id: u64, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            children: None,
            icon: None,
            color: None,
        }
    }
    /// Creates a directory with ordered children, initially collapsed unless its ID
    /// is present in the tree's expanded state.
    pub fn folder(id: u64, label: impl Into<String>, children: Vec<Self>) -> Self {
        Self {
            children: Some(children.into()),
            ..Self::file(id, label)
        }
    }
    /// Overrides the automatic file icon or the folder's open/closed icons.
    pub fn icon(mut self, icon: IconData) -> Self {
        self.icon = Some(icon);
        self
    }
    /// Overrides this node's label and icon color (for example a modified file).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
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
    /// Returns directory children; `None` identifies a file, `Some(&[])` an empty folder.
    pub fn children(&self) -> Option<&[Self]> {
        self.children.as_deref()
    }
}

/// Geometry and color overrides for [`FileTree`].
///
/// Defaults: 26-point rows, 16-point indentation/icons, subtle hierarchy guides,
/// and theme-accent selection and hover fills. Theme colors are resolved each render.
///
/// ```
/// use lemon::FileTreeStyle;
/// let style = FileTreeStyle::default().row_height(28.0).indent(18.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct FileTreeStyle {
    row_height: f32,
    indent: f32,
    icon_size: f32,
    guide_color: Option<Color>,
    selection_color: Option<Color>,
    hover_color: Option<Color>,
}
impl Default for FileTreeStyle {
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
impl FileTreeStyle {
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

/// A compact, themed file explorer with folder icons and hierarchy guides.
///
/// Click a row to select it and toggle a folder. Up/Down and Home/End navigate
/// visible rows; Right opens a folder or enters it, Left closes it or selects its
/// parent, and Enter/Space choose the current row (toggling folders).
/// The tree is one Tab stop. Collapsing a folder preserves its descendants' open state.
///
/// This widget consumes three hook slots; construct it in a stable order. Rows use
/// no hooks, so nodes can be freely inserted/reordered with stable unique IDs.
/// Data is supplied by the application; this widget never reads the filesystem.
/// Wrap it in [`super::Scroll`] for scrolling and use `.decorate(...)` for its root.
///
/// ```
/// use lemon::prelude::*;
/// let cx = Cx::new();
/// let tree = FileTree::new(&cx, vec![FileTreeNode::folder(1, "src", vec![
///     FileTreeNode::file(2, "main.rs"),
/// ])]).on_select(|id| println!("Selected {id}"));
/// ```
pub struct FileTree {
    nodes: Vec<FileTreeNode>,
    selected: State<Option<u64>>,
    expanded: State<BTreeSet<u64>>,
    hovered: State<Option<u64>>,
    on_select: Option<Rc<dyn Fn(u64)>>,
    style: FileTreeStyle,
    theme: crate::Theme,
}

impl FileTree {
    /// Creates a tree with no selection and all directories collapsed.
    ///
    /// # Panics
    /// Building the element panics if node IDs are duplicated anywhere in the tree.
    pub fn new(cx: &Cx, nodes: Vec<FileTreeNode>) -> Self {
        Self {
            nodes,
            theme: cx.theme(),
            selected: cx.state(None),
            expanded: cx.state(BTreeSet::new()),
            hovered: cx.state(None),
            on_select: None,
            style: FileTreeStyle::default(),
        }
    }
    /// Uses app-owned selection, allowing the selected node to be observed or changed.
    /// A missing or hidden ID is preserved but does not highlight a visible row.
    pub fn selection(mut self, selected: State<Option<u64>>) -> Self {
        self.selected = selected;
        self
    }
    /// Uses app-owned expanded directory IDs. IDs absent from the data are ignored.
    ///
    /// ```
    /// use lemon::prelude::*;
    /// use std::collections::BTreeSet;
    /// let cx = Cx::new();
    /// let open = State::new(BTreeSet::from([1]));
    /// let tree = FileTree::new(&cx, vec![FileTreeNode::folder(1, "src", vec![])])
    ///     .expanded(open);
    /// ```
    pub fn expanded(mut self, expanded: State<BTreeSet<u64>>) -> Self {
        self.expanded = expanded;
        self
    }
    /// Calls `action` with the chosen ID on clicks, keyboard navigation and Enter/Space.
    /// Repeated activation of the same row also calls it. Files and folders both report IDs.
    pub fn on_select(mut self, action: impl Fn(u64) + 'static) -> Self {
        self.on_select = Some(Rc::new(action));
        self
    }
    /// Configures row geometry and colors; root decoration applies afterward.
    pub fn style(mut self, style: FileTreeStyle) -> Self {
        self.style = style;
        self
    }
    /// Builds keyed rows. Panics if any IDs are duplicated, including in closed folders.
    pub fn into_element(self) -> Element {
        validate_ids(&self.nodes, &mut BTreeSet::new());
        let expanded = self.expanded.get();
        let selected = self.selected.get();
        let hovered = self.hovered.get();
        let theme = self.theme;
        let rows = visible_rows(&self.nodes, &expanded);
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
            let icon_color = node.color.unwrap_or(theme.colors.foreground_secondary);
            let icon = node.icon.unwrap_or_else(|| {
                if node.children.is_some() {
                    if is_open {
                        lucide::FOLDER_OPEN
                    } else {
                        lucide::FOLDER
                    }
                } else {
                    file_icon(&node.label)
                }
            });
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
                        .child(
                            Icon::new(icon)
                                .size(self.style.icon_size)
                                .color(icon_color)
                                .stroke_width(1.5),
                        )
                        .child(Text::new(node.label.clone()).color(color)),
                )
                .on_hover_enter(move || enter.set(Some(id)))
                .on_hover_leave(move || {
                    if leave.peek() == Some(id) {
                        leave.set(None);
                    }
                })
                .action(move || {
                    choose(id, &selection, &callback);
                    if is_folder {
                        toggle(id, &open);
                    }
                });
            root = root.child(row);
        }
        let selection = self.selected;
        let expanded = self.expanded;
        let callback = self.on_select;
        root.on_key_down(move |event| navigate(event, &rows, &selection, &expanded, &callback))
            .into_element()
    }
}
impl From<FileTree> for Element {
    fn from(tree: FileTree) -> Self {
        tree.into_element()
    }
}

#[derive(Clone)]
struct VisibleRow {
    node: FileTreeNode,
    depth: usize,
    parent: Option<u64>,
}
fn validate_ids(nodes: &[FileTreeNode], ids: &mut BTreeSet<u64>) {
    for node in nodes {
        assert!(
            ids.insert(node.id),
            "FileTree node IDs must be unique: {}",
            node.id
        );
        if let Some(children) = &node.children {
            validate_ids(children, ids);
        }
    }
}
fn visible_rows(nodes: &[FileTreeNode], expanded: &BTreeSet<u64>) -> Vec<VisibleRow> {
    fn collect(
        nodes: &[FileTreeNode],
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
            if row.node.children.is_some() && expanded.peek().contains(&row.node.id) {
                expanded.update(|open| {
                    open.remove(&row.node.id);
                });
            } else if let Some(parent) = row.parent {
                choose(parent, selected, callback);
            }
        }
        NamedKey::Enter | NamedKey::Space if !event.repeat => {
            choose(row.node.id, selected, callback);
            if row.node.children.is_some() {
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
