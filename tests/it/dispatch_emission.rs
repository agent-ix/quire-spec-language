// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-494: producer-linked dispatch decisions survive checked-package
//! emission, and IR's typed dispatch reader refuses substituted witnesses.

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::{self, DependencyInput, PackageLimits, ParseRequest, SpineLimits};
use quire_contract_model::{
    CheckedOccurrence, CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2,
    CheckedPackageV2ReadResult,
};
use quire_exact::Cancel;
use serde_json::{json, Value};

const PACKAGE: &str = "example/config-version";
const EXAMPLE: &str = include_str!("../../examples/config-version/model.semantic-ir.json");

fn operation(owner: &str, name: &str, redefines: Option<&str>) -> Value {
    let identity = format!("ix://{PACKAGE}/{owner}/{name}");
    let mut result = json!({
        "identity": identity,
        "name": name,
        "params": [],
        "returns": {
            "typeRef": "ix://quire/native/Integer",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false
        },
        "pre": [], "post": [],
        "origin": {"source": {
            "sourceIdentity": format!("ix://{PACKAGE}/spec"),
            "path": "spec.qspec", "startLine": 1, "startColumn": 1
        }},
        "frame": {"modifies": [], "creates": [], "deletes": []}
    });
    if let Some(parent) = redefines {
        result["redefines"] = json!(format!("ix://{PACKAGE}/{parent}/{name}"));
    }
    result
}

fn model() -> Vec<u8> {
    let mut envelope: Value = serde_json::from_str(EXAMPLE).expect("example model parses");
    let types = envelope["types"].as_array_mut().expect("types");
    let root = types
        .iter_mut()
        .find(|node| node["identity"] == format!("ix://{PACKAGE}/ConfigVersion"))
        .expect("ConfigVersion");
    root["operations"] = json!([
        operation("ConfigVersion", "size", None),
        operation("ConfigVersion", "probe", None)
    ]);
    for (name, parent, ops) in [
        (
            "Sub",
            "ConfigVersion",
            vec![
                operation("Sub", "size", Some("ConfigVersion")),
                operation("Sub", "probe", Some("ConfigVersion")),
            ],
        ),
        ("Leaf", "Sub", vec![operation("Leaf", "size", Some("Sub"))]),
        (
            "Other",
            "",
            vec![
                operation("Other", "size", None),
                operation("Other", "probe", None),
            ],
        ),
    ] {
        let identity = format!("ix://{PACKAGE}/{name}");
        types.push(json!({
            "identity": identity, "displayName": identity,
            "kind": {"module": PACKAGE, "name": "object_type"},
            "roles": [], "origin": {"generated": {
                "generatorIdentity": identity, "generatorVersion": "1.0.0", "inputIdentities": [identity]
            }},
            "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "supertypes": if parent.is_empty() { json!([]) } else { json!([format!("ix://{PACKAGE}/{parent}")]) },
            "fields": [], "operations": ops
        }));
    }
    serde_json::to_vec(&envelope).expect("model JSON")
}

fn emit(declarations: &str) -> Vec<u8> {
    let document = model();
    let packages =
        qsl_semantics::model::intake::package_input([document.as_slice()]).expect("model intake");
    let source_text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model Config = {PACKAGE:?};\n\
         {declarations}\n"
    );
    let source = SourceIdentity::new("test", "dispatch", "file", "1");
    let cancel = Cancel::new();
    let limits = SpineLimits::default();
    let failure = |failure| spine::refusal_or_fault(failure).expect("no internal front-end fault");
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "dispatch.native",
            bytes: source_text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(failure)
    .expect("source parses")
    .into_value();
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(failure)
        .expect("model selected")
        .into_value();
    let checked = spine::check(
        &parsed,
        PACKAGE.to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(failure)
    .expect("source checks")
    .into_value();
    spine::package(&checked, PackageLimits::default(), &cancel)
        .map_err(failure)
        .expect("package emits")
        .into_value()
        .package()
        .bytes()
        .to_vec()
}

fn evidence(wire: &Value) -> CheckedPackageEvidence {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"]
        .as_array()
        .expect("features")
    {
        evidence.support_feature(feature.as_str().expect("feature name"));
    }
    let document = model();
    for (identity, document) in
        qsl_semantics::model::intake::package_input([document.as_slice()]).expect("model intake")
    {
        evidence
            .insert_domain_package_document(identity.as_str(), document)
            .expect("evidence");
    }
    evidence
}

fn admitted(bytes: &[u8]) -> CheckedPackageV2 {
    let wire: Value = serde_json::from_slice(bytes).expect("emitted JSON");
    match CheckedPackageV2::read(bytes, CheckedPackageReadLimits::bounded(), &evidence(&wire)) {
        CheckedPackageV2ReadResult::Admitted(package) => *package,
        other => panic!("typed IR reader rejected emitted dispatch data: {other:?}"),
    }
}

#[trace("TC-1282", "FR-494-AC-1", "FR-494-AC-3", "FR-494-AC-4")]
#[test]
fn producer_dispatch_table_reads_with_ordered_candidate_witnesses() {
    let bytes = emit(
        "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         body sizeC using v on Config::Leaf::size() { 3 }\n\
         pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }\n",
    );
    let package = admitted(&bytes);
    let wire: Value = serde_json::from_slice(&bytes).unwrap();
    let rows = wire["checked_term_data"]["dispatch_tables"]
        .as_array()
        .expect("dispatch carrier");
    let family_rows: Vec<_> = rows
        .iter()
        .filter(|row| row["operation"]["operation"] == "size")
        .collect();
    assert_eq!(
        family_rows.len(),
        1,
        "one exact row for the dispatched call"
    );
    for row in family_rows {
        let application: quire_contract_model::CheckedNodeId =
            serde_json::from_value(row["application"].clone()).unwrap();
        let occurrence: CheckedOccurrence =
            serde_json::from_value(row["occurrence"].clone()).unwrap();
        let view = package
            .dispatch_candidates(&application, &occurrence, 100)
            .expect("typed reader returns exact producer row");
        assert_eq!(view.table.application, application);
        let candidates = view.candidates();
        assert_eq!(candidates.len(), 3);
        assert!(candidates
            .iter()
            .all(|candidate| candidate.selected_operation == view.table.operation));
        assert!(candidates
            .iter()
            .all(|candidate| candidate.effective_precondition.is_none()));
        assert!(candidates
            .iter()
            .all(|candidate| wire["semantic_graph"]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|node| node["node_tag"] == "function"
                    && node["node_id"]
                        == serde_json::to_value(&candidate.operation_body).unwrap())));
        assert_eq!(
            candidates[0].selection_key.original.node.as_ref(),
            "ix://example/config-version/ConfigVersion"
        );
        assert_eq!(
            candidates[1].selection_key.original.node.as_ref(),
            "ix://example/config-version/Leaf"
        );
        assert_eq!(
            candidates[2].selection_key.original.node.as_ref(),
            "ix://example/config-version/Sub"
        );
    }
    let mut missing: Value = serde_json::from_slice(&bytes).unwrap();
    missing["checked_term_data"]["dispatch_tables"][0]["candidates"][0]
        .as_object_mut()
        .unwrap()
        .remove("selected_operation");
    let refusal = match CheckedPackageV2::read(
        &serde_json::to_vec(&missing).unwrap(),
        CheckedPackageReadLimits::bounded(),
        &evidence(&missing),
    ) {
        CheckedPackageV2ReadResult::Refused(refusal) => refusal,
        other => panic!("missing candidate witness must refuse: {other:?}"),
    };
    assert_eq!(
        refusal.loci[0]
            .path
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("/checked_term_data/dispatch_tables/0/candidates/0")
    );
    let graph_ids: Vec<_> = package
        .graph()
        .nodes
        .iter()
        .map(|node| node.node_id.clone())
        .collect();
    assert_eq!(
        graph_ids.len(),
        package.graph().nodes.len(),
        "graph node identities remain unique"
    );
    let mut without_carrier = wire.clone();
    without_carrier
        .as_object_mut()
        .unwrap()
        .remove("checked_term_data");
    let without_carrier_bytes = serde_json::to_vec(&without_carrier).unwrap();
    let without_carrier = match CheckedPackageV2::read(
        &without_carrier_bytes,
        CheckedPackageReadLimits::bounded(),
        &evidence(&without_carrier),
    ) {
        CheckedPackageV2ReadResult::Admitted(package) => *package,
        other => panic!("graph-only package remains admitted: {other:?}"),
    };
    assert_eq!(package.graph(), without_carrier.graph());

    // QSL copies a checked effective-precondition function identity into the
    // carrier. The frozen IR-1214 draft currently refuses this existing
    // `pure_function` form, so this producer assertion does not claim reader
    // admission for the precondition-bearing variant.
    let with_precondition = emit(
        "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         body sizeC using v on Config::Leaf::size() { 3 }\n\
         pre inherited using v on Config::ConfigVersion::size { true }\n\
         pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }\n",
    );
    let pre_wire: Value = serde_json::from_slice(&with_precondition).unwrap();
    let precondition = &pre_wire["checked_term_data"]["dispatch_tables"][0]["candidates"][0]
        ["effective_precondition"];
    assert!(!precondition.is_null());
    assert!(pre_wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| { node["node_tag"] == "function" && node["node_id"] == *precondition }));
}

#[trace("TC-1283", "FR-494-AC-2", "FR-494-AC-4")]
#[test]
fn dispatch_candidate_witness_substitution_refuses_at_candidate_locus() {
    let bytes = emit(
        "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body otherSize using v on Config::Other::size() { 9 }\n\
         pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }\n\
         pre otherCall using v on Config::Other::probe { self.size() > 0 }\n",
    );
    let package = admitted(&bytes);
    let mut wire: Value = serde_json::from_slice(&bytes).unwrap();
    let original_rows = wire["checked_term_data"]["dispatch_tables"]
        .as_array()
        .unwrap();
    assert_eq!(original_rows.len(), 2);
    assert_ne!(
        original_rows[0]["application"],
        original_rows[1]["application"]
    );
    // Occurrence ordinals are graph-local per node, so the two calls can both
    // be ordinal zero. Their exact application/occurrence pairs must resolve
    // to distinct producer rows below.
    assert_ne!(original_rows[0]["context"], original_rows[1]["context"]);
    assert_ne!(original_rows[0]["operation"], original_rows[1]["operation"]);
    for row in original_rows {
        let application: quire_contract_model::CheckedNodeId =
            serde_json::from_value(row["application"].clone()).unwrap();
        let occurrence: CheckedOccurrence =
            serde_json::from_value(row["occurrence"].clone()).unwrap();
        let view = package
            .dispatch_candidates(&application, &occurrence, 100)
            .expect("each exact call row reads independently");
        assert!(view
            .candidates()
            .iter()
            .all(|candidate| candidate.selected_operation == view.table.operation));
    }
    let rows = wire["checked_term_data"]["dispatch_tables"]
        .as_array_mut()
        .unwrap();
    let root_index = rows
        .iter()
        .position(|row| row["context"].to_string().contains("ConfigVersion"))
        .expect("root family row");
    let other_operation = rows
        .iter()
        .find(|row| row["context"].to_string().contains("Other"))
        .expect("other family row")["operation"]
        .clone();
    let candidate = &mut rows[root_index];
    let candidates = candidate["candidates"].as_array_mut().unwrap();
    candidates[0]["selected_operation"] = other_operation;
    let refusal = match CheckedPackageV2::read(
        &serde_json::to_vec(&wire).unwrap(),
        CheckedPackageReadLimits::bounded(),
        &evidence(&wire),
    ) {
        CheckedPackageV2ReadResult::Refused(refusal) => refusal,
        other => panic!("substituted candidate witness must refuse: {other:?}"),
    };
    assert_eq!(
        refusal.loci[0]
            .path
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("/checked_term_data/dispatch_tables/0/candidates/0/selected_operation")
    );
}
