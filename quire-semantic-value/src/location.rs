// SPDX-License-Identifier: AGPL-3.0-or-later
//! Expression locations (ADR-011 §6.1 layer SV): the declaration an
//! expression belongs to ([`Origin`]) and the child-index path from that
//! declaration's root expression ([`Location`]). A checking refusal, an
//! evaluation outcome and a loss record are all located this way.
//!
//! These are not `quire_exact::{Origin, Location}`: those name a checked
//! node's source occurrence (ADR-013 O-07/T-5). These name a position
//! inside a package's expression trees.

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};

/// The declaration a location belongs to.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Origin {
    /// The body of the named function, at this zero-based declaration index.
    Body {
        /// The declared name.
        function: String,
        /// The declaration index in the package.
        index: usize,
    },
    /// The `decreases` measure of the named function.
    Measure {
        /// The declared name.
        function: String,
        /// The declaration index in the package.
        index: usize,
    },
    /// A standalone checked expression.
    Expression,
    /// A declared record, tuple, enum, dimension or unit type, by its
    /// declared name (a dimension's or unit's qualified name joined by
    /// `.`). Its
    /// `declaration` occurrence is located here (FR-322), and so is the
    /// `generated` occurrence of a node that no function body, measure,
    /// state clause or protocol attempt reaches, when this is the least
    /// declared name that reaches it (`lowering::enclosing_declarations`). It
    /// names the declared name's region when the FR-091 assembler read the
    /// name from the unit, and no region for a type declared by hand
    /// (FR-096).
    TypeDeclaration {
        /// The declared name.
        name: String,
    },
    /// The body of the named state clause (FR-104), at this zero-based
    /// index among the unit's state clauses in source order. The clause's
    /// `claim` occurrence is located at its root.
    StateClause {
        /// The declared clause name.
        clause: String,
        /// The clause's index among the package's state clauses.
        index: usize,
    },
    /// One protocol `attempt`'s own operation binding (FR-114), by
    /// the protocol's index among the package's protocols and the
    /// attempt's index among that protocol's own `attempts`, both in
    /// source order. An `operation_anchor`/`frame` node names no position
    /// of its own, so this names a real position of the unit (the
    /// attempt's own declared name) to resolve its generated occurrence's
    /// region, the same way `Origin::StateClause` does for a `pre`/`post`
    /// clause's own anchor (FR-096).
    ProtocolAttempt {
        /// The protocol's index among the package's protocols.
        protocol: usize,
        /// The attempt's index among the protocol's own `attempts`.
        attempt: usize,
    },
}

/// A located expression: its declaration and the child-index path from that
/// declaration's root expression.
///
/// The path is a shared parent chain: a child is the parent's chain plus one
/// link, so [`Location::child`] costs the same at any depth and a node a
/// hundred thousand levels down holds no copy of the path above it
/// (ADR-030 D-1). The index sequence is built only on output, by
/// [`Location::path`]. Clone, equality, hash, order, debug and drop never
/// recurse with the depth.
#[derive(Clone)]
pub struct Location {
    /// The owning declaration.
    pub origin: Origin,
    tip: Option<Arc<Link>>,
}

/// One step of a path: the child index taken from the parent chain `parent`.
struct Link {
    parent: Option<Arc<Link>>,
    index: usize,
    /// Steps from the root to here, this one included.
    depth: usize,
    /// A fingerprint of the indices from the root to here, so equal paths
    /// that share no links still compare and hash in constant time when they
    /// differ.
    fingerprint: u64,
}

impl Drop for Link {
    /// Release a long chain on a loop: a derived drop would recurse once per
    /// link.
    fn drop(&mut self) {
        let mut next = self.parent.take();
        while let Some(link) = next {
            match Arc::try_unwrap(link) {
                Ok(mut link) => next = link.parent.take(),
                Err(_) => break,
            }
        }
    }
}

impl Location {
    /// The location of the root expression of `origin`.
    pub fn root(origin: Origin) -> Self {
        Self { origin, tip: None }
    }

    /// The location `path` reaches from the root of `origin`, one child
    /// index per step.
    pub fn at(origin: Origin, path: &[usize]) -> Self {
        path.iter()
            .fold(Self::root(origin), |location, index| location.child(*index))
    }

    /// A child of this location, in constant time.
    pub fn child(&self, index: usize) -> Self {
        let (depth, fingerprint) = self
            .tip
            .as_deref()
            .map_or((0, 0), |link| (link.depth, link.fingerprint));
        let link = Link {
            parent: self.tip.clone(),
            index,
            depth: depth + 1,
            fingerprint: fingerprint
                .rotate_left(5)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (index as u64).wrapping_add(1),
        };
        Self {
            origin: self.origin.clone(),
            tip: Some(Arc::new(link)),
        }
    }

    /// The number of steps from the root.
    pub fn depth(&self) -> usize {
        self.tip.as_deref().map_or(0, |link| link.depth)
    }

    /// The child indices from the root, as the parsed form's
    /// `Expression::children` numbers them. Built here, from the chain.
    pub fn path(&self) -> Vec<usize> {
        let mut path = Vec::with_capacity(self.depth());
        let mut link = self.tip.as_deref();
        while let Some(current) = link {
            path.push(current.index);
            link = current.parent.as_deref();
        }
        path.reverse();
        path
    }

    /// The location one step nearer the root, or `None` at the root.
    pub fn parent(&self) -> Option<Self> {
        let link = self.tip.as_deref()?;
        Some(Self {
            origin: self.origin.clone(),
            tip: link.parent.clone(),
        })
    }

    /// The last child index taken, or `None` at the root.
    pub fn last_index(&self) -> Option<usize> {
        self.tip.as_deref().map(|link| link.index)
    }

    /// Whether this location and `other` end in the same link of the same
    /// chain (a clone, or the same node), so their paths are equal without a
    /// walk.
    pub fn shares_chain_with(&self, other: &Self) -> bool {
        match (&self.tip, &other.tip) {
            (None, None) => true,
            (Some(left), Some(right)) => Arc::ptr_eq(left, right),
            _ => false,
        }
    }

    /// The address of this location's last link, or `None` at the root. Two
    /// locations of one chain have the same address exactly when they name
    /// the same node, so a cache keyed on it memoizes per node.
    pub fn chain_address(&self) -> Option<usize> {
        self.tip
            .as_deref()
            .map(|link| core::ptr::from_ref(link) as usize)
    }

    /// The links from this location's tip up to, but not including, the
    /// root: each one's address (see [`Location::chain_address`]) and the
    /// child index it takes, nearest the tip first.
    pub fn ancestry(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let mut link = self.tip.as_deref();
        core::iter::from_fn(move || {
            let current = link?;
            link = current.parent.as_deref();
            Some((core::ptr::from_ref(current) as usize, current.index))
        })
    }

    fn same_path(&self, other: &Self) -> bool {
        let (mut left, mut right) = (self.tip.as_deref(), other.tip.as_deref());
        loop {
            match (left, right) {
                (None, None) => return true,
                (Some(a), Some(b)) => {
                    if core::ptr::eq(a, b) {
                        return true;
                    }
                    if a.depth != b.depth || a.fingerprint != b.fingerprint || a.index != b.index {
                        return false;
                    }
                    left = a.parent.as_deref();
                    right = b.parent.as_deref();
                }
                _ => return false,
            }
        }
    }
}

impl PartialEq for Location {
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin && self.same_path(other)
    }
}

impl Eq for Location {}

impl Hash for Location {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.origin.hash(state);
        self.depth().hash(state);
        self.tip
            .as_deref()
            .map_or(0, |link| link.fingerprint)
            .hash(state);
    }
}

/// A total order that costs the same at any depth: by origin, then depth,
/// then the path's fingerprint. It is not the order of the index sequences
/// (a map keyed by location needs a deterministic order, not that one); two
/// different paths of one depth whose fingerprints collide fall back to the
/// sequences themselves.
impl Ord for Location {
    fn cmp(&self, other: &Self) -> Ordering {
        let fingerprint =
            |location: &Self| location.tip.as_deref().map_or(0, |link| link.fingerprint);
        self.origin
            .cmp(&other.origin)
            .then_with(|| self.depth().cmp(&other.depth()))
            .then_with(|| fingerprint(self).cmp(&fingerprint(other)))
            .then_with(|| {
                if self.same_path(other) {
                    Ordering::Equal
                } else {
                    self.path().cmp(&other.path())
                }
            })
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Location")
            .field("origin", &self.origin)
            .field("depth", &self.depth())
            .field("last_index", &self.last_index())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn body() -> Origin {
        Origin::Body {
            function: String::from("f"),
            index: 2,
        }
    }

    /// A child keeps its origin and extends the path by one index, leaving
    /// the parent unchanged.
    #[test]
    fn child_extends_the_path_and_keeps_the_origin() {
        let parent = Location::at(body(), &[1]);
        let child = parent.child(3);
        assert_eq!(child.origin, parent.origin);
        assert_eq!(child.path(), vec![1, 3]);
        assert_eq!(parent.path(), vec![1]);
    }

    /// Paths built apart compare and hash by their indices, not by the
    /// chain they share, and order by depth first.
    #[test]
    fn equal_paths_built_apart_are_equal_and_order_by_depth() {
        let left = Location::at(body(), &[0, 4, 1]);
        let right = Location::root(body()).child(0).child(4).child(1);
        assert!(!left.shares_chain_with(&right));
        assert_eq!(left, right);
        assert_ne!(left, Location::at(body(), &[0, 4, 2]));
        assert_ne!(left, Location::at(body(), &[0, 4]));
        assert!(Location::at(body(), &[0, 4]) < left);
        assert!(Location::at(body(), &[7]) < Location::at(body(), &[0, 4]));
        assert_eq!(left.cmp(&right), Ordering::Equal);
        assert_ne!(left.cmp(&Location::at(body(), &[0, 4, 2])), Ordering::Equal);
    }

    /// A chain a million links long is built, cloned, compared, formatted
    /// and dropped on a small stack.
    #[test]
    fn a_million_deep_location_never_recurses() {
        let handle = std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let mut location = Location::root(body());
                for _ in 0..1_000_000 {
                    location = location.child(0);
                }
                let clone = location.clone();
                assert_eq!(clone, location);
                assert_eq!(location.depth(), 1_000_000);
                assert_eq!(location.path().len(), 1_000_000);
                assert!(!alloc::format!("{location:?}").is_empty());
                let other = Location::at(body(), &vec![0; 1_000_000]);
                assert_eq!(other, location);
            })
            .expect("spawn");
        handle.join().expect("no stack overflow");
    }
}
