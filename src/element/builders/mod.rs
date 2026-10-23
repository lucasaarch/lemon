//! Base builders for composition, presentation and scrolling.
//! Semantic controls live in [`crate::widget`].
mod component;
mod container;
pub mod icon;
pub mod image;
pub mod scroll;
mod text;
pub use component::Component;
pub use container::{ColumnView, RowView, View};
pub use icon::Icon;
pub use image::Image;
pub use scroll::{ScrollView, ScrollViewStyle};
pub use text::Text;
#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::types::Key;
    use crate::element::{
        style::{BoxShadow, Color, Edges, LinearGradient, Overflow},
        Element,
    };
    use crate::theme::{current_theme, set_active_theme, Theme};
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn column_builder_sets_gap() {
        let Element::Column(el) = ColumnView::new().spacing(8.0).into_element() else {
            panic!()
        };
        assert_eq!(el.style.gap, Some(8.0));
    }

    #[test]
    fn column_builder_sets_row_gap_and_column_gap() {
        let Element::Column(el) = ColumnView::new()
            .vertical_spacing(6.0)
            .horizontal_spacing(3.0)
            .into_element()
        else {
            panic!()
        };
        assert_eq!(el.style.row_gap, Some(6.0));
        assert_eq!(el.style.column_gap, Some(3.0));
        assert_eq!(el.style.gap, None);
    }

    #[test]
    fn column_builder_sets_padding_per_side() {
        let Element::Column(el) = ColumnView::new()
            .inset_top(1.0)
            .inset_right(2.0)
            .inset_bottom(3.0)
            .inset_left(4.0)
            .into_element()
        else {
            panic!()
        };
        let p = el.style.padding.as_ref().expect("padding");
        assert_eq!(p.top, 1.0);
        assert_eq!(p.right, 2.0);
        assert_eq!(p.bottom, 3.0);
        assert_eq!(p.left, 4.0);
    }

    #[test]
    fn column_builder_padding_x_and_y() {
        let Element::Column(el) = ColumnView::new()
            .inset_horizontal(8.0)
            .inset_vertical(4.0)
            .into_element()
        else {
            panic!()
        };
        let p = el.style.padding.as_ref().expect("padding");
        assert_eq!(p.left, 8.0);
        assert_eq!(p.right, 8.0);
        assert_eq!(p.top, 4.0);
        assert_eq!(p.bottom, 4.0);
    }

    #[test]
    fn column_builder_sets_margin_uniform() {
        let Element::Column(el) = ColumnView::new().outset(10.0).into_element() else {
            panic!()
        };
        let m = el.style.margin.as_ref().expect("margin");
        assert_eq!(m.top, 10.0);
        assert_eq!(m.right, 10.0);
        assert_eq!(m.bottom, 10.0);
        assert_eq!(m.left, 10.0);
    }

    #[test]
    fn column_builder_sets_margin_per_side() {
        let Element::Column(el) = ColumnView::new()
            .outset_top(1.0)
            .outset_right(2.0)
            .outset_bottom(3.0)
            .outset_left(4.0)
            .into_element()
        else {
            panic!()
        };
        let m = el.style.margin.as_ref().expect("margin");
        assert_eq!(m.top, 1.0);
        assert_eq!(m.right, 2.0);
        assert_eq!(m.bottom, 3.0);
        assert_eq!(m.left, 4.0);
    }

    #[test]
    fn column_builder_margin_x_and_y() {
        let Element::Column(el) = ColumnView::new()
            .outset_horizontal(6.0)
            .outset_vertical(2.0)
            .into_element()
        else {
            panic!()
        };
        let m = el.style.margin.as_ref().expect("margin");
        assert_eq!(m.left, 6.0);
        assert_eq!(m.right, 6.0);
        assert_eq!(m.top, 2.0);
        assert_eq!(m.bottom, 2.0);
    }

    #[test]
    fn margin_top_does_not_overwrite_other_sides() {
        let Element::Column(el) = ColumnView::new()
            .outset(5.0)
            .outset_top(20.0)
            .into_element()
        else {
            panic!()
        };
        let m = el.style.margin.as_ref().expect("margin");
        assert_eq!(m.top, 20.0);
        assert_eq!(m.right, 5.0);
        assert_eq!(m.bottom, 5.0);
        assert_eq!(m.left, 5.0);
    }

    #[test]
    fn border_top_does_not_overwrite_other_sides() {
        let Element::Column(el) = ColumnView::new()
            .border(Color::rgb8(1, 2, 3), 5.0)
            .border_top(Color::rgb8(1, 2, 3), 2.0)
            .into_element()
        else {
            panic!()
        };
        let b = el.paint.border_width;
        assert_eq!(b.top, 2.0);
        assert_eq!(b.right, 5.0);
        assert_eq!(b.bottom, 5.0);
        assert_eq!(b.left, 5.0);
    }

    #[test]
    fn border_color_top_does_not_overwrite_other_sides() {
        let Element::Column(el) = ColumnView::new()
            .border(Color::rgb8(1, 2, 3), 5.0)
            .border_color_top(Color::rgb8(9, 8, 7))
            .into_element()
        else {
            panic!()
        };
        let colors = el.paint.resolve().border_color;
        assert_eq!(colors.top, Some(Color::rgb8(9, 8, 7)));
        assert_eq!(colors.right, Some(Color::rgb8(1, 2, 3)));
        assert_eq!(colors.bottom, Some(Color::rgb8(1, 2, 3)));
        assert_eq!(colors.left, Some(Color::rgb8(1, 2, 3)));
    }

    #[test]
    fn linear_gradient_replaces_solid_background() {
        let Element::View(el) = View::new()
            .surface(Color::rgb8(1, 2, 3))
            .gradient(LinearGradient::new(
                (0.0, 0.0),
                (1.0, 1.0),
                Color::rgb8(10, 20, 30),
                Color::rgb8(40, 50, 60),
            ))
            .into_element()
        else {
            panic!()
        };
        assert!(el.paint.background.is_none());
        assert_eq!(
            el.paint.background_gradient,
            Some(
                LinearGradient::new(
                    (0.0, 0.0),
                    (1.0, 1.0),
                    Color::rgb8(10, 20, 30),
                    Color::rgb8(40, 50, 60),
                )
                .into()
            )
        );
    }

    #[test]
    fn box_builder_sets_box_shadow() {
        let shadow = BoxShadow::new(Color::rgb8(0, 0, 0).with_alpha(0.25), 0.0, 6.0, 12.0);
        let Element::View(el) = View::new().shadow(shadow).into_element() else {
            panic!()
        };
        assert_eq!(el.paint.box_shadow, Some(shadow));
    }

    #[test]
    fn radius_top_sets_both_top_corners() {
        let Element::Column(el) = ColumnView::new()
            .rounding(4.0)
            .rounding_top(12.0)
            .into_element()
        else {
            panic!()
        };
        let r = el.paint.radius;
        assert_eq!(r.top_left, 12.0);
        assert_eq!(r.top_right, 12.0);
        assert_eq!(r.bottom_right, 4.0);
        assert_eq!(r.bottom_left, 4.0);
    }

    #[test]
    fn row_with_children() {
        let Element::Row(el) = RowView::new()
            .child(Text::new("a"))
            .child(Text::new("b"))
            .into_element()
        else {
            panic!()
        };
        assert_eq!(el.children.len(), 2);
    }

    #[test]
    fn children_macro_accepts_mixed_builders() {
        use crate::{children, ColumnView, RowView, Text};

        let Element::Column(col) = ColumnView::new()
            .children(children![
                Text::new("title"),
                crate::View::new().child(crate::Text::new("ok")),
                RowView::new().child(Text::new("nested")),
            ])
            .into_element()
        else {
            panic!("expected Column");
        };
        assert_eq!(col.children.len(), 3);
    }

    #[test]
    fn box_builder_sets_key() {
        let Element::View(el) = View::new().key(9).into_element() else {
            panic!()
        };
        assert_eq!(el.key, Some(Key(9)));
    }

    #[test]
    fn box_builder_sets_overflow() {
        let Element::View(el) = View::new().clip().into_element() else {
            panic!()
        };
        assert_eq!(el.style.overflow, Overflow::Hidden);
    }

    #[test]
    fn box_builder_sets_z_index() {
        let Element::View(el) = View::new().layer(3).into_element() else {
            panic!()
        };
        assert_eq!(el.style.z_index, 3);
    }

    #[test]
    fn box_builder_clamps_opacity_to_valid_range() {
        let Element::View(el) = View::new().opacity(1.5).into_element() else {
            panic!()
        };
        assert_eq!(el.style.opacity, 1.0);

        let Element::View(el) = View::new().opacity(-0.5).into_element() else {
            panic!()
        };
        assert_eq!(el.style.opacity, 0.0);

        let Element::View(el) = View::new().opacity(f32::NAN).into_element() else {
            panic!()
        };
        assert_eq!(el.style.opacity, 1.0);
    }

    #[test]
    fn box_builder_sets_absolute_insets() {
        let Element::View(el) = View::new().top(12.0).left(8.0).into_element() else {
            panic!()
        };
        let inset = el.style.inset.as_ref().expect("expected inset");
        assert_eq!(inset.top, Some(12.0));
        assert_eq!(inset.left, Some(8.0));
    }

    #[test]
    fn box_builder_sets_focusable_cursor_and_handlers() {
        let Element::View(el) = View::new()
            .focusable()
            .cursor(crate::element::events::Cursor::Pointer)
            .action(|| {})
            .on_hover_enter(|| {})
            .on_hover_leave(|| {})
            .on_key_down(|_| {})
            .on_key_up(|_| {})
            .into_element()
        else {
            panic!()
        };
        assert!(el.style.focusable);
        assert_eq!(el.style.cursor, crate::element::events::Cursor::Pointer);
        assert!(el.handlers.on_click.is_some());
        assert!(el.handlers.on_hover_enter.is_some());
        assert!(el.handlers.on_hover_leave.is_some());
        assert!(el.handlers.on_key_down.is_some());
        assert!(el.handlers.on_key_up.is_some());
    }

    #[test]
    fn text_static_content() {
        let Element::Text(el) = Text::new("hello").into_element() else {
            panic!()
        };
        assert_eq!(el.content.resolve(), "hello");
    }

    #[test]
    fn text_dynamic_content() {
        let value = Rc::new(Cell::new(7u32));
        let v = value.clone();
        let Element::Text(el) = Text::new(move || v.get().to_string()).into_element() else {
            panic!()
        };
        assert_eq!(el.content.resolve(), "7");
        value.set(42);
        assert_eq!(el.content.resolve(), "42");
    }

    #[test]
    fn text_defaults_follow_active_theme_typography() {
        let previous = current_theme();
        let mut custom = Theme::default_dark();
        custom.typography.font_size_md = 19.0;
        custom.typography.font_family = "serif".to_string();
        custom.typography.line_height = 1.9;
        custom.typography.letter_spacing = 0.3;
        set_active_theme(custom.clone());

        let Element::Text(el) = Text::new("hello").into_element() else {
            panic!()
        };

        assert_eq!(el.style.font_size, custom.typography.font_size_md);
        assert_eq!(el.style.font_family, custom.typography.font_family);
        assert_eq!(el.style.line_height, custom.typography.line_height);
        assert_eq!(el.style.letter_spacing, custom.typography.letter_spacing);

        set_active_theme(previous);
    }

    #[test]
    fn text_builder_sets_font_family_and_line_height() {
        let Element::Text(el) = Text::new("hello")
            .font("Inter, system-ui")
            .leading(1.2)
            .into_element()
        else {
            panic!()
        };

        assert_eq!(el.style.font_family, "Inter, system-ui");
        assert_eq!(el.style.line_height, 1.2);
    }

    #[test]
    fn view_action_fires() {
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        let Element::View(el) = crate::View::new()
            .child(crate::Text::new("OK"))
            .action(move || f.set(true))
            .into_element()
        else {
            panic!()
        };
        el.handlers.on_click.unwrap()();
        assert!(fired.get());
    }

    #[test]
    fn view_builder_sets_padding() {
        let Element::View(el) = crate::View::new()
            .child(crate::Text::new("OK"))
            .inset(12.0)
            .into_element()
        else {
            panic!()
        };
        assert_eq!(el.style.padding, Some(Edges::all(12.0)));
    }

    #[test]
    fn component_builder_tracks_function_identity_and_key() {
        fn first(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("first").into_element()
        }

        fn second(_cx: &crate::runtime::cx::Cx) -> Element {
            Text::new("second").into_element()
        }

        let Element::Component(first_with_key) = Component::new(first).key(7).into_element() else {
            panic!("expected component element");
        };
        let Element::Component(first_again) = Component::new(first).into_element() else {
            panic!("expected component element");
        };
        let Element::Component(second_component) = Component::new(second).into_element() else {
            panic!("expected component element");
        };

        assert_eq!(first_with_key.key(), Some(&Key(7)));
        assert_eq!(first_with_key.identity(), first_again.identity());
        assert_ne!(first_with_key.identity(), second_component.identity());
    }
}
