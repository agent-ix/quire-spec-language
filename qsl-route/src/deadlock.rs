// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124 "The deadlock-freedom item" (ADR-018 DL-3): the request writer's
//! derived item.
//!
//! For each distinct model subject among a request's temporal and state-graph
//! items the writer adds one deadlock-freedom item, unless the subject's state
//! model declares `terminal any`. The item is `always holds(not deadlocked)`,
//! requested as (`temporal-satisfaction`, unbounded) and routed, negotiated
//! and settled as a [`PropertyForm::ReachableInvariant`] item. Its obligation
//! identity is the subject and the fixed kind `deadlock-freedom`, so it is
//! never an authored claim's identity over the same subject.
//!
//! A timed subject (FR-232, ADR-026 TD-2) also gets one time-lock-freedom
//! item from [`time_lock_freedom_items`], with no opt-out.
//!
//! A protocol subject (FR-211, ADR-027 PD-3) gets its item the same way,
//! from [`protocol_deadlock_freedom_items`]: one per distinct protocol
//! subject, unless its protocol clause declares `terminal any`.
//!
//! A protocol subject with a weak `parallel` and a non-atomic shared access
//! gets one race-freedom item (FR-224, ADR-025 DR-4) from
//! [`race_freedom_items`]. It is the same shape, `always holds(not raced)`,
//! with the fixed kind `race-freedom` and the subject's resolved models in its
//! identity.

use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::MemoryModelName;
use qsl_semantics::check::{
    Capability, CheckedGraph, CheckedProtocol, CheckedTerminalMember, TerminalMode,
};
use quire_semantic_value::fairness::PropertyForm;

use crate::request::ExtentClassification;

/// One temporal item over a model subject, as the writer reads it: the
/// claim's occurrence, its subject, and the checked package the subject is
/// over.
#[derive(Clone, Copy, Debug)]
pub struct ModelTemporalItem<'g, S> {
    /// The authored claim's occurrence.
    pub occurrence: &'g OccurrenceKey,
    /// The model subject (ADR-018 SM-2).
    pub subject: &'g S,
    /// The subject's checked package.
    pub package: &'g CheckedGraph,
}

/// One state-graph item over a model subject, as the writer reads it.
///
/// State-graph claims share the model subject's derived deadlock-freedom
/// obligation with temporal claims (FR-165). The claim occurrence is not
/// part of that derived obligation.
#[derive(Clone, Copy, Debug)]
pub struct ModelStateGraphItem<'g, S> {
    /// The model subject (ADR-018 SM-2).
    pub subject: &'g S,
    /// The subject's checked package.
    pub package: &'g CheckedGraph,
}

impl<S: Clone> ModelTemporalItem<'_, S> {
    /// The item's obligation identity: its subject and its occurrence.
    pub fn obligation(&self) -> ItemObligation<S> {
        ItemObligation {
            subject: self.subject.clone(),
            kind: ObligationKind::Authored(self.occurrence.clone()),
        }
    }
}

/// What an obligation over a subject is about (ADR-013 O-09).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ObligationKind {
    /// An authored claim, by its occurrence.
    Authored(OccurrenceKey),
    /// The derived deadlock-freedom item: the fixed kind
    /// `deadlock-freedom`.
    DeadlockFreedom,
    /// The derived time-lock-freedom item (FR-232): the fixed kind
    /// `time-lock-freedom`.
    TimeLockFreedom,
    /// The derived race-freedom item (FR-224, ADR-025 DR-4): the fixed kind
    /// `race-freedom`, over the subject's resolved models, which are part of
    /// its identity (ADR-013 O-09).
    RaceFreedom {
        /// The resolved model of each outermost `parallel`, by its scoped
        /// path, in ascending path order.
        models: Vec<(String, MemoryModelName)>,
    },
}

/// An obligation identity over a model subject: the subject and the kind.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ItemObligation<S> {
    /// The subject.
    pub subject: S,
    /// The kind.
    pub kind: ObligationKind,
}

/// One derived deadlock-freedom item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeadlockFreedomItem<S> {
    subject: S,
}

impl<S: Clone> DeadlockFreedomItem<S> {
    /// The subject the item is over.
    pub fn subject(&self) -> &S {
        &self.subject
    }

    /// The item's obligation identity: the subject and `deadlock-freedom`.
    pub fn obligation(&self) -> ItemObligation<S> {
        ItemObligation {
            subject: self.subject.clone(),
            kind: ObligationKind::DeadlockFreedom,
        }
    }

    /// The requested capability kind: `temporal-satisfaction`.
    pub fn kind(&self) -> Capability {
        Capability::TemporalSatisfaction
    }

    /// The extent classification: unbounded, over the infinite-trace domain
    /// no finite bound stands for.
    pub fn extent(&self) -> ExtentClassification {
        ExtentClassification::Unbounded {
            finite_bound_available: false,
        }
    }

    /// The property form the item is routed and settled as.
    pub fn property_form(&self) -> PropertyForm {
        PropertyForm::ReachableInvariant
    }
}

/// Whether `package`'s state model opts out of the deadlock-freedom item:
/// a selected model declares `terminal any`.
fn opts_out(package: &CheckedGraph) -> bool {
    package
        .terminal_nodes()
        .iter()
        .any(|terminal| terminal.mode == TerminalMode::Any)
}

/// FR-124 and FR-165: the deadlock-freedom items of a request's temporal and
/// state-graph items over model subjects. There is one per distinct subject,
/// ordered by temporal-item order then state-graph-item order, and none for a
/// subject whose state model declares `terminal any`.
pub fn deadlock_freedom_items<S: Clone + Eq>(
    temporal: &[ModelTemporalItem<'_, S>],
    state_graph: &[ModelStateGraphItem<'_, S>],
) -> Vec<DeadlockFreedomItem<S>> {
    let mut derived: Vec<DeadlockFreedomItem<S>> = Vec::new();
    for item in temporal {
        if opts_out(item.package) || derived.iter().any(|seen| seen.subject == *item.subject) {
            continue;
        }
        derived.push(DeadlockFreedomItem {
            subject: item.subject.clone(),
        });
    }
    for item in state_graph {
        if opts_out(item.package) || derived.iter().any(|seen| seen.subject == *item.subject) {
            continue;
        }
        derived.push(DeadlockFreedomItem {
            subject: item.subject.clone(),
        });
    }
    derived
}

/// One temporal item over a protocol subject, as the writer reads it: the
/// claim's occurrence, the protocol subject, and its checked protocol
/// clause.
#[derive(Clone, Copy, Debug)]
pub struct ProtocolTemporalItem<'g, S> {
    /// The authored claim's occurrence.
    pub occurrence: &'g OccurrenceKey,
    /// The protocol subject (ADR-027 PS-1).
    pub subject: &'g S,
    /// The subject's checked protocol clause.
    pub protocol: &'g CheckedProtocol,
}

/// FR-211 (ADR-027 PD-3): the deadlock-freedom items of a request whose
/// temporal items over protocol subjects are `items`: one per distinct
/// subject, in the order the subjects first appear, and none for a subject
/// whose protocol clause declares `terminal any`.
pub fn protocol_deadlock_freedom_items<S: Clone + Eq>(
    items: &[ProtocolTemporalItem<'_, S>],
) -> Vec<DeadlockFreedomItem<S>> {
    let mut derived: Vec<DeadlockFreedomItem<S>> = Vec::new();
    for item in items {
        let opts_out = matches!(item.protocol.slots.terminal, CheckedTerminalMember::Any);
        if opts_out || derived.iter().any(|seen| seen.subject == *item.subject) {
            continue;
        }
        derived.push(DeadlockFreedomItem {
            subject: item.subject.clone(),
        });
    }
    derived
}

/// One item over a protocol subject, as the race-freedom writer reads it
/// (FR-224): the subject, the resolved model of each of its outermost
/// `parallel`s, and whether it has a non-atomic shared access (ADR-025 MA-2).
#[derive(Debug)]
pub struct ProtocolMemoryItem<'g, S> {
    /// The protocol subject.
    pub subject: &'g S,
    /// The resolved model of each outermost `parallel`, by scoped path.
    pub models: &'g [(String, MemoryModelName)],
    /// Whether some access of a weak `parallel` loads or stores a shared
    /// location with no `ordering`.
    pub non_atomic_access: bool,
}

impl<S> Clone for ProtocolMemoryItem<'_, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ProtocolMemoryItem<'_, S> {}

/// One derived race-freedom item (FR-224).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RaceFreedomItem<S> {
    subject: S,
    models: Vec<(String, MemoryModelName)>,
}

impl<S: Clone> RaceFreedomItem<S> {
    /// The subject the item is over.
    pub fn subject(&self) -> &S {
        &self.subject
    }

    /// The item's obligation identity: the subject, its resolved models and
    /// `race-freedom`.
    pub fn obligation(&self) -> ItemObligation<S> {
        ItemObligation {
            subject: self.subject.clone(),
            kind: ObligationKind::RaceFreedom {
                models: self.models.clone(),
            },
        }
    }

    /// The requested capability kind: `temporal-satisfaction`.
    pub fn kind(&self) -> Capability {
        Capability::TemporalSatisfaction
    }

    /// The extent classification: unbounded, as the deadlock-freedom item's.
    pub fn extent(&self) -> ExtentClassification {
        ExtentClassification::Unbounded {
            finite_bound_available: false,
        }
    }

    /// The property form the item is routed and settled as.
    pub fn property_form(&self) -> PropertyForm {
        PropertyForm::ReachableInvariant
    }
}

/// FR-224 (ADR-025 DR-4): the race-freedom items of a request whose items
/// over protocol subjects are `items`: one per distinct (subject, resolved
/// models) among those with a weak resolved model and a non-atomic shared
/// access, in the order they first appear, and none for any other subject.
pub fn race_freedom_items<S: Clone + Eq>(
    items: &[ProtocolMemoryItem<'_, S>],
) -> Vec<RaceFreedomItem<S>> {
    let mut derived: Vec<RaceFreedomItem<S>> = Vec::new();
    for item in items {
        let weak = item.models.iter().any(|(_, model)| model.is_weak());
        let seen = derived
            .iter()
            .any(|seen| seen.subject == *item.subject && seen.models == item.models);
        if !weak || !item.non_atomic_access || seen {
            continue;
        }
        derived.push(RaceFreedomItem {
            subject: item.subject.clone(),
            models: item.models.to_vec(),
        });
    }
    derived
}

/// FR-232 (ADR-026 TD-2): one derived time-lock-freedom item, over a timed
/// subject. It is requested, routed and negotiated as an unbounded temporal
/// item, like the deadlock-freedom item, and settled by FR-235.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeLockFreedomItem<S> {
    subject: S,
}

impl<S: Clone> TimeLockFreedomItem<S> {
    /// The subject the item is over.
    pub fn subject(&self) -> &S {
        &self.subject
    }

    /// The item's obligation identity: the subject and `time-lock-freedom`.
    pub fn obligation(&self) -> ItemObligation<S> {
        ItemObligation {
            subject: self.subject.clone(),
            kind: ObligationKind::TimeLockFreedom,
        }
    }

    /// The requested capability kind: `temporal-satisfaction`.
    pub fn kind(&self) -> Capability {
        Capability::TemporalSatisfaction
    }

    /// The extent classification: unbounded.
    pub fn extent(&self) -> ExtentClassification {
        ExtentClassification::Unbounded {
            finite_bound_available: false,
        }
    }
}

/// Whether a subject over the models `models` of `package` is timed: one of
/// them has a `time` member (ADR-026 TS-1). Another model of the package
/// being timed does not make it so.
fn is_timed(package: &CheckedGraph, models: &[String]) -> bool {
    package
        .time_sources()
        .iter()
        .any(|source| models.contains(&source.model))
}

/// FR-232: the time-lock-freedom items of a request's temporal and
/// state-graph items over model subjects: one per distinct timed subject,
/// in temporal-item then state-graph-item order. `models_of` gives the
/// model aliases a subject ranges over. There is no opt-out: a `terminal`
/// member does not remove it.
pub fn time_lock_freedom_items<S: Clone + Eq>(
    temporal: &[ModelTemporalItem<'_, S>],
    state_graph: &[ModelStateGraphItem<'_, S>],
    models_of: impl Fn(&S) -> Vec<String>,
) -> Vec<TimeLockFreedomItem<S>> {
    let subjects = temporal
        .iter()
        .map(|item| (item.subject, item.package))
        .chain(state_graph.iter().map(|item| (item.subject, item.package)));
    let mut derived: Vec<TimeLockFreedomItem<S>> = Vec::new();
    for (subject, package) in subjects {
        if is_timed(package, &models_of(subject))
            && derived.iter().all(|seen| seen.subject != *subject)
        {
            derived.push(TimeLockFreedomItem {
                subject: subject.clone(),
            });
        }
    }
    derived
}
