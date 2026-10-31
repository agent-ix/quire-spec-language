// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sorting by [`NodeKey`] order in time independent of how deep keys nest.
//!
//! `NodeKey`'s order is the lexicographic order of its token stream:
//! package, then each path segment (a text, or a nested key's own stream,
//! with text before a nested key and a shorter path first), then the
//! ordinal. A comparison walks two streams until they differ, so two keys
//! that share a deep chain of nested operands cost that chain's depth, and
//! a comparison sort of `n` keys in a chain `d` deep costs `n·d·log n`
//! (QSL-381: depth is unbounded).
//!
//! [`sorted_by_key`] instead ranks every distinct key reachable from the
//! input once, children before parents. A key's place among ranked keys is
//! decided by its own package, texts and ordinal and by its children's
//! ranks, so ranking one key costs its own width times a logarithmic rank
//! comparison, never its depth. The order it produces is exactly
//! `NodeKey::cmp`'s.

use std::cmp::Ordering;
use std::collections::HashMap;

use quire_exact::{NodeKey, PathSegment, PathSegmentView};

/// `items` ascending by `key` in `NodeKey` order; items with equal keys keep
/// their relative order.
pub(crate) fn sorted_by_key<T>(items: Vec<T>, key: impl Fn(&T) -> &NodeKey) -> Vec<T> {
    if items.len() < 2 {
        return items;
    }
    let mut ranking = Ranking::new();
    let classes: Vec<usize> = items.iter().map(|item| ranking.class(key(item))).collect();
    let positions = ranking.positions();
    let mut keyed: Vec<(usize, T)> = classes
        .into_iter()
        .map(|class| positions[class])
        .zip(items)
        .collect();
    keyed.sort_by_key(|(position, _)| *position);
    keyed.into_iter().map(|(_, item)| item).collect()
}

/// No tree link.
const NIL: usize = usize::MAX;

/// One distinct key reached: the key, and the shape id of each nested key in
/// path order.
struct Shape {
    key: NodeKey,
    children: Vec<usize>,
}

/// The distinct keys reached so far, each in an equality class, and the
/// classes in a treap ordered by `NodeKey` order. A class's in-order
/// position is its rank; two classes are ordered by walking each to the root.
struct Ranking {
    ids: HashMap<NodeKey, usize>,
    shapes: Vec<Shape>,
    /// Each shape's class.
    class_of: Vec<usize>,
    /// Each class's representative shape.
    representative: Vec<usize>,
    left: Vec<usize>,
    right: Vec<usize>,
    parent: Vec<usize>,
    size: Vec<usize>,
    priority: Vec<u64>,
    root: usize,
}

/// One key being reached: its nested keys, and how many are done.
struct Frame {
    key: NodeKey,
    children: Vec<NodeKey>,
    next: usize,
}

impl Ranking {
    fn new() -> Self {
        Self {
            ids: HashMap::new(),
            shapes: Vec::new(),
            class_of: Vec::new(),
            representative: Vec::new(),
            left: Vec::new(),
            right: Vec::new(),
            parent: Vec::new(),
            size: Vec::new(),
            priority: Vec::new(),
            root: NIL,
        }
    }

    /// The class of `key`, ranking it and every nested key not reached yet.
    /// The walk is an explicit stack: nesting depth is unbounded.
    fn class(&mut self, key: &NodeKey) -> usize {
        if let Some(&id) = self.ids.get(key) {
            return self.class_of[id];
        }
        let mut stack = vec![Frame::new(key.clone())];
        while let Some(frame) = stack.last_mut() {
            if let Some(child) = frame.children.get(frame.next) {
                frame.next += 1;
                if !self.ids.contains_key(child) {
                    let child = child.clone();
                    stack.push(Frame::new(child));
                }
                continue;
            }
            let Some(frame) = stack.pop() else {
                break;
            };
            // Every child was reached before its parent is popped.
            let children = frame.children.iter().map(|child| self.ids[child]).collect();
            let id = self.shapes.len();
            self.shapes.push(Shape {
                key: frame.key.clone(),
                children,
            });
            self.ids.insert(frame.key, id);
            let class = self.insert(id);
            self.class_of.push(class);
        }
        self.class_of[self.ids[key]]
    }

    /// Place shape `id`, whose children are all placed, in the treap: the
    /// class of an equal key already there, or a new class.
    fn insert(&mut self, id: usize) -> usize {
        let mut parent = NIL;
        let mut went_left = false;
        let mut current = self.root;
        while current != NIL {
            match self.compare(id, self.representative[current]) {
                Ordering::Equal => return current,
                Ordering::Less => {
                    parent = current;
                    went_left = true;
                    current = self.left[current];
                }
                Ordering::Greater => {
                    parent = current;
                    went_left = false;
                    current = self.right[current];
                }
            }
        }
        let class = self.representative.len();
        self.representative.push(id);
        self.left.push(NIL);
        self.right.push(NIL);
        self.parent.push(parent);
        self.size.push(1);
        self.priority.push(mix(class as u64));
        if parent == NIL {
            self.root = class;
        } else if went_left {
            self.left[parent] = class;
        } else {
            self.right[parent] = class;
        }
        let mut above = parent;
        while above != NIL {
            self.size[above] += 1;
            above = self.parent[above];
        }
        while self.parent[class] != NIL && self.priority[class] > self.priority[self.parent[class]]
        {
            self.rotate_up(class);
        }
        class
    }

    /// Rotate `node` above its parent, keeping in-order and subtree sizes.
    fn rotate_up(&mut self, node: usize) {
        let parent = self.parent[node];
        let grandparent = self.parent[parent];
        if self.left[parent] == node {
            let moved = self.right[node];
            self.left[parent] = moved;
            if moved != NIL {
                self.parent[moved] = parent;
            }
            self.right[node] = parent;
        } else {
            let moved = self.left[node];
            self.right[parent] = moved;
            if moved != NIL {
                self.parent[moved] = parent;
            }
            self.left[node] = parent;
        }
        self.parent[parent] = node;
        self.parent[node] = grandparent;
        if grandparent == NIL {
            self.root = node;
        } else if self.left[grandparent] == parent {
            self.left[grandparent] = node;
        } else {
            self.right[grandparent] = node;
        }
        self.size[parent] = self.subtree(self.left[parent]) + self.subtree(self.right[parent]) + 1;
        self.size[node] = self.subtree(self.left[node]) + self.subtree(self.right[node]) + 1;
    }

    fn subtree(&self, node: usize) -> usize {
        if node == NIL {
            0
        } else {
            self.size[node]
        }
    }

    /// The in-order position of class `node`.
    fn rank(&self, mut node: usize) -> usize {
        let mut rank = self.subtree(self.left[node]);
        while self.parent[node] != NIL {
            let parent = self.parent[node];
            if self.right[parent] == node {
                rank += self.subtree(self.left[parent]) + 1;
            }
            node = parent;
        }
        rank
    }

    /// Shape `id` against shape `other` in `NodeKey` order, both of whose
    /// children are placed: nested keys compare by their classes' ranks.
    fn compare(&self, id: usize, other: usize) -> Ordering {
        let (left, right) = (&self.shapes[id], &self.shapes[other]);
        let (left_view, right_view) = (left.key.view(), right.key.view());
        let order = left_view.package().cmp(right_view.package());
        if !order.is_eq() {
            return order;
        }
        let mut left_children = left.children.iter();
        let mut right_children = right.children.iter();
        let mut left_path = left_view.path();
        let mut right_path = right_view.path();
        loop {
            let order = match (left_path.next(), right_path.next()) {
                (None, None) => break,
                (None, Some(_)) => Ordering::Less,
                (Some(_), None) => Ordering::Greater,
                (Some(PathSegmentView::Text(a)), Some(PathSegmentView::Text(b))) => a.cmp(b),
                (Some(PathSegmentView::Text(_)), Some(PathSegmentView::Node(_))) => Ordering::Less,
                (Some(PathSegmentView::Node(_)), Some(PathSegmentView::Text(_))) => {
                    Ordering::Greater
                }
                (Some(PathSegmentView::Node(_)), Some(PathSegmentView::Node(_))) => {
                    match (left_children.next(), right_children.next()) {
                        (Some(&a), Some(&b)) => self.compare_classes(a, b),
                        _ => Ordering::Equal,
                    }
                }
            };
            if !order.is_eq() {
                return order;
            }
        }
        left_view.ordinal().cmp(&right_view.ordinal())
    }

    /// Placed shapes `a` and `b` by their classes' ranks.
    fn compare_classes(&self, a: usize, b: usize) -> Ordering {
        let (a, b) = (self.class_of[a], self.class_of[b]);
        if a == b {
            Ordering::Equal
        } else {
            self.rank(a).cmp(&self.rank(b))
        }
    }

    /// Every class's rank, by an in-order walk.
    fn positions(&self) -> Vec<usize> {
        let mut positions = vec![0; self.representative.len()];
        let mut stack = Vec::new();
        let mut current = self.root;
        let mut next = 0;
        while current != NIL || !stack.is_empty() {
            while current != NIL {
                stack.push(current);
                current = self.left[current];
            }
            let Some(node) = stack.pop() else {
                break;
            };
            positions[node] = next;
            next += 1;
            current = self.right[node];
        }
        positions
    }
}

impl Frame {
    fn new(key: NodeKey) -> Self {
        let children = key
            .path()
            .into_iter()
            .filter_map(|segment| match segment {
                PathSegment::Node(child) => Some(child),
                PathSegment::Text(_) => None,
            })
            .collect();
        Self {
            key,
            children,
            next: 0,
        }
    }
}

/// A treap priority: SplitMix64 of the class index, deterministic and
/// spread so the treap's expected depth is logarithmic.
fn mix(value: u64) -> u64 {
    let mut z = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> PathSegment {
        PathSegment::Text(value.to_owned())
    }

    /// A small deterministic generator: keys over two packages, short texts,
    /// two ordinals and nested earlier keys, so equal, prefix and nested
    /// orderings all occur.
    fn keys(count: usize) -> Vec<NodeKey> {
        let mut state = 0x1234_5678_u64;
        let mut next = move |bound: usize| {
            state = mix(state);
            usize::try_from(state % bound as u64).unwrap()
        };
        let mut keys: Vec<NodeKey> = Vec::new();
        for _ in 0..count {
            let width = 1 + next(3);
            let mut path = Vec::new();
            for _ in 0..width {
                if !keys.is_empty() && next(2) == 0 {
                    let pick = next(keys.len());
                    path.push(PathSegment::Node(keys[pick].clone()));
                } else {
                    path.push(text(["a", "b", "ab", ""][next(4)]));
                }
            }
            let package = ["p", "q"][next(2)];
            keys.push(NodeKey::new(package, path, next(2) as u64).unwrap());
        }
        keys
    }

    /// The ranking orders keys exactly as `NodeKey::cmp` does, equal keys
    /// built apart included.
    #[test]
    fn ranks_agree_with_node_key_order() {
        let mut input = keys(400);
        // Rebuild some keys independently so equal keys need not share storage.
        let rebuilt: Vec<NodeKey> = input
            .iter()
            .step_by(7)
            .map(|key| key.try_clone().unwrap())
            .collect();
        input.extend(rebuilt);
        input.reverse();
        let mut expected = input.clone();
        expected.sort();
        let actual = sorted_by_key(input, |key| key);
        assert_eq!(actual, expected);
        for pair in actual.windows(2) {
            assert!(pair[0] <= pair[1]);
        }
    }

    /// Equal keys keep their input order.
    #[test]
    fn sorting_is_stable() {
        let a = NodeKey::new("p", vec![text("a")], 0).unwrap();
        let b = NodeKey::new("p", vec![text("b")], 0).unwrap();
        let items = vec![
            (b.clone(), 0),
            (a.clone(), 1),
            (b.clone(), 2),
            (a.try_clone().unwrap(), 3),
        ];
        let order: Vec<i32> = sorted_by_key(items, |(key, _)| key)
            .into_iter()
            .map(|(_, index)| index)
            .collect();
        assert_eq!(order, vec![1, 3, 0, 2]);
    }

    /// A chain 50,000 keys deep sorts as `NodeKey::cmp` orders it, from
    /// either end.
    #[test]
    fn a_deep_chain_sorts_by_rank() {
        let mut chain = vec![NodeKey::new("p", vec![text("leaf")], 0).unwrap()];
        for _ in 0..50_000 {
            let inner = chain.last().cloned().unwrap();
            chain.push(
                NodeKey::new("p", vec![text("option"), PathSegment::Node(inner)], 0).unwrap(),
            );
        }
        let positions: Vec<usize> = (0..chain.len()).collect();
        let reversed: Vec<(NodeKey, usize)> = chain.iter().cloned().zip(positions).rev().collect();
        let sorted = sorted_by_key(reversed, |(key, _)| key);
        // `leaf` sorts before `option`, so each wrapper follows its operand.
        assert!(sorted.iter().map(|(_, at)| *at).eq(0..chain.len()));
        assert!(chain[0] < chain[1] && chain[1] < chain[2]);
    }
}
