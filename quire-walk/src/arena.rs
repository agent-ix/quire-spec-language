// SPDX-License-Identifier: AGPL-3.0-or-later
//! Arena order: [`Arena`] holds each node after its children, so a
//! bottom-up computation is one forward loop with no stack.

use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

/// The source of arena tokens: each [`Arena`] takes the next value when it
/// is created or cloned.
static NEXT_ARENA: AtomicUsize = AtomicUsize::new(0);

/// A fresh arena token.
fn next_token() -> usize {
    NEXT_ARENA.fetch_add(1, AtomicOrdering::Relaxed)
}

/// The typed handle of one node in an [`Arena<T>`].
///
/// An `Id` is only made by [`Arena::push`], so a node built from the ids
/// its arena has already returned names only nodes stored before it.
///
/// **Bound to its arena.** An `Id` carries its arena's token as well as
/// its position. [`Arena::get`] and [`Results::get`] answer `None` for an
/// id from any other arena, a clone included, so a foreign id never reads
/// another node. Tokens come from one process-wide `usize` counter, so two
/// arenas share a token only after the counter wraps: 2^64 arenas on a
/// 64-bit target, 2^32 on a 32-bit one.
pub struct Id<T> {
    index: usize,
    arena: usize,
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
// `Debug` whatever `T` is; derives would require the same of `T`. Ids of
// one arena order by position.
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.arena, self.index) == (other.arena, other.index)
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
        (self.arena, self.index).cmp(&(other.arena, other.index))
    }
}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.arena.hash(state);
        self.index.hash(state);
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Id")
            .field("arena", &self.arena)
            .field("index", &self.index)
            .finish()
    }
}

/// A tree stored as one vector, each node after its children.
///
/// Nodes name their children by [`Id`]. Because an id exists only once its
/// node is pushed, building a parent from its children's ids stores the
/// parent after them. `Clone`, `PartialEq` and `Debug` run over the flat
/// vector, and dropping the arena drops each node in turn, so no trait
/// recurses in proportion to depth.
///
/// Each arena has its own token, which its ids carry (see [`Id`]). A clone
/// takes a fresh token, since the two arenas can then grow apart; an id of
/// the original is foreign to the clone. Equality compares the nodes only.
#[derive(Debug)]
pub struct Arena<T> {
    nodes: Vec<T>,
    token: usize,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            token: next_token(),
        }
    }
}

impl<T: Clone> Clone for Arena<T> {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            token: next_token(),
        }
    }
}

impl<T: PartialEq> PartialEq for Arena<T> {
    fn eq(&self, other: &Self) -> bool {
        self.nodes == other.nodes
    }
}

impl<T: Eq> Eq for Arena<T> {}

impl<T> Arena<T> {
    /// An empty arena.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store `node` after every node already pushed, and return its id.
    pub fn push(&mut self, node: T) -> Id<T> {
        let id = Id {
            index: self.nodes.len(),
            arena: self.token,
            node: PhantomData,
        };
        self.nodes.push(node);
        id
    }

    /// The node `id` names, or `None` when `id` is from another arena.
    pub fn get(&self, id: Id<T>) -> Option<&T> {
        if id.arena == self.token {
            self.nodes.get(id.index)
        } else {
            None
        }
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
                    arena: self.token,
                    node: PhantomData,
                },
                node,
            )
        })
    }

    /// Compute one result per node, bottom up, in one forward loop.
    ///
    /// `compute` receives each node, in push order, with the [`Results`] of
    /// every node before it. [`Results::get`] returns a child's result,
    /// which is always there since a child is stored before its parent. It
    /// returns `None` for the node itself, a later node, or an id of
    /// another arena. The returned vector holds each node's result at its
    /// [`Id::index`].
    pub fn bottom_up<R>(&self, mut compute: impl FnMut(&T, &Results<'_, T, R>) -> R) -> Vec<R> {
        let mut results = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let result = compute(
                node,
                &Results {
                    done: &results,
                    arena: self.token,
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
    arena: usize,
    node: PhantomData<fn() -> T>,
}

impl<'a, T, R> Results<'a, T, R> {
    /// The result for `id`, or `None` when `id` is not yet computed (the
    /// node being computed or a later one) or is from another arena.
    pub fn get(&self, id: Id<T>) -> Option<&'a R> {
        if id.arena == self.arena {
            self.done.get(id.index)
        } else {
            None
        }
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
