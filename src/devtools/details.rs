use super::{ComponentInfo, FrameSample, NodeInfo};
use crate::element::decoration::Decorate;
use crate::element::Element;
use crate::{ColumnView, RowView, Text};
use std::time::Duration;

pub(super) fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

pub(super) fn empty_details(message: &str, theme: &crate::Theme) -> Element {
    ColumnView::new()
        .fill_width()
        .inset(20.0)
        .spacing(8.0)
        .child(Text::new("Inspector").size(14.0))
        .child(
            Text::new(message)
                .size(12.0)
                .color(theme.colors.foreground_secondary),
        )
        .into_element()
}

fn property_row(label: &str, value: &str, theme: &crate::Theme) -> Element {
    RowView::new()
        .fill_width()
        .spacing(8.0)
        .child(
            Text::new(label)
                .size(12.0)
                .color(theme.colors.foreground_secondary)
                .decorate(|view| view.width(112.0)),
        )
        .child(
            Text::new(value)
                .font("Menlo")
                .size(11.0)
                .decorate(|view| view.fill_width().min_width(0.0)),
        )
        .into_element()
}

fn property_group(
    title: &str,
    properties: impl IntoIterator<Item = (String, String)>,
    theme: &crate::Theme,
) -> Element {
    let mut rows = ColumnView::new()
        .fill_width()
        .spacing(7.0)
        .child(Text::new(title).size(13.0).color(theme.colors.accent));
    for (label, value) in properties {
        rows = rows.child(property_row(&label, &value, theme));
    }
    rows.into_element()
}

pub(super) fn element_details(node: &NodeInfo, theme: &crate::Theme) -> Element {
    let bounds = node
        .rect
        .map(|rect| {
            vec![
                ("x".into(), format!("{:.1}", rect.x)),
                ("y".into(), format!("{:.1}", rect.y)),
                ("width".into(), format!("{:.1}", rect.width)),
                ("height".into(), format!("{:.1}", rect.height)),
            ]
        })
        .unwrap_or_default();
    let mut identity = vec![("kind".into(), node.kind.clone())];
    if let Some(id) = &node.debug_id {
        identity.push(("id".into(), id.clone()));
    }
    let group = |prefix: &'static str| {
        node.properties.iter().filter_map(move |(key, value)| {
            key.strip_prefix(prefix)
                .map(|label| (label.to_owned(), value.clone()))
        })
    };
    ColumnView::new()
        .fill_width()
        .spacing(18.0)
        .child(Text::new(node.label.clone()).size(18.0))
        .child(
            Text::new(format!("path {}", crate::debug::format_path(&node.path)))
                .font("Menlo")
                .size(11.0)
                .color(theme.colors.foreground_secondary),
        )
        .child(property_group("Identity", identity, theme))
        .child(property_group("Layout bounds", bounds, theme))
        .child(property_group("Layout", group("layout."), theme))
        .child(property_group("Paint", group("paint."), theme))
        .child(property_group("Content", group("content."), theme))
        .into_element()
}

pub(super) fn component_details(component: &ComponentInfo, theme: &crate::Theme) -> Element {
    let info = &component.render;
    let hooks = info
        .hooks
        .iter()
        .enumerate()
        .map(|(index, hook)| {
            format!(
                "#{index}  {:<10} {:<36} {} calls",
                hook.kind, hook.value_type, hook.calls
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    ColumnView::new()
        .fill_width()
        .spacing(12.0)
        .child(
            Text::new(format!(
                "Component {}",
                crate::debug::format_path(&component.path)
            ))
            .size(18.0),
        )
        .child(
            Text::new(format!(
                "key {:?} · {} renders",
                component.key, info.renders
            ))
            .font("Menlo")
            .size(11.0),
        )
        .child(
            Text::new(format!(
                "View CPU   last {:7.3} ms   total {:7.3} ms",
                ms(info.last_render),
                ms(info.total_render)
            ))
            .font("Menlo")
            .size(11.0),
        )
        .child(
            Text::new("Hooks · latest render")
                .size(13.0)
                .color(theme.colors.accent),
        )
        .child(Text::new(hooks).font("Menlo").size(11.0))
        .child(
            Text::new(
                "Hook counts are declarations. State values and effect executions are omitted.",
            )
            .size(11.0)
            .color(theme.colors.foreground_secondary),
        )
        .into_element()
}

pub(super) fn frame_details(frame: &FrameSample, theme: &crate::Theme) -> Element {
    let values = vec![
        ("sample".to_owned(), frame.number.to_string()),
        ("updates".to_owned(), format!("{:.3} ms", ms(frame.updates))),
        (
            "layout".to_owned(),
            format!("{:.3} ms · {} passes", ms(frame.layout), frame.layouts),
        ),
        (
            "scene".to_owned(),
            format!("{:.3} ms · {} passes", ms(frame.paint), frame.paints),
        ),
        ("patches".to_owned(), frame.patches.to_string()),
        ("fills".to_owned(), frame.fills.to_string()),
        ("strokes".to_owned(), frame.strokes.to_string()),
        ("glyph runs".to_owned(), frame.glyph_runs.to_string()),
        ("effect layers".to_owned(), frame.effect_layers.to_string()),
    ];
    ColumnView::new().fill_width().spacing(14.0)
        .child(Text::new("Frame sample").size(18.0))
        .child(property_group("CPU", values, theme))
        .child(Text::new("Times accumulate on the inspected window. DevTools rendering and GPU presentation waits are excluded.").size(11.0).color(theme.colors.foreground_secondary))
        .into_element()
}
