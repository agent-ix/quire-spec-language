// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093-AC-31 and FR-093-AC-32 (TC-415): an equality operand the checker converts
//! (`convert<T>(e) = x`) lowers as a typed `numeric.convert` node, and the
//! equality takes its operator family from the converted type, so IR's
//! checked-package/v2 reader admits the package.

use super::*;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"quire.value.complete/v1\";\n";

const DEC: &str = "Decimal[-100,100;0,0;nearest-even]";
const INT: &str = "Int[-100,100]";
const WIDE: &str = "Int[-200,200]";
const DEC2: &str = "Decimal[-100,100;0,2;nearest-even]";
const RAT: &str = "Rational[-100, 100; 1, 1]";

/// `declarations` through S1 to S4 and the v2 emitter, with nothing omitted.
fn checked_graph(declarations: &str) -> qsl_semantics::check::CheckedGraph {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:conv-eq", "fixture", "fixture:1"),
        "program.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert_eq!(parsed.diagnostics(), []);
    let raw = parsed.source().reference().clone();
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(raw, "pkg".to_owned(), unit, Vec::new(), Vec::new())
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"))
}

/// `declarations` through S1 to S4 and the v2 emitter, with nothing omitted.
fn emitted(declarations: &str) -> Emission {
    let graph = checked_graph(declarations);
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    emission
}

fn witnesses_for_occurrence<'a>(
    rows: &'a [EqualityFactWitnessRow],
    identity: &EqualityProducerEmissionRow,
) -> Vec<&'a EqualityFactWitnessRow> {
    rows.iter()
        .filter(|witness| {
            witness.application() == identity.application()
                && witness.occurrence() == identity.occurrence()
        })
        .collect()
}

/// QSpec-TC-233's EQ-01..05 positive rows: each typed equality retains its
/// selected schedule, explicit conversion target, resolved units and enum
/// sub-index before the IR-owned wire writer is connected.
#[ix_trace_rs::trace("QSpec-TC-233", "QSL-1390")]
#[test]
fn qspec_positive_equality_vectors_match_typed_producer_facts() {
    use quire_semantic_value::declaration::{EqualityOperator, EqualitySchedule};

    let declarations = "dimension Time;\n\
        unit s : Time = rational(1, 1);\n\
        unit ms : Time = rational(1, 1000) * s;\n\
        dimension Length;\n\
        unit m : Length = rational(1, 1);\n\
        enum Color { Blue, Green }\n\
        record Point { x: Integer; }\n\
        function text using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure { a = b }\n\
        function enumeration using v(a: Color, b: Color): Boolean pure { a != b }\n\
        function quantity using v(a: Quantity<ms>, b: Quantity<s>): Boolean pure { convert<Quantity<s>>(a) = b }\n\
        function plan using v(a: Point, b: Point): Boolean pure { a = b }\n\
        function compound using v(a: Quantity<m>, b: Quantity<m>): Boolean pure { a * b = a * b }\n";
    let graph = checked_graph(declarations);
    let facts: Vec<_> = graph.equality_producer_data().values().collect();
    assert_eq!(facts.len(), 5, "each equality application is retained once");

    let text = facts
        .iter()
        .find(|fact| fact.schedule() == EqualitySchedule::Text)
        .expect("EQ-01-text selects the text schedule");
    assert_eq!(text.operator(), EqualityOperator::Equal);
    assert_eq!(text.authored_operands()[0].target(), None);
    assert!(text.units().ids().next().is_none());

    let enumeration = facts
        .iter()
        .find(|fact| fact.schedule() == EqualitySchedule::Enum)
        .expect("EQ-02-enum selects the enum schedule");
    assert_eq!(enumeration.operator(), EqualityOperator::NotEqual);
    assert!(format!("{:?}", enumeration.enum_members()).contains("Blue"));
    assert!(format!("{:?}", enumeration.enum_members()).contains("Green"));
    assert!(enumeration.units().ids().next().is_none());

    let converted_quantity = facts
        .iter()
        .find(|fact| {
            fact.schedule() == EqualitySchedule::Quantity
                && fact.authored_operands()[0].target().is_some()
        })
        .expect("EQ-03-quantity retains its explicit ms-to-s conversion");
    assert_eq!(converted_quantity.operator(), EqualityOperator::Equal);
    assert_eq!(converted_quantity.units().ids().count(), 2);
    assert!(converted_quantity.compared_operands()[0].target().is_none());

    let plan = facts
        .iter()
        .find(|fact| fact.schedule() == EqualitySchedule::Plan)
        .expect("EQ-04-plan selects the plan schedule");
    assert_eq!(plan.operator(), EqualityOperator::Equal);
    assert!(plan.units().ids().next().is_none());

    let compound = facts
        .iter()
        .find(|fact| {
            fact.units().ids().count() == 1
                && fact.schedule() == EqualitySchedule::Quantity
                && fact.authored_operands()[0].target().is_none()
        })
        .expect("EQ-05-compound-quantity is backed by one compound quantity unit");
    assert_eq!(compound.operator(), EqualityOperator::Equal);
    assert!(compound.units().ids().any(|id| {
        matches!(
            compound.units().get(id),
            Some(quire_semantic_value::quantity::QuantityUnit::Compound(_))
        )
    }));
}

/// QSL-1390: every nested equality application is retained independently.
#[ix_trace_rs::trace("QSL-1390")]
#[test]
fn nested_equalities_each_retain_producer_facts() {
    let graph = checked_graph(
        "function nested using v(a: Integer, b: Integer, c: Integer): Boolean pure { a = b and b = c }",
    );
    assert_eq!(
        graph.equality_producer_data().len(),
        2,
        "both nested equality applications survive lowering"
    );
    assert!(graph.equality_producer_data().values().all(|facts| {
        facts.schedule() == quire_semantic_value::declaration::EqualitySchedule::Plan
    }));
}

/// Shared structural keys retain exact per-occurrence equality witnesses.
#[ix_trace_rs::trace("IR-FR-353-AC-1", "QSpec-FR-322-AC-288")]
#[test]
fn structurally_reused_equality_keys_retain_each_occurrence() {
    let graph = checked_graph(
        "dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;\n\
         function same using v(a: Quantity<ms>, b: Quantity<ms>): Boolean pure { a = b and convert<Quantity<ms>>(a) = b }",
    );
    let occurrences = graph.equality_producer_data();
    assert_eq!(occurrences.len(), 2, "both source occurrences are retained");
    let mut rows = occurrences.iter().collect::<Vec<_>>();
    rows.sort_by_key(|(_, data)| data.authored_operands()[0].target().is_some());

    let keys = occurrences.keys().collect::<Vec<_>>();
    assert_eq!(keys[0].0, keys[1].0, "lowering reused one structural key");
    assert_ne!(keys[0].1, keys[1].1, "source locations distinguish them");
    assert_eq!(rows[0].1.authored_operands()[0].target(), None);
    assert!(rows[1].1.authored_operands()[0].target().is_some());

    let emission = emitted(
        "dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;\n\
         function same using v(a: Quantity<ms>, b: Quantity<ms>): Boolean pure { a = b and convert<Quantity<ms>>(a) = b }",
    );
    let identity_rows = emission.equality_producer_identity_rows();
    assert_eq!(identity_rows.len(), 2);
    assert_eq!(
        identity_rows[0].application(),
        identity_rows[1].application(),
        "both authored occurrences retain the admitted application key"
    );
    assert_ne!(
        identity_rows[0].location(),
        identity_rows[1].location(),
        "each identity row retains its own exact checked occurrence"
    );
    assert_eq!(
        identity_rows[0].checked_node(),
        identity_rows[1].checked_node()
    );
    assert_ne!(identity_rows[0].occurrence(), identity_rows[1].occurrence());
    assert!(identity_rows
        .iter()
        .all(|row| row.location().node() == row.application()));
    assert!(identity_rows.iter().all(|row| {
        row.source_map_entry().node_id == *row.checked_node()
            && row.source_map_entry().role == row.occurrence().role
            && row.source_map_entry().ordinal == row.occurrence().ordinal
    }));
    let stage_rows = emission.equality_fact_witness_rows();
    let first_occurrence = witnesses_for_occurrence(stage_rows, &identity_rows[0]);
    let second_occurrence = witnesses_for_occurrence(stage_rows, &identity_rows[1]);
    assert!(!first_occurrence.is_empty());
    assert!(!second_occurrence.is_empty());
    assert!(first_occurrence
        .iter()
        .all(|witness| witness.occurrence() == identity_rows[0].occurrence()));
    assert!(second_occurrence
        .iter()
        .all(|witness| witness.occurrence() == identity_rows[1].occurrence()));
    assert_ne!(
        identity_rows[0].occurrence(),
        identity_rows[1].occurrence(),
        "a stage witness from the other structurally shared occurrence cannot satisfy this join"
    );

    let serialized = emission.package.bytes();
    let round_trip: Value =
        serde_json::from_slice(serialized).expect("the canonical package bytes read back as JSON");
    assert_eq!(
        serialized,
        jcs(&round_trip),
        "the package bytes are canonical"
    );
    let decisions = round_trip["checked_term_data"]["equality_decisions"]
        .as_array()
        .expect("the serialized package retains equality decisions");
    assert_eq!(decisions.len(), identity_rows.len());
    let producer_data = graph.equality_producer_data();
    let mut serialized_occurrences = Vec::new();
    let mut serialized_targets = Vec::new();
    let mut converted_type = None;
    for identity in identity_rows {
        let application = serde_json::to_value(identity.checked_node())
            .expect("the emitted application id serializes");
        let occurrence =
            serde_json::to_value(identity.occurrence()).expect("the emitted occurrence serializes");
        let matching = decisions
            .iter()
            .filter(|decision| {
                decision["application"] == application && decision["occurrence"] == occurrence
            })
            .collect::<Vec<_>>();
        assert_eq!(matching.len(), 1, "each application occurrence has one row");
        serialized_occurrences.push(occurrence);

        let facts = producer_data
            .get(&(
                identity.application().clone(),
                identity.producer_identity().location().clone(),
            ))
            .expect("the exact application/location pair retains producer facts");
        let decision = matching[0];
        assert_eq!(decision["left"]["operand"], json!(0));
        assert_eq!(decision["right"]["operand"], json!(1));
        serialized_targets.push(decision["left"]["conversion_target"].clone());
        if facts.authored_operands()[0].target().is_some() {
            assert_ne!(decision["left"]["conversion_target"], Value::Null);
            assert_eq!(
                decision["left"]["conversion_target"], decision["right"]["source_type"],
                "the converted occurrence targets the exact emitted ms type"
            );
            assert!(converted_type
                .replace(decision["right"]["source_type"].clone())
                .is_none());
        } else {
            assert_eq!(decision["left"]["conversion_target"], Value::Null);
        }
    }
    assert_ne!(serialized_occurrences[0], serialized_occurrences[1]);
    assert_eq!(
        serialized_targets
            .iter()
            .filter(|target| target.is_null())
            .count(),
        1,
        "the unconverted occurrence has a null target"
    );
    let converted_type = converted_type.expect("one occurrence has an exact conversion target");
    assert_eq!(
        serialized_targets
            .iter()
            .filter(|target| **target == converted_type)
            .count(),
        1,
        "the converted occurrence retains the emitted ms type"
    );
}

/// IR FR-353 AC-1 and QSpec FR-322 AC-288: exact witness joins are emitted.
#[ix_trace_rs::trace("IR-FR-353-AC-1", "QSpec-FR-322-AC-288")]
#[test]
fn emission_retains_unit_and_dimension_identity_roles() {
    use quire_exact::UnitDomain;

    let declarations =
        "dimension Length; dimension Area = Length;\n\
         unit m : Length = rational(1, 1); unit cm : Length = rational(1, 100) * m;\n\
         unit mm : Length = rational(1, 10) * cm;\n\
         unit sqm : Area = rational(1, 1);\n\
         function quantity using v(a: Quantity<mm>, b: Quantity<m>): Boolean pure { convert<Quantity<m>>(a) = b }\n\
         function area using v(a: Quantity<sqm>, b: Quantity<sqm>): Boolean pure { a = b }\n\
         function compound using v(a: Quantity<m>, b: Quantity<m>): Boolean pure { a * b = a * b }";
    let graph = checked_graph(declarations);
    let emission = emitted(declarations);
    let rows = emission.equality_producer_identity_rows();
    assert_eq!(rows.len(), 3);
    let declared = rows
        .iter()
        .find(|row| {
            row.producer_identity()
                .unit_ids()
                .iter()
                .any(|unit| unit.domain() == UnitDomain::Declared)
        })
        .expect("quantity rows retain their source UnitIds");
    assert!(declared
        .producer_identity()
        .unit_ids()
        .iter()
        .any(|unit| { unit.domain() == UnitDomain::Declared && unit.declared_key().is_some() }));

    let compound = rows
        .iter()
        .find(|row| {
            row.producer_identity()
                .unit_ids()
                .iter()
                .any(|unit| unit.domain() == UnitDomain::Compound)
        })
        .expect("compound quantity rows retain their exact compound UnitId");
    assert!(compound
        .producer_identity()
        .unit_ids()
        .iter()
        .any(|unit| unit.domain() == UnitDomain::Compound));
    assert!(compound
        .producer_identity()
        .unit_facts()
        .iter()
        .any(|fact| !fact.dimensions.is_empty()));
    let derived = rows
        .iter()
        .find(|row| {
            row.producer_identity()
                .unit_facts()
                .iter()
                .any(|fact| !fact.dimension_terms.is_empty())
        })
        .expect("derived dimension rows retain their exact term positions");
    assert!(derived
        .producer_identity()
        .unit_facts()
        .iter()
        .flat_map(|fact| &fact.dimension_terms)
        .any(|term| term.ordinal == 0));
    assert!(!declared
        .producer_identity()
        .unit_facts()
        .iter()
        .flat_map(|fact| &fact.conversion_edges)
        .collect::<Vec<_>>()
        .is_empty());

    let witnesses = emission.equality_fact_witness_rows();
    assert!(witnesses
        .iter()
        .any(|row| matches!(row.fact(), EqualityFactWitness::Unit { .. })));
    assert!(witnesses
        .iter()
        .any(|row| matches!(row.fact(), EqualityFactWitness::DimensionDeclaration { .. })));
    assert!(witnesses
        .iter()
        .any(|row| matches!(row.fact(), EqualityFactWitness::DimensionTerm { .. })));
    assert!(witnesses
        .iter()
        .any(|row| matches!(row.fact(), EqualityFactWitness::ConversionEdge { .. })));
    let emitted_ids = wire(&emission)["semantic_graph"]["nodes"]
        .as_array()
        .expect("emitted nodes")
        .iter()
        .filter_map(|node| node.get("node_id").cloned())
        .collect::<Vec<_>>();
    for row in witnesses {
        assert!(
            emitted_ids.contains(&serde_json::to_value(row.structural_node()).unwrap()),
            "fact key {:?} joins to emitted ID {:?}",
            row.structural_key(),
            row.structural_node()
        );
        assert_eq!(row.source_map_entry().node_id, *row.checked_application());
        assert_eq!(row.source_map_entry().role, row.occurrence().role);
        assert_eq!(row.source_map_entry().ordinal, row.occurrence().ordinal);
    }
    assert!(
        witnesses.iter().any(|row| row.owner().is_some()),
        "source/model fact context is retained"
    );

    for equality in rows {
        let witness_rows = witnesses_for_occurrence(witnesses, equality);
        for type_fact in equality.producer_identity().operand_type_nodes() {
            let role = match type_fact.role {
                qsl_semantics::check::EqualityUnitOperandRole::Source => {
                    EqualityOperandTypeRole::Source
                }
                qsl_semantics::check::EqualityUnitOperandRole::ConversionTarget => {
                    EqualityOperandTypeRole::ConversionTarget
                }
            };
            assert!(witness_rows.iter().any(|row| {
                row.structural_key() == &type_fact.node
                    && matches!(row.fact(), EqualityFactWitness::OperandType { position, role: actual }
                        if *position == type_fact.operand_position && *actual == role)
            }), "operand type witness matches exact producer fact {type_fact:?}");
        }
        for unit_fact in equality.producer_identity().unit_facts() {
            let unit_role = match unit_fact.operand_role {
                qsl_semantics::check::EqualityUnitOperandRole::Source => EqualityUnitRole::Source,
                qsl_semantics::check::EqualityUnitOperandRole::ConversionTarget => {
                    EqualityUnitRole::ConversionTarget
                }
            };
            let unit_row = witness_rows
                .iter()
                .find(|row| {
                    row.structural_key() == &unit_fact.structural_node
                        && matches!(row.fact(), EqualityFactWitness::Unit {
                        unit_id, quantity_type, operand_position, role,
                    } if unit_id == &unit_fact.unit_id
                        && quantity_type == row.structural_node()
                        && *operand_position == unit_fact.operand_position
                        && *role == unit_role)
                })
                .expect("UnitId joins its checker-held quantity type and role");
            assert_eq!(
                unit_row.structural_node(),
                match unit_row.fact() {
                    EqualityFactWitness::Unit { quantity_type, .. } => quantity_type,
                    _ => unreachable!(),
                }
            );

            for dimension in &unit_fact.dimensions {
                assert!(
                    witness_rows.iter().any(|row| {
                        row.structural_key() == &dimension.node
                            && matches!(row.fact(), EqualityFactWitness::DimensionDeclaration {
                            unit_id, operand_position, role, ordinal,
                        } if unit_id == &dimension.unit_id
                            && *operand_position == unit_fact.operand_position
                            && *role == dimension.role
                            && *ordinal == dimension.ordinal)
                    }),
                    "dimension role and ordinal join to exact declaration {dimension:?}"
                );
            }
            for term in &unit_fact.dimension_terms {
                let row = witness_rows
                    .iter()
                    .find(|row| {
                        row.structural_key() == &term.declaration
                            && matches!(row.fact(), EqualityFactWitness::DimensionTerm {
                            unit_id, operand_position, ordinal, base, base_node, exponent,
                        } if unit_id == &term.unit_id
                            && *operand_position == unit_fact.operand_position
                            && *ordinal == term.ordinal
                            && base == &term.base
                            && exponent.as_ref() == term.exponent.to_string())
                    })
                    .expect("dimension term joins its declaration and base");
                let base_row = witness_rows
                    .iter()
                    .find(|candidate| candidate.structural_key() == &term.base)
                    .expect("base dimension has its own structural witness");
                let EqualityFactWitness::DimensionTerm { base_node, .. } = row.fact() else {
                    unreachable!()
                };
                assert_eq!(base_node, base_row.structural_node());
            }
            for edge in &unit_fact.conversion_edges {
                let row = witness_rows
                    .iter()
                    .find(|row| {
                        row.structural_key() == &edge.source
                            && matches!(row.fact(), EqualityFactWitness::ConversionEdge {
                            unit_id, operand_position, ordinal, target, ..
                        } if unit_id == &edge.unit_id
                            && *operand_position == unit_fact.operand_position
                            && *ordinal == edge.ordinal
                            && target == &edge.target)
                    })
                    .expect("conversion edge retains source, target, and ordinal");
                let target_row = witness_rows
                    .iter()
                    .find(|candidate| candidate.structural_key() == &edge.target)
                    .expect("conversion target has its own structural witness");
                let EqualityFactWitness::ConversionEdge {
                    target_node,
                    scale,
                    offset,
                    ..
                } = row.fact()
                else {
                    unreachable!()
                };
                assert_eq!(target_node, target_row.structural_node());
                let source = graph
                    .semantic_graph()
                    .nodes()
                    .find(|node| node.key() == edge.source)
                    .expect("source nominal node is in the checked graph");
                let Some(NominalPayload::Unit(payload)) = source.nominal_payload() else {
                    panic!("unit edge payload")
                };
                assert_eq!(scale, &checked_rational(payload.factor()));
                assert_eq!(offset, &checked_rational(payload.offset()));
            }
        }
    }

    let converted = rows
        .iter()
        .find(|row| {
            row.producer_identity()
                .operand_type_nodes()
                .iter()
                .any(|fact| {
                    fact.role == qsl_semantics::check::EqualityUnitOperandRole::ConversionTarget
                })
        })
        .expect("conversion equality retained");
    let source_unit = converted
        .producer_identity()
        .unit_facts()
        .iter()
        .find(|fact| {
            fact.operand_role == qsl_semantics::check::EqualityUnitOperandRole::Source
                && !fact.conversion_edges.is_empty()
        })
        .expect("converted source unit retained");
    let roles = converted
        .producer_identity()
        .unit_facts()
        .iter()
        .filter(|fact| fact.unit_id == source_unit.unit_id)
        .flat_map(|fact| &fact.dimensions)
        .map(|fact| (fact.node.clone(), fact.role, fact.ordinal))
        .collect::<Vec<_>>();
    assert!(
        roles.iter().any(|(node, role, _)| {
            *role == qsl_semantics::check::EqualityDimensionRole::UnitDimension
                && roles.iter().any(|(other_node, other_role, other_ordinal)| {
                    other_node == node
                        && *other_role
                            == qsl_semantics::check::EqualityDimensionRole::ConversionDimension
                        && *other_ordinal != 0
                })
        }),
        "same dimension retains distinct UnitDimension and ConversionDimension roles: {roles:?}"
    );
    let source_dimension = source_unit
        .dimensions
        .iter()
        .find(|fact| fact.role == qsl_semantics::check::EqualityDimensionRole::UnitDimension)
        .expect("source dimension fact");
    let unit_dimension = witnesses
        .iter()
        .find(|row| {
            row.application() == converted.application()
                && row.structural_key() == &source_dimension.node
                && matches!(row.fact(), EqualityFactWitness::DimensionDeclaration {
                    unit_id,
                    role: qsl_semantics::check::EqualityDimensionRole::UnitDimension,
                    ..
                } if unit_id == &source_unit.unit_id)
        })
        .expect("UnitDimension witness emitted");
    let conversion_dimension = witnesses
        .iter()
        .find(|row| {
            row.application() == converted.application()
                && row.structural_key() == unit_dimension.structural_key()
                && matches!(row.fact(), EqualityFactWitness::DimensionDeclaration {
                    unit_id,
                    role: qsl_semantics::check::EqualityDimensionRole::ConversionDimension,
                    ..
                } if unit_id == &source_unit.unit_id)
        })
        .expect("ConversionDimension witness emitted for the same key");
    assert_eq!(
        unit_dimension.structural_node(),
        conversion_dimension.structural_node()
    );
    let (
        EqualityFactWitness::DimensionDeclaration {
            ordinal: unit_ordinal,
            ..
        },
        EqualityFactWitness::DimensionDeclaration {
            ordinal: conversion_ordinal,
            ..
        },
    ) = (unit_dimension.fact(), conversion_dimension.fact())
    else {
        unreachable!()
    };
    assert_ne!(unit_ordinal, conversion_ordinal);
    let ordered_edges = witnesses
        .iter()
        .filter_map(|row| match row.fact() {
            EqualityFactWitness::ConversionEdge { ordinal, .. } => Some(*ordinal),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        ordered_edges.windows(2).any(|pair| pair == [0, 1]),
        "edge order remains visible: {ordered_edges:?}"
    );
}

/// QSL-1430: missing, foreign, and mismatched node joins refuse typed.
#[ix_trace_rs::trace("QSL-1430", "IR-FR-353")]
#[test]
fn equality_fact_witness_join_refuses_missing_foreign_and_mismatched_rows() {
    let graph = checked_graph(
        "dimension Length; unit m : Length = rational(1, 1);\n\
         function quantity using v(a: Quantity<m>, b: Quantity<m>): Boolean pure { a = b }",
    );
    let emission = emit_checked(&CheckedPackage::link(checked_graph(
        "dimension Length; unit m : Length = rational(1, 1);\n\
         function quantity using v(a: Quantity<m>, b: Quantity<m>): Boolean pure { a = b }",
    )))
    .expect("valid producer emits");
    let rows = emission.equality_producer_identity_rows();
    assert!(!rows.is_empty());

    let missing = equality_fact_witness_rows(&graph, &std::collections::HashMap::new(), &[], rows);
    assert!(matches!(
        missing,
        Err(EmitRefusal::MissingEqualityIdentityContext { .. })
    ));

    let row = rows.first().expect("one equality identity row");
    let mut foreign = std::collections::HashMap::new();
    foreign.insert(row.application().clone(), row.checked_node().clone());
    let foreign = equality_fact_witness_rows(&graph, &foreign, &[], rows);
    assert!(matches!(
        foreign,
        Err(EmitRefusal::MissingEqualityIdentityContext { .. })
    ));

    let other = checked_graph(
        "dimension Time; unit s : Time = rational(1, 1);\n\
         function quantity using v(a: Quantity<s>, b: Quantity<s>): Boolean pure { a = b }",
    );
    let mismatched =
        equality_fact_witness_rows(&other, &std::collections::HashMap::new(), &[], rows);
    assert!(matches!(
        mismatched,
        Err(EmitRefusal::MissingEqualityIdentityContext { .. })
    ));
}

/// A direct compound key join refuses missing and ambiguous producer witnesses.
#[ix_trace_rs::trace("IR-FR-353-AC-1", "QSpec-FR-322-AC-288")]
#[test]
fn unit_identity_join_refuses_missing_and_ambiguous_compound_keys() {
    use qsl_semantics::check::{EqualityUnitIdentityFact, EqualityUnitOperandRole};

    let compound_key = fixture_type_key("compound_type");
    let foreign_key = fixture_type_key("other_compound_type");
    let first_id = quire_exact::UnitId::declared(fixture_type_key("unit_a"));
    let second_id = quire_exact::UnitId::declared(fixture_type_key("unit_b"));
    let fact = |unit_id, structural_node| EqualityUnitIdentityFact {
        unit_id,
        structural_node,
        operand_position: 0,
        operand_role: EqualityUnitOperandRole::Source,
        dimensions: Vec::new(),
        dimension_terms: Vec::new(),
        conversion_edges: Vec::new(),
    };
    let application = fixture_type_key("equality");
    let exact = vec![fact(first_id.clone(), compound_key.clone())];
    assert_eq!(
        equality_unit_identity_for_type(&exact, &compound_key, &application)
            .expect("direct key mapping exists")
            .unit_id,
        first_id
    );
    assert!(matches!(
        equality_unit_identity_for_type(&exact, &foreign_key, &application),
        Err(EmitRefusal::MissingEqualityIdentityContext { .. })
    ));
    let ambiguous = vec![
        fact(first_id, compound_key.clone()),
        fact(second_id, compound_key.clone()),
    ];
    assert!(matches!(
        equality_unit_identity_for_type(&ambiguous, &compound_key, &application),
        Err(EmitRefusal::MissingEqualityIdentityContext { .. })
    ));
}

/// IR FR-353 AC-1 and QSpec FR-322 AC-288: Text/Plan operand types join exactly.
#[ix_trace_rs::trace("IR-FR-353-AC-1", "QSpec-FR-322-AC-288")]
#[test]
fn text_and_plan_equality_rows_retain_operand_type_identities() {
    let emission = emitted(
        "record R { x: Integer; }\n\
         function text_eq using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure { a = b }\n\
         function plan_eq using v(a: R, b: R): Boolean pure { a = b }",
    );
    super::assert_equality_reader_outcomes(&emission, read_back(&emission));
    assert_eq!(emission.equality_producer_identity_rows().len(), 2);
    assert!(emission
        .equality_fact_witness_rows()
        .iter()
        .any(|row| matches!(row.fact(), EqualityFactWitness::TextPlanType { .. })));
    assert!(emission
        .equality_fact_witness_rows()
        .iter()
        .any(|row| matches!(row.fact(), EqualityFactWitness::OperandType { .. })));
    for row in emission.equality_producer_identity_rows() {
        let type_rows = row.producer_identity().text_plan_type_nodes();
        assert_eq!(type_rows.len(), 2);
        let occurrence_witnesses =
            witnesses_for_occurrence(emission.equality_fact_witness_rows(), row);
        for (position, (ordinal, key)) in type_rows.iter().enumerate() {
            assert_eq!(*ordinal, position);
            let operand_fact = row
                .producer_identity()
                .operand_type_nodes()
                .iter()
                .find(|fact| fact.operand_position == position)
                .expect("typed operand identity retained");
            assert_eq!(operand_fact.node, *key);
            assert_eq!(
                operand_fact.role,
                qsl_semantics::check::EqualityUnitOperandRole::Source
            );
            let type_witness = occurrence_witnesses
                .iter()
                .find(|witness| {
                    witness.application() == row.application()
                        && witness.structural_key() == key
                        && matches!(witness.fact(), EqualityFactWitness::TextPlanType {
                            position: actual_position,
                            role: EqualityOperandTypeRole::Source,
                        } if *actual_position == position)
                })
                .expect("Text/Plan type fact retains exact source role");
            let operand_witness = occurrence_witnesses
                .iter()
                .find(|witness| {
                    witness.application() == row.application()
                        && witness.structural_key() == key
                        && matches!(witness.fact(), EqualityFactWitness::OperandType {
                            position: actual_position,
                            role: EqualityOperandTypeRole::Source,
                        } if *actual_position == position)
                })
                .expect("operand source fact retains exact type node");
            assert_eq!(
                type_witness.structural_node(),
                operand_witness.structural_node()
            );
        }
    }
}

/// QSL-1390: every equality schedule is emitted to the IR carrier with its
/// exact application occurrence and schedule-specific identity facts.
#[ix_trace_rs::trace("QSpec-FR-322-AC-288", "QSL-1390")]
#[test]
fn equality_producer_rows_are_emitted_in_checked_term_data() {
    let emission = emitted(
        "dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;\n\
         enum Color { Blue, Green }\n\
         record Point { x: Integer; }\n\
         function text using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure { a = b }\n\
         function enumeration using v(a: Color, b: Color): Boolean pure { a != b }\n\
         function quantity using v(a: Quantity<ms>, b: Quantity<s>): Boolean pure { convert<Quantity<s>>(a) = b }\n\
         function plan using v(a: Point, b: Point): Boolean pure { a = b }",
    );
    super::assert_equality_reader_outcomes(&emission, read_back(&emission));
    let wire = wire(&emission);
    let sidecar = wire
        .get("checked_term_data")
        .and_then(Value::as_object)
        .expect("equalities produce checked_term_data");
    assert_eq!(
        sidecar.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "dispatch_tables",
            "equality_decisions",
            "protocol_locations"
        ]
    );
    assert_eq!(sidecar["dispatch_tables"], json!([]));
    assert_eq!(sidecar["protocol_locations"], json!([]));
    let decisions = sidecar["equality_decisions"]
        .as_array()
        .expect("equality rows are an array");
    assert_eq!(decisions.len(), 4);
    for decision in decisions {
        assert_eq!(decision["term_path"], json!([]));
        assert!(decision["application"].is_object());
        assert!(decision["occurrence"].is_object());
        assert_eq!(decision["left"]["operand"], json!(0));
        assert_eq!(decision["right"]["operand"], json!(1));
    }
    let by_schedule = decisions
        .iter()
        .map(|decision| (decision["schedule"].as_str().unwrap(), decision))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(by_schedule.len(), 4);
    assert_eq!(by_schedule["text"]["units"], json!([]));
    assert_eq!(by_schedule["plan"]["units"], json!([]));
    assert_eq!(
        by_schedule["enum"]["enum_members"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let enum_members = by_schedule["enum"]["enum_members"].as_array().unwrap();
    assert_eq!(enum_members[0]["case"], json!("Blue"));
    assert_eq!(enum_members[0]["ordinal"], json!(0));
    assert_eq!(enum_members[1]["case"], json!("Green"));
    assert_eq!(enum_members[1]["ordinal"], json!(1));
    assert_eq!(enum_members[0]["ordered"], json!(false));
    let quantity = by_schedule["quantity"];
    let units = quantity["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    assert_ne!(
        quantity["left"]["source_type"],
        quantity["left"]["conversion_target"]
    );
    assert_eq!(units[0]["form"]["kind"], json!("declared"));
    assert_eq!(units[0]["form"]["path"].as_array().unwrap().len(), 1);
    assert_eq!(units[0]["form"]["path"][0]["target"], units[1]["unit"]);
    assert_eq!(
        units[0]["form"]["path"][0]["scale"]["numerator"],
        json!("1")
    );
    assert_eq!(
        units[0]["form"]["path"][0]["scale"]["denominator"],
        json!("1000")
    );
    assert_eq!(by_schedule["enum"]["units"], json!([]));
    assert_eq!(by_schedule["quantity"]["enum_members"], json!([]));
}

/// QSL-1390: a compound quantity carries only its exact normalized dimension
/// and root-unit powers in the sidecar.
#[ix_trace_rs::trace("QSpec-FR-322-AC-288", "QSL-1390")]
#[test]
fn compound_quantity_equality_emits_exact_unit_facts() {
    let emission = emitted(
        "dimension Length; unit m : Length = rational(1, 1);\n\
         function compound using v(a: Quantity<m>, b: Quantity<m>): Boolean pure { a * b = a * b }",
    );
    super::assert_equality_reader_outcomes(&emission, read_back(&emission));
    let wire = wire(&emission);
    let decision = &wire["checked_term_data"]["equality_decisions"][0];
    let units = decision["units"].as_array().unwrap();
    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["form"]["kind"], json!("compound"));
    assert_eq!(
        units[0]["form"]["dimension_terms"][0]["dimension"],
        wire["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["semantic_form"] == "dimension")
            .map(|node| node["node_id"].clone())
            .unwrap()
    );
    assert_eq!(
        units[0]["form"]["dimension_terms"][0]["exponent"],
        json!("2")
    );
    assert_eq!(units[0]["form"]["terms"].as_array().unwrap().len(), 1);
    assert_eq!(units[0]["form"]["terms"][0]["exponent"], json!("2"));
}

/// Missing and duplicate structural/source witnesses fail with the emitter's
/// typed refusal instead of constructing a sidecar row by inference.
#[ix_trace_rs::trace("QSL-1390")]
#[test]
fn equality_emission_refuses_missing_or_duplicate_witnesses() {
    let declarations =
        "function text using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure { a = b }";
    let graph = checked_graph(declarations);
    let emission =
        emit_checked(&CheckedPackage::link(checked_graph(declarations))).expect("emission");
    let rows = emission.equality_producer_identity_rows();
    let mut ids = rows
        .iter()
        .map(|row| (row.application().clone(), row.checked_node().clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let source_map = rows
        .iter()
        .map(|row| row.source_map_entry().clone())
        .collect::<Vec<_>>();
    ids.remove(rows[0].application());
    assert!(matches!(
        equality_producer_emission_rows(&graph, &ids, &source_map),
        Err(EmitRefusal::MissingEqualityIdentityContext { .. })
    ));

    let ids = rows
        .iter()
        .map(|row| (row.application().clone(), row.checked_node().clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let mut duplicate_sources = source_map;
    duplicate_sources.push(rows[0].source_map_entry().clone());
    assert!(matches!(
        equality_producer_emission_rows(&graph, &ids, &duplicate_sources),
        Err(EmitRefusal::MissingEqualityIdentityContext { .. })
    ));

    let mut wrong_document = rows
        .iter()
        .map(|row| row.source_map_entry().clone())
        .collect::<Vec<_>>();
    wrong_document[0].regions[0].source = quire_contract_model::CheckedSourceRef {
        authority: "test".into(),
        identity: "different-document".into(),
    };
    assert!(matches!(
        equality_producer_emission_rows(&graph, &ids, &wrong_document),
        Err(EmitRefusal::MissingEqualityIdentityContext { .. })
    ));

    let mut mismatched_ordinal = rows
        .iter()
        .map(|row| row.source_map_entry().clone())
        .collect::<Vec<_>>();
    mismatched_ordinal[0].ordinal = mismatched_ordinal[0].ordinal.saturating_add(1);
    assert!(matches!(
        equality_producer_emission_rows(&graph, &ids, &mismatched_ordinal),
        Err(EmitRefusal::MissingEqualityIdentityContext { .. })
    ));
}

/// The application nodes of operation `identity` in `wire`.
fn applications<'w>(wire: &'w Value, identity: &str) -> Vec<&'w Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == identity)
        .collect()
}

/// The node `reference` term `term` names.
fn referenced<'w>(wire: &'w Value, term: &Value) -> &'w Value {
    let key = &term["target"];
    nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == key)
        .expect("the referenced node")
}

/// Read each emitted equality through IR's typed admitted-equality accessor,
/// then return the graph for producer-side assertions.
fn assert_reader_outcome_for_equality_sidecar(emission: &Emission) {
    super::assert_equality_reader_outcomes(emission, read_back(emission));
}

/// `function f using v(a: {source}, b: {other}): Boolean pure { body }`
/// emitted with its equality sidecar. Returns the wire.
fn admitted(source: &str, other: &str, body: &str) -> Value {
    let emission = emitted(&format!(
        "function f using v(a: {source}, b: {other}): Boolean pure {{ {body} }}"
    ));
    assert_reader_outcome_for_equality_sidecar(&emission);
    wire(&emission)
}

/// `body` lowers to one `numeric.convert` over the parameter `a`, typed
/// at the type node of `b`, and one `quire.op.<family>.<suffix>` whose
/// argument at `converted_at` is that node and whose other argument is `b`.
fn assert_converted(
    source: &str,
    other: &str,
    body: &str,
    family: &str,
    suffix: &str,
    converted_at: usize,
) {
    let wire = admitted(source, other, body);
    let converts = applications(&wire, "quire.op.numeric.convert");
    assert_eq!(converts.len(), 1, "{body}: one convert node");
    let convert = converts[0];
    let source = referenced(&wire, &convert["body"]["arguments"][0]);
    assert_eq!(source["semantic_form"], "parameter", "{body}");
    assert_eq!(
        source["body"]["members"][0]["value"]["value"], "a",
        "{body}: the convert reads `a`"
    );
    let identity = format!("quire.op.{family}.{suffix}");
    let equalities = applications(&wire, &identity);
    assert_eq!(equalities.len(), 1, "{body}: one {identity}");
    let arguments = equalities[0]["body"]["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 2, "{body}");
    assert_eq!(
        arguments[converted_at]["target"], convert["node_id"],
        "{body}: the converted operand is the convert node"
    );
    let peer = referenced(&wire, &arguments[1 - converted_at]);
    assert_eq!(
        peer["body"]["members"][0]["value"]["value"], "b",
        "{body}: the other operand is `b`"
    );
    assert_eq!(
        convert["semantic_type"], peer["semantic_type"],
        "{body}: the convert is typed at the peer's type"
    );
    for wrong in ["integer", "rational", "decimal"] {
        if wrong != family {
            for suffix in ["eq", "ne"] {
                assert_eq!(
                    applications(&wire, &format!("quire.op.{wrong}.{suffix}")).len(),
                    0,
                    "{body}: no {wrong}.{suffix}"
                );
            }
        }
    }
}

/// FR-093-AC-31: `convert<Decimal>(a) = b` over an Int `a` lowers a Decimal
/// `numeric.convert` over `a` and `decimal.eq` over it, and strict IR admits
/// the package.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn a_converted_decimal_equality_converts_the_int_operand() {
    assert_converted(
        INT,
        DEC,
        &format!("convert<{DEC}>(a) = b"),
        "decimal",
        "eq",
        0,
    );
}

/// FR-093-AC-31: the same for `!=`.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn a_converted_decimal_inequality_converts_the_int_operand() {
    assert_converted(
        INT,
        DEC,
        &format!("convert<{DEC}>(a) != b"),
        "decimal",
        "ne",
        0,
    );
}

/// FR-093-AC-31: the converted operand on the right.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn a_converted_operand_on_the_right_is_the_second_argument() {
    assert_converted(
        INT,
        DEC,
        &format!("b = convert<{DEC}>(a)"),
        "decimal",
        "eq",
        1,
    );
    assert_converted(
        INT,
        DEC,
        &format!("b != convert<{DEC}>(a)"),
        "decimal",
        "ne",
        1,
    );
}

/// FR-093-AC-31: the Rational family takes the same treatment.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn a_converted_rational_equality_converts_the_int_operand() {
    assert_converted(
        INT,
        RAT,
        &format!("convert<{RAT}>(a) = b"),
        "rational",
        "eq",
        0,
    );
    assert_converted(
        INT,
        RAT,
        &format!("convert<{RAT}>(a) != b"),
        "rational",
        "ne",
        0,
    );
    assert_converted(
        INT,
        RAT,
        &format!("b = convert<{RAT}>(a)"),
        "rational",
        "eq",
        1,
    );
}

/// FR-093-AC-31: an equality with no conversion is unchanged: `integer.eq`
/// over the two parameters and no convert node.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn an_unconverted_equality_builds_no_convert_node() {
    let wire = admitted("Int[-100,100]", "Int[-100,100]", "a = b");
    assert_eq!(applications(&wire, "quire.op.numeric.convert").len(), 0);
    let equalities = applications(&wire, "quire.op.integer.eq");
    assert_eq!(equalities.len(), 1);
    for (index, expected) in ["a", "b"].into_iter().enumerate() {
        let operand = referenced(&wire, &equalities[0]["body"]["arguments"][index]);
        assert_eq!(operand["semantic_form"], "parameter");
        assert_eq!(operand["body"]["members"][0]["value"]["value"], expected);
    }
}

/// FR-093-AC-31: an Int converted to a wider Int and a Decimal converted to
/// a Decimal of a larger scale are converted operands of their own family.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn a_converted_integer_or_widened_decimal_equality_converts_its_operand() {
    assert_converted(
        INT,
        WIDE,
        &format!("convert<{WIDE}>(a) = b"),
        "integer",
        "eq",
        0,
    );
    assert_converted(
        DEC,
        DEC2,
        &format!("b != convert<{DEC2}>(a)"),
        "decimal",
        "ne",
        1,
    );
}

/// FR-093-AC-31: the other numeric directions, each a converted operand of
/// the family of the converted type.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn a_converted_equality_between_decimal_rational_and_int_converts_its_operand() {
    for (source, target, family) in [
        (DEC, INT, "integer"),
        (RAT, INT, "integer"),
        (RAT, DEC, "decimal"),
        (DEC, RAT, "rational"),
    ] {
        for (body, suffix, at) in [
            (format!("convert<{target}>(a) = b"), "eq", 0),
            (format!("convert<{target}>(a) != b"), "ne", 0),
            (format!("b = convert<{target}>(a)"), "eq", 1),
        ] {
            assert_converted(source, target, &body, family, suffix, at);
        }
    }
}

/// FR-093-AC-31: both operands converted builds one convert per operand and
/// an equality over the two.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn an_equality_with_both_operands_converted_converts_each() {
    let emission = emitted(&format!(
        "function f using v(a: {INT}, c: Int[-50,50]): Boolean pure {{ convert<{DEC}>(a) = convert<{DEC}>(c) }}"
    ));
    assert_reader_outcome_for_equality_sidecar(&emission);
    let wire = wire(&emission);
    let converts = applications(&wire, "quire.op.numeric.convert");
    assert_eq!(converts.len(), 2);
    let equalities = applications(&wire, "quire.op.decimal.eq");
    assert_eq!(equalities.len(), 1);
    let arguments: Vec<&Value> = equalities[0]["body"]["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|argument| &argument["target"])
        .collect();
    for (argument, parameter) in arguments.iter().zip(["a", "c"]) {
        let convert = converts
            .iter()
            .find(|convert| &convert["node_id"] == *argument)
            .expect("each argument is a convert node");
        let source = referenced(&wire, &convert["body"]["arguments"][0]);
        assert_eq!(source["body"]["members"][0]["value"]["value"], parameter);
    }
}

/// A unit `kilometre` of 1000 `metre`, the root unit `metre` and their
/// dimension, owned by the fixture source.
fn length_units() -> (
    quire_semantic_value::quantity::UnitTable,
    ValueType,
    ValueType,
) {
    use qsl_semantics::value::{
        admit_unit_graph, DimensionPreimage, NodeOwner, OwnerSelection, UnitPreimage,
    };
    let owner = json!({"kind": "source", "authority": "a", "identity": "u", "package_name": "pkg"});
    let length = DimensionPreimage::from_json(json!({
        "version": "quire.dimension-node/v1",
        "owner": owner,
        "qualified_declaration": ["Example", "Length"],
        "terms": [],
    }))
    .expect("a base dimension");
    let length_key = length.key().expect("a dimension declaration key");
    let node_ref = |key: &NodeKey| {
        qsl_foundation::node_wire::node_key_to_json(key).expect("a structural fixture key")
    };
    let unit = |name: &str, target: Option<NodeKey>, scale: &str| {
        UnitPreimage::from_json(json!({
            "version": "quire.unit-node/v1",
            "owner": owner,
            "qualified_declaration": ["Example", name],
            "dimension_node_id": node_ref(&length_key),
            "target_unit_node_id": target.as_ref().map(node_ref),
            "scale": {"numerator": scale, "denominator": "1"},
            "offset": {"numerator": "0", "denominator": "1"},
        }))
        .expect("a unit")
    };
    let metre = unit("metre", None, "1");
    let metre_key = metre.key().expect("a unit declaration key");
    let kilometre = unit("kilometre", Some(metre_key.clone()), "1000");
    let kilometre_key = kilometre.key().expect("a unit declaration key");
    let selection: NodeOwner = serde_json::from_value(owner).expect("an owner");
    let graph = admit_unit_graph(
        [(length, length_key)],
        [
            (metre, metre_key.clone()),
            (kilometre, kilometre_key.clone()),
        ],
        &OwnerSelection::new([selection]),
    )
    .expect("the unit graph admits");
    (
        quire_semantic_value::quantity::UnitTable::declared(&graph),
        ValueType::Quantity(quire_exact::UnitId::declared(metre_key)),
        ValueType::Quantity(quire_exact::UnitId::declared(kilometre_key)),
    )
}

/// FR-093-AC-31: `convert<Km>(a) = b` over a metre quantity `a` lowers a
/// `quantity.convert` with mode `exact` and `quantity.eq`/`.ne` over it.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn a_converted_quantity_equality_converts_the_quantity_operand() {
    let (units, metre, kilometre) = length_units();
    let named = |text: &str| TypeForm::name(text, SPAN);
    for (operator, suffix, converted_first) in [
        (BinaryOperator::Equal, "eq", true),
        (BinaryOperator::NotEqual, "ne", true),
        (BinaryOperator::Equal, "eq", false),
        (BinaryOperator::NotEqual, "ne", false),
    ] {
        let converted = Expression::convert(named("Km"), name("a"));
        let (left, right) = if converted_first {
            (converted, name("b"))
        } else {
            (name("b"), converted)
        };
        let declared = FunctionDeclaration::new(
            "f",
            vec![
                ("a".to_owned(), named("Length")),
                ("b".to_owned(), named("Km")),
            ],
            boolean(),
            None,
            Expression::binary(operator, left, right),
        );
        let graph = PackageDeclarations {
            types: TypeEnvironment::default().with_units(units.clone()),
            aliases: vec![
                ("Length".to_owned(), metre.clone()),
                ("Km".to_owned(), kilometre.clone()),
            ],
            functions: vec![declared],
            ..PackageDeclarations::new(source(), "pkg", qsl_foundation::IdentityLimits::default())
        }
        .check(CheckingLimits::default())
        .expect("the package checks");
        let emission = emit(&CheckedPackage::link(graph));
        assert_reader_outcome_for_equality_sidecar(&emission);
        let wire = wire(&emission);
        let converts = applications(&wire, "quire.op.quantity.convert");
        assert_eq!(converts.len(), 1, "{suffix}: one convert node");
        assert_eq!(
            converts[0]["body"]["operation"]["mode"],
            json!({"kind": "rounding", "value": "exact"})
        );
        assert!(applications(&wire, "quire.op.numeric.convert").is_empty());
        let equalities = applications(&wire, &format!("quire.op.quantity.{suffix}"));
        assert_eq!(equalities.len(), 1, "{suffix}: one quantity.{suffix}");
        let at = usize::from(!converted_first);
        assert_eq!(
            equalities[0]["body"]["arguments"][at]["target"], converts[0]["node_id"],
            "{suffix}: the converted operand is the convert node"
        );
    }
}

/// FR-093-AC-32: a guard (`=`, `!=` or an ordering) over a value-keeping
/// integer conversion, a chain of them included, bounds its operand as the
/// guard over the operand itself does.
#[trace("FR-093-AC-32", "TC-415")]
#[test]
fn a_guard_over_an_integer_conversion_bounds_the_converted_operand() {
    for body in [
        "if x = 2 then x else 0",
        "if convert<Integer>(x) = 2 then x else 0",
        "if convert<Int[-10,10]>(x) = 2 then x else 0",
        "if 2 = convert<Integer>(x) then x else 0",
        "if convert<Integer>(x) != 2 then 0 else x",
        "if convert<Int[-5,5]>(convert<Decimal[-5,5;0,0;nearest-even]>(x)) = 2 then x else 0",
        "if convert<Integer>(x) < 4 then (if convert<Integer>(x) > -1 then x else 0) else 0",
        "if convert<Integer>(x) >= 0 then (if convert<Int[-10,10]>(x) <= 3 then x else 0) else 0",
        "if 4 > convert<Integer>(x) then (if -1 < convert<Integer>(x) then x else 0) else 0",
    ] {
        let emission = emitted(&format!(
            "function g using v(x: Int[-5,5]): Int[0,3] pure {{ {body} }}"
        ));
        assert_reader_outcome_for_equality_sidecar(&emission);
    }
}

/// FR-093-AC-32: the bound is the guard's, so an unguarded or differently
/// guarded body still refuses.
#[trace("FR-093-AC-32", "TC-415")]
#[test]
fn a_guard_over_an_integer_conversion_bounds_only_what_it_states() {
    for body in [
        "if convert<Integer>(x) = 4 then x else 0",
        "if convert<Integer>(x) != 2 then x else 0",
        "if convert<Integer>(x) < 4 then x else 0",
        "if convert<Integer>(x) > -1 then x else 0",
        "x",
    ] {
        let text =
            format!("{HEADER}function g using v(x: Int[-5,5]): Int[0,3] pure {{ {body} }}\n");
        let parsed = qsl_cst::parse(
            qsl_foundation::SourceIdentity::new("agent-ix", "test:guard", "fixture", "fixture:1"),
            "program.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 admits the unit");
        let raw = parsed.source().reference().clone();
        let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
        let refused =
            PackageDeclarations::assemble(raw, "pkg".to_owned(), unit, Vec::new(), Vec::new())
                .expect("the unit assembles")
                .check(CheckingLimits::default());
        assert!(refused.is_err(), "{body}: the body is not in [0, 3]");
    }
}

/// FR-093-AC-31: the IR-17 host conversion sources, each with its own scalar
/// domains: `convert<T>(a) = b` lowers a convert node and the equality of
/// `T`'s family, and strict IR admits the package.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn the_host_conversion_sources_lower_a_convert_and_read_back_verified() {
    for (source, target, family) in [
        ("Int[-100, 100]", "Integer", "integer"),
        (
            "Int[-100, 100]",
            "Decimal[-100, 100; 0, 0; nearest-even]",
            "decimal",
        ),
        (
            "Rational[-10, 10; 1, 1]",
            "Rational[-100, 100; 1, 5]",
            "rational",
        ),
        ("Rational[-50, 50; 1, 1]", "Integer", "integer"),
        (
            "Decimal[-100, 100; 0, 0; nearest-even]",
            "Rational[-100, 100; 1, 5]",
            "rational",
        ),
        (
            "Decimal[-100, 100; 0, 0; nearest-even]",
            "Decimal[-1000, 1000; 0, 2; nearest-even]",
            "decimal",
        ),
        (
            "Decimal[-100, 100; 0, 0; nearest-even]",
            "Integer",
            "integer",
        ),
    ] {
        assert_converted(
            source,
            target,
            &format!("convert<{target}>(a) = b"),
            family,
            "eq",
            0,
        );
    }
}
