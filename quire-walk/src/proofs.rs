// SPDX-License-Identifier: AGPL-3.0-or-later
//! Kani harnesses for the traversal (`cargo kani -p quire-walk`).
//!
//! The harnesses together cover every ordered tree of at most
//! [`MAX_NODES`] nodes: one harness per parent array (node 0 is the root
//! and every other node `i` names its parent, a node before it), ten in
//! all. On each tree a harness runs the complete walk, then a walk that
//! stops on entering each node, then one that stops on exiting each node,
//! then the arena computation.
//! Every frame carries an arbitrary payload (`kani::any`), so the frame
//! checks hold for every frame value.
//!
//! A recording walk asserts, inside its callbacks and after the walk, that
//! every node is entered and exited exactly once, that each exit receives
//! the frame its enter returned, that a node's exit follows the exits of
//! all its children (so exits come in reverse enter order along every
//! path), that children are entered in the order they were pushed, and that
//! a stop ends the walk at once with its value. Kani also checks that no
//! traversal panics, overflows or reads out of bounds.
//!
//! The tree shape is fixed per harness rather than drawn with `kani::any`:
//! a symbolic shape gives the walker's heap stack a symbolic length, and
//! CBMC then does not finish within hours even at four nodes. Enumerating
//! the trees keeps each harness to minutes or less and still covers every
//! tree within the bound.

use core::ops::ControlFlow;

use crate::{walk, Arena, Children, Id, Walk};

/// The largest tree the harnesses range over.
const MAX_NODES: u8 = 4;

/// [`MAX_NODES`] as an array length.
const SLOTS: usize = MAX_NODES as usize;

/// An arbitrary tree: its node count and each node's parent.
#[derive(Clone, Copy)]
struct Tree {
    nodes: u8,
    parent: [u8; SLOTS],
}

impl Tree {
    const fn new(nodes: u8, parent: [u8; SLOTS]) -> Self {
        Self { nodes, parent }
    }

    fn is_child(&self, node: u8, child: u8) -> bool {
        child > node && child < self.nodes && self.parent[usize::from(child)] == node
    }
}

/// The frame a recording enter returns: the node and the tick at which it
/// was entered, so an exit can be matched to its own enter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Frame {
    node: u8,
    tick: u8,
    payload: u8,
}

/// Records each node's enter and exit tick; tick 0 means "not yet".
struct Recorder {
    tree: Tree,
    entered: [u8; SLOTS],
    payloads: [u8; SLOTS],
    exited: [u8; SLOTS],
    clock: u8,
    stop_on_enter: Option<u8>,
    stop_on_exit: Option<u8>,
    stopped: bool,
}

impl Recorder {
    fn new(tree: Tree) -> Self {
        Self {
            tree,
            entered: [0; SLOTS],
            payloads: [0; SLOTS],
            exited: [0; SLOTS],
            clock: 1,
            stop_on_enter: None,
            stop_on_exit: None,
            stopped: false,
        }
    }

    fn tick(&mut self) -> u8 {
        let tick = self.clock;
        self.clock += 1;
        tick
    }
}

impl Walk for Recorder {
    type Node = u8;
    type Frame = Frame;
    type Stop = u8;

    fn enter(&mut self, node: u8, children: &mut Children<'_, u8>) -> ControlFlow<u8, Frame> {
        assert!(!self.stopped, "no callback runs after a stop");
        assert!(node < self.tree.nodes, "only the tree's nodes are entered");
        let slot = usize::from(node);
        assert_eq!(self.entered[slot], 0, "a node is entered once");
        if node > 0 {
            let parent = usize::from(self.tree.parent[slot]);
            assert_ne!(
                self.entered[parent], 0,
                "a parent is entered before its child"
            );
            assert_eq!(
                self.exited[parent], 0,
                "a child is entered before its parent exits"
            );
        }
        if self.stop_on_enter == Some(node) {
            self.stopped = true;
            return ControlFlow::Break(node);
        }
        let tick = self.tick();
        self.entered[slot] = tick;
        let payload: u8 = kani::any();
        self.payloads[slot] = payload;
        for child in 1..MAX_NODES {
            if self.tree.is_child(node, child) {
                children.push(child);
            }
        }
        ControlFlow::Continue(Frame {
            node,
            tick,
            payload,
        })
    }

    fn exit(&mut self, frame: Frame) -> ControlFlow<u8> {
        assert!(!self.stopped, "no callback runs after a stop");
        assert!(frame.node < self.tree.nodes);
        let slot = usize::from(frame.node);
        assert_eq!(
            self.entered[slot], frame.tick,
            "an exit receives the frame its enter returned"
        );
        assert_eq!(self.payloads[slot], frame.payload);
        assert_eq!(self.exited[slot], 0, "a node is exited once");
        for child in 1..MAX_NODES {
            if self.tree.is_child(frame.node, child) {
                assert_ne!(
                    self.exited[usize::from(child)],
                    0,
                    "every child exits before its parent"
                );
            }
        }
        self.exited[slot] = self.tick();
        if self.stop_on_exit == Some(frame.node) {
            self.stopped = true;
            return ControlFlow::Break(frame.node);
        }
        ControlFlow::Continue(())
    }
}

/// The complete walk over `tree` enters and exits every node exactly once,
/// each exit with its enter's frame, properly nested along every path,
/// siblings in push order, and returns `Continue`.
fn check_complete_walk(tree: Tree) {
    let mut recorder = Recorder::new(tree);
    assert!(walk(&mut recorder, 0) == ControlFlow::Continue(()));
    for node in 0..MAX_NODES {
        if node >= tree.nodes {
            continue;
        }
        let slot = usize::from(node);
        let (entered, exited) = (recorder.entered[slot], recorder.exited[slot]);
        assert!(
            entered != 0 && exited != 0,
            "every node is entered and exited"
        );
        assert!(entered < exited);
        if node > 0 {
            let parent = usize::from(tree.parent[slot]);
            // Reverse order along the path: parent in, child in, child
            // out, parent out.
            assert!(recorder.entered[parent] < entered && exited < recorder.exited[parent]);
            for later in (node + 1)..MAX_NODES {
                if later < tree.nodes && tree.parent[usize::from(later)] == tree.parent[slot] {
                    // Siblings in push order: the earlier one's subtree is
                    // finished before the later one is entered.
                    assert!(exited < recorder.entered[usize::from(later)]);
                }
            }
        }
    }
    assert!(recorder.clock == 2 * tree.nodes + 1);
}

/// A stop on entering, then on exiting, each node of `tree` is returned at
/// once, and no callback runs after it (the recorder asserts that in every
/// callback).
fn check_stops(tree: Tree) {
    for stop in 0..MAX_NODES {
        if stop >= tree.nodes {
            continue;
        }
        let mut recorder = Recorder::new(tree);
        recorder.stop_on_enter = Some(stop);
        assert!(walk(&mut recorder, 0) == ControlFlow::Break(stop));
        assert!(recorder.stopped);
        assert!(recorder.entered[usize::from(stop)] == 0);

        let mut recorder = Recorder::new(tree);
        recorder.stop_on_exit = Some(stop);
        assert!(walk(&mut recorder, 0) == ControlFlow::Break(stop));
        assert!(recorder.stopped);
        assert!(recorder.exited[usize::from(stop)] != 0);
    }
}

/// One arena node: the ids of its children, by tree index.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Node {
    children: [Option<Id<Node>>; SLOTS],
}

/// `Arena::bottom_up` over `tree`, stored children first, computes each
/// node's subtree size in one forward loop: every child's result is there
/// when its parent is computed, the node's own id is not yet computed, and
/// the root's size is the node count.
fn check_arena(tree: Tree) {
    let nodes = tree.nodes;
    // Tree nodes are pushed from the last to the first, since every child
    // has a larger index than its parent; `ids[i]` is tree node `i`'s id.
    let mut arena: Arena<Node> = Arena::new();
    let mut ids: [Option<Id<Node>>; SLOTS] = [None; SLOTS];
    for slot in 0..MAX_NODES {
        if slot >= nodes {
            continue;
        }
        let node = nodes - 1 - slot;
        let mut children = [None; SLOTS];
        for child in 1..MAX_NODES {
            if tree.is_child(node, child) {
                children[usize::from(child)] = ids[usize::from(child)];
            }
        }
        let id = arena.push(Node { children });
        assert!(id.index() == usize::from(slot));
        assert!(arena.get(id) == Some(&Node { children }));
        ids[usize::from(node)] = Some(id);
    }
    let sizes = arena.bottom_up(|node, done| {
        for slot in 0..SLOTS {
            if let Some(id) = ids[slot] {
                assert!(
                    done.get(id).is_some() == (id.index() < done.len()),
                    "exactly the nodes before this one have a result"
                );
            }
        }
        let mut size = 1;
        for slot in 0..SLOTS {
            if let Some(child) = node.children[slot] {
                let Some(result) = done.get(child) else {
                    panic!("a child is computed before its parent");
                };
                size += result;
            }
        }
        size
    });
    assert!(sizes.len() == usize::from(nodes));
    assert!(sizes[usize::from(nodes - 1)] == nodes);
}

/// One harness per tree shape: the complete walk, every stop and the arena
/// computation.
macro_rules! tree_harness {
    ($name:ident, $nodes:expr, $parent:expr) => {
        #[kani::proof]
        #[kani::unwind(10)]
        fn $name() {
            let tree = Tree::new($nodes, $parent);
            check_complete_walk(tree);
            check_stops(tree);
            check_arena(tree);
        }
    };
}

// Every tree of one to four nodes in which each node's parent comes before
// it: every parent array the bound admits, named by the parents of nodes 1
// to 3. Each ordered tree shape of up to four nodes is among them.
tree_harness!(tree_1, 1, [0, 0, 0, 0]);
tree_harness!(tree_2_p0, 2, [0, 0, 0, 0]);
tree_harness!(tree_3_p00, 3, [0, 0, 0, 0]);
tree_harness!(tree_3_p01, 3, [0, 0, 1, 0]);
tree_harness!(tree_4_p000, 4, [0, 0, 0, 0]);
tree_harness!(tree_4_p001, 4, [0, 0, 0, 1]);
tree_harness!(tree_4_p002, 4, [0, 0, 0, 2]);
tree_harness!(tree_4_p010, 4, [0, 0, 1, 0]);
tree_harness!(tree_4_p011, 4, [0, 0, 1, 1]);
tree_harness!(tree_4_p012, 4, [0, 0, 1, 2]);
