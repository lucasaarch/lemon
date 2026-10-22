use super::*;
use lemon::Runtime;

mod support;

#[test]
fn submission_requires_name_and_role_then_clears_the_input() {
    let mut runtime = Runtime::new();
    runtime.mount(app);
    support::click(&mut runtime, "Submit");
    assert!(!support::text(&runtime.root_element().unwrap())
        .iter()
        .any(|s| s.contains(" — ")));

    support::click(&mut runtime, "Choose a role");
    support::click(&mut runtime, "Reader");
    support::click(&mut runtime, "Submit");
    assert!(!support::text(&runtime.root_element().unwrap())
        .iter()
        .any(|s| s.contains(" — ")));

    let root = runtime.root_element().unwrap();
    let Element::View(input) = &support::children(&root)[0] else {
        panic!("expected text input")
    };
    input.handlers.on_key_down.as_ref().unwrap()(KeyEvent {
        key: LemonKey::Character("  Ada  ".into()),
        modifiers: Modifiers::default(),
        repeat: false,
        state: KeyState::Pressed,
    });
    runtime.flush_effects();
    let root = runtime.root_element().unwrap();
    let Element::View(password) = &support::children(&root)[1] else {
        panic!("expected secure field")
    };
    password.handlers.on_key_down.as_ref().unwrap()(KeyEvent {
        key: LemonKey::Character("secret".into()),
        modifiers: Modifiers::default(),
        repeat: false,
        state: KeyState::Pressed,
    });
    runtime.flush_effects();
    assert!(!support::text(&runtime.root_element().unwrap()).contains(&"secret".into()));
    support::click(&mut runtime, "Submit");
    let root = runtime.root_element().unwrap();
    let Element::View(password) = &support::children(&root)[1] else {
        panic!("expected secure field")
    };
    assert_eq!(password.text_input.as_ref().unwrap().value, "");
    assert!(support::text(&root).contains(&"Ada — Reader — 50%".into()));
    let Element::View(input) = &support::children(&root)[0] else {
        panic!("expected text input")
    };
    assert_eq!(input.text_input.as_ref().unwrap().value, "");
}
