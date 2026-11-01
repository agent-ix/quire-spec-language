// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-359-AC-3, FR-359-AC-8 (TC-998 step 5, TC-1001 step 1): the emitted
//! package of a unit passing a child record at an ancestor-typed parameter
//! carries each record as a model-owned `record_value_type` node, reads a
//! field by its identity, and adds no conversion node between the call and
//! its argument. The read-back halves depend on IR's reader accepting a
//! conforming record operand (quire-contract-ir FR-347).

use super::owners::node_where;
use super::*;

#[path = "../../../../qsl-semantics/tests/support/child_composite.rs"]
pub(super) mod fixture;

/// `unit_body` as model `M` over the FR-359 package, through the spine to the
/// v2 emitter with nothing omitted, and a read of the bytes with the package
/// document as evidence.
fn emit_unit(unit_body: &str) -> (Emission, Value, Read) {
    let document = serde_json::to_vec(&fixture::document()).expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?};\n\
         {unit_body}\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(&document), document.as_slice())
        .expect("a fresh evidence key");
    let read = read_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
    );
    (emission, wire, read)
}

const UNIT: &str = "\
function occurredAt using v(e: M::OrderPlaced): Integer pure { e.occurred_at }\n\
function fromExpress using v(e: M::ExpressOrderPlaced): Integer pure { occurredAt(e) }\n\
function fromRush using v(e: M::RushOrderPlaced): Integer pure { occurredAt(e) }\n";

/// FR-359-AC-3: each record value type is one `model`/`record_value_type`
/// node with a `ModelOwner` naming the declaring package and the record's
/// node, none is a `composite_type`, no node has a `supertypes` member, and
/// the call to `occurredAt` takes its argument by `ref` with no conversion
/// between them.
#[trace("FR-359-AC-3", "TC-998")]
#[test]
fn records_are_model_owned_nodes_and_the_call_takes_its_argument_by_ref() {
    let (_, wire, _) = emit_unit(UNIT);
    for name in ["OrderPlaced", "ExpressOrderPlaced", "RushOrderPlaced"] {
        let owned: Vec<&Value> = node_where(&wire, "model", "record_value_type")
            .into_iter()
            .filter(|node| node["owner"]["node"] == fixture::id(name))
            .collect();
        assert_eq!(owned.len(), 1, "{name} is one record_value_type node");
        assert_eq!(
            owned[0]["owner"],
            json!({"kind": "model", "identity": fixture::PACKAGE, "node": fixture::id(name)})
        );
    }
    assert_eq!(
        node_where(&wire, "composite_type", "record").len(),
        0,
        "no record of the package is a composite_type"
    );
    let text = wire.to_string();
    assert!(
        !text.contains("supertypes"),
        "no node has a supertypes member"
    );
    let calls: Vec<&Value> = nodes(&wire)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == "quire.op.function.call")
        .collect();
    assert_eq!(calls.len(), 2, "fromExpress and fromRush each call once");
    for call in calls {
        let arguments = call["body"]["arguments"].as_array().unwrap();
        assert_eq!(arguments.len(), 2, "the callee and one argument");
        assert_eq!(
            arguments[1]["term"], "reference",
            "the argument is taken by ref, with no node between"
        );
    }
    let projects = nodes(&wire)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == "quire.op.record.project")
        .count();
    assert_eq!(projects, 1, "occurredAt reads its field once");
}

/// Model-intake identity rows join the admitted record and its inherited
/// fields; an omitted field witness refuses the complete record projection.
#[trace("FR-390-AC-17")]
#[test]
fn selected_record_and_inherited_field_rows_join_ir_projection() {
    let document = serde_json::to_vec(&fixture::document()).expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?};\n\
         {UNIT}\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the unit checks");
    let declaration_rows = graph.selected_exact_declaration_rows().to_vec();
    let checked = CheckedPackage::link(graph);
    let emission = emit_checked(&checked).expect("the record package emits");
    assert_eq!(emission.omitted, []);

    let mut evidence = super::read_evidence(&emission);
    evidence
        .insert_domain_package_document(fixture::PACKAGE, document.as_slice())
        .expect("the selected document is retained as evidence");
    let admitted = match quire_contract_model::read_checked_package(
        emission.package.bytes(),
        quire_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        quire_contract_model::CheckedPackageDispatchResult::AdmittedV2(admitted) => admitted,
        other => panic!("the emitted record package admits: {other:?}"),
    };

    let record_identity = fixture::id("ExpressOrderPlaced");
    let record_row = declaration_rows
        .iter()
        .find(|row| row.original_identity.as_ref() == record_identity)
        .expect("intake minted the selected child-record row");
    let record_node = record_row.structural_node.clone();
    let witnesses = checked
        .graph()
        .selected_exact_identities(&admitted)
        .expect("record and inherited field rows join IR admission");
    let projection = admitted
        .project_exact_type(&record_node, &witnesses, 256)
        .expect("the selected record has a complete Exact view");
    let view = projection.record_view().expect("the record projection");
    let projected_fields: std::collections::BTreeSet<_> = view
        .fields()
        .iter()
        .map(|field| field.ir_identity().to_owned())
        .collect();
    let selected_rows: std::collections::BTreeSet<_> = witnesses
        .field_rows()
        .iter()
        .filter(|row| row.record_original_identity.as_ref() == record_identity)
        .map(|row| row.field_original_identity.to_string())
        .collect();
    assert_eq!(projected_fields, selected_rows);
    assert!(projected_fields.contains(&fixture::id("OrderPlaced/occurred_at")));
    assert!(projected_fields.contains(&fixture::id("OrderPlaced/order_id")));
    assert!(projected_fields.contains(&fixture::id("ExpressOrderPlaced/promised_by")));

    let missing_rows = witnesses
        .field_rows()
        .iter()
        .filter(|row| {
            !(row.record_original_identity.as_ref() == record_identity
                && row.field_original_identity.as_ref() == fixture::id("OrderPlaced/order_id"))
        })
        .cloned()
        .collect::<Vec<_>>();
    let declarations = witnesses.declaration_rows().cloned().collect::<Vec<_>>();
    let missing = quire_contract_model::SelectedExactIdentities::from_stage_rows(
        &admitted,
        declarations,
        missing_rows,
    )
    .expect("remaining producer rows still join");
    let refusal = admitted
        .project_exact_type(&record_node, &missing, 256)
        .expect_err("a missing inherited-field witness refuses the whole view");
    assert_eq!(
        refusal.code,
        quire_contract_model::ExactTypeProjectionErrorCode::UnavailableIdentityWitness
    );
}

/// FR-359-AC-3, FR-359-AC-8: the package reads back `Verified` through IR's
/// reader (quire-contract-ir FR-347, a conforming record at an operation
/// argument, and QSpec FR-322's `record_value_type` owner).
#[trace("FR-359-AC-3", "FR-359-AC-8", "TC-998", "TC-1001")]
#[test]
fn a_child_record_argument_reads_back_verified() {
    let (_, _, read) = emit_unit(UNIT);
    match read {
        Read::Verified { .. } => {}
        other => panic!("expected Verified, got {other:?}"),
    }
}

/// FR-359-AC-8 over the operation path: a unit with a precondition and a
/// postcondition over `Ledger.pay(m: Money): Money` and a function taking and
/// returning the same record emits with nothing omitted and reads back
/// `Verified`; the operation's parameter and result are the `Money` node, not
/// a conversion of it.
#[trace("FR-359-AC-8", "TC-1001")]
#[test]
fn an_operation_over_a_record_parameter_emits_and_reads_back() {
    let (_, wire, read) = emit_unit(
        "function keep using v(m: M::Money): M::Money pure { m }\n\
         function passTaxed using v(m: M::TaxedMoney): M::Money pure { keep(m) }\n\
         pre PayHolds using v on M::Ledger::pay { m.amount_minor > 0 }\n\
         post PayReturns using v on M::Ledger::pay { result.amount_minor = m.amount_minor }",
    );
    assert!(
        node_where(&wire, "model", "record_value_type")
            .iter()
            .any(|node| node["owner"]["node"] == fixture::id("Money")),
        "the operation's Money is the model-owned record node"
    );
    assert!(
        !wire.to_string().contains("convert"),
        "no conversion node between an argument and its parameter"
    );
    match read {
        Read::Verified { .. } => {}
        other => panic!("expected Verified, got {other:?}"),
    }
}
