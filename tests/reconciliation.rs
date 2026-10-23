use lemon::diff::{diff, NodePath};
use lemon::element::Element;
use lemon::{ColumnView, Component, Cx, RetainedTree, Runtime, State, Text};

fn tree(keys: &[u64]) -> Element {
    ColumnView::new()
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
                .root()
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
    let order = State::new(vec![0, 1, 2]);
    let source = order.clone();
    let mut runtime = Runtime::new();
    runtime.mount(move |_| tree(&source.get()));
    let mut retained = RetainedTree::mount(runtime.root_element().unwrap()).unwrap();
    for next in [vec![2, 3, 0], vec![3, 2], vec![1, 2, 0, 3]] {
        order.set(next.clone());
        runtime.flush_effects();
        retained.apply_patches(runtime.take_patches()).unwrap();
        let actual: Vec<_> = retained
            .root()
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
        let identity = cx.state(*key);
        Text::new(move || identity.get().to_string()).into_element()
    }
    let order = State::new(vec![0, 1, 2]);
    let source = order.clone();
    let mut runtime = Runtime::new();
    runtime.mount(move |_| {
        ColumnView::new()
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
            .root()
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

#[test]
fn inserted_component_subtrees_mount_and_keep_state_after_reorders() {
    use lemon::{ui::UiState, Decorate};

    fn child(cx: &Cx, key: &u64) -> Element {
        let identity = cx.state(*key);
        Text::new(move || identity.get().to_string()).into_element()
    }
    let order = State::new(vec![0, 1]);
    let source = order.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        ColumnView::new()
            .children(source.get().into_iter().map(|id| {
                Component::new_with_props(child, id)
                    .key(id)
                    .decorate(|view| view.inset(4.0))
            }))
            .into_element()
    })
    .unwrap();
    for next in [
        vec![0, 1],
        vec![2, 0, 3, 1],
        vec![3, 2],
        vec![4, 3, 2],
        vec![],
    ] {
        order.set(next.clone());
        ui.apply_updates().unwrap();

        fn content(node: &lemon::retained::RetainedNode) -> Option<&str> {
            node.text_content()
                .or_else(|| node.children.iter().find_map(content))
        }
        let actual: Vec<_> = ui
            .retained()
            .unwrap()
            .root()
            .unwrap()
            .children
            .iter()
            .map(|node| content(node).unwrap().parse::<u64>().unwrap())
            .collect();
        assert_eq!(actual, next);
    }
}

#[test]
fn replacing_component_types_or_container_subtrees_mounts_the_new_output() {
    use lemon::{ui::UiState, RowView};

    fn first(_: &Cx) -> Element {
        Text::new("first").into_element()
    }

    fn second(_: &Cx) -> Element {
        Text::new("second").into_element()
    }
    let mode = State::new(0);
    let source = mode.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| {
        ColumnView::new()
            .child(match source.get() {
                0 => Text::new("plain").into_element(),
                1 => Component::new(first).into_element(),
                2 => Component::new(second).into_element(),
                _ => RowView::new().child(Component::new(first)).into_element(),
            })
            .into_element()
    })
    .unwrap();

    fn content(node: &lemon::retained::RetainedNode) -> Option<&str> {
        node.text_content()
            .or_else(|| node.children.iter().find_map(content))
    }
    for (next, expected) in [
        (0, "plain"),
        (1, "first"),
        (2, "second"),
        (3, "first"),
        (0, "plain"),
    ] {
        mode.set(next);
        ui.apply_updates().unwrap();
        assert_eq!(
            content(ui.retained().unwrap().root().unwrap()),
            Some(expected)
        );
    }
}

#[test]
fn component_fragment_outputs_use_keyed_reconciliation_and_preserve_hook_state() {
    use lemon::ui::UiState;

    fn child(cx: &Cx, id: &u64) -> Element {
        let original = cx.state(*id);
        Text::new(move || original.get().to_string()).into_element()
    }

    #[derive(Clone, PartialEq)]
    struct Keys(Vec<u64>);

    fn fragment(_: &Cx, keys: &Keys) -> Element {
        Element::Fragment(
            keys.0
                .iter()
                .map(|id| {
                    Component::new_with_props(child, *id)
                        .key(*id)
                        .into_element()
                })
                .collect(),
        )
    }
    let order = State::new(vec![0, 1, 2]);
    let source = order.clone();
    let mut ui = UiState::default();
    ui.mount(move |_| Component::new_with_props(fragment, Keys(source.get())).into_element())
        .unwrap();
    for next in [vec![2, 0, 1], vec![3, 1, 0, 2], vec![2, 3], vec![]] {
        order.set(next.clone());
        ui.apply_updates().unwrap();
        let actual: Vec<_> = ui
            .retained()
            .unwrap()
            .root()
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
