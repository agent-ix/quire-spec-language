// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124 "The deadlock-freedom item" (ADR-018 DL-3): the request writer's
//! derived item.
//!
//! For each distinct model subject among a request's temporal items the
//! writer adds one deadlock-freedom item, unless the subject's state model
//! declares `terminal any`. The item is `always holds(not deadlocked)`,
//! requested as (`temporal-satisfaction`, unbounded) and routed,
//! negotiated and settled as a [`PropertyForm::ReachableInvariant`] item.
//! Its obligation identity is the subject and the fixed kind
//! `deadlock-freedom`, so it is never an authored claim's identity over the
//! same subject.
//!
//! A protocol subject (FR-211, ADR-027 PD-3) gets its item the same way,
//! from [`protocol_deadlock_freedom_items`]: one per distinct protocol
//! subject, unless its protocol clause declares `terminal any`.

use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::check::{
    Capability, CheckedGraph, CheckedProtocol, CheckedTerminalMember, TerminalDeclaration,
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
        .terminals()
        .iter()
        .any(|terminal| matches!(terminal.declaration(), TerminalDeclaration::Any))
}

/// FR-124: the deadlock-freedom items of a request whose temporal items over
/// model subjects are `items`: one per distinct subject, in the order the
/// subjects first appear, and none for a subject whose state model declares
/// `terminal any`.
pub fn deadlock_freedom_items<S: Clone + Eq>(
    items: &[ModelTemporalItem<'_, S>],
) -> Vec<DeadlockFreedomItem<S>> {
    let mut derived: Vec<DeadlockFreedomItem<S>> = Vec::new();
    for item in items {
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
