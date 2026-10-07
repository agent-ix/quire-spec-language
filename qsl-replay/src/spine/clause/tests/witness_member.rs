// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-740 (FR-265): the separating witness of a claim whose domain is a
//! collection-typed object member, over `Config` with a `history` member
//! (`Sequence<VersionNumber>[0, 3]`). The record's value path names the
//! stored member, in the observation the claim reads it from.

use super::*;
use crate::{ObservationIdentity, RuntimeValuePath, ValuePathStep, ValuePathSubject};
use qsl_semantics::model::observation::SelectedObject;

/// The claims over `history`: a direct read, a set conversion of it, and a
/// precondition of `attemptUpdate` reading it.
const CLAUSES: &str = "\
    type V = Int[0, 1000];\n\
    invariant MemberAll using v on Config::ConfigVersion at current { \
    forall(n in self.history: n < 500) }\n\
    invariant ConvertedAll using v on Config::ConfigVersion at current { \
    forall(n in convert<Set<V>[0, 3]>(self.history): n < 500) }\n\
    invariant MaybeAll using v on Config::ConfigVersion at current { \
    present(self.maybeHistory) implies \
    forall(n in value(self.maybeHistory): n < 500) }\n\
    pre MemberPre using v on Config::ConfigVersion::attemptUpdate { \
    forall(n in self.history: n < 500) }\n";

/// `Config` with `ConfigVersion.history`, a required ordered member of up to
/// three `VersionNumber`s, and `maybeHistory`, an optional one.
fn history_domain_document() -> Vec<u8> {
    let config_version = config_version_type();
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&config_version_domain_document()).expect("the document is JSON");
    let types = envelope["types"].as_array_mut().expect("types is an array");
    let object_type = types
        .iter_mut()
        .find(|record| record["identity"] == json!(config_version))
        .expect("ConfigVersion is declared");
    let fields = object_type["fields"]
        .as_array_mut()
        .expect("fields is an array");
    for (name, presence) in [("history", "required"), ("maybeHistory", "optional")] {
        let identity = format!("{config_version}/{name}");
        fields.push(json!({
            "identity": identity,
            "name": name,
            "typeRef": version_number_type(),
            "presence": presence,
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 0, "upper": 3, "ordered": true, "unique": false},
            "origin": {
                "generated": {
                    "generatorIdentity": identity,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [identity],
                }
            },
        }));
    }
    envelope.to_string().into_bytes()
}

fn packages() -> BTreeMap<[u8; 32], Vec<u8>> {
    let document = history_domain_document();
    qsl_semantics::model::intake::package_input(
        [document.as_slice()],
    )
}

fn model_digest_hex() -> String {
    let packages = packages();
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    hex(digest)
}

fn unit_text() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model Config = {CONFIG_VERSION_PACKAGE_IDENTITY:?} version \"1.0.0\" \
         digest \"sha256-jcs:{}\";\n{CLAUSES}",
        model_digest_hex()
    )
}

/// `self` (`mid`, version 1, parent absent) holding `history`, and holding
/// it in `maybeHistory` too.
fn snapshot(label: &DocumentRef, observation: &str, history: &[i64]) -> Vec<u8> {
    let items: Vec<_> = history
        .iter()
        .map(|version| json!({"integer": version.to_string()}))
        .collect();
    json!({
        "format": "quire.state.snapshot/v1",
        "identity": document_identity_json(label),
        "observation": observation,
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {
            "identity": CONFIG_VERSION_PACKAGE_IDENTITY, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", model_digest_hex()),
        },
        "populations": [{
            "population": config_version_population_identity(),
            "complete": true,
            "objects": [{
                "key": "mid", "type": config_version_type(),
                "fields": {
                    "versionNumber": {"integer": "1"},
                    "parent": {"absent": {}},
                    "history": {"sequence": items.clone()},
                    "maybeHistory": {"present": {"sequence": items}},
                },
            }],
        }],
    })
    .to_string()
    .into_bytes()
}

fn mid() -> SelectedObject {
    SelectedObject {
        population: config_version_population_identity(),
        key: "mid".to_owned(),
    }
}

fn request(selection: ClauseSelection) -> ClauseRunRequest {
    config_version_request_for(
        unit_text(),
        packages(),
        "witness-member.native",
        ClauseRunSelection::Clause(selection),
    )
}

/// `clause`, an invariant, over a current snapshot holding `history`.
fn run_current(clause: &str, history: &[i64]) -> super::super::ClauseRunReport {
    let (label, bytes) = document_ref_and_bytes("member-current", |label| {
        snapshot(label, "current", history)
    });
    let mut request = request(config_version_current_selection(
        clause,
        label.clone(),
        "mid",
    ));
    request.snapshots.insert(label.digest, bytes);
    run_clause(request).expect("a well-formed request always reports")
}

/// `MemberPre` over a pre-call observation whose pre snapshot holds
/// `history`.
fn run_pre_call(history: &[i64]) -> super::super::ClauseRunReport {
    let (label, bytes) =
        document_ref_and_bytes("member-pre", |label| snapshot(label, "pre", history));
    let mut request = request(ClauseSelection {
        name: "MemberPre".to_owned(),
        input: ClauseSelectionInput::PreCall {
            snapshot: label.clone(),
            self_object: mid(),
            parameters: BTreeMap::new(),
        },
    });
    request.snapshots.insert(label.digest, bytes);
    run_clause(request).expect("a well-formed request always reports")
}

/// `MemberPre` over an `attemptUpdate` invocation whose pre and post
/// snapshots, two documents, each hold `history` (`attemptUpdate`'s frame
/// leaves `history` unchanged).
fn run_invocation(history: &[i64]) -> super::super::ClauseRunReport {
    let digest_hex = model_digest_hex();
    let (pre_label, pre_bytes) =
        document_ref_and_bytes("member-pre", |label| snapshot(label, "pre", history));
    let (post_label, post_bytes) =
        document_ref_and_bytes("member-post", |label| snapshot(label, "post", history));
    let (invocation_label, invocation_bytes) = document_ref_and_bytes("member-call", |label| {
        config_version_invocation_bytes(label, &digest_hex, &pre_label, &post_label, "mid")
    });
    let mut request = request(config_version_invocation_selection(
        "MemberPre",
        invocation_label.clone(),
    ));
    request.snapshots.insert(pre_label.digest, pre_bytes);
    request.snapshots.insert(post_label.digest, post_bytes);
    request
        .invocations
        .insert(invocation_label.digest, invocation_bytes);
    run_clause(request).expect("a well-formed request always reports")
}

/// The value path of `mid.history` in the snapshot labelled `identity`.
fn history_path(report: &super::super::ClauseRunReport, identity: &str) -> RuntimeValuePath {
    let witness = report.witness.as_ref().expect("the claim is decided");
    let ValuePathSubject::Object(object) = &witness.value_path.subject else {
        panic!("a stored member's path names its object: {witness:?}");
    };
    assert_eq!(object.object().as_str(), "mid");
    RuntimeValuePath {
        observation: ObservationIdentity {
            authority: "test".to_owned(),
            identity: identity.to_owned(),
            revision_namespace: "ns".to_owned(),
            revision: "1".to_owned(),
        },
        subject: witness.value_path.subject.clone(),
        steps: vec![ValuePathStep::Member("history".to_owned())],
    }
}

/// TC-740 step 7 (FR-265-AC-7): a `forall` over `self.history` stops at
/// 600, index 1, and its record names the stored member `mid.history` in
/// the current observation, with no index step.
#[trace("TC-740", "FR-265-AC-7")]
#[test]
fn tc_740_a_member_domain_records_the_stored_member() {
    let report = run_current("MemberAll", &[0, 600, 700]);
    let witness = report.witness.as_ref().expect("the forall is refuted");
    assert_eq!(witness.index, Some(1));
    assert_eq!(
        quire_exact::compare_keys(
            &witness.deciding_element,
            &Value::Integer(quire_exact::Integer::from(600_i64))
        ),
        Some(std::cmp::Ordering::Equal)
    );
    assert_eq!(witness.value_path, history_path(&report, "member-current"));
}

/// TC-740 step 7 (FR-265-AC-7): a set conversion of the stored member keeps
/// each element's stored position. `[700, 0, 600]` converts to the set
/// `{0, 600, 700}`; the `forall` stops at 600, stored at index 2 of
/// `mid.history`.
#[trace("TC-740", "FR-265-AC-7")]
#[test]
fn tc_740_a_converted_member_keeps_the_stored_position() {
    let report = run_current("ConvertedAll", &[700, 0, 600]);
    let witness = report.witness.as_ref().expect("the forall is refuted");
    assert_eq!(witness.index, Some(2));
    assert_eq!(witness.value_path, history_path(&report, "member-current"));
}

/// TC-740 step 7 (FR-265-AC-7): a precondition reads the pre snapshot, so a
/// pre-call run and a run over an invocation give the same record, naming
/// `mid.history` in the pre observation, not the invocation's post one.
#[trace("TC-740", "FR-265-AC-7")]
#[test]
fn tc_740_a_precondition_names_the_pre_observation_in_both_forms() {
    let pre_call = run_pre_call(&[0, 600, 700]);
    let invocation = run_invocation(&[0, 600, 700]);
    let witness = pre_call.witness.as_ref().expect("the forall is refuted");
    assert_eq!(witness.index, Some(1));
    assert_eq!(witness.value_path, history_path(&pre_call, "member-pre"));
    assert_eq!(invocation.witness, pre_call.witness);
}

/// TC-740 step 7 (FR-265-AC-7): a `forall` over `value(self.maybeHistory)`,
/// a stored collection reached through a present optional member, names
/// the member and then the option's payload (QSpec FR-207's option-value
/// step), not a collection built in the claim.
#[trace("TC-740", "FR-265-AC-7")]
#[test]
fn tc_740_an_optional_member_domain_records_the_option_value_step() {
    let report = run_current("MaybeAll", &[0, 600, 700]);
    let witness = report.witness.as_ref().expect("the forall is refuted");
    assert_eq!(witness.index, Some(1));
    let mut expected = history_path(&report, "member-current");
    expected.steps = vec![
        ValuePathStep::Member("maybeHistory".to_owned()),
        ValuePathStep::OptionValue,
    ];
    assert_eq!(witness.value_path, expected);
}
