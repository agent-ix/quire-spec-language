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

use super::frame::{object, snapshot};
use super::*;

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

/// Runs `ParentPresent` over the forbidden-parent-change invocation (self
/// `child`; post sets `child.parent` absent, outside `attemptUpdate`'s
/// frame), with the post snapshot's digest finding `post` in the
/// provision.
fn run_invocation(post: PostProvision) -> super::super::ClauseRunReport {
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
            config_version_invocation_bytes(label, &model_digest_hex, &pre, &post_label, "child")
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

    let invocation = run_invocation(PostProvision::Snapshot);
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
        let report = run_invocation(post);
        assert_eq!(boolean_disposition(&report.disposition), pre_call);
    }
}
