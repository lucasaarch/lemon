use lemon_core::{
    element::Element, host::HostContext, ui::UiState, Color, ColumnView, Component, Cx, Text,
    Theme, Viewport,
};
use std::{cell::Cell, rc::Rc};

fn update(ui: &mut UiState, host: &HostContext) {
    let _scope = host.enter();
    ui.update(
        Viewport {
            width: 400.0,
            height: 300.0,
        },
        1.0,
        None,
        true,
    )
    .unwrap();
}

#[test]
fn theme_change_updates_implicit_foregrounds_without_remounting_components() {
    fn child(cx: &Cx) -> Element {
        let count = cx.state(7);
        Text::new(move || format!("Count: {}", count.get())).into_element()
    }
    let host = HostContext::new(Theme::default_dark());
    let renders = Rc::new(Cell::new(0));
    let seen = renders.clone();
    let explicit = Color::rgb8(180, 40, 80);
    let mut ui = UiState::default();
    {
        let _scope = host.enter();
        ui.mount(move |_| {
            seen.set(seen.get() + 1);
            ColumnView::new()
                .child(Text::new("Default"))
                .child(Text::new("Explicit").color(explicit))
                .child(Component::new(child).key(1))
                .into_element()
        })
        .unwrap();
    }
    update(&mut ui, &host);
    assert_eq!(renders.get(), 1);
    assert!(!host.set_theme(Theme::default_dark()));
    assert_eq!(renders.get(), 1);
    assert!(host.set_theme(Theme::default_light()));
    update(&mut ui, &host);
    assert_eq!(renders.get(), 2);
    let root = ui.retained().unwrap().root().unwrap();
    assert_eq!(
        root.children[0].text.as_ref().unwrap().style.color,
        Some(Theme::default_light().colors.foreground)
    );
    assert_eq!(
        root.children[1].text.as_ref().unwrap().style.color,
        Some(explicit)
    );
    let nested = &root.children[2].children[0];
    assert_eq!(nested.text.as_ref().unwrap().content, "Count: 7");
    assert_eq!(
        nested.text.as_ref().unwrap().style.color,
        Some(Theme::default_light().colors.foreground)
    );
}

#[test]
fn changing_an_inactive_host_preserves_active_scope_and_other_views() {
    let original = lemon_core::current_theme();
    let first = HostContext::new(Theme::default_dark());
    let second = HostContext::new(Theme::default_dark());
    let cx = {
        let _scope = first.enter();
        Cx::new()
    };
    let renders = Rc::new(Cell::new(0));
    let seen = renders.clone();
    let mut other = UiState::default();
    {
        let _scope = second.enter();
        other
            .mount(move |_| {
                seen.set(seen.get() + 1);
                Text::new("Other host").into_element()
            })
            .unwrap();
    }
    let _outer = first.enter();
    {
        let _inner = second.enter();
        first.set_theme(Theme::default_light());
        assert_eq!(lemon_core::current_theme(), Theme::default_dark());
        assert_eq!(cx.theme(), Theme::default_light());
        assert_eq!(renders.get(), 1);
    }
    assert_eq!(lemon_core::current_theme(), Theme::default_light());
    drop(_outer);
    assert_eq!(lemon_core::current_theme(), original);
}
