// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-898 steps 2 to 5: the walker toolkit and arena order over
//! 100,000-deep trees, each on a thread with a 512 KiB stack. A walk that
//! recursed per level would overflow that stack and abort the test
//! process, so reaching `join` is the stack-safety evidence.

use core::ops::ControlFlow;

use ix_trace_rs::trace;
use quire_walk::{walk, Arena, Children, Id, Walk};

const DEPTH: usize = 100_000;

/// Run `run` on a 512 KiB thread, the stack FR-356 names.
fn on_small_stack<F: FnOnce() + Send + 'static>(run: F) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(run)
        .expect("spawn a 512 KiB thread")
        .join()
        .expect("the walk must not overflow a 512 KiB stack");
}

/// The frame a chain walk pushes: the node's depth and the order in which
/// it was entered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ChainFrame {
    depth: usize,
    tick: usize,
}

/// A walk over the chain `0 -> 1 -> ... -> DEPTH - 1`, recording every
/// enter and exit frame.
struct Chain {
    len: usize,
    stop_at: Option<usize>,
    enters: Vec<ChainFrame>,
    exits: Vec<ChainFrame>,
    callbacks_after_stop: usize,
    stopped: bool,
}

impl Chain {
    fn new(len: usize) -> Self {
        Self {
            len,
            stop_at: None,
            enters: Vec::new(),
            exits: Vec::new(),
            callbacks_after_stop: 0,
            stopped: false,
        }
    }
}

impl Walk for Chain {
    type Node = usize;
    type Frame = ChainFrame;
    type Stop = String;

    fn enter(
        &mut self,
        depth: usize,
        children: &mut Children<'_, usize>,
    ) -> ControlFlow<String, ChainFrame> {
        if self.stopped {
            self.callbacks_after_stop += 1;
        }
        if self.stop_at == Some(depth) {
            self.stopped = true;
            return ControlFlow::Break(format!("stopped at {depth}"));
        }
        let frame = ChainFrame {
            depth,
            tick: self.enters.len(),
        };
        self.enters.push(frame);
        if depth + 1 < self.len {
            children.push(depth + 1);
        }
        ControlFlow::Continue(frame)
    }

    fn exit(&mut self, frame: ChainFrame) -> ControlFlow<String> {
        if self.stopped {
            self.callbacks_after_stop += 1;
        }
        self.exits.push(frame);
        ControlFlow::Continue(())
    }
}

/// Step 2: every node of a 100,000-deep chain is entered once in
/// pre-order and exited once in post-order, and each exit's frame equals
/// its enter's frame.
#[trace("TC-898", "FR-356-AC-2")]
#[test]
fn tc_898_a_100k_deep_chain_enters_in_pre_order_and_exits_in_post_order() {
    on_small_stack(|| {
        let mut chain = Chain::new(DEPTH);
        assert_eq!(walk(&mut chain, 0), ControlFlow::Continue(()));
        assert_eq!(chain.enters.len(), DEPTH);
        assert!(chain
            .enters
            .iter()
            .enumerate()
            .all(|(index, frame)| frame.depth == index && frame.tick == index));
        let mut expected_exits = chain.enters.clone();
        expected_exits.reverse();
        assert_eq!(chain.exits, expected_exits);
    });
}

/// The two node kinds of a mutually recursive walk, as an expression that
/// holds a type and a type that holds an expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Node {
    Expression(usize),
    Type(usize),
}

/// Each kind's own frame type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExpressionFrame {
    level: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TypeFrame {
    level: usize,
    entered_after: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Frame {
    Expression(ExpressionFrame),
    Type(TypeFrame),
}

/// A chain that alternates an expression node (even levels) and a type
/// node (odd levels).
struct Alternating {
    len: usize,
    enters: Vec<Frame>,
    exits: Vec<Frame>,
}

impl Walk for Alternating {
    type Node = Node;
    type Frame = Frame;
    type Stop = ();

    fn enter(&mut self, node: Node, children: &mut Children<'_, Node>) -> ControlFlow<(), Frame> {
        let frame = match node {
            Node::Expression(level) => {
                if level + 1 < self.len {
                    children.push(Node::Type(level + 1));
                }
                Frame::Expression(ExpressionFrame { level })
            }
            Node::Type(level) => {
                if level + 1 < self.len {
                    children.push(Node::Expression(level + 1));
                }
                Frame::Type(TypeFrame {
                    level,
                    entered_after: self.enters.len(),
                })
            }
        };
        self.enters.push(frame);
        ControlFlow::Continue(frame)
    }

    fn exit(&mut self, frame: Frame) -> ControlFlow<()> {
        self.exits.push(frame);
        ControlFlow::Continue(())
    }
}

/// Step 3: a 100,000-level chain alternating an expression node and a type
/// node, each with its own frame type, enters in pre-order and exits in
/// post-order with each level's own frame.
#[trace("TC-898", "FR-356-AC-2")]
#[test]
fn tc_898_a_100k_level_mutually_recursive_walk_keeps_each_kind_s_frame() {
    on_small_stack(|| {
        let mut alternating = Alternating {
            len: DEPTH,
            enters: Vec::new(),
            exits: Vec::new(),
        };
        assert_eq!(
            walk(&mut alternating, Node::Expression(0)),
            ControlFlow::Continue(())
        );
        assert_eq!(alternating.enters.len(), DEPTH);
        for (index, frame) in alternating.enters.iter().enumerate() {
            match *frame {
                Frame::Expression(ExpressionFrame { level }) => {
                    assert_eq!(level, index);
                    assert_eq!(level % 2, 0, "even levels are expressions");
                }
                Frame::Type(TypeFrame {
                    level,
                    entered_after,
                }) => {
                    assert_eq!(level, index);
                    assert_eq!(entered_after, index);
                    assert_eq!(level % 2, 1, "odd levels are types");
                }
            }
        }
        let mut expected_exits = alternating.enters.clone();
        expected_exits.reverse();
        assert_eq!(alternating.exits, expected_exits);
    });
}

/// Step 4: an enter that stops at depth 50,000 makes the walk return its
/// value, and no callback runs after the stop.
#[trace("TC-898", "FR-356-AC-2")]
#[test]
fn tc_898_a_stop_at_depth_50k_returns_its_value_and_ends_the_walk() {
    on_small_stack(|| {
        let mut chain = Chain::new(DEPTH);
        chain.stop_at = Some(50_000);
        assert_eq!(
            walk(&mut chain, 0),
            ControlFlow::Break("stopped at 50000".to_owned())
        );
        assert_eq!(chain.enters.len(), 50_000);
        assert!(chain.exits.is_empty(), "no node above the stop exits");
        assert_eq!(chain.callbacks_after_stop, 0);
    });
}

/// One node of a sum's checked body in arena order.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Term {
    Literal(u32),
    Add(Id<Term>, Id<Term>),
    Negate(Id<Term>),
}

/// A left-deep sum of 50,000 literals and its final negation, stored
/// children first: exactly 100,000 nodes, the last one the root.
fn deep_sum() -> Arena<Term> {
    let mut arena = Arena::new();
    let mut sum = arena.push(Term::Literal(0));
    for value in 1..50_000 {
        let literal = arena.push(Term::Literal(value));
        sum = arena.push(Term::Add(sum, literal));
    }
    arena.push(Term::Negate(sum));
    arena
}

/// Step 5: each node's subtree size over a 100,000-node arena is one
/// forward loop, and the root's size is 100,000. The arena's derived
/// `Clone`, `PartialEq` and its drop run over the flat vector on the same
/// small stack.
#[trace("TC-898", "FR-356-AC-3")]
#[test]
fn tc_898_a_100k_node_arena_computes_subtree_sizes_in_one_forward_loop() {
    on_small_stack(|| {
        let arena = deep_sum();
        assert_eq!(arena.len(), DEPTH);
        let size_of = |id: Id<Term>, done: &[usize]| {
            *done
                .get(id.index())
                .expect("a child is computed before its parent")
        };
        let sizes = arena.bottom_up(|term, done| match term {
            Term::Literal(_) => 1,
            Term::Add(left, right) => 1 + size_of(*left, done) + size_of(*right, done),
            Term::Negate(operand) => 1 + size_of(*operand, done),
        });
        assert_eq!(sizes.len(), DEPTH);
        assert_eq!(sizes.last().copied(), Some(DEPTH));
        let (root, term) = arena.iter().last().expect("a root");
        assert_eq!(root.index(), DEPTH - 1);
        assert!(matches!(term, Term::Negate(_)));
        assert_eq!(arena.get(root), Some(term));

        let copy = arena.clone();
        assert_eq!(copy, arena);
        drop(copy);
        drop(arena);
    });
}
