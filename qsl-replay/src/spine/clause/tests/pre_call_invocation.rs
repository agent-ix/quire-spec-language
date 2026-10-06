// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-464 step 6 (FR-106-AC-9): a precondition selected by `Invocation`
//! reads the pre-call observation alone -- the invocation's `pre`
//! snapshot, `self` and parameters -- and never its post side.
//!
//! The precondition `ParentPresent` holds over the forbidden-parent-change
//! invocation's pre snapshot (`child.parent` names `root`) and fails over
//! its post snapshot (`child.parent` absent), so a run that read the post
//! side would report `false`, and one that ran the frame check would
//! refuse `frame_violation`.

use super::frame::{identifier, object, snapshot};
use super::*;
use qsl_semantics::model::observation::SnapshotValue;

const PARENT_PRESENT: &str = "ParentPresent";

/// The base ConfigVersion unit plus the precondition `ParentPresent` on
/// `attemptUpdate`.
fn request(selection: ClauseRunSelection) -> ClauseRunRequest {
    let (base_unit, packages) = config_version_unit_and_packages();
    let unit = format!(
        "{base_unit}\
         pre {PARENT_PRESENT} using v on Config::ConfigVersion::attemptUpdate {{ \
         present(self.parent) }}\n"
    );
    config_version_request_for(unit, packages, "pre-call-invocation.native", selection)
}

/// What the post snapshot's digest finds in the snapshot provision.
enum PostProvision {
    /// The post snapshot itself.
    Snapshot,
    /// Nothing: the post snapshot is absent from the provision.
    Absent,
    /// Bytes that are not JSON.
    Malformed,
}

/// The forbidden-parent-change pre snapshot: `root` at 1, `child` at 2
/// naming `root`.
fn pre_snapshot() -> (DocumentRef, Vec<u8>) {
    document_ref_and_bytes("forbidden-parent-change-pre", |label| {
        snapshot(
            label,
            "pre",
            &[object("root", 1, None), object("child", 2, Some("root"))],
        )
    })
}

fn self_child() -> SelectedObject {
    SelectedObject {
        population: config_version_population_identity(),
        key: "child".to_owned(),
    }
}

/// `bytes` (a JSON document) with `edit` applied.
fn edited(bytes: &[u8], edit: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut document: serde_json::Value =
        serde_json::from_slice(bytes).expect("the fixture is JSON");
    edit(&mut document);
    document.to_string().into_bytes()
}

/// Runs `ParentPresent` over the forbidden-parent-change invocation (self
/// `child`; post sets `child.parent` absent, outside `attemptUpdate`'s
/// frame), with the post snapshot's digest finding `post` in the
/// provision and `edit` applied to the invocation before its digest is
/// taken.
fn run_invocation(
    post: PostProvision,
    edit: impl FnOnce(&mut serde_json::Value),
) -> super::super::ClauseRunReport {
    let model_digest_hex = config_version_model_digest_hex();
    let (pre, pre_bytes) = pre_snapshot();
    let (post_label, post_bytes) =
        document_ref_and_bytes("forbidden-parent-change-post", |label| {
            snapshot(
                label,
                "post",
                &[object("root", 1, None), object("child", 2, None)],
            )
        });
    let (invocation, invocation_bytes) =
        document_ref_and_bytes("forbidden-parent-change", |label| {
            edited(
                &config_version_invocation_bytes(
                    label,
                    &model_digest_hex,
                    &pre,
                    &post_label,
                    "child",
                ),
                edit,
            )
        });

    let mut request = request(ClauseRunSelection::Clause(
        config_version_invocation_selection(PARENT_PRESENT, invocation.clone()),
    ));
    request.snapshots.insert(pre.digest, pre_bytes);
    match post {
        PostProvision::Snapshot => {
            request.snapshots.insert(post_label.digest, post_bytes);
        }
        PostProvision::Absent => {}
        PostProvision::Malformed => {
            request
                .snapshots
                .insert(post_label.digest, b"not json {".to_vec());
        }
    }
    request
        .invocations
        .insert(invocation.digest, invocation_bytes);
    let report = run_clause(request).expect("a well-formed request always reports");
    assert_eq!(
        report.provenance.documents,
        [invocation, pre],
        "the run reads the invocation and its pre snapshot, and no post snapshot"
    );
    report
}

/// `ParentPresent` selected by `PreCall` over the invocation's pre
/// snapshot, `self` and (empty) parameters.
fn run_pre_call() -> ClauseDisposition {
    let (pre, pre_bytes) = pre_snapshot();
    let mut request = request(ClauseRunSelection::Clause(ClauseSelection {
        name: PARENT_PRESENT.to_owned(),
        input: ClauseSelectionInput::PreCall {
            snapshot: pre.clone(),
            self_object: self_child(),
            parameters: BTreeMap::new(),
        },
    }));
    request.snapshots.insert(pre.digest, pre_bytes);
    run_clause(request)
        .expect("a well-formed request always reports")
        .disposition
}

/// TC-464 step 6: the invocation admits the pre observation alone (no
/// frame check, so no `frame_violation`) and `ParentPresent` evaluates
/// over it to `true`, the `PreCall` verdict. Reading the post snapshot
/// would give `false`.
#[trace("TC-464", "FR-106-AC-9")]
#[test]
fn a_precondition_over_an_invocation_reads_only_the_pre_side() {
    let pre_call = boolean_disposition(&run_pre_call());
    assert!(pre_call, "`child.parent` names `root` in the pre snapshot");

    let invocation = run_invocation(PostProvision::Snapshot, |_| {});
    assert_eq!(boolean_disposition(&invocation.disposition), pre_call);
}

/// TC-464 step 6: with the post snapshot absent from the provision, or its
/// bytes not JSON, the invocation still admits and gives the `PreCall`
/// verdict, with no `unavailable_observation`, digest or other refusal.
#[trace("TC-464", "FR-106-AC-9")]
#[test]
fn a_precondition_over_an_invocation_ignores_a_missing_or_malformed_post() {
    let pre_call = boolean_disposition(&run_pre_call());
    for post in [PostProvision::Absent, PostProvision::Malformed] {
        let report = run_invocation(post, |_| {});
        assert_eq!(boolean_disposition(&report.disposition), pre_call);
    }
}

/// TC-464 step 6: the invocation with its `post`, `result`, `created` and
/// `deleted` members removed, and again with each of them ill-formed,
/// admits and gives the `PreCall` verdict: a precondition neither requires
/// nor reads them.
#[trace("TC-464", "FR-106-AC-9")]
#[test]
fn a_precondition_over_an_invocation_neither_requires_nor_reads_its_post_side_members() {
    const POST_SIDE: [&str; 4] = ["post", "result", "created", "deleted"];
    let pre_call = boolean_disposition(&run_pre_call());
    let removed = run_invocation(PostProvision::Absent, |document| {
        let members = document.as_object_mut().expect("an invocation object");
        for member in POST_SIDE {
            members.remove(member);
        }
    });
    assert_eq!(boolean_disposition(&removed.disposition), pre_call);
    let ill_formed = run_invocation(PostProvision::Absent, |document| {
        for member in POST_SIDE {
            document[member] = json!("ill-formed");
        }
    });
    assert_eq!(boolean_disposition(&ill_formed.disposition), pre_call);
}

/// TC-464 step 6: a `probe` invocation (no declared result) carrying
/// `result` `{"boolean": true}`, selected for the precondition
/// `ReachesTarget`, admits and gives the `PreCall` verdict: the result
/// check is a postcondition's.
#[trace("TC-464", "FR-106-AC-9")]
#[test]
fn a_precondition_over_an_invocation_reads_no_result_value() {
    let model_digest_hex = config_version_step3_model_digest_hex();
    let objects = config_version_step3_chain_objects();
    let (pre, pre_bytes) = document_ref_and_bytes("probe-result-pre", |label| {
        config_version_step3_snapshot(label, &model_digest_hex, "pre", &objects)
    });
    let (post, post_bytes) = document_ref_and_bytes("probe-result-post", |label| {
        config_version_step3_snapshot(label, &model_digest_hex, "post", &objects)
    });
    let (invocation, invocation_bytes) = document_ref_and_bytes("probe-result", |label| {
        edited(
            &config_version_probe_invocation_bytes(label, &model_digest_hex, &pre, &post, "a", "c"),
            |document| document["result"] = json!({"boolean": true}),
        )
    });

    let mut over_invocation = config_version_step3_request(ClauseRunSelection::Clause(
        config_version_invocation_selection("ReachesTarget", invocation.clone()),
    ));
    over_invocation
        .snapshots
        .insert(pre.digest, pre_bytes.clone());
    over_invocation.snapshots.insert(post.digest, post_bytes);
    over_invocation
        .invocations
        .insert(invocation.digest, invocation_bytes);

    let mut pre_call = config_version_step3_request(ClauseRunSelection::Clause(ClauseSelection {
        name: "ReachesTarget".to_owned(),
        input: ClauseSelectionInput::PreCall {
            snapshot: pre.clone(),
            self_object: SelectedObject {
                population: config_version_population_identity(),
                key: "a".to_owned(),
            },
            parameters: BTreeMap::from([(
                identifier("target"),
                SnapshotValue::reference(SelectedObject {
                    population: config_version_population_identity(),
                    key: "c".to_owned(),
                }),
            )]),
        },
    }));
    pre_call.snapshots.insert(pre.digest, pre_bytes);

    let run = |request| {
        run_clause(request)
            .expect("a well-formed request always reports")
            .disposition
    };
    let pre_call = boolean_disposition(&run(pre_call));
    assert!(pre_call, "a -> b -> c: a reaches c through parent");
    assert_eq!(boolean_disposition(&run(over_invocation)), pre_call);
}
