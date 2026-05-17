//! Árvore de nós + flexbox via [Taffy](https://github.com/DioxusLabs/taffy).
//!
//! Fluxo típico de um toolkit: declarar estilos → `compute_layout` → ler `layout.location` / `layout.size` → pintar.

use parley::{FontContext, FontWeight, LayoutContext};
use taffy::geometry::{Point, Size};
use taffy::prelude::*;
use vello::peniko::Brush;

use crate::ui::type_scale;

/// O que cada nó representa na tela (o Taffy só vê `Style`; isto é o nosso “widget type”).
#[derive(Clone, Copy, Debug)]
pub enum NodeKind {
    /// Container só de layout, sem fundo próprio.
    PassThrough,
    Sidebar,
    NavActive,
    NavItem,
    Card,
    Header,
    Chip,
    Panel,
    RectButton,
    RoundButton,
    Text(TextRole),
}

#[derive(Clone, Copy, Debug)]
pub enum TextRole {
    Title,
    Pill,
    Chip,
    Body,
    RectButton,
    RoundButton,
}

pub struct TextSpec {
    pub text: &'static str,
    pub font_size: f32,
    pub weight: FontWeight,
}

impl TextRole {
    pub fn spec(self) -> TextSpec {
        match self {
            TextRole::Title => TextSpec {
                text: "Lemon UI",
                font_size: type_scale::HEADLINE,
                weight: FontWeight::new(650.0),
            },
            TextRole::Pill => TextSpec {
                text: "Vello + Parley",
                font_size: type_scale::BODY,
                weight: FontWeight::new(600.0),
            },
            TextRole::Chip => TextSpec {
                text: "vetores 2D",
                font_size: type_scale::CAPTION,
                weight: FontWeight::new(500.0),
            },
            TextRole::Body => TextSpec {
                text: "Retângulos, cantos arredondados e texto — layout por flexbox (Taffy), pintura vetorial (Vello).",
                font_size: type_scale::BODY,
                weight: FontWeight::new(400.0),
            },
            TextRole::RectButton => TextSpec {
                text: "Retângulo",
                font_size: type_scale::BODY,
                weight: FontWeight::new(500.0),
            },
            TextRole::RoundButton => TextSpec {
                text: "Arredondado",
                font_size: type_scale::BODY,
                weight: FontWeight::new(600.0),
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NodeCtx {
    pub kind: NodeKind,
}

/// Retângulo absoluto na viewport (origem canto superior-esquerdo).
#[derive(Clone, Copy, Debug)]
pub struct NodeRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub struct UiLayout {
    pub taffy: TaffyTree<NodeCtx>,
    pub root: NodeId,
}

impl UiLayout {
    pub fn new() -> Self {
        let mut taffy = TaffyTree::new();
        let root = build_tree(&mut taffy);
        Self { taffy, root }
    }

    pub fn compute(
        &mut self,
        font_cx: &mut FontContext,
        layout_cx: &mut LayoutContext<Brush>,
        width: f32,
        height: f32,
        scale_factor: f32,
    ) {
        let viewport = Size {
            width: AvailableSpace::Definite(width),
            height: AvailableSpace::Definite(height),
        };
        self.taffy
            .compute_layout_with_measure(
                self.root,
                viewport,
                |known_dimensions, available_space, _node_id, node_ctx, _style| {
                    let Some(NodeCtx {
                        kind: NodeKind::Text(role),
                    }) = node_ctx
                    else {
                        return Size::ZERO;
                    };
                    measure_text(
                        font_cx,
                        layout_cx,
                        &role.spec(),
                        scale_factor,
                        known_dimensions,
                        available_space,
                    )
                },
            )
            .expect("taffy layout failed");
    }

    pub fn visit(
        &self,
        node: NodeId,
        offset: Point<f32>,
        f: &mut impl FnMut(NodeId, &NodeCtx, NodeRect),
    ) {
        let layout = self.taffy.layout(node).expect("layout");
        let rect = NodeRect {
            x: offset.x + layout.location.x,
            y: offset.y + layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        };

        if let Some(ctx) = self.taffy.get_node_context(node) {
            if !matches!(ctx.kind, NodeKind::PassThrough) {
                f(node, ctx, rect);
            }
        }

        if let Ok(children) = self.taffy.children(node) {
            for child in children {
                self.visit(child, Point { x: rect.x, y: rect.y }, f);
            }
        }
    }
}

fn leaf(taffy: &mut TaffyTree<NodeCtx>, style: Style, kind: NodeKind) -> NodeId {
    taffy
        .new_leaf_with_context(style, NodeCtx { kind })
        .expect("leaf")
}

fn container(
    taffy: &mut TaffyTree<NodeCtx>,
    style: Style,
    kind: NodeKind,
    children: &[NodeId],
) -> NodeId {
    taffy
        .new_with_children(style, children)
        .map(|id| {
            taffy
                .set_node_context(id, Some(NodeCtx { kind }))
                .expect("context");
            id
        })
        .expect("container")
}

fn build_tree(taffy: &mut TaffyTree<NodeCtx>) -> NodeId {
    let nav_active = leaf(
        taffy,
        Style {
            size: Size {
                width: length(32.0),
                height: length(8.0),
            },
            ..Default::default()
        },
        NodeKind::NavActive,
    );
    let nav_items: Vec<NodeId> = (0..3)
        .map(|_| {
            leaf(
                taffy,
                Style {
                    size: Size {
                        width: length(40.0),
                        height: length(16.0),
                    },
                    ..Default::default()
                },
                NodeKind::NavItem,
            )
        })
        .collect();
    let mut sidebar_children = vec![nav_active];
    sidebar_children.extend(nav_items);
    let sidebar = container(
        taffy,
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::Center),
            gap: Size {
                width: zero(),
                height: length(12.0),
            },
            size: Size {
                width: length(68.0),
                height: percent(1.0),
            },
            flex_shrink: 0.0,
            ..Default::default()
        },
        NodeKind::Sidebar,
        &sidebar_children,
    );

    let title = leaf(
        taffy,
        Style {
            flex_shrink: 0.0,
            ..Default::default()
        },
        NodeKind::Text(TextRole::Title),
    );
    let pill = leaf(
        taffy,
        Style {
            flex_shrink: 0.0,
            padding: Rect {
                left: length(16.0),
                right: length(16.0),
                top: length(8.0),
                bottom: length(8.0),
            },
            ..Default::default()
        },
        NodeKind::Text(TextRole::Pill),
    );
    let header = container(
        taffy,
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: Some(JustifyContent::SpaceBetween),
            align_items: Some(AlignItems::Center),
            size: Size {
                width: percent(1.0),
                height: length(68.0),
            },
            flex_shrink: 0.0,
            padding: Rect {
                left: length(24.0),
                right: length(24.0),
                top: length(0.0),
                bottom: length(0.0),
            },
            ..Default::default()
        },
        NodeKind::Header,
        &[title, pill],
    );

    let chip_label = leaf(
        taffy,
        Style::default(),
        NodeKind::Text(TextRole::Chip),
    );
    let chip = container(
        taffy,
        Style {
            display: Display::Flex,
            align_items: Some(AlignItems::Center),
            align_self: Some(AlignSelf::FlexStart),
            flex_shrink: 0.0,
            padding: Rect {
                left: length(16.0),
                right: length(16.0),
                top: length(10.0),
                bottom: length(10.0),
            },
            ..Default::default()
        },
        NodeKind::Chip,
        &[chip_label],
    );

    let body = leaf(
        taffy,
        Style {
            flex_grow: 1.0,
            min_size: Size {
                width: auto(),
                height: length(80.0),
            },
            ..Default::default()
        },
        NodeKind::Text(TextRole::Body),
    );
    let panel = container(
        taffy,
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            padding: Rect::length(16.0),
            ..Default::default()
        },
        NodeKind::Panel,
        &[body],
    );

    let rect_label = leaf(
        taffy,
        Style::default(),
        NodeKind::Text(TextRole::RectButton),
    );
    let rect_btn = container(
        taffy,
        Style {
            display: Display::Flex,
            justify_content: Some(JustifyContent::Center),
            align_items: Some(AlignItems::Center),
            size: Size {
                width: length(128.0),
                height: length(48.0),
            },
            flex_shrink: 0.0,
            ..Default::default()
        },
        NodeKind::RectButton,
        &[rect_label],
    );

    let round_label = leaf(
        taffy,
        Style::default(),
        NodeKind::Text(TextRole::RoundButton),
    );
    let round_btn = container(
        taffy,
        Style {
            display: Display::Flex,
            justify_content: Some(JustifyContent::Center),
            align_items: Some(AlignItems::Center),
            size: Size {
                width: length(128.0),
                height: length(48.0),
            },
            flex_shrink: 0.0,
            ..Default::default()
        },
        NodeKind::RoundButton,
        &[round_label],
    );

    let actions = container(
        taffy,
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            gap: Size {
                width: length(12.0),
                height: zero(),
            },
            flex_shrink: 0.0,
            ..Default::default()
        },
        NodeKind::PassThrough,
        &[rect_btn, round_btn],
    );

    let content = container(
        taffy,
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            gap: Size {
                width: zero(),
                height: length(16.0),
            },
            padding: Rect::length(24.0),
            ..Default::default()
        },
        NodeKind::PassThrough,
        &[chip, panel, actions],
    );

    let card = container(
        taffy,
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            ..Default::default()
        },
        NodeKind::Card,
        &[header, content],
    );

    let main = container(
        taffy,
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            min_size: Size {
                width: length(0.0),
                height: auto(),
            },
            size: Size {
                width: auto(),
                height: percent(1.0),
            },
            ..Default::default()
        },
        NodeKind::PassThrough,
        &[card],
    );

    container(
        taffy,
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            gap: Size {
                width: length(16.0),
                height: zero(),
            },
            padding: Rect::length(20.0),
            ..Default::default()
        },
        NodeKind::PassThrough,
        &[sidebar, main],
    )
}

fn measure_text(
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<Brush>,
    spec: &TextSpec,
    scale_factor: f32,
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
) -> Size<f32> {
    if let (Some(width), Some(height)) = (known_dimensions.width, known_dimensions.height) {
        return Size { width, height };
    }

    use parley::{Alignment, AlignmentOptions, GenericFamily, StyleProperty};

    let mut builder = layout_cx.ranged_builder(font_cx, spec.text, scale_factor, true);
    builder.push_default(GenericFamily::SystemUi);
    builder.push_default(StyleProperty::FontSize(spec.font_size));
    builder.push_default(StyleProperty::FontWeight(spec.weight));
    let mut layout = builder.build(spec.text);

    let max_width = known_dimensions.width.or(match available_space.width {
        AvailableSpace::Definite(w) => Some(w),
        AvailableSpace::MinContent | AvailableSpace::MaxContent => None,
    });
    layout.break_all_lines(max_width);
    layout.align(Alignment::Start, AlignmentOptions::default());

    Size {
        width: layout.width(),
        height: layout.height(),
    }
}
