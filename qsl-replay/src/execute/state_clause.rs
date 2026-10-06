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
    OutOfRange, PostStateRange, Provisions,
};
use quire_exact::Identifier;

use super::{consumed, domain_packages, labels, one_source, recompile, wire_id, ReplayRefusal};
use crate::identity::RawSourceRef;
use crate::request::{ReplayRequest, ReplayRequestWire};
use crate::result::{
    EvaluatedValue, InputArmResult, ReplayResult, SeparatingWitnessRecord, SeparationReason,
    SeparationRefusal, Verdict, WitnessArmResult, WitnessCheck, WitnessFailure,
};
use crate::spine::{
    check_clause_admitted, convert_outcome, CallOutcome, CallRefusal, CallValue, ClauseCheck,
    ClauseDisposition, ClauseRunSelection, CompiledRun, UnitProvenance,
};
use crate::witness::{ReplaySource, StateClauseCounterexample};
use crate::WitnessEnvelope;
use qsl_eval::value::{
    CallFailure, CheckedPackageEvaluation, QualifiedName, Separation, SeparationStep, WitnessClaim,
};
use qsl_foundation::diagnostic::Category;
use qsl_foundation::source::Source;
use qsl_semantics::check::CheckedGraph;
use qsl_semantics::model::observation::AdmittedObservations;
use quire_exact::Meter;

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
    range_violations: Vec<OutOfRange>,
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
    /// its pre snapshot and, for a postcondition, its post snapshot.
    pub fn documents(&self) -> &[DocumentRef] {
        &self.documents
    }
    /// FR-122's range violations: every integer the post snapshot holds
    /// outside its field's declared range, each naming the object and
    /// field, the declared range and the exact observed value, in walk
    /// order. Empty when the post snapshot is in range or the observation
    /// has no post snapshot.
    pub fn range_violations(&self) -> &[OutOfRange] {
        &self.range_violations
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
/// value is `inconclusive`, `NoValue`. A postcondition's post snapshot is
/// the subject's output: an integer in it outside its declared range is
/// admitted exactly and is itself the witness of a violation, so the
/// replay reproduces a violation whatever the clause does over the exact
/// value (its boolean, a refusal, an exhausted budget or a fault), and
/// [`StateClauseReplayResult::range_violations`] names each one.
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
    let package_id = compiled.emitted.package().package_id();
    if !package_id.matches(&envelope.package_id()) {
        return Err(ReplayRefusal::PackageIdMismatch {
            requested: envelope.package_id(),
            recompiled: package_id,
        });
    }
    let payload = envelope.family_payload();
    let graph = compiled.checked.package().graph();
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
    let mut sources = Vec::with_capacity(1 + compiled.checked.libraries().len());
    sources.push(compiled.checked.source().clone());
    sources.extend(compiled.checked.libraries().iter().cloned());
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
        observation_limits: ObservationLimits {
            post_state: PostStateRange::Witness,
            ..ObservationLimits::default()
        },
        accounting: request.accounting_limits(),
        package: compiled.checked.package(),
        package_id,
        unit: &unit,
        sources: &sources,
        model_selections: graph.model_selections().to_vec(),
        selection: ClauseRunSelection::Clause(selection.clone()),
    };
    let ClauseCheck {
        report,
        observations,
        mut meter,
    } = check_clause_admitted(&run, clause, &selection);

    let fault = |invariant| {
        Err(ReplayRefusal::Fault(InternalFault::new(
            "replay", invariant,
        )))
    };
    // FR-106 check 6.5: a post-state integer outside its declared range is
    // the witness of a violation of the operation's contract, whatever the
    // clause does over the exact value.
    let range_violations: Vec<OutOfRange> = observations
        .as_ref()
        .and_then(|observed| observed.post.as_ref())
        .map(|post| post.out_of_range.clone())
        .unwrap_or_default();
    let (replayed, mut value) = match report.disposition {
        // The violation stands even when the clause's own evaluation broke
        // on the exact value: only the clause's outcome is lost.
        ClauseDisposition::EvaluateFault(_) if !range_violations.is_empty() => {
            (Category::Inconclusive, None)
        }
        ClauseDisposition::Evaluate(CallOutcome::Completed(CallValue::Boolean(holds))) => (
            if holds {
                Category::Success
            } else {
                Category::Violation
            },
            Some(EvaluatedValue::Boolean(holds)),
        ),
        // FR-104: a state clause's body is Boolean.
        ClauseDisposition::Evaluate(CallOutcome::Completed(CallValue::Integer(_))) => {
            return fault("state-clause-completes-a-boolean")
        }
        ClauseDisposition::Evaluate(CallOutcome::Refused(_)) => (Category::Refusal, None),
        ClauseDisposition::Evaluate(CallOutcome::Incomplete(_)) => (Category::Incomplete, None),
        // O-16's `undefined` row is not a proof category (as in FR-098).
        ClauseDisposition::Evaluate(CallOutcome::Undefined { .. }) => {
            (Category::Inconclusive, None)
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
    if !range_violations.is_empty() {
        let charges = consumed(&meter);
        let result =
            match envelope.source() {
                ReplaySource::Witness(_) => ReplayResult::Witness(
                    WitnessArmResult::settle_range_violation(value, Vec::new(), charges),
                ),
                ReplaySource::Input(_) => ReplayResult::Input(
                    InputArmResult::settle_range_violation(value, Vec::new(), charges),
                ),
            };
        return Ok(StateClauseReplayResult {
            result,
            source: source.clone(),
            package_id,
            clause: payload.clause.clone(),
            clause_node: envelope.clause_node(),
            occurrence_key: envelope.occurrence_key().clone(),
            observation: payload.observation.clone(),
            documents: report.provenance.documents,
            range_violations,
        });
    }
    let proved = Verdict::from_category(Category::Violation);
    // FR-268: an agreeing verdict compares the payload's record with the
    // re-derived one, then checks the record separates the clause.
    let witness = if value == Some(EvaluatedValue::Boolean(false)) {
        let observations = observations.as_ref().ok_or_else(|| {
            ReplayRefusal::Fault(InternalFault::new(
                "replay",
                "evaluated-clause-was-admitted",
            ))
        })?;
        match compare_witness(
            compiled.checked.package(),
            &selection.name,
            observations,
            payload.witness.as_ref(),
            report.witness.as_ref(),
            &sources,
            &mut meter,
        )? {
            Some(check) => check,
            // An exhausted meter during the separation check settles
            // `NoValue`, as an exhausted evaluation does (FR-268).
            None => {
                value = None;
                WitnessCheck::Agrees(None)
            }
        }
    } else {
        WitnessCheck::Agrees(None)
    };
    let charges = consumed(&meter);
    let result = match envelope.source() {
        ReplaySource::Witness(_) => ReplayResult::Witness(WitnessArmResult::settle(
            proved,
            Verdict::from_category(replayed),
            replayed,
            value,
            witness,
            Vec::new(),
            charges,
        )),
        ReplaySource::Input(_) => ReplayResult::Input(InputArmResult::settle(
            proved,
            Verdict::from_category(replayed),
            replayed,
            value,
            &witness,
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
        range_violations: Vec::new(),
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

/// FR-268: compare the payload's record `given` with the re-derived record
/// `derived` (QSpec FR-351 identity, the deciding element under ADR-013
/// O-13), and, when both are present and equal, run the separation check
/// over `observations`, charged to `meter`. `Ok(None)` when the meter is
/// exhausted during the separation check.
fn compare_witness(
    package: &qsl_package::CheckedPackage,
    clause: &str,
    observations: &AdmittedObservations,
    given: Option<&SeparatingWitnessRecord>,
    derived: Option<&SeparatingWitnessRecord>,
    sources: &[Source],
    meter: &mut Meter,
) -> Result<Option<WitnessCheck>, ReplayRefusal> {
    match (given, derived) {
        (None, None) => Ok(Some(WitnessCheck::Agrees(None))),
        (Some(given_record), Some(derived_record)) if given_record == derived_record => {
            let outcome = separate(
                package,
                clause,
                observations,
                derived_record,
                sources,
                meter,
            )?;
            Ok(settle_separation(outcome, derived_record))
        }
        _ => Ok(Some(WitnessCheck::Disagrees {
            given: given.cloned().map(Box::new),
            derived: derived.cloned().map(Box::new),
            failure: WitnessFailure::Mismatch,
        })),
    }
}

/// FR-269: the witness check of a matching `record` whose separation check
/// ended `outcome`: it agrees when the check holds, disagrees naming the
/// failing step and reason when it fails, and `None` when the meter was
/// exhausted.
pub(crate) fn settle_separation(
    outcome: SeparationOutcome,
    record: &SeparatingWitnessRecord,
) -> Option<WitnessCheck> {
    match outcome {
        SeparationOutcome::Holds => Some(WitnessCheck::Agrees(Some(Box::new(record.clone())))),
        SeparationOutcome::Failed(step, reason) => Some(WitnessCheck::Disagrees {
            given: Some(Box::new(record.clone())),
            derived: Some(Box::new(record.clone())),
            failure: WitnessFailure::Separation { step, reason },
        }),
        SeparationOutcome::Exhausted => None,
    }
}

/// FR-268's separation check, settled: it holds, it fails at a step for a
/// reason (FR-269), or the meter was exhausted (`NoValue`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SeparationOutcome {
    /// Every step passed.
    Holds,
    /// The step failed, for the reason.
    Failed(SeparationStep, SeparationReason),
    /// The meter was exhausted during a step.
    Exhausted,
}

/// FR-268: run the separation check of `record` over the state clause
/// named `clause` and its admitted `observations`, charged to `meter`, and
/// settle the answer: an `undefined` step evaluation as
/// `UndefinedEvaluation` naming the expression and its reason, a refused
/// one as its refusal record, an exhausted meter as
/// [`SeparationOutcome::Exhausted`].
pub(crate) fn separate(
    package: &qsl_package::CheckedPackage,
    clause: &str,
    observations: &AdmittedObservations,
    record: &SeparatingWitnessRecord,
    sources: &[Source],
    meter: &mut Meter,
) -> Result<SeparationOutcome, ReplayRefusal> {
    let fault = |invariant| ReplayRefusal::Fault(InternalFault::new("replay", invariant));
    // A state clause's record always carries its index (ADR-031 SW-5);
    // a record without one names no element to check.
    let Some(index) = record.index else {
        return Ok(SeparationOutcome::Failed(
            SeparationStep::Element,
            SeparationReason::Unmet,
        ));
    };
    let qualified =
        QualifiedName::unqualified(clause).map_err(|_| fault("state-clause-name-qualifies"))?;
    let claim = WitnessClaim {
        quantifier: &record.quantifier,
        element: &record.deciding_element,
        index,
        value_path: &record.value_path,
    };
    let separation = match package.check_separation(&qualified, observations, claim, meter) {
        Ok(separation) => separation,
        Err(CallFailure::Fault(fault)) => return Err(ReplayRefusal::Fault(fault)),
        // The clause was resolved by name and admitted for that clause.
        Err(CallFailure::Input(_)) => {
            return Err(fault("separation-check-over-the-admitted-clause"))
        }
        // This meter holds no `Cancel` handle, so no charge is cancelled.
        Err(CallFailure::Cancelled(_)) => return Err(fault("cancelled-without-a-handle")),
    };
    match separation {
        Separation::Holds => Ok(SeparationOutcome::Holds),
        Separation::Unmet(step) => Ok(SeparationOutcome::Failed(step, SeparationReason::Unmet)),
        Separation::Stopped(step, evaluation) => {
            stopped_reason(*evaluation, package.graph(), sources).map(|reason| match reason {
                Some(reason) => SeparationOutcome::Failed(step, reason),
                None => SeparationOutcome::Exhausted,
            })
        }
    }
}

/// FR-269: why a separation-check evaluation that completed no value
/// fails its step; `None` for an exhausted meter, which is not a witness
/// failure.
pub(crate) fn stopped_reason(
    evaluation: qsl_eval::value::Evaluation,
    graph: &CheckedGraph,
    sources: &[Source],
) -> Result<Option<SeparationReason>, ReplayRefusal> {
    let fault = |invariant| ReplayRefusal::Fault(InternalFault::new("replay", invariant));
    let location = evaluation.location.clone();
    let outcome =
        convert_outcome(evaluation, graph, sources).map_err(|refusal| match *refusal {
            crate::spine::RunRefusal::Fault(fault) => ReplayRefusal::Fault(fault),
            _ => fault("separation-evaluation-maps-to-an-outcome"),
        })?;
    Ok(match outcome {
        CallOutcome::Incomplete(_) => None,
        CallOutcome::Undefined { reason } => Some(SeparationReason::UndefinedEvaluation {
            expression: location.ok_or_else(|| fault("undefined-evaluation-is-located"))?,
            cause: reason.to_owned(),
        }),
        CallOutcome::Refused(refusal) => {
            let (code, fields) = match refusal {
                CallRefusal::Record { code, fields, .. } => (code, fields),
                CallRefusal::Family { code, .. } => (code, std::collections::BTreeMap::new()),
            };
            Some(SeparationReason::Refused(SeparationRefusal {
                code: code.code().to_owned(),
                cause: code.cause().to_owned(),
                fields: fields
                    .into_iter()
                    .map(|(name, value)| (name.to_owned(), value))
                    .collect(),
            }))
        }
        CallOutcome::Completed(_) => return Err(fault("stopped-evaluation-completes-no-value")),
    })
}
