use lemon_core::diff::{diff, NodePath};
use lemon_core::element::Element;
use lemon_core::{Column, Component, Cx, RetainedTree, Runtime, Signal, Text};

fn tree(keys: &[u64]) -> Element {
    Column::new()
        .children(
            keys.iter()
                .map(|&key| Text::new(key.to_string()).key(key).into_element()),
        )
        .into_element()
}

fn permutations(prefix: Vec<u64>, out: &mut Vec<Vec<u64>>) {
    out.push(prefix.clone());
    for key in 0..4 {
        if !prefix.contains(&key) {
            let mut next = prefix.clone();
            next.push(key);
            permutations(next, out);
        }
    }
}

#[test]
#[should_panic(expected = "duplicate sibling keys")]
fn duplicate_keys_are_rejected_instead_of_silently_losing_identity() {
    diff(tree(&[1, 2]), tree(&[1, 1]), NodePath::root());
}

#[test]
fn all_keyed_insert_remove_reorder_combinations_apply_to_expected_tree() {
    let mut sequences = Vec::new();
    permutations(Vec::new(), &mut sequences);
    for old in &sequences {
        for new in &sequences {
            let mut retained = RetainedTree::mount(tree(old)).unwrap();
            retained
                .apply_patches(diff(tree(old), tree(new), NodePath::root()))
                .unwrap();
            let actual: Vec<_> = retained
                .root
                .as_ref()
                .unwrap()
                .children
                .iter()
                .map(|node| node.text_content().unwrap().parse::<u64>().unwrap())
                .collect();
            assert_eq!(&actual, new, "{old:?} -> {new:?}");
        }
    }
}

#[test]
fn runtime_and_plain_diff_agree_on_keyed_operations() {
    let order = Signal::new(vec![0, 1, 2]);
    let source = order.clone();
    let mut runtime = Runtime::new();
    runtime.mount(move |_| tree(&source.get()));
    let mut retained = RetainedTree::mount(runtime.root_element().unwrap()).unwrap();
    for next in [vec![2, 3, 0], vec![3, 2], vec![1, 2, 0, 3]] {
        order.set(next.clone());
        runtime.flush_effects();
        retained.apply_patches(runtime.take_patches()).unwrap();
        let actual: Vec<_> = retained
            .root
            .as_ref()
            .unwrap()
            .children
            .iter()
            .map(|node| node.text_content().unwrap().parse::<u64>().unwrap())
            .collect();
        assert_eq!(actual, next);
    }
}

#[test]
fn keyed_component_state_follows_identity_on_reorder() {
    fn child(cx: &Cx, key: &u64) -> Element {
        // Initial state identifies the original instance even if props change.
        let identity = cx.use_signal(*key);
        Text::new(move || identity.get().to_string()).into_element()
    }
    let order = Signal::new(vec![0, 1, 2]);
    let source = order.clone();
    let mut runtime = Runtime::new();
    runtime.mount(move |_| {
        Column::new()
            .children(source.get().into_iter().map(|key| {
                Component::new_with_props(child, key)
                    .key(key)
                    .into_element()
            }))
            .into_element()
    });
    let mut retained = RetainedTree::mount(runtime.root_element().unwrap()).unwrap();
    retained.apply_patches(runtime.take_patches()).unwrap();
    for next in [vec![2, 0, 1], vec![1, 2, 0]] {
        order.set(next.clone());
        runtime.flush_effects();
        retained.apply_patches(runtime.take_patches()).unwrap();
        let actual: Vec<_> = retained
            .root
            .as_ref()
            .unwrap()
            .children
            .iter()
            .map(|node| {
                node.children[0]
                    .text_content()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .collect();
        assert_eq!(actual, next);
    }
}
