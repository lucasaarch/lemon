use super::*;
mod support;
#[test]
fn apply_and_revert_keep_the_preview_on_the_last_valid_preferences() {
    let form = Form::new();
    let view = form.clone();
    let mut runtime = support::mount(move |cx| app(cx, &view));
    form.name.set(String::new());
    assert!(!form.apply());
    assert_eq!(form.applied.get().name, "Alex");
    form.name.set("Sam".into());
    form.size.set(0.9);
    support::update(&mut runtime);
    support::click(&mut runtime, "Apply");
    assert_eq!(form.applied.get().name, "Sam");
    assert_eq!(form.applied.get().size, 0.9);
    form.name.set("Unsaved".into());
    support::update(&mut runtime);
    support::click(&mut runtime, "Revert");
    assert_eq!(form.name.get(), "Sam");
}
#[test]
fn previews_share_applied_state_and_uses_the_originating_window_queue() {
    let host = lemon::host::HostContext::new(lemon::Theme::default());
    let _scope = host.enter();
    let form = Form::new();
    let view = form.clone();
    let mut runtime = support::mount(move |cx| app(cx, &view));
    support::click(&mut runtime, "Open preview");
    let request = host.take_window_requests().pop().unwrap();
    let mut preview = support::mount(move |cx| (request.root)(cx));
    form.name.set("Sam".into());
    form.apply();
    support::update(&mut preview);
    assert!(support::text(preview.retained().unwrap().root().unwrap())
        .iter()
        .any(|text| text.contains("Reading with Sam")));
}
