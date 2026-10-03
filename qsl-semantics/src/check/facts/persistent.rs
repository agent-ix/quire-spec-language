// SPDX-License-Identifier: AGPL-3.0-or-later
//! A persistent map for the facts a definedness walk proves.
//!
//! A branch of the walk adds one fact to the facts of the path above it, and
//! the pending steps of the walk keep every path's facts alive. Copying the
//! whole fact set to add one fact would make a chain of `n` nested guards
//! hold `n²/2` entries (ADR-030 D-1: no depth is a limit, so no depth may
//! make a cost quadratic). The map is a binary trie over the key's hash with
//! reference-counted nodes: [`PersistentMap::insert`] copies one root-to-leaf
//! path (at most 64 nodes, a bound of the hash's width and not of the walk's
//! depth), and a clone is one reference count. [`PersistentMap::intersect_with`]
//! walks two maps side by side and skips every subtree they share.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// The bits of a key's hash.
const HASH_BITS: usize = 64;

type Link<K, V> = Option<Rc<Node<K, V>>>;

enum Node<K, V> {
    /// The entries whose keys have this hash, in insertion order.
    Leaf { hash: u64, entries: Vec<(K, V)> },
    /// Keys whose hash has a `0` (left) or `1` (right) at this depth.
    Branch { left: Link<K, V>, right: Link<K, V> },
}

/// An immutable map from `K` to `V` with cheap updates and clones.
pub(super) struct PersistentMap<K, V> {
    root: Link<K, V>,
}

impl<K, V> Clone for PersistentMap<K, V> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

impl<K, V> Default for PersistentMap<K, V> {
    fn default() -> Self {
        Self { root: None }
    }
}

fn hash_of<K: Hash>(key: &K) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

fn bit(hash: u64, depth: usize) -> bool {
    (hash >> depth) & 1 == 1
}

/// A branch at `depth` holding `child` on the side `hash` takes there and
/// `other` on the side it does not.
fn branch<K, V>(depth: usize, hash: u64, child: Link<K, V>, other: Link<K, V>) -> Rc<Node<K, V>> {
    Rc::new(if bit(hash, depth) {
        Node::Branch {
            left: other,
            right: child,
        }
    } else {
        Node::Branch {
            left: child,
            right: other,
        }
    })
}

impl<K: Eq + Hash + Clone, V: Clone> PersistentMap<K, V> {
    /// The value of `key`.
    pub(super) fn get(&self, key: &K) -> Option<&V> {
        let hash = hash_of(key);
        let mut node = self.root.as_deref();
        let mut depth = 0;
        loop {
            match node? {
                Node::Leaf {
                    hash: leaf,
                    entries,
                } => {
                    return if *leaf == hash {
                        entries
                            .iter()
                            .find(|(entry, _)| entry == key)
                            .map(|(_, value)| value)
                    } else {
                        None
                    };
                }
                Node::Branch { left, right } => {
                    node = if bit(hash, depth) {
                        right.as_deref()
                    } else {
                        left.as_deref()
                    };
                    depth += 1;
                }
            }
        }
    }

    /// Whether `key` has a value.
    pub(super) fn contains(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Set `key`'s value, copying one path of the trie.
    pub(super) fn insert(&mut self, key: K, value: V) {
        self.root = Some(insert_at(&self.root, 0, hash_of(&key), key, value));
    }

    /// The entries both maps hold, each valued by `combine` of the two
    /// values; an entry `combine` refuses is dropped. `combine` must return
    /// a value equal to its argument when both arguments are equal, since a
    /// subtree the two maps share is kept as it is.
    pub(super) fn intersect_with(
        &self,
        other: &Self,
        combine: &dyn Fn(&V, &V) -> Option<V>,
    ) -> Self {
        Self {
            root: merge(&self.root, &other.root, 0, combine),
        }
    }
}

fn insert_at<K: Eq + Hash + Clone, V: Clone>(
    node: &Link<K, V>,
    depth: usize,
    hash: u64,
    key: K,
    value: V,
) -> Rc<Node<K, V>> {
    match node.as_deref() {
        None => Rc::new(Node::Leaf {
            hash,
            entries: vec![(key, value)],
        }),
        Some(Node::Leaf {
            hash: leaf,
            entries,
        }) => {
            if *leaf == hash {
                let mut entries = entries.clone();
                match entries.iter_mut().find(|(entry, _)| *entry == key) {
                    Some((_, slot)) => *slot = value,
                    None => entries.push((key, value)),
                }
                Rc::new(Node::Leaf { hash, entries })
            } else {
                let existing = node.clone();
                let added = Rc::new(Node::Leaf {
                    hash,
                    entries: vec![(key, value)],
                });
                split(depth, *leaf, existing, hash, added)
            }
        }
        Some(Node::Branch { left, right }) => {
            let (left, right) = if bit(hash, depth) {
                (
                    left.clone(),
                    Some(insert_at(right, depth + 1, hash, key, value)),
                )
            } else {
                (
                    Some(insert_at(left, depth + 1, hash, key, value)),
                    right.clone(),
                )
            };
            Rc::new(Node::Branch { left, right })
        }
    }
}

/// A branch holding two leaves whose hashes agree on the bits above
/// `depth` and differ in a later one.
fn split<K, V>(
    depth: usize,
    first_hash: u64,
    first: Link<K, V>,
    second_hash: u64,
    second: Rc<Node<K, V>>,
) -> Rc<Node<K, V>> {
    debug_assert!(depth < HASH_BITS && first_hash != second_hash);
    if bit(first_hash, depth) == bit(second_hash, depth) {
        let below = split(depth + 1, first_hash, first, second_hash, second);
        branch(depth, first_hash, Some(below), None)
    } else {
        branch(depth, second_hash, Some(second), first)
    }
}

fn merge<K: Eq + Hash + Clone, V: Clone>(
    left: &Link<K, V>,
    right: &Link<K, V>,
    depth: usize,
    combine: &dyn Fn(&V, &V) -> Option<V>,
) -> Link<K, V> {
    let (a, b) = match (left, right) {
        (Some(a), Some(b)) => (a, b),
        _ => return None,
    };
    if Rc::ptr_eq(a, b) {
        return Some(Rc::clone(a));
    }
    match (&**a, &**b) {
        (
            Node::Branch {
                left: a_left,
                right: a_right,
            },
            Node::Branch {
                left: b_left,
                right: b_right,
            },
        ) => {
            let left = merge(a_left, b_left, depth + 1, combine);
            let right = merge(a_right, b_right, depth + 1, combine);
            if left.is_none() && right.is_none() {
                None
            } else {
                Some(Rc::new(Node::Branch { left, right }))
            }
        }
        (Node::Leaf { entries, .. }, _) => {
            let mut merged = None;
            for (key, value) in entries {
                if let Some(other) = lookup(b, depth, key) {
                    if let Some(combined) = combine(value, other) {
                        merged = Some(insert_at(
                            &merged,
                            depth,
                            hash_of(key),
                            key.clone(),
                            combined,
                        ));
                    }
                }
            }
            merged
        }
        (_, Node::Leaf { entries, .. }) => {
            let mut merged = None;
            for (key, value) in entries {
                if let Some(other) = lookup(a, depth, key) {
                    if let Some(combined) = combine(other, value) {
                        merged = Some(insert_at(
                            &merged,
                            depth,
                            hash_of(key),
                            key.clone(),
                            combined,
                        ));
                    }
                }
            }
            merged
        }
    }
}

/// `key`'s value in the subtree `node` at `depth`.
fn lookup<'a, K: Eq + Hash, V>(node: &'a Rc<Node<K, V>>, depth: usize, key: &K) -> Option<&'a V> {
    let hash = hash_of(key);
    let mut node: &Node<K, V> = node;
    let mut depth = depth;
    loop {
        match node {
            Node::Leaf {
                hash: leaf,
                entries,
            } => {
                return if *leaf == hash {
                    entries
                        .iter()
                        .find(|(entry, _)| entry == key)
                        .map(|(_, value)| value)
                } else {
                    None
                };
            }
            Node::Branch { left, right } => {
                let next = if bit(hash, depth) { right } else { left };
                node = next.as_deref()?;
                depth += 1;
            }
        }
    }
}

impl<K, V> PersistentMap<K, V> {
    /// The trie nodes `maps` hold between them, a shared node counted once.
    #[cfg(test)]
    pub(super) fn distinct_nodes(maps: &[&Self]) -> usize {
        let mut seen = std::collections::HashSet::new();
        let mut pending: Vec<&Rc<Node<K, V>>> =
            maps.iter().filter_map(|map| map.root.as_ref()).collect();
        while let Some(node) = pending.pop() {
            if !seen.insert(Rc::as_ptr(node)) {
                continue;
            }
            if let Node::Branch { left, right } = &**node {
                pending.extend(left.iter().chain(right.iter()));
            }
        }
        seen.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inserting changes one path: a thousand versions of a growing map hold
    /// a number of nodes linear in the entries, not quadratic.
    #[test]
    fn every_version_of_a_growing_map_shares_its_nodes() {
        let mut map: PersistentMap<u32, u32> = PersistentMap::default();
        let mut versions = Vec::new();
        for key in 0..2_000_u32 {
            map.insert(key, key * 2);
            versions.push(map.clone());
        }
        let refs: Vec<&PersistentMap<u32, u32>> = versions.iter().collect();
        let nodes = PersistentMap::distinct_nodes(&refs);
        // 2,000 versions of up to 2,000 entries are 2,000,000 entries as
        // copies; shared, each insert adds one path.
        assert!(nodes < 2_000 * 40, "{nodes} nodes");
        assert_eq!(versions[999].get(&999), Some(&1_998));
        assert_eq!(versions[999].get(&1_000), None);
    }

    #[test]
    fn insert_replaces_and_clone_is_independent() {
        let mut map: PersistentMap<&str, u32> = PersistentMap::default();
        map.insert("a", 1);
        let before = map.clone();
        map.insert("a", 2);
        map.insert("b", 3);
        assert_eq!(before.get(&"a"), Some(&1));
        assert!(!before.contains(&"b"));
        assert_eq!(map.get(&"a"), Some(&2));
    }

    #[test]
    fn intersect_keeps_common_keys_combined_and_shares_what_it_can() {
        let mut base: PersistentMap<u32, u32> = PersistentMap::default();
        for key in 0..100 {
            base.insert(key, key);
        }
        let mut left = base.clone();
        left.insert(1_000, 1);
        left.insert(5, 50);
        let mut right = base.clone();
        right.insert(2_000, 2);
        right.insert(5, 20);
        let joined = left.intersect_with(&right, &|a, b| Some(*a.max(b)));
        assert_eq!(joined.get(&5), Some(&50));
        assert_eq!(joined.get(&7), Some(&7));
        assert!(!joined.contains(&1_000) && !joined.contains(&2_000));
    }
}
