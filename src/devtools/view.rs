use std::rc::Rc;

use super::details::{component_details, element_details, empty_details, frame_details, ms};
use super::snapshot::tree_nodes;
use super::{Session, HISTORY};
use crate::element::Element;
use crate::widget::file_tree::{Tree, TreeStyle};
use crate::{
    Button, ColumnView, Cx, Decorate, KeyState, LemonKey, NamedKey, RowView, ScrollAxis,
    ScrollOffset, ScrollView, Text, View,
};
use taffy::NodeId;

impl Session {
    pub fn view(&self, cx: &Cx) -> Element {
        let theme = cx.theme();
        let tab = cx.state(0usize);
        let selected_component = cx.state(None::<usize>);
        let selected_frame = cx.state(None::<u64>);
        let list_offset = cx.state(ScrollOffset::default());
        let detail_offset = cx.state(ScrollOffset::default());
        let snapshot = self.snapshot.get();
        let selected = self.selected.get();
        let tree_selection = cx.state(selected.map(u64::from));
        let selected_tree_id = selected.map(u64::from);
        if tree_selection.peek() != selected_tree_id {
            tree_selection.set(selected_tree_id);
        }
        let paused = self.paused.get();
        let tree = Tree::new(cx, tree_nodes(&snapshot.nodes, None))
            .non_collapsible()
            .selection(tree_selection)
            .style(
                TreeStyle::default()
                    .row_height(28.0)
                    .indent(15.0)
                    .icon_size(15.0)
                    .selection_color(theme.colors.accent.with_alpha(0.2))
                    .hover_color(theme.colors.surface_hover),
            )
            .on_select({
                let selected = self.selected.clone();
                move |id| selected.set(Some(NodeId::from(id)))
            })
            .on_hover({
                let hovered = self.hovered.clone();
                move |id| hovered.set(id.map(NodeId::from))
            })
            .into_element();
        let mut tabs = RowView::new().spacing(8.0);
        for (index, title) in ["Elements", "Components / hooks", "Profiler"]
            .into_iter()
            .enumerate()
        {
            let target = tab.clone();
            tabs = tabs.child(
                View::new()
                    .key(index as u64)
                    .inset(10.0)
                    .surface(if tab.get() == index {
                        theme.colors.surface_pressed
                    } else {
                        theme.colors.surface
                    })
                    .child(Text::new(title))
                    .decorate(move |view| activate(view, move || target.set(index))),
            );
        }
        let pause = self.paused.clone();
        let pause_button = Button::new(
            cx,
            if paused {
                "Resume capture"
            } else {
                "Pause capture"
            },
        )
        .action(move || pause.update(|value| *value = !*value));
        let clear = self.selected.clone();
        let clear_button = Button::new(cx, "Clear highlight").action(move || clear.set(None));
        let (list, list_axis, details) = match tab.get() {
            0 => {
                let details = snapshot
                    .nodes
                    .iter()
                    .find(|node| node.id == selected)
                    .map(|node| element_details(node, &theme))
                    .unwrap_or_else(|| {
                        empty_details(
                            "Select a node to inspect its layout and paint properties.",
                            &theme,
                        )
                    });
                (tree.clone(), ScrollAxis::Vertical, details)
            }
            1 => {
                let mut list = ColumnView::new().fill_width().spacing(2.0);
                for component in &snapshot.components {
                    let target = selected_component.clone();
                    let id = component.id;
                    list = list.child(
                        View::new()
                            .key(id as u64)
                            .fill_width()
                            .inset(8.0)
                            .surface(theme.colors.surface)
                            .child(
                                Text::new(format!(
                                    "{}  key={:?}  renders={}",
                                    crate::debug::format_path(&component.path),
                                    component.key,
                                    component.render.renders
                                ))
                                .size(12.0),
                            )
                            .decorate(move |view| activate(view, move || target.set(Some(id)))),
                    );
                }
                let details = snapshot
                    .components
                    .iter()
                    .find(|component| Some(component.id) == selected_component.get())
                    .map(|component| component_details(component, &theme))
                    .unwrap_or_else(|| {
                        empty_details("Select a component to inspect renders and hooks.", &theme)
                    });
                (list.into_element(), ScrollAxis::Both, details)
            }
            _ => {
                let mut list = ColumnView::new().fill_width().spacing(4.0);
                for frame in snapshot.frames.iter().rev() {
                    let target = selected_frame.clone();
                    let number = frame.number;
                    list = list.child(
                        View::new()
                            .key(number)
                            .inset(6.0)
                            .fill_width()
                            .surface(if selected_frame.get() == Some(number) {
                                theme.colors.surface_pressed
                            } else {
                                theme.colors.surface
                            })
                            .child(
                                Text::new(format!(
                                    "#{:04}  {:6.2} ms",
                                    frame.number,
                                    ms(frame.updates + frame.layout + frame.paint)
                                ))
                                .font("Menlo")
                                .size(11.0),
                            )
                            .decorate(move |view| activate(view, move || target.set(Some(number)))),
                    );
                }
                let details = snapshot
                    .frames
                    .iter()
                    .find(|frame| Some(frame.number) == selected_frame.get())
                    .or_else(|| snapshot.frames.last())
                    .map(|frame| frame_details(frame, &theme))
                    .unwrap_or_else(|| {
                        empty_details("Waiting for the inspected window to render.", &theme)
                    });
                (list.into_element(), ScrollAxis::Both, details)
            }
        };
        ColumnView::new()
            .fill()
            .inset(10.0)
            .spacing(8.0)
            .surface(theme.colors.background)
            .child(Text::new("Lemon DevTools").size(22.0))
            .child(
                Text::new("F12 opens this window · Debug build · CPU measurements")
                    .size(12.0)
                    .color(theme.colors.foreground_secondary),
            )
            .child(tabs)
            .child(
                RowView::new()
                    .spacing(8.0)
                    .children([pause_button.into_element(), clear_button.into_element()]),
            )
            .child(
                RowView::new()
                    .fill()
                    .spacing(12.0)
                    .child(
                        ScrollView::with_offset(list_offset, list)
                            .axis(list_axis)
                            .decorate(|view| {
                                view.width(310.0)
                                    .fill_height()
                                    .min_width(0.0)
                                    .surface(theme.colors.surface)
                                    .inset(theme.spacing.xs)
                            }),
                    )
                    .child(
                        ScrollView::with_offset(detail_offset, details)
                            .axis(ScrollAxis::Both)
                            .decorate(|view| {
                                view.fill()
                                    .min_width(0.0)
                                    .surface(theme.colors.surface)
                                    .inset(theme.spacing.md)
                            }),
                    ),
            )
            .child(
                Text::new(format!(
                    "{} elements · {} components · {} / {} samples{}",
                    snapshot.nodes.len(),
                    snapshot.components.len(),
                    snapshot.frames.len(),
                    HISTORY,
                    if paused { " · PAUSED" } else { "" }
                ))
                .size(12.0),
            )
            .into_element()
    }
}

fn activate(view: View, callback: impl Fn() + 'static) -> View {
    let callback = Rc::new(callback);
    let keyboard = callback.clone();
    view.focusable()
        .cursor(crate::element::events::Cursor::Pointer)
        .action(move || callback())
        .on_key_down(move |event| {
            if event.state == KeyState::Pressed
                && !event.repeat
                && matches!(
                    event.key,
                    LemonKey::Named(NamedKey::Enter | NamedKey::Space)
                )
            {
                keyboard();
            }
        })
}
