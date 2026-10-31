// SPDX-License-Identifier: AGPL-3.0-or-later
//! A debug-only table of structural node identities, written once each.
//!
//! A [`NodeKey`] nests the identities of its operands, so its own `Display`
//! writes a whole subtree. A collection keyed by node identities, formatted
//! key by key, therefore writes every shared subtree once per key that
//! contains it, which is quadratic in package depth. [`NodeTable`] writes
//! each distinct node once, naming its own text segments and referring to
//! each nested node by a table position ([`NodeRef`]); a collection's entries
//! then print only a [`NodeRef`]. The output is complete (no depth or size
//! cap) and linear in the number of distinct nodes and their direct segments.

use alloc::collections::BTreeMap;
use alloc::{string::String, vec::Vec};
use core::fmt;
use core::hash::{Hash, Hasher};

use quire_exact::{NodeKey, PathSegment};

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
        let mut hasher = PassThrough(0);
        key.hash(&mut hasher);
        let bucket = self.by_hash.entry(hasher.finish()).or_default();
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
