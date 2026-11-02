// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-494: producer-linked dispatch decisions survive checked-package
//! emission, and IR's typed dispatch reader refuses substituted witnesses.

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::{
    self, CheckedUnit, DependencyInput, PackageLimits, ParseRequest, SpineLimits,
};
use quire_contract_model::{
    CheckedEffectiveId, CheckedOccurrence, CheckedPackageEvidence, CheckedPackageReadLimits,
    CheckedPackageV2, CheckedPackageV2ReadResult,
};
use quire_exact::{Cancel, EffectiveId, NodeKey};
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct CapturedCandidate {
    selection_key: EffectiveId,
    operation_body: NodeKey,
    effective_precondition: Option<NodeKey>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CapturedDecision {
    application: NodeKey,
    declaring: NodeKey,
    operation: String,
    context: NodeKey,
    candidates: Vec<CapturedCandidate>,
}

fn compile(declarations: &str) -> CheckedUnit {
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
    spine::check(
        &parsed,
        PACKAGE.to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(failure)
    .expect("source checks")
    .into_value()
}

fn capture_dispatch_decisions(checked: &CheckedUnit) -> Vec<CapturedDecision> {
    let graph = checked.package().graph();
    graph
        .dispatch_producer_decisions()
        .iter()
        .map(|((application, _), decision)| CapturedDecision {
            application: application.clone(),
            declaring: decision.declaring_node.clone(),
            operation: decision.operation.selected_operation.clone(),
            context: decision.context_node.clone(),
            candidates: decision
                .table
                .entries()
                .iter()
                .map(|(selection_key, candidate)| CapturedCandidate {
                    selection_key: selection_key.clone(),
                    operation_body: graph
                        .function_identity_at(candidate.body)
                        .expect("linked candidate body has a checked function identity"),
                    effective_precondition: candidate.precondition.map(|index| {
                        graph
                            .function_identity_at(index)
                            .expect("effective precondition has a checked function identity")
                    }),
                })
                .collect(),
        })
        .collect()
}

fn emit_checked(checked: &CheckedUnit) -> Vec<u8> {
    let failure = |failure| spine::refusal_or_fault(failure).expect("no internal package fault");
    spine::package(checked, PackageLimits::default(), &Cancel::new())
        .map_err(failure)
        .expect("package emits")
        .into_value()
        .package()
        .bytes()
        .to_vec()
}

fn emit_with_capture(declarations: &str) -> (Vec<u8>, Vec<CapturedDecision>) {
    let checked = compile(declarations);
    let captured = capture_dispatch_decisions(&checked);
    (emit_checked(&checked), captured)
}

fn checked_effective_id(identity: &EffectiveId) -> CheckedEffectiveId {
    let mut current = None;
    for original in identity.owner_chain() {
        current = Some(CheckedEffectiveId {
            owner_effective_type: current.map(Box::new),
            original: quire_contract_model::CheckedOriginalDeclarationId {
                package: original.package().into(),
                node: original.node().into(),
            },
        });
    }
    current.expect("effective declaration identity has an original declaration")
}

fn assert_captured_decisions(
    bytes: &[u8],
    package: &CheckedPackageV2,
    captured: &[CapturedDecision],
) {
    let wire: Value = serde_json::from_slice(bytes).unwrap();
    let wire_graph = qsl_semantics::check::wire_id::WireGraph::of_package_bytes(bytes)
        .expect("wire graph resolves structural identities");
    let rows = wire["checked_term_data"]["dispatch_tables"]
        .as_array()
        .expect("dispatch carrier");
    assert_eq!(rows.len(), captured.len());
    for decision in captured {
        let application = wire_graph
            .wire_id(&decision.application)
            .expect("captured application is emitted");
        let row = rows
            .iter()
            .find(|row| row["application"] == serde_json::to_value(&application).unwrap())
            .expect("row uses captured application identity");
        let occurrence: CheckedOccurrence =
            serde_json::from_value(row["occurrence"].clone()).unwrap();
        assert!(
            package.source_map().iter().any(|entry| {
                entry.node_id == application
                    && entry.role == occurrence.role
                    && entry.ordinal == occurrence.ordinal
            }),
            "row application and occurrence pair with the source map"
        );
        let view = package
            .dispatch_candidates(&application, &occurrence, 100)
            .expect("typed reader returns exact producer row");
        assert_eq!(view.table.application, application);
        assert_eq!(
            view.table.operation.declaring,
            wire_graph
                .wire_id(&decision.declaring)
                .expect("declaring node is emitted")
        );
        assert_eq!(view.table.operation.operation.as_ref(), decision.operation);
        assert_eq!(
            view.table.context,
            wire_graph
                .wire_id(&decision.context)
                .expect("receiver context is emitted")
        );
        assert_eq!(view.candidates().len(), decision.candidates.len());
        for (candidate, expected) in view.candidates().iter().zip(&decision.candidates) {
            assert_eq!(
                candidate.selection_key,
                checked_effective_id(&expected.selection_key)
            );
            assert_eq!(
                candidate.operation_body,
                wire_graph
                    .wire_id(&expected.operation_body)
                    .expect("linked operation body identity is emitted")
            );
            assert_eq!(
                candidate.effective_precondition,
                expected
                    .effective_precondition
                    .as_ref()
                    .map(|key| wire_graph
                        .wire_id(key)
                        .expect("precondition identity is emitted"))
            );
            assert_eq!(candidate.selected_operation, view.table.operation);
        }
    }
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
    let (bytes, captured) = emit_with_capture(
        "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         body sizeC using v on Config::Leaf::size() { 3 }\n\
         pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }\n",
    );
    let package = admitted(&bytes);
    assert_captured_decisions(&bytes, &package, &captured);
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

    let (with_precondition, precondition_decisions) = emit_with_capture(
        "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         body sizeC using v on Config::Leaf::size() { 3 }\n\
         pre inherited using v on Config::ConfigVersion::size { true }\n\
         pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }\n",
    );
    let precondition_package = admitted(&with_precondition);
    assert_captured_decisions(
        &with_precondition,
        &precondition_package,
        &precondition_decisions,
    );
    assert!(precondition_decisions
        .iter()
        .flat_map(|decision| &decision.candidates)
        .any(|candidate| candidate.effective_precondition.is_some()));
    let pre_wire: Value = serde_json::from_slice(&with_precondition).unwrap();
    let pre_row = &pre_wire["checked_term_data"]["dispatch_tables"][0];
    let pre_application: quire_contract_model::CheckedNodeId =
        serde_json::from_value(pre_row["application"].clone()).unwrap();
    let pre_occurrence: CheckedOccurrence =
        serde_json::from_value(pre_row["occurrence"].clone()).unwrap();
    let pre_view = precondition_package
        .dispatch_candidates(&pre_application, &pre_occurrence, 100)
        .expect("typed precondition-bearing dispatch row is admitted");
    assert_eq!(
        pre_view.candidates().len(),
        precondition_decisions[0].candidates.len()
    );
    assert!(pre_view
        .candidates()
        .iter()
        .zip(&precondition_decisions[0].candidates)
        .any(|(candidate, expected)| {
            candidate.effective_precondition.is_some() && expected.effective_precondition.is_some()
        }));
}

#[trace("TC-1283", "FR-494-AC-2", "FR-494-AC-4")]
#[test]
fn dispatch_candidate_witness_substitution_refuses_at_candidate_locus() {
    let declarations = "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body otherSize using v on Config::Other::size() { 9 }\n\
         pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }\n\
         pre otherCall using v on Config::Other::probe { self.size() > 0 }\n";
    let (bytes, captured) = emit_with_capture(declarations);
    let package = admitted(&bytes);
    assert_captured_decisions(&bytes, &package, &captured);
    let wire: Value = serde_json::from_slice(&bytes).unwrap();
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

    let permuted_declarations =
        "pre otherCall using v on Config::Other::probe { self.size() > 0 }\n\
         body otherSize using v on Config::Other::size() { 9 }\n\
         pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }\n\
         body sizeA using v on Config::ConfigVersion::size() { 1 }\n";
    let (permuted_bytes, permuted_capture) = emit_with_capture(permuted_declarations);
    let permuted_package = admitted(&permuted_bytes);
    assert_captured_decisions(&permuted_bytes, &permuted_package, &permuted_capture);
    assert_eq!(
        captured, permuted_capture,
        "checked producer identities are stable"
    );
    let original_graph = qsl_semantics::check::wire_id::WireGraph::of_package_bytes(&bytes)
        .expect("original wire graph");
    let permuted_graph =
        qsl_semantics::check::wire_id::WireGraph::of_package_bytes(&permuted_bytes)
            .expect("permuted wire graph");
    let original_rows = serde_json::from_slice::<Value>(&bytes).unwrap()["checked_term_data"]
        ["dispatch_tables"]
        .clone();
    let permuted_rows = serde_json::from_slice::<Value>(&permuted_bytes).unwrap()
        ["checked_term_data"]["dispatch_tables"]
        .clone();
    assert_eq!(
        original_graph.structural_value(&original_rows).unwrap(),
        permuted_graph.structural_value(&permuted_rows).unwrap(),
        "canonical row order and all checked identities survive declaration permutation"
    );
    let mut row_keys: Vec<_> = serde_json::from_value::<Vec<Value>>(original_rows.clone())
        .unwrap()
        .iter()
        .map(|row| {
            let id: quire_contract_model::CheckedNodeId =
                serde_json::from_value(row["application"].clone()).unwrap();
            original_graph.key(&id).unwrap()
        })
        .collect();
    let canonical_keys = row_keys.clone();
    row_keys.sort();
    assert_eq!(
        row_keys, canonical_keys,
        "rows use canonical application order"
    );

    let mut wire: Value = serde_json::from_slice(&bytes).unwrap();
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
