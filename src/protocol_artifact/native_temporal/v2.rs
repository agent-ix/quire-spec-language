// SPDX-License-Identifier: AGPL-3.0-only
//! Version-two native-temporal contracts with an opaque trigger identity.
//!
//! V1 remains a stable textual-instance contract. V2 makes the selected
//! event identity bytes explicit at the public boundary and commits them in
//! the outer request/result preimages. The v1 evaluator is an implementation
//! detail: it receives a private, reversible token and that token is never a
//! v2 accessor or caller input.

use serde::{Deserialize, Serialize};

use super::{
    common::{
        decode, encode, error, exhausted, identity, invalid, raw_digest, report, Error, ErrorCode,
        Limits, Report, Usage,
    },
    request, result, EvidenceRef,
};
use crate::{protocol_artifact::temporal_subject::ValidatedTemporalSubject, temporal, ByteDigest};

/// Exact v2 request contract selection.
pub const REQUEST_CONTRACT: &str = "quire.native-temporal-request/v2";
/// Exact v2 result contract selection.
pub const RESULT_CONTRACT: &str = "quire.native-temporal-result/v2";
/// Canonical immutable v2 request schema bytes.
pub const REQUEST_SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../schemas/native-temporal-request-v2.schema.json");
/// SHA-256 digest of [`REQUEST_SCHEMA_BYTES`].
pub const REQUEST_SCHEMA_SHA256: &str =
    "9481457311f1bd634c2a1eac46139f1f28948bb7252fe37f164d307705009ce0";
/// Canonical immutable v2 result schema bytes.
pub const RESULT_SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../schemas/native-temporal-result-v2.schema.json");
/// SHA-256 digest of [`RESULT_SCHEMA_BYTES`].
pub const RESULT_SCHEMA_SHA256: &str =
    "6bcb60be139aad3fdb34d720366b31921b678f5724de044b61e39468fef55869";

/// Bounded, nonempty semantic-trigger identity selected by the observation owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticTriggerIdentity(Vec<u8>);

impl SemanticTriggerIdentity {
    /// Builds a semantic-trigger identity from opaque bytes, rejecting an
    /// empty identity.
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, Error> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(invalid("trigger"));
        }
        Ok(Self(bytes))
    }

    /// Returns the opaque identity bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// V2 trigger delivery fields. Identity is deliberately absent: it is the
/// enclosing opaque semantic trigger, not caller-authored text.
#[derive(Clone, Debug)]
pub struct TriggerInput {
    /// Opaque delivery receipt identity. Repeated delivery of one semantic
    /// trigger carries distinct receipts and creates no second instance.
    pub receipt: String,
    /// The anchor this delivery was read at.
    pub anchor: String,
    /// Opaque payload, compared only to detect a conflicting redelivery
    /// under one semantic trigger identity.
    pub payload: String,
    /// Activation guard valuation at the anchor. `None` where the guard
    /// could not be established; an unestablished guard is not a false
    /// guard. Ignored where the declaration has no guard.
    pub guard: Option<bool>,
    /// Supplied input for each declared capture, in the declaration's
    /// authored capture order.
    pub captures: Vec<temporal::CaptureInput>,
}

/// Complete event-triggered v2 input. It replaces v1's public `instance`.
#[derive(Clone, Debug)]
pub struct Input {
    /// The opaque semantic-trigger identity this input is bound to.
    pub trigger: SemanticTriggerIdentity,
    /// Evidence correspondence this input is bound to.
    pub correspondence: EvidenceRef,
    /// Observation-bound native trace positions supplied for this
    /// evaluation.
    pub positions: Vec<request::ObservedPosition>,
    /// The authored activation anchor this input binds to.
    pub anchor: String,
    /// Event triggers supplied for this input; v2 admission requires
    /// exactly one.
    pub triggers: Vec<TriggerInput>,
    /// Whether trigger evidence was admitted, absent or refused.
    pub trigger_evidence: temporal::Evidence,
    /// Trigger-scope closure, independent of the decision scope.
    pub trigger_scope: temporal::Closure,
    /// Decision-scope progress assertion.
    pub decision_progress: request::ProgressInput,
    /// Decision-scope closure assertion.
    pub decision_closure: request::ClosureInput,
    /// Surrounding-execution progress assertion.
    pub surrounding_progress: request::ProgressInput,
    /// Surrounding-execution closure assertion.
    pub surrounding_closure: request::ClosureInput,
    /// Assessment-execution disposition.
    pub execution: temporal::Execution,
    /// Completeness authority and the exact fact population it covers.
    pub completeness: request::CompletenessInput,
    /// Whether the trace's lower boundary is an authoritative execution
    /// origin.
    pub authoritative_origin: bool,
    /// Retained valuation and capture records this evaluation may still
    /// read.
    pub evicted: Vec<temporal::Eviction>,
}

#[derive(Clone, Debug)]
/// Canonical v2 request bytes and their raw and semantic identities.
pub struct Document {
    bytes: Vec<u8>,
    digest: ByteDigest,
    wire: RequestWire,
}

impl Document {
    /// Returns the canonical encoded v2 request bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Returns the raw digest of the canonical bytes.
    pub const fn digest(&self) -> ByteDigest {
        self.digest
    }
    /// Returns the request's canonical content-addressed identity.
    pub fn identity(&self) -> &str {
        &self.wire.identity
    }
}

/// Constructor-private v2 request.
pub struct ValidatedRequest {
    document: Document,
    inner: request::ValidatedRequest,
    trigger: SemanticTriggerIdentity,
}

impl ValidatedRequest {
    /// Returns the underlying canonical v2 request document.
    pub fn document(&self) -> &Document {
        &self.document
    }
    /// Returns the opaque semantic-trigger identity bytes bound to this
    /// request.
    pub fn semantic_trigger(&self) -> &[u8] {
        self.trigger.as_bytes()
    }
    /// Returns the identity of the observation subject this request
    /// concerns.
    pub fn subject_identity(&self) -> &str {
        self.inner.subject_identity()
    }
    /// Returns the declaration index within the admitted package this
    /// request targets.
    pub const fn declaration(&self) -> u32 {
        self.inner.declaration()
    }

    /// Returns the exact evaluation anchor admitted into this request.
    ///
    /// This is an owner fact for the downstream FR-300 bridge. It carries no
    /// protocol control or observation-subject selection.
    pub fn evaluation_anchor(&self) -> &str {
        self.inner.anchor()
    }

    /// Returns the selected event trigger's immutable capture population.
    ///
    /// V2 admission proves that there is exactly one trigger. The public
    /// identity remains opaque through [`Self::semantic_trigger`]; this view
    /// deliberately exposes only its already-admitted captures.
    pub fn activation_captures(&self) -> impl ExactSizeIterator<Item = request::CaptureView<'_>> {
        self.inner
            .triggers()
            .next()
            .expect("v2 validated requests retain exactly one trigger")
            .captures()
    }

    /// Returns the admitted decision-progress evidence and watermark.
    pub fn decision_progress(&self) -> request::ProgressView<'_> {
        self.inner.decision_progress()
    }

    /// Returns the admitted decision-closure evidence and state.
    pub fn decision_closure(&self) -> request::ClosureView<'_> {
        self.inner.decision_closure()
    }

    /// Returns the admitted surrounding-progress evidence and watermark.
    pub fn surrounding_progress(&self) -> request::ProgressView<'_> {
        self.inner.surrounding_progress()
    }

    /// Returns the admitted surrounding-closure evidence and state.
    pub fn surrounding_closure(&self) -> request::ClosureView<'_> {
        self.inner.surrounding_closure()
    }

    /// Returns the admitted execution state.
    pub fn execution(&self) -> temporal::Execution {
        self.inner.execution()
    }

    /// Returns the admitted completeness authority, state, and fact population.
    pub fn completeness(&self) -> request::CompletenessView<'_> {
        self.inner.completeness()
    }
}

#[derive(Clone, Debug)]
/// Canonical v2 result bytes and their raw and semantic identities.
pub struct ResultDocument {
    bytes: Vec<u8>,
    digest: ByteDigest,
    wire: ResultWire,
}

impl ResultDocument {
    /// Returns the canonical encoded v2 result bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Returns the raw digest of the canonical bytes.
    pub const fn digest(&self) -> ByteDigest {
        self.digest
    }
    /// Returns the result's canonical content-addressed identity.
    pub fn identity(&self) -> &str {
        &self.wire.identity
    }
}

/// Constructor-private v2 result.
pub struct ValidatedResult {
    document: ResultDocument,
    inner: result::ValidatedResult,
    trigger: SemanticTriggerIdentity,
}

impl ValidatedResult {
    /// Returns the underlying canonical v2 result document.
    pub fn document(&self) -> &ResultDocument {
        &self.document
    }
    /// Returns the opaque semantic-trigger identity bytes committed in this
    /// result.
    pub fn semantic_trigger(&self) -> &[u8] {
        self.trigger.as_bytes()
    }
    /// Returns the identity of the request this result was evaluated from.
    pub fn request_identity(&self) -> &str {
        &self.document.wire.request_identity
    }
    /// Returns the settled truth, or `None` when no truth value is
    /// reported.
    pub fn truth(&self) -> Option<result::Truth> {
        self.inner.truth()
    }

    /// Returns the checked activation state from strict result admission.
    ///
    /// The result owner evaluates this state; downstream consumers may retain
    /// it but cannot submit or reconstruct it.
    pub fn activation(&self) -> result::ActivationState {
        self.inner.activation()
    }
}

/// Immutable v2 result lineage. A correction can never change trigger bytes.
pub enum Relation<'a> {
    /// This result has no predecessor; it is the first in its lineage.
    Original,
    /// This result supersedes the given predecessor, replacing it as the
    /// current answer within the same lineage.
    Superseding(&'a ValidatedResult),
    /// This result invalidates the given predecessor, retracting it within
    /// the same lineage.
    Invalidating(&'a ValidatedResult),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestWire {
    contract: String,
    identity: String,
    /// Lowercase hexadecimal encoding of the exact opaque bytes.
    trigger: String,
    /// Lowercase hexadecimal encoding of canonical v1 evaluator request bytes.
    request: String,
}

#[derive(Serialize)]
struct RequestPreimage<'a> {
    contract: &'a str,
    trigger: &'a str,
    request: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultWire {
    contract: String,
    identity: String,
    request_identity: String,
    trigger: String,
    /// Lowercase hexadecimal encoding of canonical v1 evaluator result bytes.
    result: String,
}

#[derive(Serialize)]
struct ResultPreimage<'a> {
    contract: &'a str,
    request_identity: &'a str,
    trigger: &'a str,
    result: &'a str,
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn unhex(value: &str, path: &'static str, limits: Limits) -> Result<Vec<u8>, Error> {
    if value.is_empty() || !value.len().is_multiple_of(2) || value.len() / 2 > limits.string_bytes {
        return Err(invalid(path));
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    let (pairs, remainder) = value.as_bytes().as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(invalid(path));
    }
    for pair in pairs {
        fn nibble(value: u8) -> Option<u8> {
            match value {
                b'0'..=b'9' => Some(value - b'0'),
                b'a'..=b'f' => Some(value - b'a' + 10),
                _ => None,
            }
        }
        let (Some(left), Some(right)) = (nibble(pair[0]), nibble(pair[1])) else {
            return Err(invalid(path));
        };
        bytes.push((left << 4) | right);
    }
    if hex(&bytes) != value {
        return Err(invalid(path));
    }
    Ok(bytes)
}

fn token(trigger: &[u8], limits: Limits) -> Result<String, Error> {
    const PREFIX: &str = "opaque-trigger-v2:";
    let encoded = trigger
        .len()
        .checked_mul(2)
        .ok_or_else(|| exhausted("trigger"))?;
    let total = PREFIX
        .len()
        .checked_add(encoded)
        .ok_or_else(|| exhausted("trigger"))?;
    if trigger.is_empty() || total > limits.string_bytes {
        return Err(exhausted("trigger"));
    }
    Ok(format!("{PREFIX}{}", hex(trigger)))
}

fn request_wire(
    trigger: &[u8],
    inner: &request::Document,
    limits: Limits,
) -> Result<RequestWire, Error> {
    let trigger = hex(trigger);
    let request = hex(inner.bytes());
    let mut wire = RequestWire {
        contract: REQUEST_CONTRACT.into(),
        identity: String::new(),
        trigger,
        request,
    };
    wire.identity = identity(
        REQUEST_CONTRACT,
        &encode(
            &RequestPreimage {
                contract: &wire.contract,
                trigger: &wire.trigger,
                request: &wire.request,
            },
            limits,
        )?,
    );
    Ok(wire)
}

fn finish_request(wire: RequestWire, limits: Limits, usage: &mut Usage) -> Result<Document, Error> {
    let bytes = encode(&wire, limits)?;
    usage.output_bytes = bytes.len();
    Ok(Document {
        digest: raw_digest(&bytes),
        bytes,
        wire,
    })
}

fn admit_request(
    wire: RequestWire,
    subject: &ValidatedTemporalSubject,
    limits: Limits,
    usage: &mut Usage,
) -> Result<ValidatedRequest, Error> {
    if wire.contract != REQUEST_CONTRACT {
        return Err(invalid("contract"));
    }
    let trigger = SemanticTriggerIdentity::new(unhex(&wire.trigger, "trigger", limits)?)?;
    let inner_bytes = unhex(&wire.request, "request", limits)?;
    let inner = request::read(&inner_bytes, subject, limits).into_result()?;
    let private = token(trigger.as_bytes(), limits)?;
    if inner.instance() != private
        || inner.triggers().len() != 1
        || inner
            .triggers()
            .next()
            .is_none_or(|value| value.identity() != private)
    {
        return Err(invalid("trigger.binding"));
    }
    let expected = request_wire(trigger.as_bytes(), inner.document(), limits)?;
    if wire != expected {
        return Err(error(ErrorCode::NonCanonical, "document"));
    }
    let document = finish_request(expected, limits, usage)?;
    Ok(ValidatedRequest {
        document,
        inner,
        trigger,
    })
}

/// Produces a v2 request. The one required event trigger supplies the opaque identity.
pub fn produce(
    subject: &ValidatedTemporalSubject,
    input: Input,
    limits: Limits,
) -> Report<Document> {
    let limits = limits.bounded();
    let mut usage = Usage::default();
    let result = (|| {
        if input.triggers.len() != 1 || input.trigger_evidence != temporal::Evidence::Admitted {
            return Err(invalid("trigger"));
        }
        let private = token(input.trigger.as_bytes(), limits)?;
        let triggers = input
            .triggers
            .into_iter()
            .map(|value| temporal::Trigger {
                identity: private.clone(),
                receipt: value.receipt,
                anchor: value.anchor,
                payload: value.payload,
                guard: value.guard,
                captures: value.captures,
            })
            .collect();
        let inner = request::produce(
            subject,
            request::Input {
                instance: private,
                correspondence: input.correspondence,
                positions: input.positions,
                anchor: input.anchor,
                triggers,
                trigger_evidence: input.trigger_evidence,
                trigger_scope: input.trigger_scope,
                decision_progress: input.decision_progress,
                decision_closure: input.decision_closure,
                surrounding_progress: input.surrounding_progress,
                surrounding_closure: input.surrounding_closure,
                execution: input.execution,
                completeness: input.completeness,
                authoritative_origin: input.authoritative_origin,
                evicted: input.evicted,
            },
            limits,
        )
        .into_result()?;
        let wire = request_wire(input.trigger.as_bytes(), &inner, limits)?;
        finish_request(wire, limits, &mut usage)
    })();
    report(limits, usage, result)
}

/// Strictly reads only canonical v2 request bytes against the checked subject.
pub fn read(
    bytes: &[u8],
    subject: &ValidatedTemporalSubject,
    limits: Limits,
) -> Report<ValidatedRequest> {
    let limits = limits.bounded();
    let mut usage = Usage::default();
    let result = (|| {
        let wire: RequestWire = decode(bytes, limits, &mut usage)?;
        let request = admit_request(wire, subject, limits, &mut usage)?;
        if request.document.bytes() != bytes {
            return Err(error(ErrorCode::NonCanonical, "document"));
        }
        Ok(request)
    })();
    report(limits, usage, result)
}

fn result_relation<'a>(
    request: &ValidatedRequest,
    relation: Relation<'a>,
) -> Result<result::Relation<'a>, Error> {
    match relation {
        Relation::Original => Ok(result::Relation::Original),
        Relation::Superseding(previous) => {
            if previous.semantic_trigger() != request.semantic_trigger() {
                Err(invalid("relation.trigger"))
            } else {
                Ok(result::Relation::Superseding(&previous.inner))
            }
        }
        Relation::Invalidating(previous) => {
            if previous.semantic_trigger() != request.semantic_trigger() {
                Err(invalid("relation.trigger"))
            } else {
                Ok(result::Relation::Invalidating(&previous.inner))
            }
        }
    }
}

fn result_wire(
    request: &ValidatedRequest,
    inner: &result::Document,
    limits: Limits,
) -> Result<ResultWire, Error> {
    let trigger = hex(request.semantic_trigger());
    let result = hex(inner.bytes());
    let mut wire = ResultWire {
        contract: RESULT_CONTRACT.into(),
        identity: String::new(),
        request_identity: request.document.identity().into(),
        trigger,
        result,
    };
    wire.identity = identity(
        RESULT_CONTRACT,
        &encode(
            &ResultPreimage {
                contract: &wire.contract,
                request_identity: &wire.request_identity,
                trigger: &wire.trigger,
                result: &wire.result,
            },
            limits,
        )?,
    );
    Ok(wire)
}

fn finish_result(
    wire: ResultWire,
    limits: Limits,
    usage: &mut Usage,
) -> Result<ResultDocument, Error> {
    let bytes = encode(&wire, limits)?;
    usage.output_bytes = bytes.len();
    Ok(ResultDocument {
        digest: raw_digest(&bytes),
        bytes,
        wire,
    })
}

/// Evaluates an exact v2 request and commits its opaque identity in the result.
pub fn evaluate(
    request: &ValidatedRequest,
    relation: Relation<'_>,
    limits: Limits,
) -> Report<ResultDocument> {
    let limits = limits.bounded();
    let mut usage = Usage::default();
    let result = (|| {
        let relation = result_relation(request, relation)?;
        let inner = result::evaluate(&request.inner, relation, limits).into_result()?;
        let wire = result_wire(request, &inner, limits)?;
        finish_result(wire, limits, &mut usage)
    })();
    report(limits, usage, result)
}

/// Strictly reads canonical v2 results, including the request and correction trigger binding.
pub fn read_result(
    bytes: &[u8],
    request: &ValidatedRequest,
    relation: Relation<'_>,
    limits: Limits,
) -> Report<ValidatedResult> {
    let limits = limits.bounded();
    let mut usage = Usage::default();
    let outcome = (|| {
        let wire: ResultWire = decode(bytes, limits, &mut usage)?;
        if wire.contract != RESULT_CONTRACT || wire.request_identity != request.document.identity()
        {
            return Err(invalid("request"));
        }
        let trigger = SemanticTriggerIdentity::new(unhex(&wire.trigger, "trigger", limits)?)?;
        if trigger.as_bytes() != request.semantic_trigger() {
            return Err(invalid("trigger.binding"));
        }
        let relation = result_relation(request, relation)?;
        let inner_bytes = unhex(&wire.result, "result", limits)?;
        let inner = result::read(&inner_bytes, &request.inner, relation, limits).into_result()?;
        let expected = result_wire(request, inner.document(), limits)?;
        if wire != expected {
            return Err(error(ErrorCode::NonCanonical, "document"));
        }
        let document = finish_result(expected, limits, &mut usage)?;
        if document.bytes() != bytes {
            return Err(error(ErrorCode::NonCanonical, "document"));
        }
        Ok(ValidatedResult {
            document,
            inner,
            trigger,
        })
    })();
    report(limits, usage, outcome)
}
