// SPDX-License-Identifier: AGPL-3.0-or-later
//! Kani harnesses for the traversal (`cargo kani -p quire-walk`).
//!
//! Each harness ranges over every tree of at most [`MAX_NODES`] nodes: node
//! 0 is the root and every other node `i` has an arbitrary parent before
//! it. A recording walk asserts, inside its callbacks and after the walk,
//! that every node is entered and exited exactly once, that each exit
//! receives the frame its enter returned, that a node's exit follows the
//! exits of all its children (so exits come in reverse enter order along
//! every path), that children are entered in the order they were pushed,
//! and that a stop ends the walk at once. Kani also checks that no
//! traversal panics.

use core::ops::ControlFlow;

use crate::{walk, Arena, Children, Walk};

/// The largest tree the harnesses range over.
const MAX_NODES: usize = 3;

/// An arbitrary tree: its node count and each node's parent.
struct Tree {
    nodes: usize,
    parent: [usize; MAX_NODES],
}

impl Tree {
    fn any() -> Self {
        let nodes: usize = kani::any();
        kani::assume(nodes >= 1 && nodes <= MAX_NODES);
        let mut parent = [0; MAX_NODES];
        let mut index = 1;
        while index < MAX_NODES {
            let chosen: usize = kani::any();
            kani::assume(chosen < index);
            parent[index] = chosen;
            index += 1;
        }
        Self { nodes, parent }
    }

    fn is_child(&self, node: usize, child: usize) -> bool {
        child > node && child < self.nodes && self.parent[child] == node
    }
}

/// The frame a recording enter returns: the node and the tick at which it
/// was entered, so an exit can be matched to its own enter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Frame {
    node: usize,
    tick: usize,
}

struct Recorder {
    tree: Tree,
    entered: [Option<usize>; MAX_NODES],
    exited: [Option<usize>; MAX_NODES],
    clock: usize,
    stop_on_enter: Option<usize>,
    stop_on_exit: Option<usize>,
    stopped: bool,
}

impl Recorder {
    fn new(tree: Tree) -> Self {
        Self {
            tree,
            entered: [None; MAX_NODES],
            exited: [None; MAX_NODES],
            clock: 0,
            stop_on_enter: None,
            stop_on_exit: None,
            stopped: false,
        }
    }
}

impl Walk for Recorder {
    type Node = usize;
    type Frame = Frame;
    type Stop = usize;

    fn enter(
        &mut self,
        node: usize,
        children: &mut Children<'_, usize>,
    ) -> ControlFlow<usize, Frame> {
        assert!(!self.stopped, "no callback runs after a stop");
        assert!(node < self.tree.nodes, "only the tree's nodes are entered");
        assert!(self.entered[node].is_none(), "a node is entered once");
        if node > 0 {
            let parent = self.tree.parent[node];
            assert!(
                self.entered[parent].is_some(),
                "a parent is entered before its child"
            );
            assert!(
                self.exited[parent].is_none(),
                "a child is entered before its parent exits"
            );
        }
        if self.stop_on_enter == Some(node) {
            self.stopped = true;
            return ControlFlow::Break(node);
        }
        let tick = self.clock;
        self.entered[node] = Some(tick);
        self.clock += 1;
        let mut child = node + 1;
        while child < self.tree.nodes {
            if self.tree.is_child(node, child) {
                children.push(child);
            }
            child += 1;
        }
        ControlFlow::Continue(Frame { node, tick })
    }

    fn exit(&mut self, frame: Frame) -> ControlFlow<usize> {
        assert!(!self.stopped, "no callback runs after a stop");
        assert!(frame.node < self.tree.nodes);
        assert_eq!(
            self.entered[frame.node],
            Some(frame.tick),
            "an exit receives the frame its enter returned"
        );
        assert!(self.exited[frame.node].is_none(), "a node is exited once");
        let mut child = frame.node + 1;
        while child < self.tree.nodes {
            if self.tree.is_child(frame.node, child) {
                assert!(
                    self.exited[child].is_some(),
                    "every child exits before its parent"
                );
            }
            child += 1;
        }
        self.exited[frame.node] = Some(self.clock);
        self.clock += 1;
        if self.stop_on_exit == Some(frame.node) {
            self.stopped = true;
            return ControlFlow::Break(frame.node);
        }
        ControlFlow::Continue(())
    }
}

/// A walk with no stop enters and exits every node exactly once, each exit
/// with its enter's frame, properly nested along every path, siblings in
/// push order, and returns `Continue`.
#[kani::proof]
#[kani::unwind(8)]
fn every_node_is_entered_and_exited_once_in_nested_order() {
    let mut recorder = Recorder::new(Tree::any());
    assert_eq!(walk(&mut recorder, 0), ControlFlow::Continue(()));
    let tree = &recorder.tree;
    let mut node = 0;
    while node < tree.nodes {
        let (Some(entered), Some(exited)) = (recorder.entered[node], recorder.exited[node]) else {
            panic!("every node is entered and exited");
        };
        assert!(entered < exited);
        if node > 0 {
            let parent = tree.parent[node];
            let (Some(parent_entered), Some(parent_exited)) =
                (recorder.entered[parent], recorder.exited[parent])
            else {
                panic!("every parent is entered and exited");
            };
            // Reverse order along the path: parent in, child in, child
            // out, parent out.
            assert!(parent_entered < entered && exited < parent_exited);
        }
        let mut later = node + 1;
        while later < tree.nodes {
            if node > 0 && tree.parent[later] == tree.parent[node] {
                // Siblings in push order: the earlier one's subtree is
                // finished before the later one is entered.
                assert!(recorder.exited[node] < recorder.entered[later]);
            }
            later += 1;
        }
        node += 1;
    }
    assert_eq!(recorder.clock, 2 * tree.nodes);
}

/// A stop from an enter is returned at once and no callback runs after it.
#[kani::proof]
#[kani::unwind(8)]
fn a_stop_on_enter_ends_the_walk_at_once() {
    let tree = Tree::any();
    let stop: usize = kani::any();
    kani::assume(stop < tree.nodes);
    let mut recorder = Recorder::new(tree);
    recorder.stop_on_enter = Some(stop);
    assert_eq!(walk(&mut recorder, 0), ControlFlow::Break(stop));
    assert!(recorder.stopped);
    assert!(recorder.entered[stop].is_none());
}

/// A stop from an exit is returned at once and no callback runs after it.
#[kani::proof]
#[kani::unwind(8)]
fn a_stop_on_exit_ends_the_walk_at_once() {
    let tree = Tree::any();
    let stop: usize = kani::any();
    kani::assume(stop < tree.nodes);
    let mut recorder = Recorder::new(tree);
    recorder.stop_on_exit = Some(stop);
    assert_eq!(walk(&mut recorder, 0), ControlFlow::Break(stop));
    assert!(recorder.stopped);
    assert!(recorder.exited[stop].is_some());
}

/// `Arena::bottom_up` computes a subtree size per node in one forward loop:
/// each node's children are already computed, and the root's size is the
/// node count.
#[kani::proof]
#[kani::unwind(5)]
fn arena_bottom_up_sees_every_child_before_its_parent() {
    let tree = Tree::any();
    // Store the tree children first: node `i` of `tree` goes in arena slot
    // `tree.nodes - 1 - i`, since every child has a larger index than its
    // parent.
    let mut arena: Arena<[bool; MAX_NODES]> = Arena::new();
    let mut slot = 0;
    while slot < tree.nodes {
        let node = tree.nodes - 1 - slot;
        let mut children = [false; MAX_NODES];
        let mut child = node + 1;
        while child < tree.nodes {
            if tree.is_child(node, child) {
                children[tree.nodes - 1 - child] = true;
            }
            child += 1;
        }
        let id = arena.push(children);
        assert_eq!(id.index(), slot);
        slot += 1;
    }
    let sizes = arena.bottom_up(|children, done: &[usize]| {
        let mut size = 1;
        let mut slot = 0;
        while slot < MAX_NODES {
            if children[slot] {
                assert!(slot < done.len(), "a child is computed before its parent");
                size += done[slot];
            }
            slot += 1;
        }
        size
    });
    assert_eq!(sizes.len(), tree.nodes);
    assert_eq!(sizes[tree.nodes - 1], tree.nodes);
}
