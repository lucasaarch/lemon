use super::*;
use lemon::Runtime;

mod support;

#[test]
fn increment_and_reset_update_the_memo() {
    let mut runtime = Runtime::new();
    runtime.mount(app);
    runtime.flush_deferred_effects();
    support::click(&mut runtime, "Increment");
    assert_eq!(
        &support::text(&runtime.root_element().unwrap())[..2],
        ["Count: 1", "Doubled (memo): 2"],
    );
    support::click(&mut runtime, "Reset");
    assert_eq!(
        &support::text(&runtime.root_element().unwrap())[..2],
        ["Count: 0", "Doubled (memo): 0"],
    );
}
