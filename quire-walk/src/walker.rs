// SPDX-License-Identifier: AGPL-3.0-or-later
//! The iterative traversal: [`Walk`], [`Children`] and [`walk`].

use alloc::vec::Vec;
use core::ops::ControlFlow;

/// A top-down or mutually recursive walk, run by [`walk`] on an explicit
/// heap stack.
///
/// The walk declares its own node, frame and stop types. A walk over
/// several kinds of node (for example expressions that hold types, and
/// types that hold expressions) declares an enum for [`Walk::Node`] and one
/// for [`Walk::Frame`], with a variant per kind, so each level carries a
/// frame of its own kind.
pub trait Walk {
    /// The handle of one node to enter, usually a reference or an index.
    type Node;
    /// What the walk keeps for an entered node until it is exited.
    type Frame;
    /// The value a callback returns to end the walk early.
    type Stop;

    /// Enter `node`. Push the node's children onto `children`, in the
    /// order they are to be entered, and return the node's frame. The
    /// toolkit enters each child, and that child's whole subtree, before
    /// the next one, and passes the returned frame to [`Walk::exit`] after
    /// the last of them.
    ///
    /// Return [`ControlFlow::Break`] to end the walk: [`walk`] returns that
    /// value at once, and no callback runs after it. Children pushed before
    /// a break are discarded.
    fn enter(
        &mut self,
        node: Self::Node,
        children: &mut Children<'_, Self::Node>,
    ) -> ControlFlow<Self::Stop, Self::Frame>;

    /// Exit the node whose [`Walk::enter`] returned `frame`, once every
    /// child it pushed has been exited. Return [`ControlFlow::Break`] to end
    /// the walk as from [`Walk::enter`].
    fn exit(&mut self, frame: Self::Frame) -> ControlFlow<Self::Stop>;
}

/// The children [`Walk::enter`] names for the node it is entering.
///
/// The toolkit enters them in the order they are pushed.
pub struct Children<'a, N> {
    pending: &'a mut Vec<N>,
}

impl<N> Children<'_, N> {
    /// Name `node` as the next child to enter.
    pub fn push(&mut self, node: N) {
        self.pending.push(node);
    }
}

impl<N> Extend<N> for Children<'_, N> {
    fn extend<I: IntoIterator<Item = N>>(&mut self, nodes: I) {
        self.pending.extend(nodes);
    }
}

/// One entry of the explicit stack: a node still to enter, or an entered
/// node's frame waiting for its exit.
enum Task<N, F> {
    Enter(N),
    Exit(F),
}

/// Run `walker` over the tree below `root`, depth first.
///
/// Every node is entered once, before any of its children, and exited
/// once, after all of them, with the frame its enter returned. Along every
/// root-to-leaf path the exits come in the reverse order of the enters.
/// The walk returns [`ControlFlow::Continue`] when every node has been
/// exited, or the first [`ControlFlow::Break`] a callback returns.
///
/// Native stack use is constant. The heap stack holds one frame per entered
/// node not yet exited and one entry per named child not yet entered, so its
/// size is bounded by the nodes the walk names, which the calling stage
/// charges (ADR-030 D-1 item 3).
pub fn walk<W: Walk + ?Sized>(walker: &mut W, root: W::Node) -> ControlFlow<W::Stop> {
    let mut tasks: Vec<Task<W::Node, W::Frame>> = Vec::new();
    tasks.push(Task::Enter(root));
    // Reused for every node: `enter` pushes its children here in entry
    // order, and they move onto `tasks` reversed, so the first child is
    // popped first.
    let mut pending: Vec<W::Node> = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            Task::Enter(node) => {
                let frame = walker.enter(
                    node,
                    &mut Children {
                        pending: &mut pending,
                    },
                )?;
                tasks.push(Task::Exit(frame));
                tasks.extend(pending.drain(..).rev().map(Task::Enter));
            }
            Task::Exit(frame) => walker.exit(frame)?,
        }
    }
    ControlFlow::Continue(())
}
