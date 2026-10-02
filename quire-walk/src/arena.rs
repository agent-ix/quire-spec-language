// SPDX-License-Identifier: AGPL-3.0-or-later
//! Arena order: [`Arena`] holds each node after its children, so a
//! bottom-up computation is one forward loop with no stack.

use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;

/// The typed handle of one node in an [`Arena<T>`].
///
/// An `Id` is only made by [`Arena::push`], so a node built from the ids
/// its arena has already returned names only nodes stored before it.
pub struct Id<T> {
    index: usize,
    node: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// The node's position in its arena: the number of nodes pushed before
    /// it. A bottom-up result for the node is at this index of the slice
    /// [`Arena::bottom_up`] passes.
    pub fn index(self) -> usize {
        self.index
    }
}

// Written by hand so that `Id<T>` is `Copy`, `Eq`, `Ord`, `Hash` and
// `Debug` whatever `T` is; derives would require the same of `T`.
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for Id<T> {}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.index.cmp(&other.index)
    }
}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Id").field(&self.index).finish()
    }
}

/// A tree stored as one vector, each node after its children.
///
/// Nodes name their children by [`Id`]. Because an id exists only once its
/// node is pushed, building a parent from its children's ids stores the
/// parent after them. The derived `Clone`, `PartialEq` and `Debug` run over
/// the flat vector, and dropping the arena drops each node in turn, so no
/// trait recurses in proportion to depth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arena<T> {
    nodes: Vec<T>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self { nodes: Vec::new() }
    }
}

impl<T> Arena<T> {
    /// An empty arena.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store `node` after every node already pushed, and return its id.
    pub fn push(&mut self, node: T) -> Id<T> {
        let id = Id {
            index: self.nodes.len(),
            node: PhantomData,
        };
        self.nodes.push(node);
        id
    }

    /// The node `id` names, or `None` when `id` is from a longer arena.
    pub fn get(&self, id: Id<T>) -> Option<&T> {
        self.nodes.get(id.index)
    }

    /// The number of nodes stored.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether no node is stored.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Every node with its id, children before parents.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (Id<T>, &T)> + '_ {
        self.nodes.iter().enumerate().map(|(index, node)| {
            (
                Id {
                    index,
                    node: PhantomData,
                },
                node,
            )
        })
    }

    /// Compute one result per node, bottom up, in one forward loop.
    ///
    /// `compute` receives each node, in push order, with the results of
    /// every node before it. A child's result is at its [`Id::index`] in
    /// that slice, which always holds it, since a child is stored before
    /// its parent. The returned vector holds each node's result at its
    /// index.
    pub fn bottom_up<R>(&self, mut compute: impl FnMut(&T, &[R]) -> R) -> Vec<R> {
        let mut results = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let result = compute(node, &results);
            results.push(result);
        }
        results
    }
}
