// SPDX-License-Identifier: AGPL-3.0-or-later
//! Debug helpers for structural node identities.
//!
//! A [`NodeKey`] nests the identities of its operands, so its debug output
//! includes the complete subtree. [`NodeTable`] remains available to callers
//! that explicitly want to intern nodes for a compact diagnostic. The
//! [`NodeKeyDebug`], [`NodeMap`] and [`NodeList`] wrappers preserve complete
//! keys in ordinary `Debug` output and do not use ambient state.

use alloc::collections::BTreeMap;
use alloc::{string::String, vec::Vec};
use core::fmt;
use core::hash::{Hash, Hasher};

use quire_exact::{NodeKey, PathSegment};

/// Format `value` through the compatibility scope wrapper. It deliberately
/// carries no hidden per-thread context; keys remain complete in every build.
pub fn scoped(f: &mut fmt::Formatter<'_>, value: &dyn fmt::Debug) -> fmt::Result {
    value.fmt(f)
}

/// A key written into an explicitly supplied local table, or in full when
/// the caller does not supply one.
pub(super) struct NodeKeyDebugScoped<'a>(
    pub &'a NodeKey,
    pub Option<&'a core::cell::RefCell<NodeTable>>,
);

impl fmt::Debug for NodeKeyDebugScoped<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.1 {
            Some(table) => table.borrow_mut().intern(self.0).fmt(f),
            None => self.0.fmt(f),
        }
    }
}

/// The contents of an explicitly supplied local node table.
pub(super) struct NodeTableContents<'a>(pub &'a core::cell::RefCell<NodeTable>);

impl fmt::Debug for NodeTableContents<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.borrow().nodes().fmt(f)
    }
}

/// A [`NodeKey`] written in full through its ordinary `Debug` representation.
pub struct NodeKeyDebug<'a>(pub &'a NodeKey);

impl fmt::Debug for NodeKeyDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A map keyed by node identities, each key written through [`NodeKeyDebug`].
/// `I` is the map's `iter()`.
pub struct NodeMap<I>(pub I);

impl<'a, I, V> fmt::Debug for NodeMap<I>
where
    I: Iterator<Item = (&'a NodeKey, V)> + Clone,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(
                self.0
                    .clone()
                    .map(|(key, value)| (NodeKeyDebug(key), value)),
            )
            .finish()
    }
}

/// A sequence or set of node identities, each written through
/// [`NodeKeyDebug`]. `I` is the collection's `iter()`.
pub struct NodeList<I>(pub I);

impl<'a, I> fmt::Debug for NodeList<I>
where
    I: Iterator<Item = &'a NodeKey> + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.0.clone().map(NodeKeyDebug))
            .finish()
    }
}

/// The position of one node in a [`NodeTable`], written `#n`.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct NodeRef(usize);

impl fmt::Debug for NodeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

enum Segment {
    Text(String),
    Node(NodeRef),
}

/// Every distinct node reachable from the keys interned into it, with its
/// direct segments.
#[derive(Default)]
pub struct NodeTable {
    nodes: Vec<NodeKey>,
    segments: Vec<Vec<Segment>>,
    /// Node positions by the key's structural hash; a bucket holds the
    /// positions whose keys share that hash.
    by_hash: BTreeMap<u64, Vec<usize>>,
}

/// Reads the `u64` a [`NodeKey`] hashes to.
struct PassThrough(u64);

impl Hasher for PassThrough {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 << 8) | u64::from(*byte);
        }
    }
}

/// The cached hash used to order debug rows before structural collision fallback.
pub(super) fn structural_hash(key: &NodeKey) -> u64 {
    let mut hasher = PassThrough(0);
    key.hash(&mut hasher);
    hasher.finish()
}

impl NodeTable {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// The position of `key`, adding it and every node nested in it when not
    /// yet present. Walks with an explicit work list, so depth does not grow
    /// the call stack.
    pub fn intern(&mut self, key: &NodeKey) -> NodeRef {
        let mut pending = Vec::new();
        let root = self.position(key, &mut pending);
        while let Some(at) = pending.pop() {
            let node = self.nodes[at].clone();
            let mut direct = Vec::new();
            for segment in node.path() {
                direct.push(match segment {
                    PathSegment::Text(text) => Segment::Text(text),
                    PathSegment::Node(child) => {
                        Segment::Node(NodeRef(self.position(&child, &mut pending)))
                    }
                });
            }
            self.segments[at] = direct;
        }
        NodeRef(root)
    }

    fn position(&mut self, key: &NodeKey, pending: &mut Vec<usize>) -> usize {
        let bucket = self.by_hash.entry(structural_hash(key)).or_default();
        if let Some(at) = bucket.iter().copied().find(|at| self.nodes[*at] == *key) {
            return at;
        }
        let at = self.nodes.len();
        bucket.push(at);
        self.nodes.push(key.clone());
        self.segments.push(Vec::new());
        pending.push(at);
        at
    }

    /// The table as a debug value: each node's position and its own package,
    /// segments and ordinal, nested nodes named by position.
    pub fn nodes(&self) -> impl fmt::Debug + '_ {
        Nodes(self)
    }
}

struct Nodes<'a>(&'a NodeTable);

impl fmt::Debug for Nodes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();
        for (at, node) in self.0.nodes.iter().enumerate() {
            map.entry(&NodeRef(at), &Row(node, &self.0.segments[at]));
        }
        map.finish()
    }
}

struct Row<'a>(&'a NodeKey, &'a [Segment]);

impl fmt::Debug for Row<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:[", self.0.package())?;
        for segment in self.1 {
            match segment {
                Segment::Text(text) => write!(f, "{text:?},")?,
                Segment::Node(node) => write!(f, "{node:?},")?,
            }
        }
        write!(f, "]:{}", self.0.ordinal())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// A hash bucket only narrows candidates; distinct complete identities
    /// remain distinct even when both candidates occupy the same bucket.
    #[test]
    fn a_hash_collision_keeps_distinct_nodes_and_all_their_segments() {
        let first = NodeKey::new("pkg", vec![PathSegment::Text("first".into())], 7).unwrap();
        let second = NodeKey::new("pkg", vec![PathSegment::Text("second".into())], 9).unwrap();
        let mut table = NodeTable::new();
        let first_ref = table.intern(&first);
        table
            .by_hash
            .entry(structural_hash(&second))
            .or_default()
            .push(first_ref.0);
        let second_ref = table.intern(&second);
        assert_ne!(first_ref.0, second_ref.0);
        assert_eq!(table.intern(&first).0, first_ref.0);
        assert_eq!(table.intern(&second).0, second_ref.0);
        assert_eq!(
            alloc::format!("{:?}", table.nodes()),
            "{#0: pkg:[\"first\",]:7, #1: pkg:[\"second\",]:9}"
        );
    }

    struct Keys(NodeKey, NodeKey);

    impl fmt::Debug for Keys {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            scoped(f, &(NodeKeyDebug(&self.0), NodeKeyDebug(&self.1)))
        }
    }

    /// The scope wrapper preserves complete identities and has no ambient
    /// state, regardless of the crate's feature set.
    #[test]
    fn scoped_preserves_complete_keys_without_ambient_state() {
        let key = NodeKey::new("pkg", vec![PathSegment::Text("k".into())], 1).unwrap();
        let text = alloc::format!("{:?}", Keys(key.clone(), key));
        assert_eq!(text, "(NodeKey(pkg:[\"k\",]:1), NodeKey(pkg:[\"k\",]:1))");
    }

    /// An explicitly shared local table interns repeated keys without a
    /// process- or thread-wide registry.
    #[test]
    fn explicit_table_shares_repeated_keys() {
        let key = NodeKey::new("pkg", vec![PathSegment::Text("k".into())], 1).unwrap();
        let table = core::cell::RefCell::new(NodeTable::new());
        let text = alloc::format!(
            "{:?}",
            (
                NodeKeyDebugScoped(&key, Some(&table)),
                NodeKeyDebugScoped(&key, Some(&table))
            )
        );
        assert_eq!(text, "(#0, #0)");
        assert_eq!(
            alloc::format!("{:?}", NodeTableContents(&table)),
            "{#0: pkg:[\"k\",]:1}"
        );
    }
}
