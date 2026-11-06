// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected-document object roots reach the ordinary identity handoff and IR.

use ix_trace_rs::trace;
use qsl_package::ReadIdentityRows;
use quire_contract_model::{
    CheckedNodeOwner, CheckedOccurrenceRole, CheckedOperationEffect, CheckedOperationEffectOrigin,
    CheckedPackageEvidence, CheckedPackageV2, ExactTypeProjectionErrorCode,
    SelectedExactIdentities, SelectedExactIdentity, SelectedTypeDeclarations,
};
use quire_exact::{Presence, ValueType};
use quire_spec_language::checked_handoff::{evidence, write, CheckedHandoffRequest, Corpus};
use quire_spec_language::emit_checked_v2::Request;
use serde_json::Value;

use super::bare_equality::tc1070;
use super::checked_handoff::{admitted, tree};

struct Fixture {
    _directory: tempfile::TempDir,
    package: Box<CheckedPackageV2>,
    rows: ReadIdentityRows,
    bytes: Vec<u8>,
    evidence: CheckedPackageEvidence,
}

fn fixture(document: &Value) -> Fixture {
    fixture_with_source(document, tc1070::SOURCE)
}

fn fixture_with_source(document: &Value, source_text: &str) -> Fixture {
    let directory = tempfile::tempdir().expect("fixture directory");
    let source = directory.path().join("unit.native");
    let model = directory.path().join("model.json");
    std::fs::write(&source, source_text).expect("genuine protocol source");
    std::fs::write(&model, serde_json::to_vec(document).expect("model bytes"))
        .expect("genuine model document");
    let request = CheckedHandoffRequest {
        emit: Request {
            source,
            package_name: "selected-object-closure".into(),
            models: vec![model],
            libraries: Vec::new(),
            execution_points: None,
            temporal_selections: Vec::new(),
            out: None,
        },
        corpus: Corpus::None,
        native_join: qsl_package::V2ReadLimits::fixture().native_join_limits(),
    };
    let output = directory.path().join("handoff");
    write(&request, &output).expect("the genuine handoff writes");
    let files = tree(&output);
    let evidence = evidence(&request).expect("evidence");
    let bytes = files["checked-package.json"].clone();
    let package = admitted(&bytes, &evidence);
    let rows = ReadIdentityRows::read(
        &files["identities.json"],
        &package,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("the required identity carrier joins the admitted package");
    Fixture {
        _directory: directory,
        package,
        rows,
        bytes,
        evidence,
    }
}

fn project(fixture: &Fixture) -> SelectedTypeDeclarations {
    let identities = fixture
        .rows
        .selected(
            &fixture.package,
            &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect("native row join");
    fixture
        .package
        .project_selected_type_declarations(
            &identities,
            &[],
            &[],
            100_000,
            &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect("every selected document object has its genuine declaration witness")
}

#[trace("TC-417", "FR-094-AC-14")]
#[test]
fn selected_object_closure_projects_standard_e_and_unused_ep_objects() {
    let standard = fixture(&tc1070::document());
    let extended = fixture(&tc1070::document_ep());
    let base = project(&standard);
    let ep = project(&extended);
    assert_eq!(base.objects().len(), 1);
    assert_eq!(ep.objects().len(), 3);
    assert_eq!(base.records().len(), 4);
    assert_eq!(ep.records().len(), 5);
    let node = |view: &SelectedTypeDeclarations| {
        view.objects()
            .iter()
            .find(|object| object.original_identity() == tc1070::model::id("Node"))
            .expect("Node is selected")
            .clone()
    };
    assert_eq!(
        node(&base),
        node(&ep),
        "the original standard E object is unchanged"
    );
    assert_eq!(node(&ep).operations()[0].name(), "step");
    assert!(node(&ep).operations()[0].result().is_some());
    let original_node = node(&ep);
    let step = &original_node.operations()[0];
    assert_eq!(step.identity(), tc1070::model::id("Node/step"));
    assert!(step.parameters().is_empty());
    assert_eq!(
        step.effect(),
        &CheckedOperationEffect {
            modifies: Vec::new(),
            creates: Vec::new(),
            deletes: Vec::new(),
            deletes_scoped: Vec::new(),
        }
    );
    assert_eq!(
        step.provenance().origin,
        CheckedOperationEffectOrigin::ModelDocumentOperationFrame
    );
    assert_eq!(
        step.provenance().member.as_ref(),
        tc1070::model::id("Node/step")
    );
    for name in ["Node", "Other", "Elsewhere"] {
        let original = tc1070::model::id(name);
        let object = ep
            .objects()
            .iter()
            .find(|object| object.original_identity() == original)
            .expect("every selected ordinary object projects");
        let rows: Vec<_> = extended
            .rows
            .declarations()
            .iter()
            .filter(|row| row.original_identity.as_ref() == original)
            .collect();
        assert_eq!(rows.len(), 1, "one final identity row for {name}");
        assert_eq!(
            rows[0].exact_identity,
            SelectedExactIdentity::Object(object.effective_id().clone())
        );
        let graph_node = extended
            .package
            .graph()
            .nodes
            .iter()
            .find(|node| node.node_id == rows[0].structural_node)
            .expect("real node");
        assert_eq!(
            graph_node.owner,
            Some(CheckedNodeOwner::Model {
                identity: tc1070::model::PACKAGE.into(),
                node: original.clone().into(),
            })
        );
        assert_eq!(graph_node.semantic_form.as_ref(), "object_type");
        assert!(graph_node.declaration.is_none());
        let model_start = tc1070::SOURCE.find("model M =").expect("model declaration") as u64;
        let model_end = model_start
            + tc1070::SOURCE[model_start as usize..]
                .find(';')
                .expect("model declaration end") as u64
            + 1;
        let generated_at_alias: Vec<_> = extended
            .package
            .source_map()
            .iter()
            .filter(|entry| {
                entry.node_id == graph_node.node_id
                    && entry.role == CheckedOccurrenceRole::Generated
                    && entry.regions.iter().any(|region| {
                        region.start >= model_start
                            && region.start < region.end
                            && region.end <= model_end
                    })
            })
            .collect();
        assert_eq!(
            generated_at_alias.len(),
            1,
            "{name} is located at its selecting alias"
        );
        assert_eq!(object.fields(), object.declared_fields());
        assert!(object.supertypes().is_empty());
    }
    let other = ep
        .objects()
        .iter()
        .find(|object| object.original_identity() == tc1070::model::id("Other"))
        .unwrap();
    assert_eq!(
        other
            .fields()
            .iter()
            .map(|field| field.name())
            .collect::<Vec<_>>(),
        ["marker", "next"]
    );
    assert_eq!(other.fields()[0].value_type(), &ValueType::Boolean);
    assert_eq!(
        other.fields()[0].original_identity(),
        tc1070::model::id("Other/marker")
    );
    assert_eq!(
        other.fields()[1].original_identity(),
        tc1070::model::id("Other/next")
    );
    assert_eq!(
        other.fields()[1].value_type(),
        &ValueType::Reference(other.effective_id().clone())
    );
    assert_eq!(other.fields()[1].presence(), Presence::Optional);
}

#[trace("TC-417", "FR-094-AC-14")]
#[test]
fn selected_object_closure_roots_a_package_selected_by_two_aliases_once() {
    let source = tc1070::SOURCE.replace(
        "model M = \"test/orders\";",
        "model M = \"test/orders\";\nmodel Again = \"test/orders\";",
    );
    let fixture = fixture_with_source(&tc1070::document_ep(), &source);
    let projection = project(&fixture);
    assert_eq!(projection.objects().len(), 3);
    let first_start = u64::try_from(source.find("model M =").unwrap()).unwrap();
    let second_start = u64::try_from(source.find("model Again =").unwrap()).unwrap();
    let second_end = second_start + u64::try_from("model Again = \"test/orders\";".len()).unwrap();
    for object in projection.objects() {
        assert_eq!(
            fixture
                .rows
                .declarations()
                .iter()
                .filter(|row| { row.original_identity.as_ref() == object.original_identity() })
                .count(),
            1,
            "duplicate aliases retain one original object declaration"
        );
        let row = fixture
            .rows
            .declarations()
            .iter()
            .find(|row| row.original_identity.as_ref() == object.original_identity())
            .unwrap();
        let generated: Vec<_> = fixture
            .package
            .source_map()
            .iter()
            .filter(|entry| {
                entry.node_id == row.structural_node
                    && entry.role == CheckedOccurrenceRole::Generated
            })
            .flat_map(|entry| entry.regions.iter())
            .collect();
        assert_eq!(
            generated
                .iter()
                .filter(|region| { region.start >= first_start && region.end < second_start })
                .count(),
            1,
            "the first selecting alias owns the root occurrence"
        );
        assert!(
            generated
                .iter()
                .all(|region| { region.start < second_start || region.end > second_end }),
            "the duplicate alias adds no root occurrence"
        );
    }
}

#[trace("TC-417", "FR-094-AC-14")]
#[test]
fn selected_object_closure_refuses_missing_and_foreign_object_witnesses() {
    let fixture = fixture(&tc1070::document_ep());
    let declarations = fixture.rows.declarations();
    let other = declarations
        .iter()
        .find(|row| row.original_identity.as_ref() == tc1070::model::id("Other"))
        .expect("Other has an authentic final row");
    let missing = SelectedExactIdentities::from_stage_rows(
        &fixture.package,
        declarations
            .iter()
            .filter(|row| row.structural_node != other.structural_node)
            .cloned(),
        fixture.rows.fields().iter().cloned(),
        &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
    )
    .expect("the remaining genuine rows join individually");
    let refusal = fixture
        .package
        .project_selected_type_declarations(
            &missing,
            &[],
            &[],
            100_000,
            &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect_err("a model reference cannot substitute for its missing object witness");
    assert_eq!(
        refusal.code,
        ExactTypeProjectionErrorCode::UnavailableIdentityWitness
    );

    let mut foreign = declarations.to_vec();
    let elsewhere_node = declarations
        .iter()
        .find(|row| row.original_identity.as_ref() == tc1070::model::id("Elsewhere"))
        .expect("Elsewhere has an authentic final node")
        .structural_node
        .clone();
    foreign
        .iter_mut()
        .find(|row| row.structural_node == other.structural_node)
        .unwrap()
        .structural_node = elsewhere_node;
    let refusal = SelectedExactIdentities::from_stage_rows(
        &fixture.package,
        foreign,
        fixture.rows.fields().iter().cloned(),
        &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
    )
    .expect_err("an original object identity cannot join another object's real node");
    assert_eq!(
        refusal.code,
        ExactTypeProjectionErrorCode::MismatchedIdentityWitness
    );
}

#[trace("TC-417", "FR-094-AC-14")]
#[test]
fn selected_object_closure_refuses_a_model_reference_whose_object_node_is_absent() {
    let fixture = fixture(&tc1070::document_ep());
    let other = fixture
        .rows
        .declarations()
        .iter()
        .find(|row| row.original_identity.as_ref() == tc1070::model::id("Other"))
        .expect("Other has its real emitted declaration");
    let other_json = serde_json::to_value(&other.structural_node).expect("node reference");
    let mut package: Value = serde_json::from_slice(&fixture.bytes).expect("original package");
    let nodes = package["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("nodes");
    let before = nodes.len();
    nodes.retain(|node| node["node_id"] != other_json);
    assert_eq!(nodes.len() + 1, before, "remove only the actual Other node");
    package["source_map"]
        .as_array_mut()
        .expect("source map")
        .retain(|entry| entry["node_id"] != other_json);
    let package = admitted(
        &serde_json::to_vec(&package).expect("changed package"),
        &fixture.evidence,
    );
    let identities = SelectedExactIdentities::from_stage_rows(
        &package,
        fixture
            .rows
            .declarations()
            .iter()
            .filter(|row| row.structural_node != other.structural_node)
            .cloned(),
        fixture.rows.fields().iter().cloned(),
        &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
    )
    .expect("only the remaining genuine declaration rows are supplied");
    let refusal = package
        .project_selected_type_declarations(
            &identities,
            &[],
            &[],
            100_000,
            &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect_err("retained model references cannot invent their absent object declaration");
    assert_eq!(
        refusal.code,
        ExactTypeProjectionErrorCode::UnresolvedSelectedDeclaration
    );
    assert_eq!(refusal.form.as_ref(), "selected_document_type");
}
