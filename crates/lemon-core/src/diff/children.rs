use std::collections::{HashMap, HashSet};

use super::{element_key, Element};

/// Structural operations use current indices; updates use final indices.
pub(crate) enum ChildOp {
    Remove(usize),
    Move { from: usize, to: usize },
    Insert(usize),
    Update { old: usize, new: usize },
}

/// Plans keyed reconciliation once for both plain elements and component slots.
pub(crate) fn keyed_plan(old: &[Element], new: &[Element]) -> Vec<ChildOp> {
    let old_keys: Vec<_> = old.iter().map(|el| element_key(el).unwrap()).collect();
    let new_keys: Vec<_> = new.iter().map(|el| element_key(el).unwrap()).collect();
    let old_indices: HashMap<_, _> = old_keys
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, key)| (key, index))
        .collect();
    let new_set: HashSet<_> = new_keys.iter().cloned().collect();
    assert_eq!(old_indices.len(), old.len(), "duplicate sibling keys");
    assert_eq!(new_set.len(), new.len(), "duplicate sibling keys");

    let mut ops = Vec::new();
    let mut current = old_keys.clone();
    for index in (0..old.len()).rev() {
        if !new_set.contains(&old_keys[index]) {
            ops.push(ChildOp::Remove(index));
            current.remove(index);
        }
    }
    for (to, key) in new_keys.iter().enumerate() {
        if let Some(from) = current.iter().position(|candidate| candidate == key) {
            if from != to {
                ops.push(ChildOp::Move { from, to });
                let moved = current.remove(from);
                current.insert(to, moved);
            }
        } else {
            ops.push(ChildOp::Insert(to));
            current.insert(to, key.clone());
        }
    }
    for (new, key) in new_keys.iter().enumerate() {
        if let Some(&old) = old_indices.get(key) {
            ops.push(ChildOp::Update { old, new });
        }
    }
    ops
}
