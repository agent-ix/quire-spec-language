// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-212 (ADR-027 PA-1 to PA-3, QSpec FR-430): the fairness set of an
//! infinite-trace clause over a protocol subject, and the class of each of
//! its members over protocol transition identities.
//!
//! A member is `{kind, target, granularity, origin}`. Its target is an
//! operation, whose `whole` class is every `attempt` that applies it and
//! whose `each` classes are one per FR-120 application, or a thread target
//! (a `parallel` branch, or the root), whose `whole` class is every step its
//! threads own and whose `each` classes are one per thread. A thread owns
//! the steps that carry its path: its event steps, its fences, the forks it
//! performs and the joins it waits at. Channel and memory steps carry no
//! thread and are in no class. A branch target covers the threads whose
//! path ends in a step of that `parallel` and label, so the threads of a
//! `parallel` nested inside the branch are not its own.
//!
//! The set is the clause's authored operation constraints, each with origin
//! `authored`, and, unless the protocol declares `scheduling adversarial`,
//! one `fair weak whole` scheduler constraint per `parallel` branch and one
//! over the root, each with origin `scheduler`; it is ordered by the JCS
//! bytes of its members (QSpec FR-430 "Fairness set members").

use std::collections::BTreeMap;

use qsl_semantics::check::{CheckedProtocol, CheckedScheduling, ControlStep};
use quire_canonical::{Encode, Error, Sink, Writer};
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity, FairnessKind};
use quire_semantic_value::model_transition::ModelTransition;

use super::identity::{ProtocolTransition, ThreadPath};
use crate::fairness::{Constraint, Identity, Strength};
use crate::simulation::model::encode::Enc;

/// What a protocol fairness constraint names (ADR-027 PA-1).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProtocolTarget {
    /// An operation, by the text its applications' FR-120 identities carry.
    Operation {
        /// The operation's text.
        name: String,
    },
    /// The root thread target, named by the protocol's name.
    Root,
    /// The thread target of the branch `branch` of the `parallel` whose
    /// static declaration path is `parallel`.
    Branch {
        /// The `parallel` node's static declaration path.
        parallel: String,
        /// The branch label.
        branch: String,
    },
}

impl ProtocolTarget {
    /// The branch `branch` of the `parallel` at `parallel`.
    pub fn branch(parallel: impl Into<String>, branch: impl Into<String>) -> Self {
        Self::Branch {
            parallel: parallel.into(),
            branch: branch.into(),
        }
    }

    fn enc(&self) -> Enc {
        match self {
            Self::Operation { name } => {
                Enc::tagged("operation", vec![("name", Enc::text(name.clone()))])
            }
            Self::Root => Enc::tagged(
                "thread-target",
                vec![("node", Enc::Null), ("branch", Enc::Null)],
            ),
            Self::Branch { parallel, branch } => Enc::tagged(
                "thread-target",
                vec![
                    ("node", Enc::text(parallel.clone())),
                    ("branch", Enc::text(branch.clone())),
                ],
            ),
        }
    }
}

/// Where a fairness set member comes from (QSpec FR-430).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FairnessOrigin {
    /// Written by the clause.
    Authored,
    /// Derived from the protocol clause (ADR-027 PA-3).
    Scheduler,
}

impl FairnessOrigin {
    fn text(self) -> &'static str {
        match self {
            Self::Authored => "authored",
            Self::Scheduler => "scheduler",
        }
    }
}

/// One member of a fairness set over a protocol subject (QSpec FR-430).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProtocolFairness {
    kind: FairnessKind,
    target: ProtocolTarget,
    granularity: FairnessGranularity,
    origin: FairnessOrigin,
}

/// The class of a step under one member: the member's one `whole` class, or
/// the `each` class of one application or one thread.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProtocolClass {
    /// The member's `whole` class.
    Whole,
    /// The `each` class of one FR-120 application.
    Application(ModelTransition),
    /// The `each` class of one thread.
    Thread(ThreadPath),
}

impl ProtocolFairness {
    /// A member of `kind` and `granularity` over `target` from `origin`.
    pub const fn new(
        kind: FairnessKind,
        target: ProtocolTarget,
        granularity: FairnessGranularity,
        origin: FairnessOrigin,
    ) -> Self {
        Self {
            kind,
            target,
            granularity,
            origin,
        }
    }

    /// The `fair weak whole` scheduler constraint over `target` (PA-3).
    pub const fn scheduler(target: ProtocolTarget) -> Self {
        Self::new(
            FairnessKind::Weak,
            target,
            FairnessGranularity::Whole,
            FairnessOrigin::Scheduler,
        )
    }

    /// The member's strength.
    pub const fn kind(&self) -> FairnessKind {
        self.kind
    }

    /// The member's target.
    pub const fn target(&self) -> &ProtocolTarget {
        &self.target
    }

    /// The member's granularity.
    pub const fn granularity(&self) -> FairnessGranularity {
        self.granularity
    }

    /// The member's origin.
    pub const fn origin(&self) -> FairnessOrigin {
        self.origin
    }

    /// The class `transition` belongs to under this member (PA-1, PA-2);
    /// `None` when it belongs to none.
    pub fn class_of(&self, transition: &ProtocolTransition) -> Option<ProtocolClass> {
        let each = self.granularity == FairnessGranularity::Each;
        match &self.target {
            ProtocolTarget::Operation { name } => {
                let application = transition.application()?;
                (application.operation() == name).then(|| {
                    if each {
                        ProtocolClass::Application(application.clone())
                    } else {
                        ProtocolClass::Whole
                    }
                })
            }
            ProtocolTarget::Root | ProtocolTarget::Branch { .. } => {
                let thread = transition.thread()?;
                let holds = match (&self.target, thread.steps.last()) {
                    (ProtocolTarget::Root, None) => true,
                    (ProtocolTarget::Branch { parallel, branch }, Some(step)) => {
                        step.node == *parallel && step.branch == *branch
                    }
                    _ => false,
                };
                holds.then(|| {
                    if each {
                        ProtocolClass::Thread(thread.clone())
                    } else {
                        ProtocolClass::Whole
                    }
                })
            }
        }
    }

    fn enc(&self) -> Enc {
        Enc::Object(vec![
            (
                "kind",
                Enc::text(match self.kind {
                    FairnessKind::Weak => "weak",
                    FairnessKind::Strong => "strong",
                }),
            ),
            ("target", self.target.enc()),
            (
                "granularity",
                Enc::text(match self.granularity {
                    FairnessGranularity::Whole => "whole",
                    FairnessGranularity::Each => "each",
                }),
            ),
            ("origin", Enc::text(self.origin.text())),
        ])
    }
}

impl Encode for ProtocolFairness {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.enc().encode_into(writer)
    }
}

/// Why a protocol fairness set could not be built.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProtocolFairnessError {
    /// An authored constraint names an operation the subject's model does
    /// not apply.
    #[error("the fairness constraint on `{name}` names an operation the subject does not apply")]
    UnknownOperation {
        /// The operation's resolved name.
        name: String,
    },
    /// A member has no canonical encoding.
    #[error("a fairness set member has no canonical encoding: {0}")]
    Encoding(String),
}

/// The scheduler constraints of `protocol` (PA-3): one per branch of each
/// `parallel` and one over the root; none under `scheduling adversarial`.
pub(super) fn scheduler_constraints(protocol: &CheckedProtocol) -> Vec<ProtocolFairness> {
    if protocol.slots.scheduling == CheckedScheduling::Adversarial {
        return Vec::new();
    }
    let tree = &protocol.controls;
    let mut members = vec![ProtocolFairness::scheduler(ProtocolTarget::Root)];
    for control in &tree.controls {
        if control.step() != ControlStep::Parallel {
            continue;
        }
        let parallel = control.path.join("/");
        for child in control
            .children
            .iter()
            .filter_map(|node| tree.control(*node))
        {
            if child.step() == ControlStep::Branch {
                members.push(ProtocolFairness::scheduler(ProtocolTarget::branch(
                    parallel.clone(),
                    child.name.clone(),
                )));
            }
        }
    }
    members
}

/// The fairness set over `protocol` of a clause whose authored constraints
/// are `authored`, each operation named by the text `operation_text` gives
/// it: the authored members and the scheduler constraints, each once, in
/// ascending JCS bytes (QSpec FR-430).
///
/// # Errors
///
/// [`ProtocolFairnessError`] when an authored operation has no text or a
/// member no canonical encoding.
pub(super) fn fairness_set<'t>(
    protocol: &CheckedProtocol,
    authored: &[FairnessConstraint],
    operation_text: impl Fn(&FairnessConstraint) -> Option<&'t str>,
) -> Result<Vec<ProtocolFairness>, ProtocolFairnessError> {
    let mut members = Vec::with_capacity(authored.len());
    for constraint in authored {
        let name =
            operation_text(constraint).ok_or_else(|| ProtocolFairnessError::UnknownOperation {
                name: constraint.operation().name().to_owned(),
            })?;
        members.push(ProtocolFairness::new(
            constraint.kind(),
            ProtocolTarget::Operation {
                name: name.to_owned(),
            },
            constraint.granularity(),
            FairnessOrigin::Authored,
        ));
    }
    members.extend(scheduler_constraints(protocol));
    let mut keyed: BTreeMap<Vec<u8>, ProtocolFairness> = BTreeMap::new();
    for member in members {
        let bytes = quire_canonical::to_vec(&member, quire_canonical::Limits::new(u64::MAX))
            .map_err(|error| ProtocolFairnessError::Encoding(error.to_string()))?;
        keyed.entry(bytes).or_insert(member);
    }
    Ok(keyed.into_values().collect())
}

/// The fairness kernels' constraints of `fairness` over `transitions`, the
/// protocol transition identities a graph or a loop holds, numbered by
/// `identity`, each with the index of the member it comes from: a `whole`
/// member is one constraint over its class, an `each` member one per class
/// any of `transitions` is in, in class order (ADR-018 FA-1, PA-1).
///
/// # Errors
///
/// The error `identity` returns.
pub fn protocol_constraints<E>(
    fairness: &[ProtocolFairness],
    transitions: &[ProtocolTransition],
    identity: impl Fn(&ProtocolTransition) -> Result<Identity, E>,
) -> Result<Vec<(usize, Constraint)>, E> {
    let mut constraints = Vec::new();
    for (index, member) in fairness.iter().enumerate() {
        let strength = Strength::from(member.kind());
        let mut classes: BTreeMap<ProtocolClass, Vec<Identity>> = BTreeMap::new();
        if member.granularity() == FairnessGranularity::Whole {
            classes.insert(ProtocolClass::Whole, Vec::new());
        }
        for transition in transitions {
            if let Some(class) = member.class_of(transition) {
                classes
                    .entry(class)
                    .or_default()
                    .push(identity(transition)?);
            }
        }
        for members in classes.into_values() {
            constraints.push((index, Constraint::new(strength, members)));
        }
    }
    Ok(constraints)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn bytes(member: &ProtocolFairness) -> String {
        String::from_utf8(
            quire_canonical::to_vec(member, quire_canonical::Limits::new(4096))
                .expect("a member encodes"),
        )
        .expect("JCS is UTF-8")
    }

    /// QSpec FR-430 "Fairness set members": a scheduler constraint over a
    /// branch and over the root encode with their thread targets and the
    /// `scheduler` origin.
    #[trace("TC-657")]
    #[test]
    fn a_scheduler_constraint_encodes_its_thread_target_and_origin() {
        assert_eq!(
            bytes(&ProtocolFairness::scheduler(ProtocolTarget::branch(
                "Main/Both",
                "left"
            ))),
            r#"{"granularity":"whole","kind":"weak","origin":"scheduler","target":{"branch":"left","node":"Main/Both","type":"thread-target"}}"#
        );
        assert_eq!(
            bytes(&ProtocolFairness::scheduler(ProtocolTarget::Root)),
            r#"{"granularity":"whole","kind":"weak","origin":"scheduler","target":{"branch":null,"node":null,"type":"thread-target"}}"#
        );
    }

    /// ADR-027 PA-1: a branch target holds its own threads, not the root's
    /// and not those of a `parallel` nested inside the branch; the root
    /// target holds the root threads only. Channel steps carry no thread.
    #[trace("TC-657")]
    #[test]
    fn a_branch_target_does_not_hold_a_nested_parallel_s_threads() {
        let root = ThreadPath::root(0);
        let left = root.child("Main/Both", "left", 0);
        let nested = left.child("Main/Both/left/Inner", "x", 0);
        let branch = ProtocolFairness::scheduler(ProtocolTarget::branch("Main/Both", "left"));
        let fork = |thread: &ThreadPath, node: &str| ProtocolTransition::fork(thread.clone(), node);
        assert_eq!(
            branch.class_of(&fork(&left, "Main/Both/left/Inner")),
            Some(ProtocolClass::Whole)
        );
        assert_eq!(branch.class_of(&fork(&nested, "Deep")), None);
        assert_eq!(branch.class_of(&fork(&root, "Main/Both")), None);
        let each = ProtocolFairness::new(
            FairnessKind::Weak,
            ProtocolTarget::Root,
            FairnessGranularity::Each,
            FairnessOrigin::Authored,
        );
        assert_eq!(
            each.class_of(&fork(&root, "Main/Both")),
            Some(ProtocolClass::Thread(root.clone()))
        );
        assert_eq!(each.class_of(&fork(&left, "Main/Both/left/Inner")), None);
    }
}
