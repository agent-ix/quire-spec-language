// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-122 (ADR-013 O-25 to O-27): [`replay_state_clause`], the replay
//! facade's entry for a state-clause counterexample.
//!
//! It recompiles the request's package by FR-098's rules ([`recompile`]),
//! resolves the payload's clause by name in the recompiled package, refuses
//! an envelope whose clause node or `claim` occurrence is not the
//! recompiled clause's, or whose observation form is not the clause kind's,
//! before any admission, then admits the observation by FR-106 from the
//! byte provision and evaluates the clause once by FR-107 -- the very
//! evaluation `run_clause`'s `Clause` selection runs (FR-109) -- and
//! settles an FR-072 result on the envelope's arm.

use std::collections::BTreeMap;

use qsl_forms::StateClauseKind;
use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::{DigestDomain, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::ByteDigest;
use qsl_semantics::check::CheckedStateClause;
use qsl_semantics::library::PackageId;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::observation::{
    ClauseSelection, ClauseSelectionInput, DocumentRef, ObservationForm, ObservationLimits,
    Provisions,
};
use quire_exact::Identifier;

use super::{charges, domain_packages, labels, one_source, recompile, wire_id, ReplayRefusal};
use crate::identity::RawSourceRef;
use crate::proof_result::ProofCategory;
use crate::request::{ReplayRequest, ReplayRequestWire};
use crate::result::{
    EvaluatedValue, InputArmResult, ReplayResult, SeparatingWitnessRecord, Verdict,
    WitnessArmResult,
};
use crate::spine::{
    check_clause, CallOutcome, CallValue, ClauseDisposition, ClauseRunSelection, CompiledRun,
    UnitProvenance,
};
use crate::witness::{ReplaySource, StateClauseCounterexample};
use crate::WitnessEnvelope;

/// FR-122's `stale_dependency`/`revision-mismatch`: an envelope clause
/// identity that is not the recompiled clause's, naming both.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ClauseIdentityMismatch {
    /// The clause's `state`/`state_clause` node.
    #[error(
        "the envelope names clause node {envelope} but the package recompiles it as {recompiled}"
    )]
    Node {
        /// The envelope's clause node.
        envelope: WireNodeId,
        /// The recompiled clause node.
        recompiled: WireNodeId,
    },
    /// The clause declaration's `claim` occurrence.
    #[error("the envelope names occurrence {envelope:?} but the package recompiles it as {recompiled:?}")]
    Occurrence {
        /// The envelope's occurrence key.
        envelope: OccurrenceKey,
        /// The recompiled `claim` occurrence key.
        recompiled: OccurrenceKey,
    },
}

/// FR-122 Outputs: an FR-072 result on the envelope's arm, with the
/// identities the replay kept: the source unit's identity and digest, the
/// `package_id`, the payload's clause and observation (for a pre-call
/// observation, its self object and parameter values), the envelope's
/// clause node and occurrence key (each equal to the recompiled one, or the
/// replay refused), and every document admission read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateClauseReplayResult {
    result: ReplayResult,
    source: RawSourceRef,
    package_id: PackageId,
    clause: Identifier,
    clause_node: WireNodeId,
    occurrence_key: OccurrenceKey,
    observation: ClauseSelectionInput,
    documents: Vec<DocumentRef>,
}

impl StateClauseReplayResult {
    /// The FR-072 result, on the envelope's arm.
    pub fn result(&self) -> &ReplayResult {
        &self.result
    }
    /// The recompiled source unit's identity and digest.
    pub fn source(&self) -> &RawSourceRef {
        &self.source
    }
    /// The recompiled `package_id`.
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }
    /// The payload's clause name.
    pub fn clause(&self) -> &Identifier {
        &self.clause
    }
    /// The envelope's clause node.
    pub fn clause_node(&self) -> WireNodeId {
        self.clause_node
    }
    /// The envelope's occurrence key: the clause's `claim` occurrence.
    pub fn occurrence_key(&self) -> &OccurrenceKey {
        &self.occurrence_key
    }
    /// The payload's observation. A pre-call observation holds the self
    /// object and the parameter values the replay evaluated the clause
    /// over.
    pub fn observation(&self) -> &ClauseSelectionInput {
        &self.observation
    }
    /// The identity and digest of every document admission read, in read
    /// order: the current or pre-call snapshot, or the invocation and then
    /// its pre and post snapshots.
    pub fn documents(&self) -> &[DocumentRef] {
        &self.documents
    }
}

/// FR-122: replay `envelope`'s state-clause counterexample. `wire` is
/// FR-098's request, supplying the package reference, byte provision and
/// limits; its function selection and replay source are FR-098's
/// function-replay members, which a state-clause replay does not read: its
/// selection is the payload and its result arm the envelope's.
///
/// Refuses, with no partial result, when the request does not decode, then
/// by FR-098's rules (in FR-098's order), when the recompiled `package_id`
/// is not the envelope's, when the payload's clause names no state clause,
/// when the envelope's clause node or occurrence key is stale, when the
/// observation's form is not the clause kind's (each before any
/// admission), and when FR-106 admission fails. The counterexample refuted
/// the clause, so the proved verdict is `violation`: `false` reproduces;
/// `true` is `inconclusive`, `Verdicts`; an evaluation that completed no
/// value is `inconclusive`, `NoValue`.
///
/// Admission and the domain package re-normalization run under their
/// published defaults: no `quire.value.accounting/v1` counter names either.
/// The request's accounting limits build the evaluation meter.
pub fn replay_state_clause(
    wire: ReplayRequestWire,
    envelope: &WitnessEnvelope<StateClauseCounterexample>,
) -> Result<StateClauseReplayResult, ReplayRefusal> {
    let request = ReplayRequest::decode(wire)?;
    let compiled = recompile(&request)?;
    let package_id = compiled.emitted.package_id();
    if !package_id.matches(&envelope.package_id()) {
        return Err(ReplayRefusal::PackageIdMismatch {
            requested: envelope.package_id(),
            recompiled: package_id,
        });
    }
    let payload = envelope.family_payload();
    let graph = compiled.package.graph();
    let clause = graph.state_clause(payload.clause.as_str()).ok_or_else(|| {
        ReplayRefusal::UnknownClause {
            clause: payload.clause.clone(),
            package: package_id,
        }
    })?;
    check_identities(envelope, clause).map_err(ReplayRefusal::ClauseIdentity)?;
    let form = payload.observation.form();
    if form != expected_form(clause.kind()) {
        return Err(ReplayRefusal::WrongObservation {
            kind: clause.kind(),
            form,
        });
    }

    // `recompile` already required exactly one source, provided.
    let source = one_source(request.source_digests())?;
    let bytes = request
        .byte_provision()
        .get(source.digest())
        .ok_or_else(|| {
            ReplayRefusal::Fault(InternalFault::new(
                "replay",
                "decoded-request-byte-provision-complete",
            ))
        })?;
    let unit = UnitProvenance {
        digest: ByteDigest::of(bytes).to_string(),
        source: labels(source),
        extraction: None,
    };
    let packages = domain_packages(&request);
    // FR-106 reads every document only from the byte provision, by its
    // `sha256-jcs` digest.
    let documents: BTreeMap<[u8; 32], Vec<u8>> = request
        .byte_provision()
        .entries()
        .filter(|(digest, _)| digest.domain() == DigestDomain::Sha256Jcs)
        .map(|(digest, bytes)| (*digest.as_bytes(), bytes.to_vec()))
        .collect();
    let mut sources = Vec::with_capacity(1 + compiled.libraries.len());
    sources.push(compiled.source.clone());
    sources.extend(compiled.libraries.iter().cloned());
    let selection = ClauseSelection {
        name: payload.clause.as_str().to_owned(),
        input: payload.observation.clone(),
    };
    let run = CompiledRun {
        packages: &packages,
        model_limits: ModelNormalizationLimits::default(),
        provisions: Provisions {
            snapshots: &documents,
            invocations: &documents,
        },
        observation_limits: ObservationLimits::default(),
        accounting: request.accounting_limits(),
        package: &compiled.package,
        package_id,
        unit: &unit,
        sources: &sources,
        model_selections: graph.model_selections().to_vec(),
        selection: ClauseRunSelection::Clause(selection.clone()),
    };
    let report = check_clause(&run, clause, &selection);

    let fault = |invariant| {
        Err(ReplayRefusal::Fault(InternalFault::new(
            "replay", invariant,
        )))
    };
    let (replayed, value) = match report.disposition {
        ClauseDisposition::Evaluate(CallOutcome::Completed(CallValue::Boolean(holds))) => (
            if holds {
                ProofCategory::Success
            } else {
                ProofCategory::Violation
            },
            Some(EvaluatedValue::Boolean(holds)),
        ),
        // FR-104: a state clause's body is Boolean.
        ClauseDisposition::Evaluate(CallOutcome::Completed(CallValue::Integer(_))) => {
            return fault("state-clause-completes-a-boolean")
        }
        ClauseDisposition::Evaluate(CallOutcome::Refused(_)) => (ProofCategory::Refusal, None),
        ClauseDisposition::Evaluate(CallOutcome::Incomplete { .. }) => {
            (ProofCategory::Incomplete, None)
        }
        // O-16's `undefined` row is not a proof category (as in FR-098).
        ClauseDisposition::Evaluate(CallOutcome::Undefined { .. }) => {
            (ProofCategory::Inconclusive, None)
        }
        ClauseDisposition::Admit(failure) => return Err(ReplayRefusal::Admission(failure)),
        ClauseDisposition::EvaluateFault(fault) => return Err(ReplayRefusal::Fault(fault)),
        // FR-107's `UnknownClause`: the clause was resolved by name above,
        // so a clause the evaluation cannot name is a broken invariant.
        ClauseDisposition::MissingName { .. } => return fault("state-clause-resolved-by-name"),
        // `check_clause` reports only admission or evaluation: the compile
        // and selection stages are behind it, and a clause run takes no
        // argument and checks no frame.
        ClauseDisposition::Compile(_)
        | ClauseDisposition::UnknownLanguage { .. }
        | ClauseDisposition::StalePackage { .. }
        | ClauseDisposition::NotAPredicate { .. }
        | ClauseDisposition::ArgumentRefusal(_)
        | ClauseDisposition::FrameViolation(_) => {
            return fault("clause-check-reports-admission-or-evaluation")
        }
    };
    let proved = Verdict::from_category(ProofCategory::Violation);
    let charges = charges(report.usage.evaluation_consumed.iter().copied());
    let result = match envelope.source() {
        ReplaySource::Witness(_) => ReplayResult::Witness(WitnessArmResult::settle(
            proved,
            Verdict::from_category(replayed),
            replayed,
            // As in FR-098: the verdict's whole value decides it.
            value.map(|value| {
                (
                    value,
                    SeparatingWitnessRecord {
                        deciding_element: value,
                        index: 0,
                        value_path: Vec::new(),
                        trace_position: None,
                    },
                )
            }),
            Vec::new(),
            charges,
        )),
        ReplaySource::Input(_) => ReplayResult::Input(InputArmResult::settle(
            proved,
            Verdict::from_category(replayed),
            replayed,
            value,
            Vec::new(),
            charges,
        )),
    };
    Ok(StateClauseReplayResult {
        result,
        source: source.clone(),
        package_id,
        clause: payload.clause.clone(),
        clause_node: envelope.clause_node(),
        occurrence_key: envelope.occurrence_key().clone(),
        observation: payload.observation.clone(),
        documents: report.provenance.documents,
    })
}

/// FR-122: the observation form a counterexample to a clause of `kind`
/// carries. A violated precondition means the operation never ran, so its
/// observation is the pre-call state alone.
fn expected_form(kind: StateClauseKind) -> ObservationForm {
    match kind {
        StateClauseKind::Invariant => ObservationForm::Current,
        StateClauseKind::Precondition => ObservationForm::PreCall,
        StateClauseKind::Postcondition => ObservationForm::Invocation,
    }
}

/// FR-122: the envelope's clause node, then its occurrence key, each
/// against the recompiled clause. The node goes first because the
/// occurrence key holds it. No identity is recovered from a display name.
fn check_identities(
    envelope: &WitnessEnvelope<StateClauseCounterexample>,
    clause: &CheckedStateClause,
) -> Result<(), Box<ClauseIdentityMismatch>> {
    let node = wire_id(clause.identity());
    if envelope.clause_node() != node {
        return Err(Box::new(ClauseIdentityMismatch::Node {
            envelope: envelope.clause_node(),
            recompiled: node,
        }));
    }
    let occurrence = OccurrenceKey::new(node, clause.claim().clone());
    if *envelope.occurrence_key() != occurrence {
        return Err(Box::new(ClauseIdentityMismatch::Occurrence {
            envelope: envelope.occurrence_key().clone(),
            recompiled: occurrence,
        }));
    }
    Ok(())
}
