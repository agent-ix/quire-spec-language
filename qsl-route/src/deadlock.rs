// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124 "The deadlock-freedom item" (ADR-018 DL-3): the request writer's
//! derived item.
//!
//! For each distinct model subject among a request's temporal and state-graph
//! items the writer adds one deadlock-freedom item, unless the subject's state
//! model declares `terminal any`. The item is `always holds(not deadlocked)`,
//! requested as (`temporal-satisfaction`, unbounded) and routed, negotiated
//! and settled as a [`ReachableInvariant`] item. Its obligation identity is
//! the subject and the fixed kind `deadlock-freedom`, so it is never an
//! authored claim's identity over the same subject.
//!
//! A refinement item (FR-144) counts as a temporal item over its concrete
//! subject: it brings that subject's deadlock-freedom item unless the
//! concrete model itself declares `terminal any`, and never one for the
//! abstract subject (ADR-020 RS-9).
//!
//! A timed subject (FR-232, ADR-026 TD-2) also gets one time-lock-freedom
//! item, with no opt-out.
//!
//! A protocol subject (FR-211, ADR-027 PD-3; QSpec FR-429) gets its
//! deadlock-freedom item the same way, unless its protocol clause declares
//! `terminal any`; the model's own `terminal` is not read for it. A protocol
//! subject with a weak `parallel` and a non-atomic shared access also gets
//! one race-freedom item (FR-224, ADR-025 DR-4; QSpec FR-438), `always
//! holds(not raced)` with the fixed kind `race-freedom` and the subject's
//! resolved models in its identity. A protocol subject and a model subject
//! over the same model are two subjects.
//!
//! Which subjects get which item is read from each caller item's
//! [`SubjectReading`]: for a model subject, the selections it reads, and
//! whether one of their models opts out, is timed or is dense; for a
//! protocol subject, its clause's node, `terminal` member and memory. The
//! request writer, [`crate::model_request::ModelRequest::derived`], writes
//! the items in QSpec FR-368's order and anchors each at its subject's
//! `model_root`, or at a protocol subject's clause node.
//!
//! [`ReachableInvariant`]: quire_semantic_value::fairness::PropertyForm::ReachableInvariant

use std::collections::BTreeSet;

use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::MemoryModelName;
use qsl_semantics::check::{
    CheckedGraph, CheckedProtocol, CheckedRefinement, CheckedTerminalMember, TerminalDeclaration,
};
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::NodeKey;

use crate::reductions::{ObligationReductions, Reductions};

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
    /// An authored claim over a protocol subject, by its occurrence and the
    /// subject's resolved models, which are part of the protocol subject
    /// (QSpec FR-425) and so of its identity.
    ProtocolAuthored {
        /// The claim's occurrence.
        occurrence: OccurrenceKey,
        /// The resolved model of each outermost `parallel`, by its scoped
        /// path, in ascending path order.
        models: Vec<(String, MemoryModelName)>,
    },
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
    /// The derived deadlock-freedom item of a protocol subject (QSpec
    /// FR-429): the fixed kind `deadlock-freedom`, over the subject's
    /// resolved models, which are part of the protocol subject (QSpec
    /// FR-425) and so of its identity.
    ProtocolDeadlockFreedom {
        /// The resolved model of each outermost `parallel`, by its scoped
        /// path, in ascending path order.
        models: Vec<(String, MemoryModelName)>,
    },
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

/// What the request writer reads about the subject of one caller item to
/// derive the subject's items (FR-124, FR-232, FR-243, QSpec FR-368): the
/// `model_root` node of each `model_selections` row the subject reads, and
/// whether one of the models it reads declares `terminal any` or has a
/// `time` member. Only a subject's own models count:
/// another model of the package has no effect on it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SubjectReading {
    roots: BTreeSet<NodeKey>,
    opted_out: bool,
    timed: bool,
    /// For a protocol subject, what its clause gives the derivation.
    protocol: Option<ProtocolReading>,
}

/// What a protocol subject's clause gives the derivation (QSpec FR-368,
/// FR-438).
#[derive(Clone, Debug, Eq, PartialEq)]
struct ProtocolReading {
    /// The clause's node; `None` for a clause with no node. A subject names
    /// one protocol clause, so every reading of it has the same node.
    node: Option<NodeKey>,
    /// The resolved model of each outermost `parallel`, by scoped path, in
    /// ascending path order. They are part of the protocol subject (QSpec
    /// FR-425), so two readings with other models are two subjects.
    models: Vec<(String, MemoryModelName)>,
    /// Whether a resolved model is weak and an access is non-atomic, which
    /// gives the subject its race-freedom item (FR-224, QSpec FR-438).
    races: bool,
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
                .map(|(_, key)| key)
                .collect(),
            opted_out: package.terminals().iter().any(|terminal| {
                own(terminal.model()) && matches!(terminal.declaration(), TerminalDeclaration::Any)
            }),
            timed: package
                .time_sources()
                .iter()
                .any(|source| own(&source.model)),
            protocol: None,
        }
    }

    /// Whether a model the subject reads declares `terminal any`, which
    /// opts it out of the deadlock-freedom item (FR-124).
    pub fn opted_out(&self) -> bool {
        self.opted_out
    }

    /// Whether a model the subject reads is timed (ADR-026 TS-1), which
    /// gives it the time-lock-freedom item (FR-232) and routes its derived
    /// items to EN-6 alone (FR-243).
    pub fn timed(&self) -> bool {
        self.timed
    }

    /// The reading of a subject that two caller items read: the union of
    /// their rows and marks.
    fn merge(&mut self, other: &Self) {
        self.roots.extend(other.roots.iter().copied());
        self.opted_out |= other.opted_out;
        self.timed |= other.timed;
        match (&mut self.protocol, &other.protocol) {
            (None, Some(protocol)) => self.protocol = Some(protocol.clone()),
            (Some(seen), Some(protocol)) => {
                seen.node = seen.node.or(protocol.node);
                seen.races |= protocol.races;
            }
            (_, None) => {}
        }
    }

    /// A protocol subject's resolved models (QSpec FR-425); `None` for
    /// every other subject.
    pub fn protocol_models(&self) -> Option<&[(String, MemoryModelName)]> {
        self.protocol
            .as_ref()
            .map(|protocol| protocol.models.as_slice())
    }

    /// Whether a protocol subject gets the race-freedom item: a resolved
    /// model is weak and an access is non-atomic (FR-224, QSpec FR-438).
    pub fn races(&self) -> bool {
        self.protocol
            .as_ref()
            .is_some_and(|protocol| protocol.races)
    }

    /// QSpec FR-368 "Claim and anchor of a derived or refinement item": the
    /// claim of a derived item over the subject. A protocol subject's is its
    /// clause node's `declaration` occurrence at ordinal 0. Any other
    /// subject's is the `model_root` node of
    /// the one selection it reads and that node's one occurrence, of role
    /// `generated` and ordinal 0 (QSpec FR-322-AC-132). A subject that
    /// reads no row or several has no anchor (QSpec FR-361-AC-17).
    pub fn anchor(&self) -> Result<OccurrenceKey, NoAnchor> {
        if let Some(protocol) = &self.protocol {
            return protocol
                .node
                .map(OccurrenceKey::declaration)
                .ok_or(NoAnchor::NoProtocolProfile);
        }
        let mut roots = self.roots.iter();
        match (roots.next(), roots.next()) {
            (Some(root), None) => Ok(OccurrenceKey::generated(*root)),
            (None, _) => Err(NoAnchor::NoSelection),
            (Some(_), Some(_)) => Err(NoAnchor::SeveralSelections {
                rows: self.roots.len(),
            }),
        }
    }
}

/// Why a subject has no anchor for its derived items: a model subject
/// reads not exactly one `model_selections` row (QSpec FR-361-AC-17,
/// FR-368-AC-22), or a protocol subject's unit selects no protocol profile
/// (FR-368-AC-24).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoAnchor {
    /// The subject reads no row: no universe of it covers a population of a
    /// selected domain package.
    NoSelection,
    /// The subject reads several rows.
    SeveralSelections {
        /// The number of rows it reads.
        rows: usize,
    },
    /// The protocol subject's unit selects no protocol profile, so its
    /// checked clause has no node (QSpec FR-368-AC-24, FR-322
    /// "Properties").
    NoProtocolProfile,
}

/// The subjects of `readings`, in the order each first occurs, each with
/// the merged reading of every caller item over it (QSpec FR-368
/// "Position of derived items in the request"). A protocol subject's
/// resolved models are part of it (QSpec FR-425), so readings of one
/// caller subject under other models, or one read as a model subject and
/// one as a protocol subject, are other subjects.
pub(crate) fn by_subject<S: Clone + Eq>(
    readings: impl IntoIterator<Item = (S, SubjectReading)>,
) -> Vec<(S, SubjectReading)> {
    let mut subjects: Vec<(S, SubjectReading)> = Vec::new();
    for (subject, reading) in readings {
        match subjects.iter_mut().find(|(seen, known)| {
            *seen == subject && known.protocol_models() == reading.protocol_models()
        }) {
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

/// One temporal item over a protocol subject, as the writer reads it: the
/// claim's occurrence, the protocol subject, its checked protocol clause,
/// the resolved model of each of its outermost `parallel`s, and whether it
/// has a non-atomic shared access (ADR-025 MA-2).
#[derive(Debug)]
pub struct ProtocolTemporalItem<'g, S> {
    /// The authored claim's occurrence.
    pub occurrence: &'g OccurrenceKey,
    /// The protocol subject (ADR-027 PS-1).
    pub subject: &'g S,
    /// The subject's checked protocol clause.
    pub protocol: &'g CheckedProtocol,
    /// The resolved model of each outermost `parallel`, by scoped path, in
    /// ascending path order.
    pub models: &'g [(String, MemoryModelName)],
    /// Whether some access of a weak `parallel` loads or stores a shared
    /// location with no `ordering`.
    pub non_atomic_access: bool,
}

impl<S> Clone for ProtocolTemporalItem<'_, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ProtocolTemporalItem<'_, S> {}

impl<S: Clone> ProtocolTemporalItem<'_, S> {
    /// The item's obligation identity: its subject, its resolved models,
    /// which are part of the protocol subject (QSpec FR-425), and its
    /// occurrence.
    pub fn obligation(&self) -> ItemObligation<S> {
        ItemObligation {
            subject: self.subject.clone(),
            kind: ObligationKind::ProtocolAuthored {
                occurrence: self.occurrence.clone(),
                models: self.models.to_vec(),
            },
            reductions: ObligationReductions::default(),
        }
    }

    /// The reading of the item's protocol subject (QSpec FR-429, FR-438):
    /// its clause's node, whether the clause declares `terminal any`, and,
    /// when a resolved model is weak and an access is non-atomic, the
    /// models the race-freedom item is over. A protocol subject is never
    /// read as timed, so it gets no `time-lock-freedom` item (QSpec
    /// FR-368-AC-25 for a unit that selects no protocol profile), and its
    /// model's `terminal` is not read.
    pub fn reading(&self) -> SubjectReading {
        let weak = self.models.iter().any(|(_, model)| model.is_weak());
        SubjectReading {
            roots: BTreeSet::new(),
            opted_out: matches!(self.protocol.slots.terminal, CheckedTerminalMember::Any),
            timed: false,
            protocol: Some(ProtocolReading {
                node: self.protocol.node(),
                models: self.models.to_vec(),
                races: weak && self.non_atomic_access,
            }),
        }
    }
}
