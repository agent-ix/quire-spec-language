// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-516: the typed request and result of a temporal evidence call. There
//! is no JSON document, schema or content-derived identity string: these are
//! Rust values, and the evaluator takes the clause's profile, clock row,
//! activation, captures and formula from the checked package, never from the
//! request (FR-337).

use std::collections::BTreeMap;

use qsl_package::{CheckedNodeId, CheckedOccurrence, CheckedSelection, TemporalBinding};
use qsl_semantics::check::CheckedTemporalClause;

use super::activation::{ExecutionDisposition, Instance};
use super::binding::{AuthenticatedBinding, TraceBinding};
use super::counters::LimitReached;
use super::identity::{InputIdentity, ResultIdentity};
use super::ledger::EvidenceLedger;
use super::limits::{EvidenceUsage, TemporalEvidenceLimits};
use super::trigger::{
    EmptyIdentity, Evicted, ExecutionOrigin, SemanticTriggerIdentity, TriggerDelivery,
    TriggerEvidence,
};
use crate::temporal::finite_window::{ClockCover, OrderKey, Valuation};
use crate::temporal::ledger::{
    LedgerBinding, ProgressClosure, ProgressCompleteness, ProgressLedger,
};

/// The observation owner's reference to a piece of evidence (FR-516
/// `EvidenceRef`). `contract`, `schema_digest`, `digest` and
/// `authority_digest` are opaque caller-supplied members: QSL does not
/// compute, verify or interpret them, echoes each byte for byte, and no
/// truth, settlement, support or activation value depends on them.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EvidenceRef {
    /// The evidence's identity.
    pub identity: String,
    /// The authority's identity.
    pub authority_identity: String,
    /// The authority's revision.
    pub authority_revision: String,
    /// The evidence's scope.
    pub scope: String,
    /// The population the evidence covers.
    pub population: String,
    /// The contract text.
    pub contract: String,
    /// The schema digest, opaque.
    pub schema_digest: Vec<u8>,
    /// The evidence digest, opaque.
    pub digest: Vec<u8>,
    /// The authority digest, opaque.
    pub authority_digest: Vec<u8>,
}

impl EvidenceRef {
    /// A reference with only an identity, the other members empty.
    pub fn named(identity: impl Into<String>) -> Self {
        Self {
            identity: identity.into(),
            ..Self::default()
        }
    }

    /// The byte lengths of the opaque members.
    pub(super) fn member_lengths(&self) -> [usize; 8] {
        [
            self.identity.len(),
            self.authority_identity.len(),
            self.authority_revision.len(),
            self.scope.len(),
            self.population.len(),
            self.contract.len(),
            self.schema_digest.len() + self.digest.len(),
            self.authority_digest.len(),
        ]
    }
}

/// One observed position of the request (FR-516 `positions`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EvidencePosition {
    /// The observation.
    pub observation: EvidenceRef,
    /// The position's stamp: its tick under the timestamped-event clock.
    pub position: u64,
    /// The order key, when the observation has one.
    pub order_key: Option<OrderKey>,
    /// The valuation of each atom (by the atom's node index in the clause's
    /// formula) that does not depend on an instance. An absent atom has no
    /// valuation at the position.
    pub valuations: BTreeMap<u64, Valuation>,
    /// The valuations that belong to one instance, by its trigger identity
    /// (FR-524 "Instance-bound valuations").
    pub instance_valuations: BTreeMap<SemanticTriggerIdentity, BTreeMap<u64, Valuation>>,
}

/// The completeness record (FR-516 `completeness`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletenessRecord {
    /// The record's reference.
    pub reference: EvidenceRef,
    /// Complete through a tick, or incomplete.
    pub state: ProgressCompleteness,
    /// The facts the record rests on.
    pub facts: Vec<String>,
}

/// The relation of a result to an earlier one (FR-526).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Relation {
    /// The first result of a chain.
    Original,
    /// The result replaces the predecessor with this `result_identity`.
    Superseding(ResultIdentity),
    /// The result retracts the predecessor with this `result_identity`.
    Invalidating(ResultIdentity),
}

/// The request of one temporal evidence call (FR-516 "Inputs"). It has no
/// truth, settlement, support or instance-text member:
///
/// ```compile_fail,E0560
/// use qsl_eval::temporal::evidence::EvidenceRequest;
/// fn build(request: EvidenceRequest) -> EvidenceRequest {
///     EvidenceRequest { truth: None, ..request }
/// }
/// ```
#[derive(Clone, Debug)]
pub struct EvidenceRequest {
    /// The instance the call assesses among the resolved ones.
    pub trigger: Option<SemanticTriggerIdentity>,
    /// The correspondence evidence.
    pub correspondence: EvidenceRef,
    /// The observed positions.
    pub positions: Vec<EvidencePosition>,
    /// The identity of the position the instance activates at.
    pub anchor: Option<String>,
    /// The ordered deliveries of a clause that activates `on each`.
    pub deliveries: Vec<TriggerDelivery>,
    /// The ordered origins of a clause that activates `on origin`.
    pub origins: Vec<ExecutionOrigin>,
    /// The trigger evidence.
    pub trigger_evidence: TriggerEvidence,
    /// The trigger scope's closure.
    pub trigger_scope: ProgressClosure,
    /// The decision scope's watermark.
    pub decision_progress: Option<u64>,
    /// The decision scope's closure.
    pub decision_closure: Option<ProgressClosure>,
    /// The surrounding execution's watermark.
    pub surrounding_progress: Option<u64>,
    /// The surrounding execution's closure.
    pub surrounding_closure: Option<ProgressClosure>,
    /// The surrounding execution's disposition.
    pub execution: ExecutionDisposition,
    /// The completeness record.
    pub completeness: Option<CompletenessRecord>,
    /// Whether history before the first position is empty.
    pub authoritative_origin: Option<bool>,
    /// The retained records the caller reports as gone.
    pub evicted: Vec<Evicted>,
    /// The clock cover of a stamp.
    pub cover: ClockCover,
    /// The caller's progress ledger.
    pub progress_ledger: Option<ProgressLedger>,
    /// The caller's ledger of retained instances and latest results.
    pub ledger: Option<EvidenceLedger>,
    /// The relation to an earlier result.
    pub relation: Relation,
    /// The protocol clause the caller selected.
    pub control: Option<CheckedNodeId>,
    /// The profile, clock name and parameters the request asserts.
    pub binding: TraceBinding,
    /// The occurrence of the declaration the call assesses; may be omitted
    /// for a node with one occurrence (FR-516).
    pub occurrence: Option<CheckedOccurrence>,
    /// The thirteen ceilings.
    pub limits: TemporalEvidenceLimits,
}

impl EvidenceRequest {
    /// The trigger identity of `bytes`, refused when empty (FR-053).
    ///
    /// # Errors
    ///
    /// [`EmptyIdentity`].
    pub fn trigger_of(bytes: &[u8]) -> Result<SemanticTriggerIdentity, EmptyIdentity> {
        SemanticTriggerIdentity::new(bytes)
    }
}

/// The activation state of a result (FR-516 "Outputs").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ActivationState {
    /// No trigger was admitted within a closed, complete scope.
    Inactive,
    /// The scope was open, or its evidence missing or refused.
    Unknown,
    /// A trigger was admitted or the origin activated the obligation.
    Active,
    /// A fact that decides activation was withdrawn, the assessed instance
    /// has a capture or guard fault, or the obligation was never activated.
    Unavailable,
}

/// A temporal truth value of a result.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Truth {
    /// True.
    True,
    /// False.
    False,
    /// Declared progress could still resolve it.
    Pending,
}

/// What a settled result rests on (FR-516 `settlement`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Settlement {
    /// The scope is closed.
    ClosedScope,
    /// A decisive witness.
    DecisiveWitness,
    /// A decisive counterexample.
    DecisiveCounterexample,
    /// Unsettled.
    Unsettled,
    /// No settlement is available.
    Unavailable,
}

/// The kind of a non-value (FR-516 `non_value.kind`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NonValueKind {
    /// Inactive.
    Inactive,
    /// The activation is unknown.
    ActivationUnknown,
    /// A fact is missing.
    Missing,
    /// A fact is refused.
    Refused,
    /// The obligation was never activated.
    Unactivated,
}

/// The reason a result has no value for something (FR-516 `non_value`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NonValue {
    /// The kind.
    pub kind: NonValueKind,
    /// The code.
    pub code: &'static str,
    /// The dimension or input the non-value names.
    pub dimension: Option<String>,
}

/// A fact reference inside a decision support.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportRef {
    /// The observation the fact was read from.
    pub observation: EvidenceRef,
    /// The atom's node index.
    pub atom: u64,
}

/// Why a closed window's verdict stays open, with the instance a missing
/// valuation names (FR-524 "Instance-bound atoms"; FR-501).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Indeterminate {
    /// An atom's valuation is absent at an eligible position.
    MissingValuation {
        /// The position's index in canonical order.
        position: u64,
        /// The atom.
        atom: u64,
        /// The assessed instance, for an instance-bound atom.
        instance: Option<SemanticTriggerIdentity>,
        /// The other instances that hold an entry for the atom at the
        /// position.
        also_held_by: Vec<SemanticTriggerIdentity>,
    },
    /// Two differing valuations of an atom were admitted at one position.
    ConflictingValuation {
        /// The position.
        position: u64,
        /// The atom.
        atom: u64,
    },
    /// The completeness assertion stops short of the window.
    IncompleteInput,
    /// A past operator reaches before a history cutoff.
    MissingHistory,
    /// A counter cover straddles an interval bound.
    StraddlesBound,
}

/// Why a call returned no truth although it ran (FR-523, FR-524).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceIncomplete {
    /// A counter reached its ceiling.
    Limit(LimitReached),
    /// The meter denied a `temporal.visit` charge.
    Meter {
        /// The meter's denial, with its charge point.
        incomplete: quire_exact::Incomplete,
        /// The opaque identity of the assessed instance, when the stop is at
        /// the instance.
        instance: Option<Box<[u8]>>,
    },
    /// A valuation the activation required is in `evicted`.
    EvictedValuation {
        /// The instance the valuation is bound to, when it is.
        instance: Option<SemanticTriggerIdentity>,
        /// The position's stamp coordinate.
        position: u64,
        /// The atom.
        atom: u64,
    },
}

/// The result of a temporal evidence call (FR-516 "Outputs").
#[derive(Clone, Debug)]
pub struct EvidenceResult {
    /// The activation state.
    pub activation: ActivationState,
    /// The surrounding execution's disposition, echoed.
    pub execution: ExecutionDisposition,
    /// The truth, when one exists.
    pub truth: Option<Truth>,
    /// The non-value, when there is no truth or a fact is withdrawn.
    pub non_value: Option<NonValue>,
    /// The settlement.
    pub settlement: Option<Settlement>,
    /// Why a closed window stays open, when it does.
    pub indeterminate: Option<Indeterminate>,
    /// Why the call stopped without a truth, when it did.
    pub incomplete: Option<EvidenceIncomplete>,
    /// The fact references inside the decision support.
    pub support: Vec<SupportRef>,
    /// The trigger scope's evidence, echoed.
    pub trigger_evidence: TriggerEvidence,
    /// The trigger scope's closure, echoed.
    pub trigger_scope: ProgressClosure,
    /// The decision scope's watermark, echoed.
    pub decision_progress: Option<u64>,
    /// The decision scope's closure, echoed.
    pub decision_closure: Option<ProgressClosure>,
    /// The surrounding execution's watermark, echoed.
    pub surrounding_progress: Option<u64>,
    /// The surrounding execution's closure, echoed.
    pub surrounding_closure: Option<ProgressClosure>,
    /// The completeness record, echoed.
    pub completeness: Option<CompletenessRecord>,
    /// The correspondence, echoed byte for byte.
    pub correspondence: EvidenceRef,
    /// The assessed trigger identity, echoed byte for byte.
    pub trigger: Option<SemanticTriggerIdentity>,
    /// The work the call charged on the thirteen counters.
    pub usage: EvidenceUsage,
    /// The relation, echoed with its predecessor.
    pub relation: Relation,
    /// The lineage number: 0 for `Original`.
    pub lineage: u64,
    /// The identity over the evidence members only.
    pub input_identity: InputIdentity,
    /// The result's identity.
    pub result_identity: ResultIdentity,
    /// The ledger the caller keeps for the next call.
    pub ledger: EvidenceLedger,
    /// The progress ledger the caller keeps for the next call.
    pub progress_ledger: Option<ProgressLedger>,
    /// The binding the request agreed with.
    pub authenticated: AuthenticatedBinding,
    /// Every instance the lists resolved, with its activation state.
    pub instances: Vec<Instance>,
}

/// An entry of the package's FR-449 activation map: the protocol clause and
/// the temporal clause its row names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivationRow {
    /// The protocol clause.
    pub control: CheckedNodeId,
    /// The temporal clause the row names.
    pub clause: CheckedNodeId,
}

/// One occurrence of a clause's declaration and its clock row, keyed by
/// `{ role, ordinal }` (QSpec FR-322; FR-515 `clock_of`).
#[derive(Clone, Debug)]
pub struct SubjectOccurrence<'p> {
    /// The occurrence's key.
    pub occurrence: CheckedOccurrence,
    /// The `temporal_bindings` row keyed by the declaration and this
    /// occurrence, or `None` when the lock selects no clock configuration.
    pub clock: Option<&'p TemporalBinding>,
}

/// The checked package's facts about the clause a call assesses (FR-515):
/// the declaration and profile row, each occurrence with the clock row its
/// key selects, the clause's formula, activation and captures, the atoms that
/// read an instance's binders, and the FR-449 rows.
///
/// The occurrences are listed in the order the package carries them, which
/// need not ascend by ordinal; the call pairs a request's `occurrence` with
/// its row by the key, never by position.
pub struct EvidenceSubject<'p> {
    /// The clause's declaration node.
    pub declaration: &'p CheckedNodeId,
    /// The lock's `profile_selections` row of the clause.
    pub profile: &'p CheckedSelection,
    /// The declaration's occurrences with their clock rows.
    pub occurrences: Vec<SubjectOccurrence<'p>>,
    /// The checked clause.
    pub clause: &'p CheckedTemporalClause,
    /// The node indices of the `holds` atoms whose operand reads the trigger
    /// binder or a capture (FR-515 `binders()`).
    pub instance_bound: std::collections::BTreeSet<u64>,
    /// The package's FR-449 activation-map rows.
    pub activation_map: &'p [ActivationRow],
    /// The binding progress is retained under (FR-502); the call sets its
    /// `occurrence` to the selected occurrence's ordinal.
    pub progress: LedgerBinding,
}
