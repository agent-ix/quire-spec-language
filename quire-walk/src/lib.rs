// SPDX-License-Identifier: AGPL-3.0-or-later
//! `quire-walk`: the shared `no_std` walker toolkit (ADR-011 §6.1 layer W,
//! ADR-030 D-1 item 2).
//!
//! A walk over a nested structure never uses native recursion in
//! proportion to its input. It takes one of two shapes:
//!
//! - **Top-down or mutually recursive:** implement [`Walk`] and run it with
//!   [`walk`]. The toolkit keeps an explicit heap stack. Entering a node
//!   pushes the frame the walk's [`Walk::enter`] returns; leaving it pops
//!   that frame into [`Walk::exit`]. Native stack use is constant whatever
//!   the depth.
//! - **Bottom-up:** store the tree in an [`Arena`], which holds each node
//!   after its children, and compute over it with one forward loop
//!   ([`Arena::bottom_up`]). No stack at all.
//!
//! The crate depends on `core` and `alloc` only and has no features, so
//! QSL, IR, CG and RT can all depend on it (ADR-011 FB-05's shared-leaf
//! class).
//!
//! # Example
//!
//! Count the nodes of a tree held as child lists, then stop at a named
//! node:
//!
//! ```
//! use core::ops::ControlFlow;
//! use quire_walk::{walk, Children, Walk};
//!
//! struct Tree(Vec<Vec<usize>>);
//!
//! struct Count<'a> {
//!     tree: &'a Tree,
//!     entered: usize,
//!     stop_at: Option<usize>,
//! }
//!
//! impl Walk for Count<'_> {
//!     type Node = usize;
//!     type Frame = usize;
//!     type Stop = &'static str;
//!
//!     fn enter(
//!         &mut self,
//!         node: usize,
//!         children: &mut Children<'_, usize>,
//!     ) -> ControlFlow<&'static str, usize> {
//!         if self.stop_at == Some(node) {
//!             return ControlFlow::Break("found");
//!         }
//!         self.entered += 1;
//!         children.extend(self.tree.0[node].iter().copied());
//!         ControlFlow::Continue(node)
//!     }
//!
//!     fn exit(&mut self, _frame: usize) -> ControlFlow<&'static str> {
//!         ControlFlow::Continue(())
//!     }
//! }
//!
//! let tree = Tree(vec![vec![1, 2], vec![], vec![3], vec![]]);
//! let mut count = Count { tree: &tree, entered: 0, stop_at: None };
//! assert_eq!(walk(&mut count, 0), ControlFlow::Continue(()));
//! assert_eq!(count.entered, 4);
//!
//! let mut find = Count { tree: &tree, entered: 0, stop_at: Some(2) };
//! assert_eq!(walk(&mut find, 0), ControlFlow::Break("found"));
//! ```

#![no_std]

extern crate alloc;

mod arena;
mod walker;

#[cfg(kani)]
mod proofs;

pub use arena::{Arena, Computed, Id, Results};
pub use walker::{walk, Children, Walk};
