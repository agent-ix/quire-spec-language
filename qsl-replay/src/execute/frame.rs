// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-116 (ADR-012 §12.2 Witness and replay row, ADR-013 O-25 to O-27):
//! [`replay_frame`], the replay facade's entry for a frame counterexample.
//!
//! It checks the envelope's clause node and occurrence key against the
//! payload's frame node and occurrence, recompiles the request's package by
//! FR-098's rules ([`recompile`]),
//! resolves the payload's operation in the recompiled package, refuses a
//! payload whose frame, occurrence or anchor identity is not the recompiled
//! one before any admission, then runs FR-115's frame run -- the very
//! `check_frame` that `run_clause`'s `Frame` selection runs (FR-109) --
//! over the invocation, admitted by FR-106 from the byte provision, and
//! settles an FR-072 result on the envelope's arm.

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::{DigestDomain, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::ByteDigest;
use qsl_semantics::check::CheckedOperationFrame;
use qsl_semantics::library::PackageId;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::observation::{DocumentRef, FrameWitness, ObservationLimits, Provisions};

use super::{charges, domain_packages, labels, one_source, recompile, wire_id, ReplayRefusal};
use crate::identity::RawSourceRef;
use crate::proof_result::ProofCategory;
use crate::request::{ReplayRequest, ReplayRequestWire};
use crate::result::{
    EvaluatedValue, InputArmResult, ReplayResult, SeparatingWitnessRecord, Verdict,
    WitnessArmResult,
};
use crate::spine::{
    check_frame, resolve_frame, CallOutcome, CallValue, ClauseDisposition, ClauseRunSelection,
    CompiledRun, OperationName, UnitProvenance,
};
use crate::witness::{ClaimedChange, FrameCounterexample, FrameOperation, ReplaySource};
use crate::WitnessEnvelope;

/// FR-116's `stale_dependency`/`revision-mismatch`: one payload identity
/// that is not the recompiled package's, or one envelope identity that is
/// not the payload's, naming both.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FrameIdentityMismatch {
    /// The envelope's `clause_node`, which for a frame packet is the
    /// payload's frame node.
    #[error(
        "the envelope names clause node {envelope} but its payload names frame node {payload}"
    )]
    EnvelopeFrame {
        /// The envelope's clause node.
        envelope: WireNodeId,
        /// The payload's frame node.
        payload: WireNodeId,
    },
    /// The envelope's `occurrence_key`, which for a frame packet is the
    /// payload's frame occurrence.
    #[error("the envelope names occurrence {envelope:?} but its payload names frame occurrence {payload:?}")]
    EnvelopeOccurrence {
        /// The envelope's occurrence key.
        envelope: OccurrenceKey,
        /// The payload's frame occurrence key.
        payload: OccurrenceKey,
    },
    /// The operation's `state`/`operation_anchor` node.
    #[error(
        "the payload names anchor node {payload} but the package recompiles it as {recompiled}"
    )]
    Anchor {
        /// The payload's anchor node.
        payload: WireNodeId,
        /// The recompiled anchor node.
        recompiled: WireNodeId,
    },
    /// The operation's `state`/`frame` node.
    #[error(
        "the payload names frame node {payload} but the package recompiles it as {recompiled}"
    )]
    Frame {
        /// The payload's frame node.
        payload: WireNodeId,
        /// The recompiled frame node.
        recompiled: WireNodeId,
    },
    /// The frame node's `generated` occurrence.
    #[error("the payload names frame occurrence {payload:?} but the package recompiles it as {recompiled:?}")]
    Occurrence {
        /// The payload's occurrence key.
        payload: OccurrenceKey,
        /// The recompiled occurrence key.
        recompiled: OccurrenceKey,
    },
}

/// FR-116 Outputs: an FR-072 result on the envelope's arm, with the
/// identities the replay kept: the source unit's identity and digest, the
/// `package_id`, the payload's operation, anchor, frame and occurrence
/// (each equal to the recompiled one, or the replay refused), the three
/// admitted documents, the payload's claimed change and the change the
/// replay found.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameReplayResult {
    result: ReplayResult,
    source: RawSourceRef,
    package_id: PackageId,
    operation: FrameOperation,
    anchor: WireNodeId,
    frame: WireNodeId,
    occurrence: OccurrenceKey,
    invocation: DocumentRef,
    pre: DocumentRef,
    post: DocumentRef,
    claimed: ClaimedChange,
    found: Option<FrameWitness>,
}

impl FrameReplayResult {
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
    /// The payload's operation.
    pub fn operation(&self) -> &FrameOperation {
        &self.operation
    }
    /// The operation's anchor node.
    pub fn anchor(&self) -> WireNodeId {
        self.anchor
    }
    /// The operation's frame node.
    pub fn frame(&self) -> WireNodeId {
        self.frame
    }
    /// The frame node's `generated` occurrence key.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }
    /// The admitted invocation's identity and digest.
    pub fn invocation(&self) -> &DocumentRef {
        &self.invocation
    }
    /// The admitted pre snapshot's identity and digest.
    pub fn pre(&self) -> &DocumentRef {
        &self.pre
    }
    /// The admitted post snapshot's identity and digest.
    pub fn post(&self) -> &DocumentRef {
        &self.post
    }
    /// The change the payload claims.
    pub fn claimed(&self) -> &ClaimedChange {
        &self.claimed
    }
    /// The frame witness the replay's check found (FR-115), when it found a
    /// change outside the frame; it may name another change than
    /// [`Self::claimed`], and both are kept as found.
    pub fn found(&self) -> Option<&FrameWitness> {
        self.found.as_ref()
    }
}

/// FR-116: replay `envelope`'s frame counterexample. `wire` is FR-098's
/// request, supplying the package reference, byte provision and limits;
/// its function selection and replay source are FR-098's function-replay
/// members, which a frame replay does not read: its selection is the
/// payload and its result arm the envelope's.
///
/// Refuses, with no partial result, when the request does not decode, then
/// when the envelope's `clause_node` or `occurrence_key` is not the
/// payload's frame node or frame occurrence (before recompiling), then by
/// FR-098's rules (in FR-098's order), when the recompiled `package_id` is not the
/// envelope's, when the payload's operation names no operation frame, when
/// a payload identity is stale (before any admission), and when FR-106
/// admission fails. The counterexample refuted the frame, so the proved
/// verdict is `violation`: a violation reproduces; a frame the invocation
/// respects is `inconclusive`, `Verdicts`; a check that completed no value
/// (a declared delta that disagrees) is `inconclusive`, `NoValue`.
///
/// Admission and the domain package re-normalization run under their
/// published defaults: no `quire.value.accounting/v1` counter names either.
pub fn replay_frame(
    wire: ReplayRequestWire,
    envelope: &WitnessEnvelope<FrameCounterexample>,
) -> Result<FrameReplayResult, ReplayRefusal> {
    let request = ReplayRequest::decode(wire)?;
    check_envelope(envelope).map_err(ReplayRefusal::FrameIdentity)?;
    let compiled = recompile(&request)?;
    let package_id = compiled.emitted.package_id();
    if !package_id.matches(&envelope.package_id()) {
        return Err(ReplayRefusal::PackageIdMismatch {
            requested: envelope.package_id(),
            recompiled: package_id,
        });
    }
    let payload = envelope.family_payload();
    let unknown = || ReplayRefusal::UnknownOperation {
        operation: payload.operation.clone(),
        package: package_id,
    };
    let operation = operation_name(&payload.operation).ok_or_else(unknown)?;
    let graph = compiled.package.graph();
    let (context, operation_frame) = resolve_frame(graph, &operation).ok_or_else(unknown)?;
    check_identities(payload, operation_frame).map_err(ReplayRefusal::FrameIdentity)?;

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
    // FR-106 reads the invocation and its snapshots only from the byte
    // provision, by their `sha256-jcs` digests.
    let documents: BTreeMap<[u8; 32], Vec<u8>> = request
        .byte_provision()
        .entries()
        .filter(|(digest, _)| digest.domain() == DigestDomain::Sha256Jcs)
        .map(|(digest, bytes)| (*digest.as_bytes(), bytes.to_vec()))
        .collect();
    let mut sources = Vec::with_capacity(1 + compiled.libraries.len());
    sources.push(compiled.source.clone());
    sources.extend(compiled.libraries.iter().cloned());
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
        selection: ClauseRunSelection::Frame {
            operation,
            invocation: payload.invocation.clone(),
        },
    };
    let report = check_frame(&run, context, operation_frame, &payload.invocation);

    let (replayed, value, found) = match report.disposition {
        ClauseDisposition::FrameViolation(witness) => (
            ProofCategory::Violation,
            Some(EvaluatedValue::Boolean(false)),
            Some(*witness),
        ),
        ClauseDisposition::Evaluate(CallOutcome::Completed(CallValue::Boolean(true))) => (
            ProofCategory::Success,
            Some(EvaluatedValue::Boolean(true)),
            None,
        ),
        // FR-115: a false verdict always carries its witness, and a frame
        // check completes a Boolean.
        ClauseDisposition::Evaluate(CallOutcome::Completed(
            CallValue::Boolean(false) | CallValue::Integer(_),
        )) => {
            return Err(ReplayRefusal::Fault(InternalFault::new(
                "replay",
                "frame-check-completes-true-or-a-witnessed-violation",
            )))
        }
        ClauseDisposition::Evaluate(CallOutcome::Refused(_)) => {
            (ProofCategory::Refusal, None, None)
        }
        ClauseDisposition::Evaluate(CallOutcome::Incomplete { .. }) => {
            (ProofCategory::Incomplete, None, None)
        }
        // O-16's `undefined` row is not a proof category (as in FR-098).
        ClauseDisposition::Evaluate(CallOutcome::Undefined { .. }) => {
            (ProofCategory::Inconclusive, None, None)
        }
        ClauseDisposition::Admit(failure) => return Err(ReplayRefusal::Admission(failure)),
        ClauseDisposition::EvaluateFault(fault) => return Err(ReplayRefusal::Fault(fault)),
        // `check_frame` reports only admission or evaluation: the compile,
        // selection and argument stages are behind it.
        ClauseDisposition::Compile(_)
        | ClauseDisposition::UnknownLanguage { .. }
        | ClauseDisposition::StalePackage { .. }
        | ClauseDisposition::MissingName { .. }
        | ClauseDisposition::NotAPredicate { .. }
        | ClauseDisposition::ArgumentRefusal(_) => {
            return Err(ReplayRefusal::Fault(InternalFault::new(
                "replay",
                "frame-check-reports-admission-or-evaluation",
            )))
        }
    };
    let [invocation, pre, post] = <[DocumentRef; 3]>::try_from(report.provenance.documents)
        .map_err(|_| {
            ReplayRefusal::Fault(InternalFault::new(
                "replay",
                "an-evaluated-frame-check-read-three-documents",
            ))
        })?;
    let proved = Verdict::from_category(ProofCategory::Violation);
    let charges = charges(report.usage.evaluation_consumed.iter().copied());
    // The frame check reports no evaluation location, so the result cites
    // no region; its identities are the payload's and the documents'.
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
    Ok(FrameReplayResult {
        result,
        source: source.clone(),
        package_id,
        operation: payload.operation.clone(),
        anchor: payload.anchor,
        frame: payload.frame,
        occurrence: payload.occurrence.clone(),
        invocation,
        pre,
        post,
        claimed: payload.change.clone(),
        found,
    })
}

/// `operation` as a `Frame` selection's `M::T::op`: its declaring type
/// must be exactly a model alias and an object type. `None` otherwise, so
/// it names no operation.
fn operation_name(operation: &FrameOperation) -> Option<OperationName> {
    let [model, object] = operation.object.segments() else {
        return None;
    };
    Some(OperationName {
        model: model.clone(),
        object: object.clone(),
        operation: operation.operation.clone(),
    })
}

/// FR-116: a frame packet carries its frame node and frame occurrence
/// twice, as the envelope's `clause_node` and `occurrence_key` and as the
/// payload's `frame` and `occurrence`; the two must agree. The frame goes
/// first, as in [`check_identities`], because the occurrence key is
/// derived from it. The envelope's `selected_function` is not read.
fn check_envelope(
    envelope: &WitnessEnvelope<FrameCounterexample>,
) -> Result<(), Box<FrameIdentityMismatch>> {
    let payload = envelope.family_payload();
    if envelope.clause_node() != payload.frame {
        return Err(Box::new(FrameIdentityMismatch::EnvelopeFrame {
            envelope: envelope.clause_node(),
            payload: payload.frame,
        }));
    }
    if *envelope.occurrence_key() != payload.occurrence {
        return Err(Box::new(FrameIdentityMismatch::EnvelopeOccurrence {
            envelope: envelope.occurrence_key().clone(),
            payload: payload.occurrence.clone(),
        }));
    }
    Ok(())
}

/// FR-116: the payload's frame, occurrence and anchor identities, each
/// against the recompiled one, in that order. No identity is recovered
/// from a display name.
///
/// The frame goes first because the other two are derived from it: the
/// occurrence key is (frame node, origin), and the `operation_anchor` node's
/// content references the frame node, so a changed frame changes both. A
/// payload produced from a package whose frame differs is then refused
/// naming the two frame identities (FR-116-AC-3), not the anchors that
/// changed only because the frame did.
fn check_identities(
    payload: &FrameCounterexample,
    recompiled: &CheckedOperationFrame,
) -> Result<(), Box<FrameIdentityMismatch>> {
    let frame = wire_id(recompiled.frame());
    if frame != payload.frame {
        return Err(Box::new(FrameIdentityMismatch::Frame {
            payload: payload.frame,
            recompiled: frame,
        }));
    }
    let occurrence = OccurrenceKey::new(frame, recompiled.frame_origin().clone());
    if occurrence != payload.occurrence {
        return Err(Box::new(FrameIdentityMismatch::Occurrence {
            payload: payload.occurrence.clone(),
            recompiled: occurrence,
        }));
    }
    let anchor = wire_id(recompiled.anchor());
    if anchor != payload.anchor {
        return Err(Box::new(FrameIdentityMismatch::Anchor {
            payload: payload.anchor,
            recompiled: anchor,
        }));
    }
    Ok(())
}
