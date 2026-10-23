use lemon::prelude::*;

#[test]
fn facade_and_prelude_export_plain_and_secure_fields_with_shared_styles() {
    let cx = Cx::new();
    let plain = State::new("name".to_owned());
    let secret = State::new("password".to_owned());
    let style: lemon::TextFieldStyle = TextFieldStyle::Bordered;
    let field: lemon::TextField = TextField::new(&cx, plain).style(style).rounding(8.0);
    let secure: lemon::SecureField = SecureField::new(&cx, secret).style(style).rounding(8.0);
    let Element::Column(root) = ColumnView::new().child(field).child(secure).into_element() else {
        panic!("expected column")
    };
    assert_eq!(root.children.len(), 2);
}

fn press(key: LemonKey) -> KeyEvent {
    KeyEvent {
        key,
        modifiers: Modifiers::default(),
        repeat: false,
        state: KeyState::Pressed,
    }
}

#[test]
fn string_state_edits_and_external_unicode_changes_keep_internal_caret_valid() {
    let cx = Cx::new();
    let value = State::new("abcd".to_owned());
    let Element::View(field) = TextField::new(&cx, value.clone()).into_element() else {
        panic!()
    };
    let handler = field.handlers.on_key_down.unwrap();
    handler(press(LemonKey::Named(NamedKey::End)));
    value.set("é".to_owned());
    handler(press(LemonKey::Character("!".to_owned())));
    assert_eq!(value.get(), "é!");
    value.set(String::new());
    handler(press(LemonKey::Character("ok".to_owned())));
    assert_eq!(value.get(), "ok");
    cx.reset_hooks();
    let Element::View(field) = TextField::new(&cx, value).into_element() else {
        panic!()
    };
    assert_eq!(field.text_input.unwrap().cursor, 2);
}

#[test]
fn typing_replaces_selection_and_external_writes_discard_old_selection() {
    let cx = Cx::new();
    let value = State::new("abcd".to_owned());
    let Element::View(field) = TextField::new(&cx, value.clone()).into_element() else {
        panic!()
    };
    let handler = field.handlers.on_key_down.unwrap();
    let mut select = press(LemonKey::Character("a".to_owned()));
    select.modifiers.ctrl = true;
    handler(select.clone());
    handler(press(LemonKey::Character("x".to_owned())));
    assert_eq!(value.get(), "x");
    handler(select);
    value.set("new".to_owned());
    handler(press(LemonKey::Character("!".to_owned())));
    assert_eq!(value.get(), "n!ew");
}

#[test]
fn alignment_moves_values_placeholders_and_masks_without_resetting_editing() {
    use lemon::{host::HostContext, input::InputEvent, ui::UiState, Theme, Viewport};
    let viewport = Viewport {
        width: 500.0,
        height: 300.0,
    };
    for theme in [Theme::default_light(), Theme::default_dark()] {
        let host = HostContext::new(theme);
        let _scope = host.enter();
        for secure in [false, true] {
            let value = State::new("héllo".to_owned());
            let alignment = State::new(TextAlignment::Start);
            let source = value.clone();
            let placement = alignment.clone();
            let mut ui = UiState::default();
            ui.mount(move |cx| {
                if secure {
                    SecureField::new(cx, source.clone())
                        .width(300.0)
                        .inset(8.0)
                        .placeholder("Placeholder")
                        .alignment(placement.get())
                        .into_element()
                } else {
                    TextField::new(cx, source.clone())
                        .width(300.0)
                        .inset(8.0)
                        .placeholder("Placeholder")
                        .alignment(placement.get())
                        .into_element()
                }
            })
            .unwrap();
            ui.update(viewport, 1.0, None, true).unwrap();
            ui.dispatch_input(InputEvent::Key(press(LemonKey::Named(NamedKey::Tab))))
                .unwrap();
            ui.dispatch_input(InputEvent::Key(press(LemonKey::Named(NamedKey::End))))
                .unwrap();
            for empty in [false, true] {
                if empty {
                    value.set(String::new());
                }
                for mode in [
                    TextAlignment::Start,
                    TextAlignment::Center,
                    TextAlignment::End,
                ] {
                    alignment.set(mode);
                    ui.update(viewport, 1.0, ui.focused(), true).unwrap();
                    let field = ui.retained().unwrap().root().unwrap();
                    let meta = field.text_input.as_ref().unwrap();
                    assert_eq!(meta.alignment, mode);
                    assert_eq!(
                        meta.cursor,
                        if empty {
                            0
                        } else if secure {
                            15
                        } else {
                            6
                        }
                    );
                    assert_eq!(
                        meta.value,
                        if empty {
                            ""
                        } else if secure {
                            "•••••"
                        } else {
                            "héllo"
                        }
                    );
                    let text = &field.children[0].children[0];
                    assert_eq!(
                        text.text_content(),
                        Some(if empty {
                            "Placeholder"
                        } else if secure {
                            "•••••"
                        } else {
                            "héllo"
                        })
                    );
                    let rect = ui.layout_map().get(text.taffy_id.unwrap()).unwrap();
                    let expected = match mode {
                        TextAlignment::Start => 8.0,
                        TextAlignment::Center => (300.0 - rect.width) / 2.0,
                        TextAlignment::End => 292.0 - rect.width,
                    };
                    assert!((rect.x - expected).abs() < 0.1, "{mode:?}: {rect:?}");
                    assert_eq!(value.get(), if empty { "" } else { "héllo" });
                }
            }
        }
    }
}

#[test]
fn clean_fields_paint_only_the_caret_when_focused_unless_explicitly_overridden() {
    use lemon::{host::HostContext, paint::paint_pass, ui::UiState, Theme, Viewport};
    for theme in [Theme::default_light(), Theme::default_dark()] {
        let accent = theme.colors.accent;
        let host = HostContext::new(theme);
        let _scope = host.enter();
        for secure in [false, true] {
            for explicit in [false, true] {
                let mut ui = UiState::default();
                ui.mount(move |cx| {
                    let value = State::new("hello".to_owned());
                    if secure {
                        let field = SecureField::new(cx, value)
                            .style(TextFieldStyle::Clean)
                            .width(200.0);
                        if explicit {
                            field.focus_ring_color(accent)
                        } else {
                            field
                        }
                        .into_element()
                    } else {
                        let field = TextField::new(cx, value)
                            .style(TextFieldStyle::Clean)
                            .width(200.0);
                        if explicit {
                            field.focus_ring_color(accent)
                        } else {
                            field
                        }
                        .into_element()
                    }
                })
                .unwrap();
                ui.layout(Viewport {
                    width: 300.0,
                    height: 200.0,
                })
                .unwrap();
                let tree = ui.retained().unwrap();
                let root = tree.root().unwrap();
                for focused in [None, root.taffy_id] {
                    let mut scene = vello::Scene::new();
                    let stats = paint_pass(tree, ui.layout_map(), &mut scene, 1.0, focused, true);
                    let expected = if focused.is_some() {
                        1 + usize::from(explicit)
                    } else {
                        0
                    };
                    assert_eq!(
                        stats.strokes as usize, expected,
                        "secure={secure}, override={explicit}"
                    );
                    assert_eq!(root.paint.background.unwrap().a, 0.0);
                }
            }
        }
    }
}
