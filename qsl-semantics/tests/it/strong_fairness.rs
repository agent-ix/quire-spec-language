// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-129 (TC-530): the `TemporalTrace` family's check of strong fairness
//! constraints and of the unmarked fairness kind, over ADR-019 §6's mutex:
//! object type `Mutex` with `owner: Int[0, 2]`, population `locks`,
//! operations `acquire(p: Int[1, 2])` and `release()`, both with frame
//! `modifies [owner]`. The S3 check reads only the declarations, so the
//! operations carry no contracts here.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::{Code, StageFailure};
use qsl_foundation::Span;
use qsl_semantics::check::{
    check_clauses, CheckCause, CheckRefusal, CheckedTemporalClause, TemporalCause,
};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity, FairnessKind};
use serde_json::{json, Value};

use crate::model_operations::{
    admit_and_assemble_with_profiles, config_unit_with_profiles, wire_construct, wire_envelope,
    wire_field, wire_type, PACKAGE_IDENTITY,
};
use qsl_semantics::model::intake::meaning;

const EVENT: &str = "quire.temporal.event-position.false-extension/v1";
const INFINITE: &str = "quire.temporal.infinite-trace/v1";

const ACQUIRE: &str = "Config::Mutex::acquire";
const SERVED: &str = "always eventually holds(m.owner = 1)";

fn identity(suffix: &str) -> String {
    format!("ix://{PACKAGE_IDENTITY}/{suffix}")
}

fn generated(identity: &str) -> Value {
    json!({
        "generated": {
            "generatorIdentity": identity,
            "generatorVersion": "1.0.0",
            "inputIdentities": [identity],
        }
    })
}

/// A package-declared integer value type `name` bound to `[min, max]`.
fn bound_integer(name: &str, min: &str, max: &str) -> Value {
    let type_identity = identity(name);
    let constraint = |keyword: &str, value: &str| {
        json!({
            "identity": format!("{type_identity}/constraints/{keyword}"),
            "keyword": keyword,
            "operands": {"value": value},
            "appliesTo": "ix://quire/native/Integer",
            "diagnosticCode": format!("bound.{keyword}"),
            "origin": generated(&type_identity),
        })
    };
    wire_type(
        &type_identity,
        "value_type",
        json!({
            "scalar": "integer",
            "constraints": [constraint("min", min), constraint("max", max)],
        }),
    )
}

/// The `Mutex` operation `name` with `params`, no result, modifying `owner`.
fn mutex_operation(name: &str, params: Value) -> Value {
    json!({
        "identity": identity(&format!("Mutex/{name}")),
        "name": name,
        "params": params,
        "pre": [],
        "post": [],
        "origin": {
            "source": {
                "sourceIdentity": identity("spec"),
                "path": "spec.qspec",
                "startLine": 1,
                "startColumn": 1,
            },
        },
        "frame": {
            "modifies": [identity("Mutex/owner")],
            "creates": [],
            "deletes": [],
        },
    })
}

/// ADR-019 §6's mutex package document.
fn mutex_document() -> Vec<u8> {
    let mutex = identity("Mutex");
    let process = identity("Mutex/acquire/p");
    let constructs = json!([
        wire_construct("object_type", meaning::OBJECT_TYPE, json!({})),
        wire_construct("population", meaning::POPULATION, json!({})),
        wire_construct("value_type", meaning::VALUE_TYPE, json!({})),
    ]);
    let acquire = mutex_operation(
        "acquire",
        json!([{
            "identity": process,
            "name": "p",
            "typeRef": identity("Process"),
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": generated(&process),
        }]),
    );
    let types = json!([
        bound_integer("Owner", "0", "2"),
        bound_integer("Process", "1", "2"),
        wire_type(
            &mutex,
            "object_type",
            json!({
                "supertypes": [],
                "fields": [wire_field(
                    &identity("Mutex/owner"),
                    "owner",
                    &identity("Owner"),
                    "required",
                    1,
                )],
                "operations": [acquire, mutex_operation("release", json!([]))],
                "relationships": [],
            }),
        ),
    ]);
    let locks = identity("locks");
    let populations = json!([{
        "identity": locks,
        "displayName": locks,
        "kind": {"module": PACKAGE_IDENTITY, "name": "population"},
        "members": [mutex],
        "extent": "closed",
        "origin": generated(&locks),
    }]);
    wire_envelope(constructs, types, populations)
        .to_string()
        .into_bytes()
}

/// The unit's header profile `t` selecting `profile`.
fn profile_header(profile: &str) -> String {
    format!("profile t = \"{profile}\";\n")
}

/// The unit's one clause `Served` over `m: Config::Mutex` with `body`.
fn clause_body(body: &str) -> String {
    format!(
        "temporal Served using t over (m: Config::Mutex) clock \"model-steps\" on origin {{ {body} }}"
    )
}

/// The unit text and the clause `body` checks to under `profile`.
fn check_in(
    profile: &str,
    body: &str,
) -> (
    String,
    Result<CheckedTemporalClause, StageFailure<CheckRefusal>>,
) {
    let (profiles, declarations) = (profile_header(profile), clause_body(body));
    let package = admit_and_assemble_with_profiles(&mutex_document(), &profiles, &declarations)
        .unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"));
    let (text, _) = config_unit_with_profiles(&mutex_document(), &profiles, &declarations);
    let mut checked = check_clauses(&package);
    let Some(outcome) = checked.pop() else {
        panic!("{body}: one clause");
    };
    assert!(checked.is_empty(), "{body}: one clause");
    (text, outcome)
}

/// The fairness set of the clause `fair <written>; always eventually ...`
/// under the infinite-trace profile.
fn fairness_of(written: &str) -> Vec<FairnessConstraint> {
    let body = format!("{written} {SERVED}");
    match check_in(INFINITE, &body) {
        (_, Ok(clause)) => clause.fairness().to_vec(),
        (_, Err(failure)) => panic!("{body}: the clause checks, not {failure:?}"),
    }
}

/// The node identity of the clause `<written> always eventually ...` under
/// the infinite-trace profile, through the whole package check (FR-337).
fn node_identity(written: &str) -> quire_exact::NodeKey {
    let body = format!("{written} {SERVED}");
    let package = admit_and_assemble_with_profiles(
        &mutex_document(),
        &profile_header(INFINITE),
        &clause_body(&body),
    )
    .unwrap_or_else(|refusal| panic!("{body}: the unit assembles: {refusal:?}"));
    let graph = package
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{body}: the package checks: {refusals:?}"));
    let [clause] = graph.temporal_clauses() else {
        panic!("{body}: one emitted clause");
    };
    clause.identity()
}

/// `(kind, operation name, granularity)` of each member, in set order.
fn members(set: &[FairnessConstraint]) -> Vec<(FairnessKind, &str, FairnessGranularity)> {
    set.iter()
        .map(|constraint| {
            (
                constraint.kind(),
                constraint.operation().name(),
                constraint.granularity(),
            )
        })
        .collect()
}

/// FR-129-AC-1: a constraint on `acquire` with no kind and no granularity
/// checks to `{Weak, acquire, Whole}` and gives the clause the identity of
/// `weak whole`; `strong` checks to `{Strong, acquire, Whole}` and `strong
/// each` to `{Strong, acquire, Each}`; the `strong each` and `weak each`
/// clauses have different node identities.
#[trace("TC-530", "FR-129-AC-1")]
#[test]
fn an_unmarked_kind_is_weak_and_the_kind_enters_the_identity() {
    use FairnessGranularity::{Each, Whole};
    use FairnessKind::{Strong, Weak};

    let unmarked = fairness_of(&format!("fair {ACQUIRE};"));
    assert_eq!(members(&unmarked), [(Weak, "acquire", Whole)]);
    assert_eq!(
        unmarked,
        fairness_of(&format!("fair weak whole {ACQUIRE};"))
    );
    assert_eq!(
        members(&fairness_of(&format!("fair strong {ACQUIRE};"))),
        [(Strong, "acquire", Whole)]
    );
    assert_eq!(
        members(&fairness_of(&format!("fair strong each {ACQUIRE};"))),
        [(Strong, "acquire", Each)]
    );

    assert_eq!(
        node_identity(&format!("fair {ACQUIRE};")),
        node_identity(&format!("fair weak whole {ACQUIRE};"))
    );
    let strong_each = node_identity(&format!("fair strong each {ACQUIRE};"));
    assert_ne!(
        strong_each,
        node_identity(&format!("fair weak each {ACQUIRE};"))
    );
    assert_ne!(
        node_identity(&format!("fair strong {ACQUIRE};")),
        node_identity(&format!("fair weak {ACQUIRE};"))
    );
}

/// FR-129-AC-2: `strong each` and `weak each` on `acquire` check to a
/// two-member set in source order; `strong` written twice to one member; a
/// `strong` constraint under the event-position profile refuses
/// `unsupported_construct`/`expression-form` at the constraint's span.
#[trace("TC-530", "FR-129-AC-2")]
#[test]
fn weak_and_strong_stay_apart_and_strong_needs_an_infinite_trace() {
    use FairnessGranularity::Each;
    use FairnessKind::{Strong, Weak};

    assert_eq!(
        members(&fairness_of(&format!(
            "fair strong each {ACQUIRE}; fair weak each {ACQUIRE};"
        ))),
        [(Strong, "acquire", Each), (Weak, "acquire", Each)]
    );
    assert_eq!(
        members(&fairness_of(&format!(
            "fair weak each {ACQUIRE}; fair strong each {ACQUIRE};"
        ))),
        [(Weak, "acquire", Each), (Strong, "acquire", Each)]
    );
    assert_eq!(
        fairness_of(&format!("fair strong {ACQUIRE}; fair strong {ACQUIRE};")).len(),
        1
    );

    let constraint = format!("fair strong {ACQUIRE};");
    let body = format!("{constraint} eventually[0,5] holds(m.owner = 1)");
    let (text, outcome) = check_in(EVENT, &body);
    let Err(StageFailure::Refused(CheckRefusal {
        cause: CheckCause::Temporal(cause),
        ..
    })) = outcome
    else {
        panic!("{body}: a temporal refusal, not {outcome:?}");
    };
    let start = text
        .find(&constraint)
        .expect("the constraint is in the unit");
    assert_eq!(
        *cause,
        TemporalCause::FairnessUnderBoundedProfile {
            span: Span {
                start,
                end: start + constraint.len(),
            }
        }
    );
    assert_eq!(cause.code(), Code::UnsupportedConstruct);
    assert_eq!(cause.tag(), "expression-form");
}
