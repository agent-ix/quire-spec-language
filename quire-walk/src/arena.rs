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
///
/// **Same-arena precondition.** An `Id` carries its position, not its
/// arena. Use it only with the arena that returned it. An id from another
/// arena is not refused: [`Arena::get`] and [`Results::get`] answer `None`
/// when its position is past the end, but an id whose position is in range
/// names whatever node sits there.
pub struct Id<T> {
    index: usize,
    node: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// The node's position in its arena: the number of nodes pushed before
    /// it.
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
    /// `compute` receives each node, in push order, with the [`Results`] of
    /// every node before it. For an id of this arena (see [`Id`]'s
    /// same-arena precondition), [`Results::get`] returns a child's result,
    /// which is always there since a child is stored before its parent; it
    /// returns `None` for the node itself, a later node, or an id past the
    /// end. The returned vector holds each node's result at its
    /// [`Id::index`].
    pub fn bottom_up<R>(&self, mut compute: impl FnMut(&T, &Results<'_, T, R>) -> R) -> Vec<R> {
        let mut results = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let result = compute(
                node,
                &Results {
                    done: &results,
                    node: PhantomData,
                },
            );
            results.push(result);
        }
        results
    }
}

/// The results [`Arena::bottom_up`] has computed so far: one for each node
/// stored before the node being computed.
pub struct Results<'a, T, R> {
    done: &'a [R],
    node: PhantomData<fn() -> T>,
}

impl<'a, T, R> Results<'a, T, R> {
    /// The result for `id`, or `None` when `id` is not yet computed: the
    /// node being computed, a later one, or a position past the arena's
    /// end. Subject to [`Id`]'s same-arena precondition.
    pub fn get(&self, id: Id<T>) -> Option<&'a R> {
        self.done.get(id.index)
    }

    /// The number of results computed so far, which is the index of the
    /// node being computed.
    pub fn len(&self) -> usize {
        self.done.len()
    }

    /// Whether no result is computed yet, which holds for the first node.
    pub fn is_empty(&self) -> bool {
        self.done.is_empty()
    }
}
