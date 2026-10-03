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

/// One pass over a clause body.
struct Walk {
    roots: usize,
    /// The observation each `let` or binder slot's reference was read at,
    /// when it holds a reference read at one.
    binders: BTreeMap<Slot, Option<Observation>>,
    reads: BTreeMap<Location, Observation>,
}

/// One pending step of [`Walk::run`]. Each [`Task::Visit`] leaves exactly
/// one result: the observation the reference value its node yields was
/// read at, when it yields one.
enum Task<'n> {
    /// Record the reads of a node under an ambient observation.
    Visit(CheckedNode<'n>, Observation),
    /// An attribute read's reference is visited: record the read, at the
    /// reference's own observation or else the ambient one.
    Attribute(&'n Location, Observation),
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

impl Walk {
    /// Record the reads of `root` under the ambient observation `ambient`.
    fn run(&mut self, root: CheckedNode<'_>, ambient: Observation) {
        let mut pending = vec![Task::Visit(root, ambient)];
        let mut results: Vec<Option<Observation>> = Vec::new();
        while let Some(task) = pending.pop() {
            match task {
                Task::Visit(node, ambient) => self.visit(node, ambient, &mut pending, &mut results),
                Task::Attribute(location, ambient) => {
                    let read = results.pop().flatten().unwrap_or(ambient);
                    self.reads.insert(location.clone(), read);
                    results.push(Some(read));
                }
                Task::Let(slot, body, ambient) => {
                    let bound = results.pop().flatten();
                    self.binders.insert(slot, bound);
                    pending.push(Task::Visit(body, ambient));
                }
                Task::Query(slot, body, ambient) => {
                    let bound = results.pop().flatten();
                    self.binders.insert(slot, bound);
                    pending.push(Task::Collapse {
                        count: 1,
                        read: None,
                        value: None,
                    });
                    pending.push(Task::Visit(body, ambient));
                }
                Task::Fold {
                    binder,
                    accumulator,
                    identity,
                    step,
                    ambient,
                } => {
                    let bound = results.pop().flatten();
                    self.binders.insert(binder, bound);
                    self.binders.insert(accumulator, None);
                    pending.push(Task::Collapse {
                        count: 1 + usize::from(identity.is_some()),
                        read: None,
                        value: None,
                    });
                    pending.push(Task::Visit(step, ambient));
                    if let Some(identity) = identity {
                        pending.push(Task::Visit(identity, ambient));
                    }
                }
                Task::If => {
                    let otherwise = results.pop().flatten();
                    let then = results.pop().flatten();
                    results.pop();
                    results.push(then.filter(|_| then == otherwise));
                }
                Task::Collapse { count, read, value } => {
                    results.truncate(results.len().saturating_sub(count));
                    if let Some((location, observation)) = read {
                        self.reads.insert(location.clone(), observation);
                    }
                    results.push(value);
                }
            }
        }
    }

    /// Start visiting `node` under `ambient`: a node with no operand to
    /// visit leaves its result now; any other pushes the steps that visit
    /// its operands, in evaluation order, and finish it.
    fn visit<'n>(
        &self,
        node: CheckedNode<'n>,
        ambient: Observation,
        pending: &mut Vec<Task<'n>>,
        results: &mut Vec<Option<Observation>>,
    ) {
        let visit = |id| Task::Visit(node.at(id), ambient);
        match node.kind() {
            NodeKind::Local(slot) if *slot < self.roots => results.push(Some(ambient)),
            NodeKind::Local(slot) => results.push(self.binders.get(slot).copied().flatten()),
            NodeKind::Attribute { reference, .. } => {
                pending.push(Task::Attribute(node.location(), ambient));
                pending.push(visit(*reference));
            }
            NodeKind::Reaches { source, target, .. } => {
                pending.push(Task::Collapse {
                    count: 2,
                    read: Some((node.location(), ambient)),
                    value: None,
                });
                pending.push(visit(*target));
                pending.push(visit(*source));
            }
            NodeKind::AllInstances { population } => {
                pending.push(Task::Collapse {
                    count: 1,
                    read: Some((node.location(), ambient)),
                    value: Some(ambient),
                });
                pending.push(visit(*population));
            }
            NodeKind::Lookup {
                population,
                reference,
                ..
            } => {
                pending.push(Task::Collapse {
                    count: 2,
                    read: Some((node.location(), ambient)),
                    value: Some(ambient),
                });
                pending.push(visit(*reference));
                pending.push(visit(*population));
            }
            NodeKind::Pre(operand) => {
                pending.push(Task::Visit(node.at(*operand), Observation::Pre))
            }
            NodeKind::Value(operand) | NodeKind::Coerce(operand, _) => {
                pending.push(visit(*operand))
            }
            NodeKind::Let { slot, value, body } => {
                pending.push(Task::Let(*slot, node.at(*body), ambient));
                pending.push(visit(*value));
            }
            NodeKind::Query {
                slot, source, body, ..
            } => {
                pending.push(Task::Query(*slot, node.at(*body), ambient));
                pending.push(visit(*source));
            }
            NodeKind::Fold {
                accumulator,
                binder,
                source,
                step,
                identity,
            } => {
                pending.push(Task::Fold {
                    binder: *binder,
                    accumulator: *accumulator,
                    identity: identity.map(|identity| node.at(identity)),
                    step: node.at(*step),
                    ambient,
                });
                pending.push(visit(*source));
            }
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => {
                pending.push(Task::If);
                pending.push(visit(*otherwise));
                pending.push(visit(*then));
                pending.push(visit(*condition));
            }
            _ => {
                let children = node.kind().children();
                pending.push(Task::Collapse {
                    count: children.len(),
                    read: None,
                    value: None,
                });
                pending.extend(children.into_iter().rev().map(visit));
            }
        }
    }
}
