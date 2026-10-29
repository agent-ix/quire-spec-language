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
//! A refinement item (FR-144) counts as a temporal item over its concrete
//! subject: it brings that subject's deadlock-freedom item unless the
//! concrete model itself declares `terminal any`, and never one for the
//! abstract subject (ADR-020 RS-9).
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
//!
//! Which subjects get which item is read from each caller item's
//! [`SubjectReading`]: the selections its subject reads, and whether one of
//! their models opts out, is timed or is dense. The request writer,
//! [`crate::model_request::ModelRequest::derived`], writes the items in
//! QSpec FR-368's order and anchors each at its subject's `model_root`.

use std::collections::BTreeSet;

use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::MemoryModelName;
use qsl_semantics::check::{
    Capability, CheckedGraph, CheckedProtocol, CheckedRefinement, CheckedTerminalMember,
    TerminalDeclaration, TimeSourceKind,
};
use qsl_semantics::model::observation::DocumentRef;
use quire_semantic_value::fairness::PropertyForm;

use crate::reductions::{ObligationReductions, Reductions};
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
            reductions: ObligationReductions::default(),
        }
    }
}

/// One refinement item (FR-144), as the writer reads it: the checked
/// refinement, its concrete subject, the abstract subject's initial
/// snapshots and the checked package the refinement is declared in.
#[derive(Clone, Copy, Debug)]
pub struct ModelRefinementItem<'g, S> {
    /// The checked refinement.
    pub refinement: &'g CheckedRefinement,
    /// The concrete subject (ADR-018 SM-2), which the item is over.
    pub subject: &'g S,
    /// The abstract subject's FR-106 initial snapshots.
    pub abstract_initial: &'g [DocumentRef],
    /// The declaring package.
    pub package: &'g CheckedGraph,
}

impl<S: Clone> ModelRefinementItem<'_, S> {
    /// The occurrence keying the refinement's requirement record; `None`
    /// for a refinement the package wrote no record for.
    pub fn occurrence(&self) -> Option<&OccurrenceKey> {
        self.refinement.requirement_occurrence()
    }

    /// The item's obligation identity (FR-144, ADR-013 O-09): the concrete
    /// subject, the refinement's occurrence, which names its node, and the
    /// abstract subject's initial snapshots, so a verdict holds for exactly
    /// that concrete subject against that abstract subject. `None` for a
    /// refinement with no record.
    pub fn obligation(&self) -> Option<ItemObligation<S>> {
        Some(ItemObligation {
            subject: self.subject.clone(),
            kind: ObligationKind::Refinement {
                occurrence: self.occurrence()?.clone(),
                abstract_initial: self.abstract_initial.iter().cloned().collect(),
            },
            reductions: ObligationReductions::default(),
        })
    }
}

/// What an obligation over a subject is about (ADR-013 O-09).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ObligationKind {
    /// An authored claim, by its occurrence.
    Authored(OccurrenceKey),
    /// A refinement (FR-144), by its record's occurrence and the abstract
    /// subject's initial snapshots, in ascending order.
    Refinement {
        /// The occurrence of the refinement's record.
        occurrence: OccurrenceKey,
        /// The abstract subject's initial snapshots.
        abstract_initial: BTreeSet<DocumentRef>,
    },
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

/// An obligation identity over a model subject: the subject, the kind
/// and, beside the subject, the reductions the request selects (FR-159
/// "Identity"), so a reduced and an unreduced request for one claim are
/// different obligations.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ItemObligation<S> {
    /// The subject.
    pub subject: S,
    /// The kind.
    pub kind: ObligationKind,
    /// The symmetry declarations and partial-order selection; none for an
    /// unreduced request. The constraint stays out (FR-158).
    pub reductions: ObligationReductions,
}

impl<S> ItemObligation<S> {
    /// This obligation as selected with `reductions` (FR-159 "Identity").
    #[must_use]
    pub fn reduced(self, reductions: &Reductions) -> Self {
        Self {
            reductions: reductions.obligation_identity(),
            ..self
        }
    }
}

/// One derived deadlock-freedom item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeadlockFreedomItem<S> {
    subject: S,
    dense_time: bool,
}

impl<S: Clone> DeadlockFreedomItem<S> {
    /// The subject the item is over.
    pub fn subject(&self) -> &S {
        &self.subject
    }

    /// Whether the subject is `time dense` (FR-243): one of its models
    /// declares `time dense`, so the item routes to EN-6
    /// ([`crate::Registry::restrict_to_dense_time`]).
    pub fn dense_time(&self) -> bool {
        self.dense_time
    }

    /// The item's obligation identity: the subject and `deadlock-freedom`.
    pub fn obligation(&self) -> ItemObligation<S> {
        ItemObligation {
            subject: self.subject.clone(),
            kind: ObligationKind::DeadlockFreedom,
            reductions: ObligationReductions::default(),
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

/// What the request writer reads about the subject of one caller item to
/// derive the subject's items (FR-124, FR-232, FR-243, QSpec FR-368): the
/// `model_root` node of each `model_selections` row the subject reads, and
/// whether one of the models it reads declares `terminal any`, has a `time`
/// member, or declares `time dense`. Only a subject's own models count:
/// another model of the package has no effect on it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SubjectReading {
    roots: BTreeSet<WireNodeId>,
    opted_out: bool,
    timed: bool,
    dense: bool,
}

impl SubjectReading {
    /// The reading of a subject over `package` that reads the
    /// `model_selections` rows whose identities `reads` holds for.
    pub fn read(package: &CheckedGraph, reads: impl Fn(&str) -> bool) -> Self {
        let own = |model: &str| package.model_package_identity(model).is_some_and(&reads);
        Self {
            roots: package
                .model_roots()
                .filter(|(selection, _)| reads(&selection.identity))
                .map(|(_, key)| WireNodeId::from_digest(*key.as_bytes()))
                .collect(),
            opted_out: package.terminals().iter().any(|terminal| {
                own(terminal.model()) && matches!(terminal.declaration(), TerminalDeclaration::Any)
            }),
            timed: package
                .time_sources()
                .iter()
                .any(|source| own(&source.model)),
            dense: package
                .time_sources()
                .iter()
                .any(|source| own(&source.model) && source.source == TimeSourceKind::Dense),
        }
    }

    /// The reading of a subject over the model aliases `models` of
    /// `package`.
    fn of_models(package: &CheckedGraph, models: &[String]) -> Self {
        Self::read(package, |identity| {
            models
                .iter()
                .any(|model| package.model_package_identity(model) == Some(identity))
        })
    }

    /// Whether a model the subject reads declares `terminal any`, which
    /// opts it out of the deadlock-freedom item (FR-124).
    pub fn opted_out(&self) -> bool {
        self.opted_out
    }

    /// Whether a model the subject reads is timed (ADR-026 TS-1), which
    /// gives it the time-lock-freedom item (FR-232).
    pub fn timed(&self) -> bool {
        self.timed
    }

    /// Whether a model the subject reads declares `time dense` (FR-243).
    pub fn dense(&self) -> bool {
        self.dense
    }

    /// The reading of a subject that two caller items read: the union of
    /// their rows and marks.
    fn merge(&mut self, other: &Self) {
        self.roots.extend(other.roots.iter().copied());
        self.opted_out |= other.opted_out;
        self.timed |= other.timed;
        self.dense |= other.dense;
    }

    /// QSpec FR-368 "Claim and anchor of a derived or refinement item": the
    /// claim of a derived item over the subject, the `model_root` node of
    /// the one selection it reads and that node's one occurrence, of role
    /// `generated` and ordinal 0 (QSpec FR-322-AC-132). A subject that
    /// reads another number of rows has no stated anchor, and the count is
    /// returned.
    pub fn anchor(&self) -> Result<OccurrenceKey, usize> {
        let mut roots = self.roots.iter();
        match (roots.next(), roots.next()) {
            (Some(root), None) => Ok(OccurrenceKey::generated(*root)),
            _ => Err(self.roots.len()),
        }
    }
}

/// The subjects of `readings`, in the order each first occurs, each with
/// the merged reading of every caller item over it (QSpec FR-368
/// "Position of derived items in the request").
pub(crate) fn by_subject<S: Clone + Eq>(
    readings: impl IntoIterator<Item = (S, SubjectReading)>,
) -> Vec<(S, SubjectReading)> {
    let mut subjects: Vec<(S, SubjectReading)> = Vec::new();
    for (subject, reading) in readings {
        match subjects.iter_mut().find(|(seen, _)| *seen == subject) {
            Some((_, seen)) => seen.merge(&reading),
            None => subjects.push((subject, reading)),
        }
    }
    subjects
}

impl<S: Clone> ModelRefinementItem<'_, S> {
    /// The reading of the refinement's concrete subject: the concrete
    /// model's selection alone, so the abstract model brings no item and
    /// opts nothing out (ADR-020 RS-9).
    pub fn reading(&self) -> SubjectReading {
        let concrete = &self.refinement.concrete_model_selection().identity;
        SubjectReading::read(self.package, |identity| identity == concrete)
    }
}

/// FR-124, FR-144 and FR-165: the deadlock-freedom items of a request's
/// temporal, refinement and state-graph items over model subjects. There is
/// one per distinct subject, ordered by temporal-item order, then
/// refinement-item order, then state-graph-item order, and none for a subject
/// one of whose own models declares `terminal any`. `models_of` gives the
/// model aliases a subject ranges over, as [`time_lock_freedom_items`]
/// reads them. A refinement item brings its concrete subject's item, opted
/// out only by its concrete model's `terminal any`, and none for its
/// abstract subject (ADR-020 RS-9).
pub fn deadlock_freedom_items<S: Clone + Eq>(
    temporal: &[ModelTemporalItem<'_, S>],
    refinements: &[ModelRefinementItem<'_, S>],
    state_graph: &[ModelStateGraphItem<'_, S>],
    models_of: impl Fn(&S) -> Vec<String>,
) -> Vec<DeadlockFreedomItem<S>> {
    let temporal = temporal.iter().map(|item| {
        let reading = SubjectReading::of_models(item.package, &models_of(item.subject));
        (item.subject.clone(), reading)
    });
    let refinements = refinements
        .iter()
        .map(|item| (item.subject.clone(), item.reading()));
    let state_graph = state_graph.iter().map(|item| {
        let reading = SubjectReading::of_models(item.package, &models_of(item.subject));
        (item.subject.clone(), reading)
    });
    by_subject(temporal.chain(refinements).chain(state_graph))
        .into_iter()
        .filter(|(_, reading)| !reading.opted_out)
        .map(|(subject, reading)| DeadlockFreedomItem {
            subject,
            dense_time: reading.dense,
        })
        .collect()
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
            dense_time: false,
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
            reductions: ObligationReductions::default(),
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
    dense_time: bool,
}

impl<S: Clone> TimeLockFreedomItem<S> {
    /// The subject the item is over.
    pub fn subject(&self) -> &S {
        &self.subject
    }

    /// Whether the subject is `time dense` (FR-243): one of its models
    /// declares `time dense`, so the item routes to EN-6
    /// ([`crate::Registry::restrict_to_dense_time`]).
    pub fn dense_time(&self) -> bool {
        self.dense_time
    }

    /// The item's obligation identity: the subject and `time-lock-freedom`.
    pub fn obligation(&self) -> ItemObligation<S> {
        ItemObligation {
            subject: self.subject.clone(),
            kind: ObligationKind::TimeLockFreedom,
            reductions: ObligationReductions::default(),
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
    let readings = temporal
        .iter()
        .map(|item| (item.subject, item.package))
        .chain(state_graph.iter().map(|item| (item.subject, item.package)))
        .map(|(subject, package)| {
            let reading = SubjectReading::of_models(package, &models_of(subject));
            (subject.clone(), reading)
        });
    by_subject(readings)
        .into_iter()
        .filter(|(_, reading)| reading.timed)
        .map(|(subject, reading)| TimeLockFreedomItem {
            subject,
            dense_time: reading.dense,
        })
        .collect()
}
