// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-104 "Observations of reads": the one observation each model read of a
//! checked state clause body takes.
//!
//! A model read is an attribute read (`self.f`, `deref(r).f`), `reaches`,
//! `allInstances` or `lookup`. Every read observes the clause's own
//! observation (`current` in an invariant, `pre` in a precondition, `post`
//! in a postcondition), except inside `pre(e)`, where reads of `self`, of
//! an operation parameter, and through references obtained inside `e`
//! observe `pre`. A reference value keeps the observation it was read in,
//! and a read through it uses that observation (QSpec `state-contract.md`:
//! "Dereference follows the reference's own universe/type/observation"); a
//! `let` or query binder keeps its initializer's or source's observation.
//! `self`, `result` and the operation's parameters take the clause's own
//! observation, the qualification the binding supplies (QSpec
//! `state-contract.md`: "The model/binding must supply any reference
//! qualification before evaluation").

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::marker::PhantomData;
use std::ops::ControlFlow;

use super::ir::{CheckedNode, NodeKind, Observation, Slot};
use quire_semantic_value::location::Location;

/// The observation of every model read of one state clause body, by the
/// read's own location.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Observations {
    reads: BTreeMap<Location, Observation>,
}

impl Observations {
    /// The observations of `body`'s reads, in a clause whose own
    /// observation is `clause` and whose slots below `roots` are its
    /// `self`, `result` and operation parameter bindings.
    ///
    /// The walk keeps its pending nodes on a heap stack, so no depth of
    /// checked expression is a limit (ADR-030 D-1).
    pub(crate) fn of(body: CheckedNode<'_>, clause: Observation, roots: usize) -> Self {
        let mut walk = Walk {
            roots,
            binders: BTreeMap::new(),
            reads: BTreeMap::new(),
            results: Vec::new(),
            nodes: PhantomData,
        };
        walk.run(body, clause);
        Self { reads: walk.reads }
    }

    /// The observation of the read at `location`, if a model read is there.
    pub(crate) fn of_read(&self, location: &Location) -> Option<Observation> {
        self.reads.get(location).copied()
    }

    /// Every model read's location and observation, in location order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Location, Observation)> {
        self.reads
            .iter()
            .map(|(location, observation)| (location, *observation))
    }
}

/// One pass over a clause body, on the walker toolkit: each node's exit
/// leaves exactly one result on `results`, the observation the reference
/// value its node yields was read at, when it yields one. The toolkit
/// enters a node's later operands only after its earlier ones are exited, so
/// a binding step sits between operands as a node of its own.
struct Walk<'n> {
    roots: usize,
    /// The observation each `let` or binder slot's reference was read at,
    /// when it holds a reference read at one.
    binders: BTreeMap<Slot, Option<Observation>>,
    reads: BTreeMap<Location, Observation>,
    results: Vec<Option<Observation>>,
    nodes: PhantomData<&'n ()>,
}

/// One node of the walk.
enum Step<'n> {
    /// Record the reads of a node under an ambient observation.
    Visit(CheckedNode<'n>, Observation),
    /// Bind a `let` slot to the visited value's observation, then visit the
    /// body, whose result is the `let`'s.
    Let(Slot, CheckedNode<'n>, Observation),
    /// Bind a query binder to the visited source's observation, then visit
    /// the body.
    Query(Slot, CheckedNode<'n>, Observation),
    /// Bind a fold's binder to the visited source's observation, then visit
    /// the identity and the step.
    Fold {
        binder: Slot,
        accumulator: Slot,
        identity: Option<CheckedNode<'n>>,
        step: CheckedNode<'n>,
        ambient: Observation,
    },
}

/// What the walk keeps for an entered node until its operands are exited.
enum Exit<'n> {
    /// The operand's result is the node's.
    Pass,
    /// An attribute read's reference is visited: record the read, at the
    /// reference's own observation or else the ambient one.
    Attribute(&'n Location, Observation),
    /// A conditional's operands are visited: it keeps an observation only
    /// when both branches read it at the same one.
    If,
    /// Drop `count` results, record a read at `read` when given, and leave
    /// `value`.
    Collapse {
        count: usize,
        read: Option<(&'n Location, Observation)>,
        value: Option<Observation>,
    },
}

impl<'n> Walk<'n> {
    /// Record the reads of `root` under the ambient observation `ambient`.
    fn run(&mut self, root: CheckedNode<'n>, ambient: Observation) {
        let ControlFlow::Continue(()) = quire_walk::walk(self, Step::Visit(root, ambient));
    }

    /// Enter `node` under `ambient`: a node with no operand to visit leaves
    /// its result now; any other names its operands in evaluation order.
    fn visit(
        &mut self,
        node: CheckedNode<'n>,
        ambient: Observation,
        children: &mut quire_walk::Children<'_, Step<'n>>,
    ) -> Exit<'n> {
        let visit = |id| Step::Visit(node.at(id), ambient);
        match node.kind() {
            NodeKind::Local(slot) if *slot < self.roots => {
                self.results.push(Some(ambient));
                Exit::Pass
            }
            NodeKind::Local(slot) => {
                self.results
                    .push(self.binders.get(slot).copied().flatten());
                Exit::Pass
            }
            NodeKind::Attribute { reference, .. } => {
                children.push(visit(*reference));
                Exit::Attribute(node.location(), ambient)
            }
            NodeKind::Reaches { source, target, .. } => {
                children.extend([visit(*source), visit(*target)]);
                Exit::Collapse {
                    count: 2,
                    read: Some((node.location(), ambient)),
                    value: None,
                }
            }
            NodeKind::AllInstances { population } => {
                children.push(visit(*population));
                Exit::Collapse {
                    count: 1,
                    read: Some((node.location(), ambient)),
                    value: Some(ambient),
                }
            }
            NodeKind::Lookup {
                population,
                reference,
                ..
            } => {
                children.extend([visit(*population), visit(*reference)]);
                Exit::Collapse {
                    count: 2,
                    read: Some((node.location(), ambient)),
                    value: Some(ambient),
                }
            }
            NodeKind::Pre(operand) => {
                children.push(Step::Visit(node.at(*operand), Observation::Pre));
                Exit::Pass
            }
            NodeKind::Value(operand) | NodeKind::Coerce(operand, _) => {
                children.push(visit(*operand));
                Exit::Pass
            }
            NodeKind::Let { slot, value, body } => {
                children.extend([visit(*value), Step::Let(*slot, node.at(*body), ambient)]);
                Exit::Pass
            }
            NodeKind::Query {
                slot, source, body, ..
            } => {
                children.extend([visit(*source), Step::Query(*slot, node.at(*body), ambient)]);
                Exit::Pass
            }
            NodeKind::Fold {
                accumulator,
                binder,
                source,
                step,
                identity,
            } => {
                children.extend([
                    visit(*source),
                    Step::Fold {
                        binder: *binder,
                        accumulator: *accumulator,
                        identity: identity.map(|identity| node.at(identity)),
                        step: node.at(*step),
                        ambient,
                    },
                ]);
                Exit::Pass
            }
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => {
                children.extend([visit(*condition), visit(*then), visit(*otherwise)]);
                Exit::If
            }
            _ => {
                let operands = node.kind().children();
                let count = operands.len();
                children.extend(operands.into_iter().map(visit));
                Exit::Collapse {
                    count,
                    read: None,
                    value: None,
                }
            }
        }
    }
}

impl<'n> quire_walk::Walk for Walk<'n> {
    type Node = Step<'n>;
    type Frame = Exit<'n>;
    type Stop = Infallible;

    fn enter(
        &mut self,
        step: Step<'n>,
        children: &mut quire_walk::Children<'_, Step<'n>>,
    ) -> ControlFlow<Infallible, Exit<'n>> {
        ControlFlow::Continue(match step {
            Step::Visit(node, ambient) => self.visit(node, ambient, children),
            Step::Let(slot, body, ambient) => {
                let bound = self.results.pop().flatten();
                self.binders.insert(slot, bound);
                children.push(Step::Visit(body, ambient));
                Exit::Pass
            }
            Step::Query(slot, body, ambient) => {
                let bound = self.results.pop().flatten();
                self.binders.insert(slot, bound);
                children.push(Step::Visit(body, ambient));
                Exit::Collapse {
                    count: 1,
                    read: None,
                    value: None,
                }
            }
            Step::Fold {
                binder,
                accumulator,
                identity,
                step,
                ambient,
            } => {
                let bound = self.results.pop().flatten();
                self.binders.insert(binder, bound);
                self.binders.insert(accumulator, None);
                if let Some(identity) = identity {
                    children.push(Step::Visit(identity, ambient));
                }
                children.push(Step::Visit(step, ambient));
                Exit::Collapse {
                    count: 1 + usize::from(identity.is_some()),
                    read: None,
                    value: None,
                }
            }
        })
    }

    fn exit(&mut self, exit: Exit<'n>) -> ControlFlow<Infallible> {
        match exit {
            Exit::Pass => {}
            Exit::Attribute(location, ambient) => {
                let read = self.results.pop().flatten().unwrap_or(ambient);
                self.reads.insert(location.clone(), read);
                self.results.push(Some(read));
            }
            Exit::If => {
                let otherwise = self.results.pop().flatten();
                let then = self.results.pop().flatten();
                self.results.pop();
                self.results.push(then.filter(|_| then == otherwise));
            }
            Exit::Collapse { count, read, value } => {
                self.results
                    .truncate(self.results.len().saturating_sub(count));
                if let Some((location, observation)) = read {
                    self.reads.insert(location.clone(), observation);
                }
                self.results.push(value);
            }
        }
        ControlFlow::Continue(())
    }
}
