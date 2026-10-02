// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-265 and FR-268 (ADR-031 SW-1, SW-2, SW-9, SW-13): what a state-clause
//! evaluation reports beside its outcome so a caller can derive the
//! clause's separating witness, and the separation check replay runs over
//! a claimed witness.
//!
//! The S6a evaluator keeps a [`Trail`] while it evaluates a state clause:
//! the value of the left operand of each `and`, `or` and `implies` and of
//! each `if` condition on the claim's own level, one [`StopReport`] for each
//! `forall` or `exists` there that stopped before its last element, and,
//! for each collection built or read there, where its elements are stored
//! ([`Provenance`]). The claim's own level is every node outside a
//! quantifier or fold body and outside a called function's body, which is
//! where every node of a decision path (ADR-031 SW-2) and a decisive
//! occurrence's domain (SW-4) sit, so each such node is recorded at most
//! once. No report is put inside a `Value`.

use std::collections::BTreeMap;
use std::sync::Arc;

use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
pub use qsl_foundation::witness::{
    ObservationIdentity, RuntimeValuePath, SeparationStep, ValuePathStep, ValuePathSubject,
};
use qsl_semantics::check::{CheckedGraph, Operator, SemanticTerm};
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{CollectionValue, ObjectReference, Value};
use quire_semantic_value::location::Location;

use super::super::evaluate::Evaluation;

/// QSpec FR-207's selected observation identity of `document`.
pub(crate) fn observation_identity(document: &DocumentRef) -> ObservationIdentity {
    ObservationIdentity {
        authority: document.authority.clone(),
        identity: document.identity.clone(),
        revision_namespace: document.revision_namespace.clone(),
        revision: document.revision.clone(),
    }
}

/// FR-265 (ADR-031 SW-1, SW-9): one quantifier occurrence on the claim's
/// own level that stopped before its last element: a `forall` at the first
/// element whose body is `false`, an `exists` at the first element whose
/// body is `true`.
#[derive(Clone, Debug)]
pub struct StopReport {
    /// The quantifier's occurrence key (ADR-013 O-07).
    pub quantifier: OccurrenceKey,
    /// The element's position in the stored or built collection it is
    /// bound from (QSpec FR-041-AC-4, FR-207 "Computed collections").
    pub index: u64,
    /// The value bound to the quantifier's binder at the stop.
    pub element: Value,
    /// The path to the element's location (QSpec FR-207): to the stored
    /// collection for a stored domain, and to the element itself, ending in
    /// an index step, for a collection built inside the claim.
    pub value_path: RuntimeValuePath,
}

/// FR-107 and FR-265: a state clause's evaluation, and what it reports
/// beside the outcome.
#[derive(Debug)]
pub struct ClauseEvaluation {
    /// The S6a evaluation, unchanged.
    pub evaluation: Evaluation,
    trail: Trail,
}

impl ClauseEvaluation {
    pub(crate) fn new(evaluation: Evaluation, trail: Trail) -> Self {
        Self { evaluation, trail }
    }

    /// The decision recorded at the `and`, `or`, `implies` or `if` node at
    /// `at`, on the claim's own level: the value of its left operand or of
    /// its condition. `None` when that node was not evaluated.
    pub fn decision(&self, at: &Location) -> Option<bool> {
        self.trail.decisions.get(at).copied()
    }

    /// The stop report of the quantifier node at `at`, on the claim's own
    /// level, when it stopped before its last element.
    pub fn stop(&self, at: &Location) -> Option<&StopReport> {
        self.trail.stops.get(at)
    }
}

/// FR-268: the claimed witness the separation check reads.
#[derive(Clone, Copy, Debug)]
pub struct WitnessClaim<'w> {
    /// The deciding quantifier's occurrence key.
    pub quantifier: &'w OccurrenceKey,
    /// The deciding element.
    pub element: &'w Value,
    /// The element's position in its stored or built collection.
    pub index: u64,
    /// The element's value path.
    pub value_path: &'w RuntimeValuePath,
}

/// FR-268: the separation check's answer.
#[derive(Debug)]
pub enum Separation {
    /// Every step passed: the element alone separates the clause.
    Holds,
    /// The step's requirement does not hold.
    Unmet(SeparationStep),
    /// The step's evaluation completed no value: the evaluation, which is
    /// `undefined`, refused, incomplete (an exhausted meter) or a family
    /// result.
    Stopped(SeparationStep, Box<Evaluation>),
}

/// Where a collection's elements are stored.
#[derive(Clone, Debug)]
pub(crate) struct Provenance {
    observation: ObservationIdentity,
    subject: ValuePathSubject,
    steps: Vec<ValuePathStep>,
    /// Each element's position in the stored or built collection, in the
    /// collection's own order; `None` when element `j` is at position `j`.
    positions: Option<Arc<[u64]>>,
}

impl Provenance {
    /// A collection built at `occurrence` inside the claim.
    pub(crate) fn built(observation: ObservationIdentity, occurrence: OccurrenceKey) -> Self {
        Self {
            observation,
            subject: ValuePathSubject::Built(occurrence),
            steps: Vec::new(),
            positions: None,
        }
    }

    /// A collection stored in `member` of `object`.
    pub(crate) fn member(
        observation: ObservationIdentity,
        object: ObjectReference,
        member: String,
    ) -> Self {
        Self {
            observation,
            subject: ValuePathSubject::Object(object),
            steps: vec![ValuePathStep::Member(member)],
            positions: None,
        }
    }

    /// The provenance of a collection whose element `j` is this
    /// collection's element `sources[j]`.
    pub(crate) fn select(&self, sources: &[usize]) -> Option<Self> {
        let positions = sources
            .iter()
            .map(|source| self.position(*source))
            .collect::<Option<Arc<[u64]>>>()?;
        Some(Self {
            positions: Some(positions),
            ..self.clone()
        })
    }

    /// Element `j`'s position in the stored or built collection.
    pub(crate) fn position(&self, j: usize) -> Option<u64> {
        match &self.positions {
            Some(positions) => positions.get(j).copied(),
            None => u64::try_from(j).ok(),
        }
    }

    /// The value path of the element at `position` (QSpec FR-207): the
    /// stored collection's path, or, for a built collection, the path to
    /// the element itself.
    pub(crate) fn path(&self, position: u64) -> RuntimeValuePath {
        let mut steps = self.steps.clone();
        if matches!(self.subject, ValuePathSubject::Built(_)) {
            steps.push(ValuePathStep::Index(position));
        }
        RuntimeValuePath {
            observation: self.observation.clone(),
            subject: self.subject.clone(),
            steps,
        }
    }
}

/// What the S6a evaluator records while it evaluates a state clause.
#[derive(Debug)]
pub(crate) struct Trail {
    /// The clause's own observation: `current` for an invariant, `post`
    /// for a postcondition, `pre` for a precondition.
    pub(crate) observation: ObservationIdentity,
    /// The pre observation, when admission gave one.
    pub(crate) pre: Option<ObservationIdentity>,
    decisions: BTreeMap<Location, bool>,
    stops: BTreeMap<Location, StopReport>,
    /// Each recorded collection beside its provenance; looked up by
    /// pointer identity, which a `let` binding or a stored copy keeps.
    collections: Vec<(Arc<CollectionValue>, Provenance)>,
}

impl Trail {
    pub(crate) fn new(observation: ObservationIdentity, pre: Option<ObservationIdentity>) -> Self {
        Self {
            observation,
            pre,
            decisions: BTreeMap::new(),
            stops: BTreeMap::new(),
            collections: Vec::new(),
        }
    }

    pub(crate) fn decide(&mut self, at: &Location, value: bool) {
        self.decisions.insert(at.clone(), value);
    }

    pub(crate) fn stopped(&mut self, at: &Location, report: StopReport) {
        self.stops.insert(at.clone(), report);
    }

    pub(crate) fn record(&mut self, collection: &Arc<CollectionValue>, provenance: Provenance) {
        self.collections.push((Arc::clone(collection), provenance));
    }

    pub(crate) fn provenance(&self, collection: &Arc<CollectionValue>) -> Option<&Provenance> {
        self.collections
            .iter()
            .find(|(recorded, _)| Arc::ptr_eq(recorded, collection))
            .map(|(_, provenance)| provenance)
    }
}

/// The occurrence key of a node lowered at `location` (ADR-013 O-07): the
/// `expression`-recorded occurrence there whose node is an application of
/// `operator`, or, for `None`, the first one whose node is an application of
/// any operator, else the first occurrence recorded there.
pub(crate) fn occurrence_at(
    graph: &CheckedGraph,
    location: &Location,
    operator: Option<Operator>,
) -> Option<OccurrenceKey> {
    let semantic = graph.semantic_graph();
    let applies = |key: quire_exact::NodeKey, wanted: Option<Operator>| {
        matches!(
            semantic.node(key).map(|node| node.body()),
            Some(SemanticTerm::Application { operator: applied, .. })
                if wanted.is_none_or(|wanted| *applied == wanted)
        )
    };
    let at = || graph.occurrences().filter(|(_, _, at)| *at == location);
    at().find(|(key, _, _)| applies(*key, operator))
        .or_else(|| operator.is_none().then(|| at().next()).flatten())
        .map(|(key, origin, _)| {
            OccurrenceKey::new(WireNodeId::from_digest(*key.as_bytes()), origin)
        })
}
