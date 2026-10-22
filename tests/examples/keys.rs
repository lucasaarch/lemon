use super::*;
use lemon::Runtime;

mod support;

fn row_keys(runtime: &Runtime) -> Vec<u64> {
    let root = runtime.root_element().unwrap();
    let list = &support::children(&root)[2];
    support::children(list)
        .iter()
        .map(|row| {
            let Element::Row(row) = row else {
                panic!("expected row")
            };
            row.key.as_ref().unwrap().0
        })
        .collect()
}

#[test]
fn removal_and_reordering_keep_original_ids() {
    let mut runtime = Runtime::new();
    runtime.mount(app);
    support::click(&mut runtime, "Reverse");
    assert_eq!(row_keys(&runtime), [3, 2, 1]);
    support::click(&mut runtime, "Remove");
    assert_eq!(row_keys(&runtime), [2, 1]);
    support::click(&mut runtime, "Reset");
    assert_eq!(row_keys(&runtime), [1, 2, 3]);
}
