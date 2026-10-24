// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-012-AC-6, FR-012-AC-7 and FR-013-AC-3 over `run_clause`: an
//! operation parameter keeps the invocation-supplied value in a
//! precondition and in a postcondition, a `pre` that holds a parameter
//! beside an eligible state read keeps that value, and a frame the
//! invocation document carries is not the selected frame.
//!
//! The fixture adds `bump(expectedVersion: VersionNumber)` to the
//! ConfigVersion package: it modifies `versionNumber`, creates and deletes
//! nothing. One invocation supplies `expectedVersion = 1` on `child`, whose
//! `versionNumber` is 1 in the pre snapshot and 2 in the post snapshot.

use super::frame::{identifier, object, snapshot};
use super::*;
use crate::spine::OperationName;
use qsl_semantics::model::observation::{AdmissionFailure, FrameChange};

const BUMP_CLAUSES: &str = "\
    pre PreMatches using v on Config::ConfigVersion::bump { \
    self.versionNumber = expectedVersion }\n\
    post PostMatches using v on Config::ConfigVersion::bump { \
    self.versionNumber = expectedVersion }\n\
    post ParamUnderPre using v on Config::ConfigVersion::bump { \
    pre(self.versionNumber = expectedVersion) }\n";

fn bump_operation() -> serde_json::Value {
    let config_version = config_version_type();
    let operation_identity = format!("{config_version}/bump");
    let parameter_identity = format!("{operation_identity}/expectedVersion");
    json!({
        "identity": operation_identity,
        "name": "bump",
        "params": [{
            "identity": parameter_identity,
            "name": "expectedVersion",
            "typeRef": version_number_type(),
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": {
                "generated": {
                    "generatorIdentity": parameter_identity,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [parameter_identity],
                }
            },
        }],
        "pre": [],
        "post": [],
        "origin": {
            "source": {
                "sourceIdentity": format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/spec"),
                "path": "spec.qspec",
                "startLine": 1,
                "startColumn": 1,
            },
        },
        "frame": {
            "modifies": [format!("{config_version}/versionNumber")],
            "creates": [],
            "deletes": [],
        },
    })
}

fn bump_unit_and_packages() -> (String, BTreeMap<[u8; 32], Vec<u8>>, String) {
    let document = config_version_domain_document_with_operations(vec![bump_operation()]);
    let (unit, packages) = config_version_unit_and_packages_for(document);
    let digest = {
        let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
            panic!("one supplied document");
        };
        hex(digest)
    };
    (format!("{unit}{BUMP_CLAUSES}"), packages, digest)
}

fn bump_request(selection: ClauseRunSelection) -> ClauseRunRequest {
    let (unit, packages, _) = bump_unit_and_packages();
    config_version_request_for(unit, packages, "invocation-parameters.native", selection)
}

/// The documents of one `bump` invocation: `child` at 1 before and 2 after,
/// `expectedVersion = 1`, `edit` applied to the invocation document before
/// its digest is taken.
struct BumpInvocation {
    invocation: DocumentRef,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    invocations: BTreeMap<[u8; 32], Vec<u8>>,
}

fn bump_invocation(
    post_child_version: i64,
    edit: impl FnOnce(&mut serde_json::Value),
) -> BumpInvocation {
    let (_, _, model_digest_hex) = bump_unit_and_packages();
    let model_snapshot = |label: &DocumentRef, observation: &str, child_version: i64| {
        let mut document: serde_json::Value = serde_json::from_slice(&snapshot(
            label,
            observation,
            &[
                object("root", 1, None),
                object("child", child_version, Some("root")),
            ],
        ))
        .expect("the fixture is JSON");
        document["model"]["digest"] = json!(format!("sha256-jcs:{model_digest_hex}"));
        document.to_string().into_bytes()
    };
    let (pre, pre_bytes) =
        document_ref_and_bytes("bump-pre", |label| model_snapshot(label, "pre", 1));
    let (post, post_bytes) = document_ref_and_bytes("bump-post", |label| {
        model_snapshot(label, "post", post_child_version)
    });
    let (invocation, invocation_bytes) = document_ref_and_bytes("bump-invocation", |label| {
        let mut document: serde_json::Value = serde_json::from_slice(
            &config_version_invocation_bytes(label, &model_digest_hex, &pre, &post, "child"),
        )
        .expect("the fixture is JSON");
        document["operation"] = json!("bump");
        document["parameters"] = json!({"expectedVersion": {"integer": "1"}});
        document["result"] = serde_json::Value::Null;
        edit(&mut document);
        document.to_string().into_bytes()
    });
    BumpInvocation {
        invocation: invocation.clone(),
        snapshots: BTreeMap::from([(pre.digest, pre_bytes), (post.digest, post_bytes)]),
        invocations: BTreeMap::from([(invocation.digest, invocation_bytes)]),
    }
}

fn run_bump(
    input: BumpInvocation,
    selection: impl FnOnce(DocumentRef) -> ClauseRunSelection,
) -> super::super::ClauseRunReport {
    let mut request = bump_request(selection(input.invocation));
    request.snapshots.extend(input.snapshots);
    request.invocations.extend(input.invocations);
    run_clause(request).expect("a well-formed request always reports")
}

fn run_clause_over(input: BumpInvocation, clause: &str) -> super::super::ClauseRunReport {
    run_bump(input, |invocation| {
        ClauseRunSelection::Clause(config_version_invocation_selection(clause, invocation))
    })
}

/// QSpec FR-012-AC-6: the parameter keeps `expectedVersion = 1` in the
/// precondition, which reads `self.versionNumber` = 1 from the pre
/// snapshot (true), and in the postcondition, which reads 2 from the post
/// snapshot (false). Had either clause read the other observation's value
/// of the parameter, or a field of that name, the verdicts would swap.
#[trace("QSpec-V1-EXPR-019", "QSpec-V1-EXPR-020", "QSpec-FR-012-AC-6")]
#[test]
fn an_operation_parameter_keeps_the_invocation_value_in_a_precondition_and_a_postcondition() {
    let precondition = run_clause_over(bump_invocation(2, |_| {}), "PreMatches");
    assert!(boolean_disposition(&precondition.disposition));
    let postcondition = run_clause_over(bump_invocation(2, |_| {}), "PostMatches");
    assert!(!boolean_disposition(&postcondition.disposition));
    // Control: a post snapshot that still holds 1 makes the postcondition
    // true, so the verdict follows the observation, not a constant.
    let unchanged = run_clause_over(bump_invocation(1, |_| {}), "PostMatches");
    assert!(boolean_disposition(&unchanged.disposition));
}

/// QSpec FR-012-AC-7: `pre` applied to an expression that holds the
/// parameter beside an eligible state read keeps the parameter's
/// invocation-supplied value: `pre(self.versionNumber = expectedVersion)`
/// compares the pre value 1 with the supplied 1 (true) even though the post
/// snapshot holds 2, and with a supplied 2 it is false.
#[trace("QSpec-V1-EXPR-020", "QSpec-FR-012-AC-7")]
#[test]
fn pre_over_an_expression_holding_a_parameter_keeps_the_parameter_value() {
    let kept = run_clause_over(bump_invocation(2, |_| {}), "ParamUnderPre");
    assert!(boolean_disposition(&kept.disposition));
    let other_value = run_clause_over(
        bump_invocation(2, |document| {
            document["parameters"] = json!({"expectedVersion": {"integer": "2"}});
        }),
        "ParamUnderPre",
    );
    assert!(!boolean_disposition(&other_value.disposition));
}

fn bump_frame_selection(invocation: DocumentRef) -> ClauseRunSelection {
    ClauseRunSelection::Frame {
        operation: OperationName {
            model: identifier("Config"),
            object: identifier("ConfigVersion"),
            operation: identifier("bump"),
        },
        invocation,
    }
}

/// QSpec FR-013-AC-3: the frame comes only from the compiled package. An
/// invocation document that carries a `frame` granting `parent` -- a
/// replacement for the package's frame, which nothing selected -- is
/// refused, and the same change without that member is a violation of the
/// package's frame.
#[trace("QSpec-V1-EXPR-022", "QSpec-FR-013-AC-3")]
#[test]
fn an_invocation_carrying_a_replacement_frame_is_refused_not_obeyed() {
    // The post snapshot changes `child.parent`, which `bump` does not grant.
    let widen = |document: &mut serde_json::Value| {
        document["frame"] = json!({"modifies": [format!("{}/parent", config_version_type())]});
    };
    let changed_parent = |edit: fn(&mut serde_json::Value)| {
        let mut input = bump_invocation(1, edit);
        // Re-point `post` at a snapshot whose `child.parent` is absent.
        let (_, _, model_digest_hex) = bump_unit_and_packages();
        let (post, post_bytes) = document_ref_and_bytes("bump-post-parent", |label| {
            let mut document: serde_json::Value = serde_json::from_slice(&snapshot(
                label,
                "post",
                &[object("root", 1, None), object("child", 1, None)],
            ))
            .expect("the fixture is JSON");
            document["model"]["digest"] = json!(format!("sha256-jcs:{model_digest_hex}"));
            document.to_string().into_bytes()
        });
        input.snapshots.insert(post.digest, post_bytes);
        let (invocation, invocation_bytes) = document_ref_and_bytes("bump-invocation", |label| {
            let pre = input
                .snapshots
                .keys()
                .find(|digest| **digest != post.digest)
                .copied()
                .expect("the pre snapshot");
            let pre_ref = DocumentRef {
                digest: pre,
                ..config_version_document_ref("bump-pre")
            };
            let mut document: serde_json::Value =
                serde_json::from_slice(&config_version_invocation_bytes(
                    label,
                    &model_digest_hex,
                    &pre_ref,
                    &post,
                    "child",
                ))
                .expect("the fixture is JSON");
            document["operation"] = json!("bump");
            document["parameters"] = json!({"expectedVersion": {"integer": "1"}});
            document["result"] = serde_json::Value::Null;
            edit(&mut document);
            document.to_string().into_bytes()
        });
        input.invocation = invocation.clone();
        input.invocations = BTreeMap::from([(invocation.digest, invocation_bytes)]);
        input
    };

    let package_frame = run_bump(changed_parent(|_| {}), bump_frame_selection);
    let ClauseDisposition::FrameViolation(witness) = &package_frame.disposition else {
        panic!(
            "expected FrameViolation, got {:?}",
            package_frame.disposition
        );
    };
    let FrameChange::FieldWrite { field, .. } = &witness.change else {
        panic!("expected a field write, got {:?}", witness.change);
    };
    assert_eq!(field, "parent");

    let replacement = run_bump(changed_parent(widen), bump_frame_selection);
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = &replacement.disposition
    else {
        panic!(
            "expected a refused admission, got {:?}",
            replacement.disposition
        );
    };
    assert_eq!(record.code.as_str(), "invalid_runtime_input");
    assert_eq!(record.cause, "unknown-member");
}
