// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251-AC-42 (TC-1081 step 26): the plan-root writer carries every witness
//! row of the emission it writes onto, with each node id re-keyed through the
//! writer's renames. Each test builds one emission holding rows of every
//! kind, re-keys it through a rename of every node id the rows name, and
//! reads each id field of each row back.

use quire_contract_model::{
    CheckedNodeId, ClockConfiguration, SelectedTupleDeclarationRow, TemporalBinding,
};

use super::converted_equality::emitted as emitted_quantities;
use super::state_transition::{operation_member_rows, operation_unit};
use super::variant_types::{emitted_enums, UNIT};
use super::*;
use crate::emit::monitor_plan::carry_rows;
use crate::emit::{EqualityFactWitness, OmissionCause, OmittedNode};

/// The id the writer renames `id` to in these tests: a different package.
fn renamed(id: &CheckedNodeId) -> CheckedNodeId {
    CheckedNodeId {
        package: format!("renamed:{}", id.package).into(),
        path: id.path.clone(),
        ordinal: id.ordinal,
    }
}

/// One emission whose rows of every kind are non-empty: the quantity
/// equality fixture's producer and fact rows (unit, dimension, conversion),
/// the enum fixture's enum rows and enum-member facts, and synthetic rows of
/// the kinds those fixtures do not produce.
fn composite() -> Emission {
    let mut base = emitted_quantities(
        "dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;\n\
         function same using v(a: Quantity<ms>, b: Quantity<ms>): Boolean pure { a = b and convert<Quantity<ms>>(a) = b }",
    );
    let compound = emitted_quantities(
        "dimension Length; dimension Area = Length;\n\
         unit m : Length = rational(1, 1); unit cm : Length = rational(1, 100) * m;\n\
         unit mm : Length = rational(1, 10) * cm;\n\
         unit sqm : Area = rational(1, 1);\n\
         function quantity using v(a: Quantity<mm>, b: Quantity<m>): Boolean pure { convert<Quantity<m>>(a) = b }\n\
         function area using v(a: Quantity<sqm>, b: Quantity<sqm>): Boolean pure { a = b }\n\
         function compound using v(a: Quantity<m>, b: Quantity<m>): Boolean pure { a * b = a * b }",
    );
    base.equality_producer_identity_rows
        .extend(compound.equality_producer_identity_rows.iter().cloned());
    base.equality_fact_witness_rows
        .extend(compound.equality_fact_witness_rows.iter().cloned());
    base.wire_nodes.extend(compound.wire_nodes.iter().cloned());
    let enums = emitted_enums(UNIT);
    base.selected_model_enum_declaration_rows = enums.selected_model_enum_declaration_rows.clone();
    base.selected_model_enum_member_rows = enums.selected_model_enum_member_rows.clone();
    base.equality_producer_identity_rows
        .extend(enums.equality_producer_identity_rows.iter().cloned());
    base.equality_fact_witness_rows
        .extend(enums.equality_fact_witness_rows.iter().cloned());
    base.wire_nodes.extend(enums.wire_nodes.iter().cloned());
    let id = |name: &str| CheckedNodeId {
        package: "p".into(),
        path: vec![quire_contract_model::CheckedNodePathSegment::Name(
            name.into(),
        )],
        ordinal: 0,
    };
    base.selected_operation_member_rows = operation_member_rows(&operation_unit());
    assert!(!base.selected_operation_member_rows.is_empty());
    base.selected_tuple_declaration_rows = vec![SelectedTupleDeclarationRow::new(
        id("tuple"),
        "doc",
        "Pair",
        base.equality_producer_identity_rows[0]
            .application()
            .clone(),
    )];
    base.selected_exact_declaration_rows =
        vec![quire_contract_model::SelectedExactDeclarationRow::new(
            id("declaration"),
            "doc",
            quire_contract_model::SelectedExactDeclarationForm::RecordValueType,
            "R",
            quire_contract_model::SelectedExactIdentity::Record(
                base.equality_producer_identity_rows[0]
                    .application()
                    .clone(),
            ),
        )];
    base.selected_exact_field_rows = vec![quire_contract_model::SelectedExactFieldRow::new(
        id("record"),
        "doc",
        "R",
        "f",
        quire_exact::MemberId::new(
            quire_exact::MemberOwner::Node(
                base.equality_producer_identity_rows[0]
                    .application()
                    .clone(),
            ),
            "f",
        )
        .expect("a member id"),
    )];
    base.selected_source_unit_rows = vec![quire_contract_model::SelectedSourceUnitRow::new(
        id("unit"),
        "authority",
        "identity",
        vec!["ms".into()],
        quire_exact::UnitId::declared(
            base.equality_producer_identity_rows[0]
                .application()
                .clone(),
        ),
    )];
    base.omitted = vec![
        OmittedNode {
            node: id("omitted"),
            cause: OmissionCause::NamesAbsentNode(id("absent")),
        },
        OmittedNode {
            node: id("omitted-2"),
            cause: OmissionCause::NamesOmittedNode(id("omitted")),
        },
    ];
    base.temporal_binding_rows = crate::emit::TemporalBindingRows {
        rows: vec![(
            base.equality_producer_identity_rows[0]
                .application()
                .clone(),
            TemporalBinding {
                declaration: id("clause"),
                occurrence: 0,
                clock_name: "model-time".into(),
                clock: ClockConfiguration::EventPosition {
                    sequence_authority: "authority".into(),
                },
            },
        )],
    };
    base
}

/// The renames of every node id any row of `emission` names.
fn renames_of(emission: &Emission) -> Vec<(CheckedNodeId, CheckedNodeId)> {
    let mut ids: Vec<CheckedNodeId> = emission
        .wire_nodes
        .iter()
        .map(|node| node.node_id.clone())
        .collect();
    let mut push = |id: &CheckedNodeId| ids.push(id.clone());
    for row in &emission.selected_operation_member_rows {
        push(&row.anchor);
    }
    for row in &emission.selected_tuple_declaration_rows {
        push(&row.structural_node);
    }
    for row in &emission.selected_source_unit_rows {
        push(&row.structural_node);
    }
    for row in &emission.selected_exact_declaration_rows {
        push(&row.structural_node);
    }
    for row in &emission.selected_exact_field_rows {
        push(&row.record_node);
    }
    for row in &emission.omitted {
        push(&row.node);
        if let OmissionCause::NamesAbsentNode(id) | OmissionCause::NamesOmittedNode(id) = &row.cause
        {
            push(id);
        }
    }
    for (_, row) in &emission.temporal_binding_rows.rows {
        push(&row.declaration);
    }
    ids.sort();
    ids.dedup();
    ids.iter().map(|id| (id.clone(), renamed(id))).collect()
}

fn fact_kind(fact: &EqualityFactWitness) -> &'static str {
    match fact {
        EqualityFactWitness::Unit { .. } => "unit",
        EqualityFactWitness::DimensionTerm { .. } => "dimension term",
        EqualityFactWitness::ConversionEdge { .. } => "conversion edge",
        EqualityFactWitness::EnumMember { .. } => "enum member",
        _ => "other",
    }
}

/// The node ids a fact names.
fn fact_ids(fact: &EqualityFactWitness) -> Vec<&CheckedNodeId> {
    match fact {
        EqualityFactWitness::OperandType { .. }
        | EqualityFactWitness::DimensionDeclaration { .. }
        | EqualityFactWitness::TextPlanType { .. } => Vec::new(),
        EqualityFactWitness::Unit { quantity_type, .. } => vec![quantity_type],
        EqualityFactWitness::DimensionTerm { base_node, .. } => vec![base_node],
        EqualityFactWitness::ConversionEdge { target_node, .. } => vec![target_node],
        EqualityFactWitness::EnumMember {
            source_declaration_node,
            declaration_node,
            member_node,
            ..
        } => vec![source_declaration_node, declaration_node, member_node],
    }
}

/// Every row kind is carried, and each node id each row holds is the renamed
/// id of the id the original row held.
#[trace("TC-1081", "FR-251-AC-42")]
#[test]
fn every_retained_row_kind_is_carried_with_its_node_ids_rekeyed() {
    let emission = composite();
    let changes = renames_of(&emission);
    let rows = carry_rows(&emission, &changes);

    assert!(!emission.selected_operation_member_rows.is_empty());
    assert_eq!(
        rows.selected_operation_member_rows.len(),
        emission.selected_operation_member_rows.len()
    );
    for (got, was) in rows
        .selected_operation_member_rows
        .iter()
        .zip(&emission.selected_operation_member_rows)
    {
        assert_eq!(got.anchor, renamed(&was.anchor), "operation-member anchor");
    }

    assert_eq!(rows.selected_tuple_declaration_rows.len(), 1);
    assert_eq!(
        rows.selected_tuple_declaration_rows[0].structural_node,
        renamed(&emission.selected_tuple_declaration_rows[0].structural_node),
        "tuple declaration"
    );

    assert_eq!(rows.selected_source_unit_rows.len(), 1);
    assert_eq!(
        rows.selected_source_unit_rows[0].structural_node,
        renamed(&emission.selected_source_unit_rows[0].structural_node),
        "source unit"
    );

    assert!(!emission.selected_model_enum_declaration_rows.is_empty());
    assert_eq!(
        rows.selected_model_enum_declaration_rows.len(),
        emission.selected_model_enum_declaration_rows.len()
    );
    for (got, was) in rows
        .selected_model_enum_declaration_rows
        .iter()
        .zip(&emission.selected_model_enum_declaration_rows)
    {
        assert_eq!(got.structural_node, renamed(&was.structural_node), "enum");
    }
    assert!(!emission.selected_model_enum_member_rows.is_empty());
    assert_eq!(
        rows.selected_model_enum_member_rows.len(),
        emission.selected_model_enum_member_rows.len()
    );
    for (got, was) in rows
        .selected_model_enum_member_rows
        .iter()
        .zip(&emission.selected_model_enum_member_rows)
    {
        assert_eq!(got.structural_node, renamed(&was.structural_node), "member");
        assert_eq!(
            got.declaration_node,
            renamed(&was.declaration_node),
            "member's declaration"
        );
    }

    assert_eq!(rows.selected_exact_declaration_rows.len(), 1);
    assert_eq!(
        rows.selected_exact_declaration_rows[0].structural_node,
        renamed(&emission.selected_exact_declaration_rows[0].structural_node),
        "exact declaration"
    );
    assert_eq!(rows.selected_exact_field_rows.len(), 1);
    assert_eq!(
        rows.selected_exact_field_rows[0].record_node,
        renamed(&emission.selected_exact_field_rows[0].record_node),
        "exact field's record"
    );

    assert!(!emission.equality_producer_identity_rows.is_empty());
    assert_eq!(
        rows.equality_producer_identity_rows.len(),
        emission.equality_producer_identity_rows.len()
    );
    for (got, was) in rows
        .equality_producer_identity_rows
        .iter()
        .zip(&emission.equality_producer_identity_rows)
    {
        assert_eq!(*got.checked_node(), renamed(was.checked_node()));
        assert_eq!(
            got.source_map_entry().node_id,
            renamed(&was.source_map_entry().node_id)
        );
    }

    assert!(!emission.equality_fact_witness_rows.is_empty());
    assert_eq!(
        rows.equality_fact_witness_rows.len(),
        emission.equality_fact_witness_rows.len()
    );
    let mut seen: Vec<&'static str> = Vec::new();
    for (got, was) in rows
        .equality_fact_witness_rows
        .iter()
        .zip(&emission.equality_fact_witness_rows)
    {
        assert_eq!(
            *got.checked_application(),
            renamed(was.checked_application())
        );
        assert_eq!(*got.structural_node(), renamed(was.structural_node()));
        assert_eq!(
            got.source_map_entry().node_id,
            renamed(&was.source_map_entry().node_id)
        );
        let (got_ids, was_ids) = (fact_ids(got.fact()), fact_ids(was.fact()));
        assert_eq!(got_ids.len(), was_ids.len());
        for (got_id, was_id) in got_ids.into_iter().zip(was_ids) {
            assert_eq!(*got_id, renamed(was_id), "{:?}", was.fact());
            let kind = fact_kind(was.fact());
            if !seen.contains(&kind) {
                seen.push(kind);
            }
        }
    }
    // The fixtures hold a node-naming fact of each variant that names one.
    for kind in ["unit", "dimension term", "conversion edge", "enum member"] {
        assert!(
            seen.contains(&kind),
            "no {kind} fact names a node: {seen:?}"
        );
    }

    assert_eq!(rows.omitted.len(), 2);
    assert_eq!(rows.omitted[0].node, renamed(&emission.omitted[0].node));
    assert_eq!(
        rows.omitted[0].cause,
        OmissionCause::NamesAbsentNode(renamed(&emission_id(&emission.omitted[0].cause)))
    );
    assert_eq!(
        rows.omitted[1].cause,
        OmissionCause::NamesOmittedNode(renamed(&emission_id(&emission.omitted[1].cause)))
    );

    assert_eq!(rows.temporal_binding_rows.len(), 1);
    assert_eq!(
        rows.temporal_binding_rows.rows[0].1.declaration,
        renamed(&emission.temporal_binding_rows.rows[0].1.declaration),
        "temporal binding declaration"
    );
}

fn emission_id(cause: &OmissionCause) -> CheckedNodeId {
    match cause {
        OmissionCause::NamesAbsentNode(id) | OmissionCause::NamesOmittedNode(id) => id.clone(),
        other => panic!("{other:?} names no node"),
    }
}

/// An id no rename names is carried unchanged, and a row naming a renamed
/// node and one naming another are each re-keyed independently.
#[trace("TC-1081", "FR-251-AC-42")]
#[test]
fn a_row_naming_a_node_no_rename_touches_is_carried_unchanged() {
    let emission = composite();
    let rows = carry_rows(&emission, &[]);
    assert_eq!(rows.omitted, emission.omitted);
    assert_eq!(
        rows.selected_operation_member_rows,
        emission.selected_operation_member_rows
    );
    assert_eq!(
        rows.selected_tuple_declaration_rows,
        emission.selected_tuple_declaration_rows
    );
    assert_eq!(
        rows.equality_producer_identity_rows,
        emission.equality_producer_identity_rows
    );
    assert_eq!(
        rows.equality_fact_witness_rows,
        emission.equality_fact_witness_rows
    );
    assert_eq!(rows.temporal_binding_rows, emission.temporal_binding_rows);
    let one = emission.omitted[0].node.clone();
    let rows = carry_rows(&emission, &[(one.clone(), renamed(&one))]);
    assert_eq!(rows.omitted[0].node, renamed(&one));
    assert_eq!(rows.omitted[1].node, emission.omitted[1].node);
    assert_eq!(
        rows.omitted[1].cause,
        OmissionCause::NamesOmittedNode(renamed(&one)),
        "a cause naming the renamed node follows it"
    );
}
