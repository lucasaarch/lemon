use lemon::{host::HostContext, prelude::*, ui::UiState, Theme, Viewport};

#[test]
fn default_button_and_fields_have_the_same_height() {
    for mut theme in [Theme::default_light(), Theme::default_dark()] {
        for font_size in [14.0, 20.0] {
            theme.typography.font_size_md = font_size;
            theme.spacing.sm = font_size / 2.0;
            let host = HostContext::new(theme.clone());
            let _scope = host.enter();
            let mut ui = UiState::default();
            ui.mount(|cx| {
                ColumnView::new()
                    .child(Button::new(cx, "Save"))
                    .child(TextField::new(cx, State::new(String::new())))
                    .child(TextField::new(cx, State::new("Name".to_owned())))
                    .child(SecureField::new(cx, State::new("Secret".to_owned())))
                    .into_element()
            })
            .unwrap();
            ui.layout(Viewport {
                width: 400.0,
                height: 600.0,
            })
            .unwrap();
            let root = ui.retained().unwrap().root().unwrap();
            let heights: Vec<_> = root
                .children
                .iter()
                .map(|node| ui.layout_map().get(node.taffy_id.unwrap()).unwrap().height)
                .collect();
            assert!(
                heights
                    .iter()
                    .all(|height| (height - heights[0]).abs() < 0.01),
                "font {font_size}: {heights:?}"
            );
        }
    }
}

#[test]
fn explicit_button_height_and_padding_still_override_defaults() {
    let mut ui = UiState::default();
    ui.mount(|cx| {
        ColumnView::new()
            .child(Button::new(cx, "Save").height(60.0))
            .child(Button::new(cx, "Save").inset(12.0).disabled(true))
            .child(
                TextField::new(cx, State::new("Name".to_owned()))
                    .inset(12.0)
                    .disabled(true),
            )
            .into_element()
    })
    .unwrap();
    ui.layout(Viewport {
        width: 400.0,
        height: 600.0,
    })
    .unwrap();
    let root = ui.retained().unwrap().root().unwrap();
    let heights: Vec<_> = root
        .children
        .iter()
        .map(|node| ui.layout_map().get(node.taffy_id.unwrap()).unwrap().height)
        .collect();
    assert_eq!(heights[0], 60.0);
    assert_eq!(heights[1], heights[2]);
}
