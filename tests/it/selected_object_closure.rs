// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected-document object roots reach the ordinary identity handoff and IR.

use ix_trace_rs::trace;
use qsl_package::ReadIdentityRows;
use quire_contract_model::{
    CheckedNodeOwner, CheckedPackageV2, ExactTypeProjectionErrorCode, SelectedExactIdentities,
    SelectedExactIdentity, SelectedTypeDeclarations,
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
}

fn fixture(document: &Value) -> Fixture {
    let directory = tempfile::tempdir().expect("fixture directory");
    let source = directory.path().join("unit.native");
    let model = directory.path().join("model.json");
    std::fs::write(&source, tc1070::SOURCE).expect("genuine protocol source");
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
    };
    let output = directory.path().join("handoff");
    write(&request, &output).expect("the genuine handoff writes");
    let files = tree(&output);
    let package = admitted(
        &files["checked-package.json"],
        &evidence(&request).expect("evidence"),
    );
    let rows = ReadIdentityRows::read(&files["identities.json"], &package)
        .expect("the required identity carrier joins the admitted package");
    Fixture {
        _directory: directory,
        package,
        rows,
    }
}

fn project(fixture: &Fixture) -> SelectedTypeDeclarations {
    let identities = fixture
        .rows
        .selected(&fixture.package)
        .expect("native row join");
    fixture
        .package
        .project_selected_type_declarations(&identities, &[], &[], 100_000)
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
            .node(&rows[0].structural_node)
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
        other.fields()[1].value_type(),
        &ValueType::Reference(other.effective_id().clone())
    );
    assert_eq!(other.fields()[1].presence(), Presence::Optional);
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
    )
    .expect("the remaining genuine rows join individually");
    let refusal = fixture
        .package
        .project_selected_type_declarations(&missing, &[], &[], 100_000)
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
    )
    .expect_err("an original object identity cannot join another object's real node");
    assert_eq!(
        refusal.code,
        ExactTypeProjectionErrorCode::MismatchedIdentityWitness
    );
}
