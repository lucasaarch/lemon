use lemon::{
    element::types::BoxElement, host::HostContext, prelude::*, ui::UiState, Theme, Viewport,
};

fn container(element: &Element) -> &BoxElement {
    match element {
        Element::View(node) | Element::Column(node) | Element::Row(node) => node,
        _ => panic!("expected container"),
    }
}

#[test]
fn all_widget_defaults_follow_live_theme_without_losing_values_or_local_overrides() {
    let host = HostContext::new(Theme::default_dark());
    let name = State::new("Ada".to_owned());
    let secret = State::new("secret".to_owned());
    let selected = State::new(Some(1));
    let open = State::new(true);
    let volume = State::new(0.5);
    let source_name = name.clone();
    let source_secret = secret.clone();
    let source_selected = selected.clone();
    let source_open = open.clone();
    let source_volume = volume.clone();
    let override_color = Color::rgb8(120, 60, 180);
    let mut ui = UiState::default();
    {
        let _scope = host.enter();
        ui.mount(move |cx| {
            ColumnView::new()
                .spacing(12.0)
                .child(Button::new(cx, "Button"))
                .child(TextField::new(cx, source_name.clone()))
                .child(SecureField::new(cx, source_secret.clone()))
                .child(Select::with_open(
                    cx,
                    source_selected.clone(),
                    source_open.clone(),
                    vec![(1, "One".into())],
                ))
                .child(Slider::new(cx, source_volume.clone()))
                .child(ScrollView::new(cx, Text::new("ScrollView content")).height(80.0))
                .child(Button::new(cx, "Custom").surface(override_color))
                .into_element()
        })
        .unwrap();
    }
    let mut dark = Theme::default_dark();
    dark.colors.accent = Color::rgb8(90, 180, 130);
    for theme in [dark.clone(), Theme::default_light(), dark] {
        host.set_theme(theme.clone());
        let _scope = host.enter();
        ui.update(
            Viewport {
                width: 640.0,
                height: 900.0,
            },
            1.0,
            None,
            true,
        )
        .unwrap();
        let root = ui.runtime().root_element().unwrap();
        let root = container(&root);
        assert_eq!(
            container(&root.children[0]).paint.resolve().background,
            Some(theme.colors.accent)
        );
        for index in [1, 2] {
            let field = container(&root.children[index]);
            let paint = field.paint.resolve();
            assert_eq!(paint.background, Some(theme.colors.surface));
            assert_eq!(paint.border_color.top, Some(theme.colors.border));
            assert_eq!(paint.focus_ring_color, Some(theme.chrome.focus_ring));
        }
        assert_eq!(
            container(&root.children[1])
                .text_input
                .as_ref()
                .unwrap()
                .value,
            "Ada"
        );
        assert_eq!(
            container(&root.children[2])
                .text_input
                .as_ref()
                .unwrap()
                .value,
            "••••••"
        );
        let select = container(&root.children[3]);
        assert_eq!(select.children.len(), 2, "an open dropdown stays open");
        assert_eq!(
            container(&select.children[0]).paint.resolve().background,
            Some(theme.colors.surface)
        );
        assert_eq!(
            container(&select.children[1]).paint.resolve().background,
            Some(theme.colors.surface)
        );
        let slider = container(&root.children[4]);
        assert_eq!(slider.paint.resolve().background, Some(theme.colors.border));
        assert_eq!(
            container(&slider.children[0]).paint.resolve().background,
            Some(theme.colors.accent)
        );
        assert_eq!(
            container(&root.children[6]).paint.resolve().background,
            Some(override_color)
        );
        assert_eq!(name.get(), "Ada");
        assert_eq!(secret.get(), "secret");
        assert_eq!(selected.get(), Some(1));
        assert_eq!(volume.get(), 0.5);
        assert!(open.get());
    }
}
