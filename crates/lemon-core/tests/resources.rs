use lemon_core::{AnimationConfig, Cx};

#[test]
#[allow(deprecated)]
fn legacy_and_canonical_resource_names_share_component_identity() {
    let cx = Cx::new();
    let count = cx.state(2);
    let source = count.clone();
    let doubled = cx.derive(move || source.get() * 2);
    let motion = cx.motion(AnimationConfig::default());
    assert_eq!(doubled.get(), 4);

    cx.reset_hooks();
    let old_count = cx.use_signal(99);
    let old_doubled = cx.use_memo(|| 999);
    let old_motion = cx.use_animation(AnimationConfig::default());
    assert_eq!(old_count.get(), 2);
    old_count.set(3);
    assert_eq!(count.get(), 3);
    assert_eq!(doubled.get(), 6);
    assert_eq!(old_doubled.get(), 6);
    old_motion.play();
    assert!(motion.is_playing());
    assert_eq!(cx.theme(), cx.use_theme());
}

#[test]
#[allow(deprecated)]
fn migrating_a_watcher_does_not_rerun_or_replace_its_callback() {
    use lemon_core::{Runtime, State, Text};
    use std::{cell::RefCell, rc::Rc};
    let legacy = State::new(true);
    let source = legacy.clone();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let output = seen.clone();
    let mut runtime = Runtime::new();
    runtime.mount(move |cx| {
        let legacy = source.get();
        let output = output.clone();
        if legacy {
            cx.use_effect(move || output.borrow_mut().push("original"));
        } else {
            cx.watch(move || output.borrow_mut().push("replacement"));
        }
        Text::new("stable").into_element()
    });
    assert!(seen.borrow().is_empty());
    runtime.flush_deferred_effects();
    legacy.set(false);
    runtime.flush_effects();
    runtime.flush_deferred_effects();
    assert_eq!(*seen.borrow(), ["original"]);
}
