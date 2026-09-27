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

use super::ir::{Node, NodeKind, Observation, Slot};
use super::refusal::Location;

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
    /// The walk recurses over the checked tree, whose depth typing already
    /// bounded by the checking depth limit, as the definedness guard
    /// helpers do (`facts`).
    pub(crate) fn of(body: &Node, clause: Observation, roots: usize) -> Self {
        let mut walk = Walk {
            roots,
            binders: BTreeMap::new(),
            reads: BTreeMap::new(),
        };
        walk.node(body, clause);
        Self { reads: walk.reads }
    }

    /// The observation of the read at `location`, if a model read is there.
    pub(crate) fn of_read(&self, location: &Location) -> Option<Observation> {
        self.reads.get(location).copied()
    }

    /// Every model read's location and observation, in location order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Location, Observation)> {
        self.reads.iter().map(|(location, observation)| (location, *observation))
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

impl Walk {
    /// Record the reads of `node` under the ambient observation `ambient`,
    /// and return the observation a reference value `node` yields was read
    /// at, when it yields one.
    fn node(&mut self, node: &Node, ambient: Observation) -> Option<Observation> {
        match node.kind() {
            NodeKind::Local(slot) if *slot < self.roots => Some(ambient),
            NodeKind::Local(slot) => self.binders.get(slot).copied().flatten(),
            NodeKind::Attribute { reference, .. } => {
                let read = self.node(reference, ambient).unwrap_or(ambient);
                self.reads.insert(node.location().clone(), read);
                Some(read)
            }
            NodeKind::Reaches { source, target, .. } => {
                self.node(source, ambient);
                self.node(target, ambient);
                self.reads.insert(node.location().clone(), ambient);
                None
            }
            NodeKind::AllInstances { population } => {
                self.node(population, ambient);
                self.reads.insert(node.location().clone(), ambient);
                Some(ambient)
            }
            NodeKind::Lookup {
                population,
                reference,
                ..
            } => {
                self.node(population, ambient);
                self.node(reference, ambient);
                self.reads.insert(node.location().clone(), ambient);
                Some(ambient)
            }
            NodeKind::Pre(operand) => self.node(operand, Observation::Pre),
            NodeKind::Value(operand) | NodeKind::Coerce(operand, _) => {
                self.node(operand, ambient)
            }
            NodeKind::Let { slot, value, body } => {
                let bound = self.node(value, ambient);
                self.binders.insert(*slot, bound);
                self.node(body, ambient)
            }
            NodeKind::Query {
                slot, source, body, ..
            } => {
                let bound = self.node(source, ambient);
                self.binders.insert(*slot, bound);
                self.node(body, ambient);
                None
            }
            NodeKind::Fold {
                accumulator,
                binder,
                source,
                step,
                identity,
            } => {
                let bound = self.node(source, ambient);
                self.binders.insert(*binder, bound);
                self.binders.insert(*accumulator, None);
                if let Some(identity) = identity {
                    self.node(identity, ambient);
                }
                self.node(step, ambient);
                None
            }
            // A reference chosen by a conditional keeps an observation only
            // when both branches read it at the same one.
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => {
                self.node(condition, ambient);
                let then = self.node(then, ambient);
                let otherwise = self.node(otherwise, ambient);
                then.filter(|_| then == otherwise)
            }
            _ => {
                for child in node.children() {
                    self.node(child, ambient);
                }
                None
            }
        }
    }
}
