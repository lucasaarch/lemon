use lemon::{AnimationConfig, Cx};

#[test]
fn canonical_resources_keep_identity_across_component_renders() {
    let cx = Cx::new();
    let count = cx.state(2);
    let source = count.clone();
    let doubled = cx.derive(move || source.get() * 2);
    let motion = cx.motion(AnimationConfig::default());
    assert_eq!(doubled.get(), 4);

    cx.reset_hooks();
    let next_count = cx.state(99);
    let next_doubled = cx.derive(|| 999);
    let next_motion = cx.motion(AnimationConfig::default());
    assert_eq!(next_count.get(), 2);
    next_count.set(3);
    assert_eq!(count.get(), 3);
    assert_eq!(doubled.get(), 6);
    assert_eq!(next_doubled.get(), 6);
    next_motion.play();
    assert!(motion.is_playing());
}

#[test]
fn rerendering_a_watcher_does_not_rerun_or_replace_its_callback() {
    use lemon::{Runtime, State, Text};
    use std::{cell::RefCell, rc::Rc};
    let label = State::new("original");
    let source = label.clone();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let output = seen.clone();
    let mut runtime = Runtime::new();
    runtime.mount(move |cx| {
        let label = source.get();
        let output = output.clone();
        cx.watch(move || output.borrow_mut().push(label));
        Text::new("stable").into_element()
    });
    assert!(seen.borrow().is_empty());
    runtime.flush_deferred_effects();
    label.set("replacement");
    runtime.flush_effects();
    runtime.flush_deferred_effects();
    assert_eq!(*seen.borrow(), ["original"]);
}
