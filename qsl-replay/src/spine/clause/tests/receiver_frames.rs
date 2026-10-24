// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1043 step 6 (FR-115-AC-11): `Frame` runs over TC-1043's `Counter`
//! package, whose operations carry receiver-scoped and scoped-delete
//! entries. The run reads the frame from the checked graph of the emitted
//! package, so a receiver-scoped entry grants the receiver only.

use super::frame::identifier;
use super::*;
use crate::spine::{CallOutcome, OperationName};
use qsl_semantics::model::observation::{AdmissionFailure, FrameChange};

#[path = "../../../../../qsl-semantics/tests/support/receiver_frames.rs"]
pub(super) mod fixture;

fn type_of(name: &str) -> String {
    fixture::id(name)
}

fn population() -> String {
    fixture::id("counters")
}

/// The unit selecting the fixture's package for `operations`, and its
/// package input.
pub(super) fn unit_and_packages(operations: &[&str]) -> (String, BTreeMap<[u8; 32], Vec<u8>>) {
    let document = serde_json::to_vec(&fixture::document(operations, &[])).expect("fixture");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model Counters = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         {posts}",
        package = fixture::PACKAGE,
        digest = hex(digest),
        posts = fixture::posts(operations).replace("M::", "Counters::"),
    );
    (unit, packages)
}

pub(super) fn counter(key: &str, value: i64) -> serde_json::Value {
    json!({
        "key": key, "type": type_of("Counter"),
        "fields": {
            "counter_id": {"integer": "1"},
            "value": {"integer": value.to_string()},
            "step": {"integer": "0"},
        },
    })
}

pub(super) fn snapshot(
    operations: &[&str],
    label: &DocumentRef,
    observation: &str,
    objects: &[serde_json::Value],
) -> Vec<u8> {
    json!({
        "format": "quire.state.snapshot/v1",
        "identity": document_identity_json(label),
        "observation": observation,
        "model": {"identity": fixture::PACKAGE, "version": "1", "digest": model_digest(operations)},
        "populations": [{"population": population(), "complete": true, "objects": objects}],
    })
    .to_string()
    .into_bytes()
}

fn model_digest(operations: &[&str]) -> String {
    let (_, packages) = unit_and_packages(operations);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    format!("sha256-jcs:{}", hex(digest))
}

/// Runs the `Frame` of `operation` over an invocation with `self` `c1` whose
/// pre snapshot holds `c1` and `c2` at value 0 and whose post holds
/// `post_objects`, declaring `deleted`.
fn run_frame(
    operations: &[&str],
    operation: &str,
    post_objects: &[serde_json::Value],
    deleted: &[&str],
) -> super::super::ClauseRunReport {
    run_selected(
        operations,
        operation,
        post_objects,
        deleted,
        |name, invocation| ClauseRunSelection::Frame {
            operation: name,
            invocation,
        },
    )
}

/// Runs `selection` over the invocation [`run_frame`] describes.
fn run_selected(
    operations: &[&str],
    operation: &str,
    post_objects: &[serde_json::Value],
    deleted: &[&str],
    selection: impl FnOnce(OperationName, DocumentRef) -> ClauseRunSelection,
) -> super::super::ClauseRunReport {
    let (unit, packages) = unit_and_packages(operations);
    let (pre, pre_bytes) = document_ref_and_bytes("counters-pre", |label| {
        snapshot(
            operations,
            label,
            "pre",
            &[counter("c1", 0), counter("c2", 0)],
        )
    });
    let (post, post_bytes) = document_ref_and_bytes("counters-post", |label| {
        snapshot(operations, label, "post", post_objects)
    });
    let (invocation, invocation_bytes) = document_ref_and_bytes("counters-invocation", |label| {
        json!({
            "format": "quire.state.invocation/v1",
            "identity": document_identity_json(label),
            "model": {"identity": fixture::PACKAGE, "version": "1", "digest": model_digest(operations)},
            "context": type_of("Counter"),
            "operation": operation,
            "self": {"population": population(), "key": "c1"},
            "pre": {"identity": document_identity_json(&pre),
                    "digest": format!("sha256-jcs:{}", hex(&pre.digest))},
            "post": {"identity": document_identity_json(&post),
                     "digest": format!("sha256-jcs:{}", hex(&post.digest))},
            "parameters": {},
            "result": null,
            "created": [],
            "deleted": deleted.iter().map(|key| json!({"population": population(), "key": key})).collect::<Vec<_>>(),
        })
        .to_string()
        .into_bytes()
    });
    let mut request = config_version_request_for(
        unit,
        packages,
        "counters.native",
        selection(
            OperationName {
                model: identifier("Counters"),
                object: identifier("Counter"),
                operation: identifier(operation),
            },
            invocation.clone(),
        ),
    );
    request.snapshots.insert(pre.digest, pre_bytes);
    request.snapshots.insert(post.digest, post_bytes);
    request
        .invocations
        .insert(invocation.digest, invocation_bytes);
    run_clause(request).expect("a well-formed request always reports")
}

/// F1 and F2: `c1.value` stays and `c2.value` becomes 1. Under `own`
/// (`modifies self.value`, self `c1`) that is a violation naming `c2`,
/// `value` and `modifies` holding the key of `value`; under `all`
/// (`modifies value`) the run succeeds.
#[trace("TC-1043", "FR-115-AC-11")]
#[test]
fn a_receiver_scoped_entry_grants_the_receiver_and_not_another_object() {
    let post = [counter("c1", 0), counter("c2", 1)];
    let report = run_frame(&["own", "all"], "own", &post, &[]);
    let ClauseDisposition::FrameViolation(witness) = &report.disposition else {
        panic!("expected FrameViolation, got {:?}", report.disposition);
    };
    let FrameChange::FieldWrite {
        object,
        field,
        modifies,
    } = &witness.change
    else {
        panic!("expected a field write, got {:?}", witness.change);
    };
    assert_eq!(object, "c2");
    assert_eq!(field, "value");
    let modifies: Vec<&str> = modifies.iter().map(|key| key.node.as_str()).collect();
    assert_eq!(modifies, [fixture::id("Counter/value")]);
    assert_eq!(witness.cause, "unauthorized-change");

    let report = run_frame(&["own", "all"], "all", &post, &[]);
    assert!(
        matches!(
            report.disposition,
            ClauseDisposition::Evaluate(CallOutcome::Completed(super::super::CallValue::Boolean(
                true
            )))
        ),
        "{:?}",
        report.disposition
    );
}

/// F3: `dispose` (`deletes self`, `deletes Gauge`) with self `c1` and a post
/// deleting `c2` is a violation with the `Deleted` change naming `c2`, type
/// `Counter` and `deletes` holding the key of `Gauge` only.
#[trace("TC-1043", "FR-115-AC-11")]
#[test]
fn deleting_self_grants_the_receiver_and_not_another_object() {
    let report = run_frame(&["dispose"], "dispose", &[counter("c1", 0)], &["c2"]);
    let ClauseDisposition::FrameViolation(witness) = &report.disposition else {
        panic!("expected FrameViolation, got {:?}", report.disposition);
    };
    let FrameChange::Deleted {
        object,
        type_name,
        deletes,
    } = &witness.change
    else {
        panic!("expected a deletion, got {:?}", witness.change);
    };
    assert_eq!(object, "c2");
    assert_eq!(type_name.node, type_of("Counter"));
    let deletes: Vec<&str> = deletes.iter().map(|key| key.node.as_str()).collect();
    assert_eq!(deletes, [type_of("Gauge")]);
}

/// FR-154-AC-3 (TC-573 step 1): an invocation of `increment` (`own`, framed
/// `modifies self.value`, and `all`, framed `modifies value`) on `self = c1`
/// whose post snapshot changes `c2.value`. The `Frame` run settles
/// `violation` (exit 10) with the witness naming `c2.value` under `own` and
/// `success` (exit 0) under `all`; the clause run of the operation's
/// postcondition over the same invocation refuses `frame_violation` at
/// admission under `own` and admits and evaluates under `all`.
#[trace("TC-573", "FR-154-AC-3")]
#[test]
fn the_frame_run_and_the_clause_admission_apply_the_receiver_scope() {
    let post = [counter("c1", 0), counter("c2", 1)];
    let clause = |name: &'static str| {
        move |_: OperationName, invocation: DocumentRef| {
            ClauseRunSelection::Clause(ClauseSelection {
                name: name.to_owned(),
                input: ClauseSelectionInput::Invocation { invocation },
            })
        }
    };

    let own = run_frame(&["own", "all"], "own", &post, &[]);
    let ClauseDisposition::FrameViolation(witness) = &own.disposition else {
        panic!("expected FrameViolation, got {:?}", own.disposition);
    };
    let FrameChange::FieldWrite { object, field, .. } = &witness.change else {
        panic!("expected a field write, got {:?}", witness.change);
    };
    assert_eq!((object.as_str(), field.as_str()), ("c2", "value"));
    assert_eq!(
        own.disposition.category(),
        quire_semantic_value::catalog::Category::Violation
    );
    assert_eq!(own.disposition.category().exit_code(), 10);

    let all = run_frame(&["own", "all"], "all", &post, &[]);
    assert!(
        matches!(
            all.disposition,
            ClauseDisposition::Evaluate(CallOutcome::Completed(super::super::CallValue::Boolean(
                true
            )))
        ),
        "{:?}",
        all.disposition
    );
    assert_eq!(
        all.disposition.category(),
        quire_semantic_value::catalog::Category::Success
    );
    assert_eq!(all.disposition.category().exit_code(), 0);

    let admitted_own = run_selected(&["own", "all"], "own", &post, &[], clause("DoneOwn"));
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = &admitted_own.disposition
    else {
        panic!(
            "expected a refused admission, got {:?}",
            admitted_own.disposition
        );
    };
    assert_eq!(
        record.code,
        qsl_foundation::diagnostic::Code::FrameViolation
    );
    assert_eq!(record.cause, "unauthorized-change");

    let admitted_all = run_selected(&["own", "all"], "all", &post, &[], clause("DoneAll"));
    assert!(
        matches!(admitted_all.disposition, ClauseDisposition::Evaluate(_)),
        "{:?}",
        admitted_all.disposition
    );
}
