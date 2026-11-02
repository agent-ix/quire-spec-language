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
//!
//! The table is passed explicitly. A type that holds a [`NodeKey`] implements
//! [`DebugWith`], receives the [`DebugTable`] of the formatting call, and
//! writes each key through [`NodeKeyDebug`]; a container passes the same table
//! on to each [`DebugWith`] value it holds, through [`With`]. The outermost
//! value to format opens one table with [`scoped`], so a node shared by many
//! collections of one package is written once for the whole package. No state
//! lives outside the formatting call: there is no thread-local, no `static`
//! and no registry. A value formatted on its own, outside any [`scoped`]
//! call, writes its keys in full. Without `std` there is no table and a key is
//! written in full everywhere. Keys stay complete as data in every case; only
//! this debug text is shared.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::{string::String, vec::Vec};
use core::cell::RefCell;
use core::fmt;
use core::hash::{Hash, Hasher};

use quire_exact::{NodeKey, PathSegment};

/// The node table of one formatting call, or the marker that keys are written
/// in full.
pub struct DebugTable {
    table: RefCell<NodeTable>,
    shared: bool,
}

impl DebugTable {
    /// A table that every key written through it joins.
    pub fn shared() -> Self {
        Self {
            table: RefCell::new(NodeTable::new()),
            shared: true,
        }
    }

    /// A marker that writes every key in full.
    pub fn unshared() -> Self {
        Self {
            table: RefCell::new(NodeTable::new()),
            shared: false,
        }
    }

    /// The table of a [`scoped`] call: shared with `std`, otherwise a marker
    /// that writes every key in full.
    fn for_scope() -> Self {
        if cfg!(feature = "std") {
            Self::shared()
        } else {
            Self::unshared()
        }
    }

    /// The contents of the table, as a debug value.
    fn contents(&self) -> TableContents<'_> {
        TableContents(self)
    }
}

struct TableContents<'a>(&'a DebugTable);

impl fmt::Debug for TableContents<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.table.borrow().nodes().fmt(f)
    }
}

/// A value whose debug text writes its node keys into the formatting call's
/// [`DebugTable`].
pub trait DebugWith {
    /// Write `self`, with each node key joining `table`.
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, table: &DebugTable) -> fmt::Result;
}

impl<T: DebugWith + ?Sized> DebugWith for &T {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, table: &DebugTable) -> fmt::Result {
        (**self).fmt_with(f, table)
    }
}

impl<T: DebugWith + ?Sized> DebugWith for Box<T> {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, table: &DebugTable) -> fmt::Result {
        (**self).fmt_with(f, table)
    }
}

impl<T: DebugWith + ?Sized> DebugWith for alloc::sync::Arc<T> {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, table: &DebugTable) -> fmt::Result {
        (**self).fmt_with(f, table)
    }
}

impl<T: DebugWith> DebugWith for [T] {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, table: &DebugTable) -> fmt::Result {
        f.debug_list()
            .entries(self.iter().map(|value| With(value, table)))
            .finish()
    }
}

impl<T: DebugWith> DebugWith for Vec<T> {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, table: &DebugTable) -> fmt::Result {
        self.as_slice().fmt_with(f, table)
    }
}

impl<T: DebugWith> DebugWith for Option<T> {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, table: &DebugTable) -> fmt::Result {
        fmt::Debug::fmt(&self.as_ref().map(|value| With(value, table)), f)
    }
}

impl<A: DebugWith, B: DebugWith> DebugWith for (A, B) {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, table: &DebugTable) -> fmt::Result {
        f.debug_tuple("")
            .field(&With(&self.0, table))
            .field(&With(&self.1, table))
            .finish()
    }
}

impl<A: DebugWith, B: DebugWith, C: DebugWith> DebugWith for (A, B, C) {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, table: &DebugTable) -> fmt::Result {
        f.debug_tuple("")
            .field(&With(&self.0, table))
            .field(&With(&self.1, table))
            .field(&With(&self.2, table))
            .finish()
    }
}

/// A [`DebugWith`] value paired with the table it writes into, as a plain
/// [`fmt::Debug`] value.
pub struct With<'a, T: ?Sized>(pub &'a T, pub &'a DebugTable);

impl<T: DebugWith + ?Sized> fmt::Debug for With<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt_with(f, self.1)
    }
}

/// Format `value` with one [`DebugTable`] for every key under it. With `std`
/// this writes `WithNodes { value: .., nodes: {..} }`; without `std` it writes
/// `value` with every key in full.
pub fn scoped(f: &mut fmt::Formatter<'_>, value: &dyn DebugWith) -> fmt::Result {
    let table = DebugTable::for_scope();
    if !table.shared {
        return value.fmt_with(f, &table);
    }
    f.debug_struct("WithNodes")
        .field("value", &With(value, &table))
        .field("nodes", &table.contents())
        .finish()
}

/// Format `value` on its own: every key is written in full.
pub fn standalone<T: DebugWith + ?Sized>(f: &mut fmt::Formatter<'_>, value: &T) -> fmt::Result {
    value.fmt_with(f, &DebugTable::unshared())
}

/// A [`NodeKey`] written as its position in the table it is paired with, or
/// in full when that table is not shared.
pub struct NodeKeyDebug<'a>(pub &'a NodeKey, pub &'a DebugTable);

impl fmt::Debug for NodeKeyDebug<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.1.shared {
            let at = self.1.table.borrow_mut().intern(self.0);
            at.fmt(f)
        } else {
            self.0.fmt(f)
        }
    }
}

/// A map keyed by node identities whose values are [`DebugWith`]: each key is
/// written as its position and each value into the same table. `I` is the
/// map's `iter()`.
pub struct NodeValueMap<'t, I>(pub I, pub &'t DebugTable);

impl<'a, I, V> fmt::Debug for NodeValueMap<'_, I>
where
    I: Iterator<Item = (&'a NodeKey, &'a V)> + Clone,
    V: DebugWith + 'a,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(
                self.0
                    .clone()
                    .map(|(key, value)| (NodeKeyDebug(key, self.1), With(value, self.1))),
            )
            .finish()
    }
}

/// A slice of [`DebugWith`] values, each written into the table it is paired
/// with.
pub struct Seq<'a, T>(pub &'a [T], pub &'a DebugTable);

impl<T: DebugWith> fmt::Debug for Seq<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(|value| With(value, self.1)))
            .finish()
    }
}

/// A map whose values are [`DebugWith`], written into the table they are
/// paired with; the keys are written as ordinary debug values. `I` is the
/// map's `iter()`.
pub struct ValueMap<'t, I>(pub I, pub &'t DebugTable);

impl<'a, I, K, V> fmt::Debug for ValueMap<'_, I>
where
    I: Iterator<Item = (K, &'a V)> + Clone,
    K: fmt::Debug,
    V: DebugWith + 'a,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(
                self.0
                    .clone()
                    .map(|(key, value)| (key, With(value, self.1))),
            )
            .finish()
    }
}

/// An optional [`DebugWith`] value, written into the table it is paired with.
pub struct Maybe<'a, T>(pub &'a Option<T>, pub &'a DebugTable);

impl<T: DebugWith> fmt::Debug for Maybe<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.as_ref().map(|value| With(value, self.1)).fmt(f)
    }
}

/// A map keyed by node identities, each key written through [`NodeKeyDebug`].
/// `I` is the map's `iter()`.
pub struct NodeMap<'t, I>(pub I, pub &'t DebugTable);

impl<'a, I, V> fmt::Debug for NodeMap<'_, I>
where
    I: Iterator<Item = (&'a NodeKey, V)> + Clone,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(
                self.0
                    .clone()
                    .map(|(key, value)| (NodeKeyDebug(key, self.1), value)),
            )
            .finish()
    }
}

/// A sequence or set of node identities, each written through
/// [`NodeKeyDebug`]. `I` is the collection's `iter()`.
pub struct NodeList<'t, I>(pub I, pub &'t DebugTable);

impl<'a, I> fmt::Debug for NodeList<'_, I>
where
    I: Iterator<Item = &'a NodeKey> + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.0.clone().map(|key| NodeKeyDebug(key, self.1)))
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

/// Define [`DebugWith`] for a struct from its field list, and `Debug` for it
/// as a value formatted on its own (every key in full). Each field is `plain`
/// (written as an ordinary debug value) or `with` (a [`DebugWith`] value
/// written into the same table).
#[macro_export]
macro_rules! debug_with_struct {
    ($ty:ty, $name:literal { $($field:ident : $kind:ident),* $(,)? }) => {
        impl $crate::source::node_table::DebugWith for $ty {
            fn fmt_with(
                &self,
                f: &mut ::core::fmt::Formatter<'_>,
                table: &$crate::source::node_table::DebugTable,
            ) -> ::core::fmt::Result {
                let mut out = f.debug_struct($name);
                $( $crate::debug_with_struct!(@field out, self, table, $field, $kind); )*
                out.finish()
            }
        }

        impl ::core::fmt::Debug for $ty {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                $crate::source::node_table::standalone(f, self)
            }
        }
    };
    (@field $out:ident, $this:ident, $table:ident, $field:ident, plain) => {
        $out.field(stringify!($field), &$this.$field);
    };
    (@field $out:ident, $this:ident, $table:ident, $field:ident, values) => {
        $out.field(
            stringify!($field),
            &$crate::source::node_table::ValueMap($this.$field.iter(), $table),
        );
    };
    (@field $out:ident, $this:ident, $table:ident, $field:ident, with) => {
        $out.field(
            stringify!($field),
            &$crate::source::node_table::With(&$this.$field, $table),
        );
    };
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

    impl DebugWith for Keys {
        fn fmt_with(&self, f: &mut fmt::Formatter<'_>, table: &DebugTable) -> fmt::Result {
            f.debug_tuple("")
                .field(&NodeKeyDebug(&self.0, table))
                .field(&NodeKeyDebug(&self.1, table))
                .finish()
        }
    }

    impl fmt::Debug for Keys {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            scoped(f, self)
        }
    }

    /// With `std`, the outermost `scoped` call writes one table and each key as
    /// its position in it; without `std` every key is written in full.
    #[test]
    fn scoped_writes_one_table_and_keys_by_position_with_std() {
        let key = NodeKey::new("pkg", vec![PathSegment::Text("k".into())], 1).unwrap();
        let text = alloc::format!("{:?}", Keys(key.clone(), key));
        if cfg!(feature = "std") {
            assert_eq!(
                text,
                "WithNodes { value: (#0, #0), nodes: {#0: pkg:[\"k\",]:1} }"
            );
        } else {
            assert_eq!(text, "(NodeKey(pkg:[\"k\",]:1), NodeKey(pkg:[\"k\",]:1))");
        }
    }

    /// A value formatted outside any `scoped` call keeps complete keys.
    #[test]
    fn a_standalone_value_writes_complete_keys() {
        let key = NodeKey::new("pkg", vec![PathSegment::Text("k".into())], 1).unwrap();
        let keys = Keys(key.clone(), key);
        struct Alone<'a>(&'a Keys);
        impl fmt::Debug for Alone<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                standalone(f, self.0)
            }
        }
        assert_eq!(
            alloc::format!("{:?}", Alone(&keys)),
            "(NodeKey(pkg:[\"k\",]:1), NodeKey(pkg:[\"k\",]:1))"
        );
    }

    /// A chain of nested nodes written as a collection's keys is linear in the
    /// chain's depth, not quadratic: each doubling at most doubles the text.
    #[cfg(feature = "std")]
    #[test]
    fn a_node_list_of_a_deep_chain_grows_linearly_with_depth() {
        struct Chain(Vec<NodeKey>);
        impl DebugWith for Chain {
            fn fmt_with(&self, f: &mut fmt::Formatter<'_>, table: &DebugTable) -> fmt::Result {
                fmt::Debug::fmt(&NodeList(self.0.iter(), table), f)
            }
        }
        impl fmt::Debug for Chain {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                scoped(f, self)
            }
        }
        let size = |depth: u64| {
            let mut keys = Vec::new();
            let mut key = NodeKey::new("pkg", vec![PathSegment::Text("leaf".into())], 0).unwrap();
            for ordinal in 0..depth {
                keys.push(key.clone());
                key = NodeKey::new(
                    "pkg",
                    vec![PathSegment::Text("add".into()), PathSegment::Node(key)],
                    ordinal,
                )
                .unwrap();
            }
            keys.push(key);
            alloc::format!("{:?}", Chain(keys)).len()
        };
        let (small, middle, large) = (size(250), size(500), size(1_000));
        assert!(middle < small * 5 / 2, "{small} -> {middle}");
        assert!(large < middle * 5 / 2, "{middle} -> {large}");
    }
}
