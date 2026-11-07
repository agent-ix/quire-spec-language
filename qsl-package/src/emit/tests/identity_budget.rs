// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531 identity producer inputs, native admission and owned output charges.

use super::*;
use crate::{IdentityFileRefusal, NativeRowError, NativeRowErrorCode, ReadIdentityRows};
use quire_contract_model::{NativeJoinBudget, NativeJoinChargeKind, SelectedExactIdentities};

fn fixture() -> (Emission, Arc<quire_contract_model::CheckedPackageV2>) {
    let text = format!(
        "{}\n{}\n{}",
        child_composite::UNIT,
        source_record_rows::RECORDS,
        source_union_rows::SHAPE
    );
    let (emission, _, read) = child_composite::emit_unit(&text);
    let Read::Verified { admitted, .. } = read else {
        panic!("genuine combined package admits")
    };
    assert!(!emission.selected_exact_declaration_rows().is_empty());
    assert!(!emission.selected_exact_field_rows().is_empty());
    assert_eq!(emission.selected_source_record_rows().len(), 2);
    assert_eq!(emission.selected_source_union_rows().len(), 1);
    (emission, admitted)
}

fn ample() -> NativeJoinBudget {
    NativeJoinBudget::new(1_000_000_000, 1_000_000_000)
}

fn adapter_error(error: IdentityFileRefusal) -> NativeRowError {
    match error {
        IdentityFileRefusal::Rows(error) => error,
        other => panic!("compact adapter denial, got {other:?}"),
    }
}

#[test]
#[trace("TC-1331", "FR-531-AC-31")]
fn authentic_identity_rows_pay_producer_join_and_output_once_and_preserve_native_values() {
    let (emission, package) = fixture();
    let mut phases = ample();
    let (declarations, fields) = emission
        .identity_input_rows(&mut phases)
        .expect("producer inputs");
    let producer = phases.consumed();
    assert!(producer.work > 0 && producer.storage > 0);
    let joined =
        SelectedExactIdentities::from_stage_rows(&package, declarations, fields, &mut phases)
            .expect("one actual native join");
    let native = phases.last_success_delta().expect("completed native join");
    assert_eq!(phases.consumed().work, producer.work + native.work);
    assert_eq!(phases.consumed().storage, producer.storage + native.storage);
    let before_output = phases.consumed();
    let expected = emission
        .identity_output_rows(&package, &joined, &mut phases)
        .expect("post-join owned output");
    assert!(phases.consumed().work > before_output.work);
    assert!(phases.consumed().storage > before_output.storage);
    let need = phases.consumed();
    let mut exact = NativeJoinBudget::new(need.work, need.storage);
    let actual = emission
        .identity_rows(&package, &mut exact)
        .expect("exact complete budget");
    assert_eq!(actual, expected);
    assert_eq!(exact.consumed(), need);
    assert_eq!(
        actual.declarations(),
        joined.declaration_rows().cloned().collect::<Vec<_>>()
    );
    assert_eq!(actual.fields(), joined.field_rows());
    assert_eq!(
        actual.source_unions(),
        emission.selected_source_union_rows()
    );
    assert_eq!(
        actual.source_records().len(),
        emission.selected_source_record_rows().len()
    );
    for (original, copied) in emission
        .selected_source_record_rows()
        .iter()
        .zip(actual.source_records())
    {
        assert_eq!(
            (
                &original.structural_node,
                &original.record_key,
                &original.qualified_declaration
            ),
            (
                &copied.structural_node,
                &copied.record_key,
                &copied.qualified_declaration
            )
        );
        assert_eq!(original.members.len(), copied.members.len());
        for (original, copied) in original.members.iter().zip(&copied.members) {
            assert_eq!(
                (
                    &original.original_member_identity,
                    &original.admitted_name,
                    original.presence,
                    &original.type_node,
                    &original.value_type
                ),
                (
                    &copied.original_member_identity,
                    &copied.admitted_name,
                    copied.presence,
                    &copied.type_node,
                    &copied.value_type
                )
            );
        }
    }
    let bytes = actual.to_bytes(&package).expect("genuine carrier");
    let read = ReadIdentityRows::read(&bytes, &package, &mut ample()).expect("native carrier join");
    assert_eq!(read, actual);
    let selected = actual
        .selected(&package, &mut ample())
        .expect("native record and union joins");
    for row in actual.source_records() {
        let projected = package
            .project_exact_type(&row.structural_node, &selected, &mut ample())
            .expect("actual native record projection");
        assert_eq!(projected.source_record_view(), Some(row));
        assert_eq!(
            projected.value_type(),
            &ValueType::Composite(row.record_key.clone())
        );
    }
    for row in actual.source_unions() {
        let projected = package
            .project_exact_type(&row.structural_node, &selected, &mut ample())
            .expect("actual native union projection");
        assert_eq!(
            projected.value_type(),
            &ValueType::Composite(row.type_handle.clone())
        );
    }
}

#[test]
#[trace("TC-1331", "FR-531-AC-31")]
#[trace("TC-1331", "FR-531-AC-33")]
fn identity_one_short_and_zero_denials_are_compact_and_keep_the_first_unpaid_charge() {
    let (emission, package) = fixture();
    let mut measure = ample();
    emission
        .identity_rows(&package, &mut measure)
        .expect("measure genuine identity output");
    let need = measure.consumed();
    for (work, storage, kind) in [
        (need.work - 1, need.storage, NativeJoinChargeKind::Work),
        (need.work, need.storage - 1, NativeJoinChargeKind::Storage),
    ] {
        let mut budget = NativeJoinBudget::new(work, storage);
        let error = adapter_error(
            emission
                .identity_rows(&package, &mut budget)
                .expect_err("one short returns no rows"),
        );
        assert_eq!(error.denied(), budget.first_unpaid());
        assert_eq!(error.denied().expect("owner first charge").kind, kind);
        assert_eq!(error.locus(), budget.first_unpaid_locus());
        assert_eq!(
            error.configured_bound(),
            Some(if kind == NativeJoinChargeKind::Work {
                work
            } else {
                storage
            })
        );
        let prefix = budget.consumed();
        let next = adapter_error(
            emission
                .identity_rows(&package, &mut budget)
                .expect_err("no reset after denial"),
        );
        assert_eq!(next, error);
        assert_eq!(budget.consumed(), prefix);
    }
    let mut budget = NativeJoinBudget::new(need.work, 0);
    let _ = allocation_counter::measure(|| {});
    let mut outcome = None;
    let allocations = allocation_counter::measure(|| {
        outcome = Some(emission.identity_rows(&package, &mut budget))
    });
    let error = adapter_error(
        outcome
            .expect("call ran")
            .expect_err("zero storage refuses"),
    );
    assert_eq!(error.code(), NativeRowErrorCode::StorageLimit);
    assert_eq!(error.configured_bound(), Some(0));
    assert_eq!(allocations.count_total, 0);
    assert_eq!(allocations.bytes_total, 0);
}

#[test]
#[trace("TC-1331", "FR-531-AC-31")]
#[trace("TC-1331", "FR-531-AC-33")]
fn two_identity_materializations_compose_without_restoring_allowance() {
    let (emission, package) = fixture();
    let mut measure = ample();
    let mut rows = None;
    let _ = allocation_counter::measure(|| {});
    let observed =
        allocation_counter::measure(|| rows = Some(emission.identity_rows(&package, &mut measure)));
    let expected = rows.expect("observed call").expect("success");
    assert!(observed.count_total > 0 && observed.bytes_total > 0);
    let need = measure.consumed();
    let mut exact_two = NativeJoinBudget::new(need.work * 2, need.storage * 2);
    assert_eq!(
        emission
            .identity_rows(&package, &mut exact_two)
            .expect("first"),
        expected
    );
    assert_eq!(exact_two.consumed(), need);
    assert_eq!(
        emission
            .identity_rows(&package, &mut exact_two)
            .expect("second"),
        expected
    );
    assert_eq!(exact_two.consumed().work, need.work * 2);
    assert_eq!(exact_two.consumed().storage, need.storage * 2);
    let mut short_two = NativeJoinBudget::new(need.work * 2, need.storage * 2 - 1);
    assert_eq!(
        emission
            .identity_rows(&package, &mut short_two)
            .expect("prefix"),
        expected
    );
    let error = adapter_error(
        emission
            .identity_rows(&package, &mut short_two)
            .expect_err("second materialization keeps first consumption"),
    );
    assert_eq!(error.configured_bound(), Some(need.storage * 2 - 1));
    assert!(short_two.consumed().storage >= need.storage);
}

#[test]
#[trace("TC-1331", "FR-531-AC-33")]
fn denied_identity_output_copies_do_not_allocate_payloads_or_diagnostics() {
    let (emission, package) = fixture();
    let (declarations, fields) = emission
        .identity_input_rows(&mut ample())
        .expect("real producer inputs");
    let joined =
        SelectedExactIdentities::from_stage_rows(&package, declarations, fields, &mut ample())
            .expect("actual native join");
    let mut budget = NativeJoinBudget::new(1_000_000_000, 0);
    let _ = allocation_counter::measure(|| {});
    let mut result = None;
    let observed = allocation_counter::measure(|| {
        result = Some(emission.identity_output_rows(&package, &joined, &mut budget))
    });
    let denied = adapter_error(
        result
            .expect("call executed")
            .expect_err("unpaid output refuses"),
    );
    assert_eq!(denied.code(), NativeRowErrorCode::StorageLimit);
    assert_eq!(
        denied.locus().expect("output location").form,
        "identity_declaration_output"
    );
    assert_eq!(denied.denied(), budget.first_unpaid());
    assert_eq!(observed.bytes_total, 0);
    assert_eq!(observed.count_total, 0);
}

#[test]
#[trace("TC-1331", "FR-531-AC-31")]
fn final_emission_input_selection_matches_the_existing_native_stage_filter() {
    let (emission, package) = fixture();
    let existing = qsl_semantics::model::intake::selected_exact_identities_from_rows(
        &package,
        emission.selected_exact_declaration_rows(),
        emission.selected_exact_field_rows(),
        &mut ample(),
    )
    .expect("existing selected membership");
    let actual = emission
        .identity_rows(&package, &mut ample())
        .expect("accounted selected membership");
    assert_eq!(
        actual.declarations(),
        existing.declaration_rows().cloned().collect::<Vec<_>>()
    );
    assert_eq!(actual.fields(), existing.field_rows());
}

#[test]
#[trace("TC-1331", "FR-531-AC-31")]
#[trace("TC-1331", "FR-531-AC-33")]
fn source_record_admitted_i128_endpoint_types_are_copied_exactly_with_precharged_storage() {
    // Authored Int bounds are admitted only within i128. Heap-backed values
    // beyond that boundary are covered at the actual shared copy helper.
    let source = "record Boundary { sample: Int[-170141183460469231731687303715884105728, 170141183460469231731687303715884105727]; }\nfunction keep using v(b: Boundary): Boolean pure { true }\n";
    let (_, checked) = package_from_text(source);
    let emission = emit_checked(&checked).expect("genuine boundary record emits");
    let package = source_union_rows::admit_bytes(&emission, &wire(&emission));
    assert_eq!(emission.selected_source_record_rows().len(), 1);
    let _ = allocation_counter::measure(|| {});
    let mut budget = ample();
    let mut copied = None;
    let observed = allocation_counter::measure(|| {
        copied = Some(emission.identity_rows(&package, &mut budget))
    });
    let copied = copied
        .expect("call executed")
        .expect("charged boundary output");
    assert!(observed.bytes_total > 0);
    let ValueType::Int(interval) = &copied.source_records()[0].members[0].value_type else {
        panic!("original bounded integer retained")
    };
    let lower: quire_exact::Integer = "-170141183460469231731687303715884105728".parse().unwrap();
    let upper: quire_exact::Integer = "170141183460469231731687303715884105727".parse().unwrap();
    assert_eq!(interval.lower(), &lower);
    assert_eq!(interval.upper(), &upper);
    let need = budget.consumed();
    let mut exact = NativeJoinBudget::new(need.work, need.storage);
    assert_eq!(
        emission
            .identity_rows(&package, &mut exact)
            .expect("exact bound"),
        copied
    );
}

#[test]
#[trace("TC-1331", "FR-531-AC-31")]
fn final_monitor_plan_carry_recomputes_producer_membership_after_real_writer_rekeys() {
    use super::admission_corpus::config_version_unit;
    use crate::emit::{emit_monitor_plan_over, MonitorPlanWire};
    let body = "temporal C using tt over (c: Config::ConfigVersion) clock \"model-time\" on origin { always holds(c.versionNumber = 2) }\n\
record State { now: Integer; }\n\
record Inputs { x: Integer; }\n\
record Scope { y: Integer; }\n\
record Result { lab: String; }\n\
record Pair { fst: Integer; }\n\
function mkPair using v(): Pair pure { Pair { fst: 1 } }\n\
function results using v(): Sequence<Result> pure { sequence[Result { lab: \"a\" }] }\n\
function e0 using v(s: State): State pure { s }\n\
function e1 using v(s: State, i: Inputs): State pure { s }\n\
function e2 using v(s: State, p: Scope): State pure { s }\n\
function e3 using v(s: State, g: Integer): State pure { s }\n";
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let record_document =
        serde_json::to_vec(&child_composite::fixture::document()).expect("real record domain");
    let packages =
        qsl_semantics::model::intake::package_input([document, record_document.as_slice()])
            .expect("both actual domain packages");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"quire.model.complete/v1\";\n\
profile t = \"quire.temporal.complete/v1\";\n\
profile tt = \"quire.temporal.timed/v1\";\n\
model Config = {config:?};\n\
model M = {records:?};\n\
dimension Time;\n\
unit ms : Time = rational(1, 1);\n\
time Config dense unit ms;\n\
function occurredAt using v(e: M::OrderPlaced): Integer pure {{ e.occurred_at }}\n\
function fromExpress using v(e: M::ExpressOrderPlaced): Integer pure {{ occurredAt(e) }}\n\
function fromRush using v(e: M::RushOrderPlaced): Integer pure {{ occurredAt(e) }}\n\
{body}",
        config = config_version_unit::PACKAGE_IDENTITY,
        records = child_composite::fixture::PACKAGE,
    );
    let graph = assemble_with_models(source.as_bytes(), &packages)
        .expect("author unit")
        .check(CheckingLimits::default())
        .expect("checked unit");
    let checked = CheckedPackage::link(graph);
    let base = emit_checked(&checked).expect("original emission");
    let clause = &checked.graph().temporal_clauses()[0];
    let key = clause.identity();
    let formula = clause.clause().formula();
    let mut evidence = base.evidence().clone();
    evidence
        .insert_domain_package_document(package_identity(document), document)
        .expect("real domain evidence");
    evidence
        .insert_domain_package_document(
            package_identity(&record_document),
            record_document.as_slice(),
        )
        .expect("the selected record domain evidence");
    let rate = quire_exact::Rational::new(1_i64.into(), 500_i64.into()).unwrap();
    let zero = quire_exact::Rational::from_integer(0_i64.into());
    let plan = MonitorPlanWire {
        clause: &key,
        formula,
        counter: "counter",
        counter_width_bits: 12,
        max_reading_gap: 3600,
        max_event_rate: &rate,
        uncertainty: &zero,
        buffers: &[],
        tick_unit: "ms",
        activation_capacity: 8,
        state: "State",
        inputs: "Inputs",
        scope: "Scope",
        result: "Result",
        entries: ["e0", "e1", "e2", "e3"],
        renamed_records: &["Pair"],
        renames: &[("fst", "one")],
    };
    let rooted = emit_monitor_plan_over(
        &base,
        LibraryName::new(base.package().name()).unwrap(),
        &evidence,
        &plan,
        V2ReadLimits::fixture(),
        crate::EmitLimits::fixture(),
    )
    .expect("real writer carry");
    // This writer fixture exercises final carry and rename. Its separate
    // monitor-entry ABI is not a native identity-admission oracle; genuine
    // admitted identity rows are projected in the combined fixture above.
    let expected_declarations: Vec<usize> = rooted
        .selected_exact_declaration_rows()
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            rooted
                .wire_nodes
                .iter()
                .any(|node| {
                    qsl_semantics::model::intake::selected_exact_declaration_row_matches_node(
                        row, node,
                    )
                })
                .then_some(index)
        })
        .collect();
    let expected_fields: Vec<usize> = rooted
        .selected_exact_field_rows()
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            rooted
                .wire_nodes
                .iter()
                .any(|node| {
                    qsl_semantics::model::intake::selected_exact_field_row_matches_node(row, node)
                })
                .then_some(index)
        })
        .collect();
    assert!(
        !expected_declarations.is_empty(),
        "the actual carried model declarations are selected"
    );
    assert!(
        !expected_fields.is_empty(),
        "real selected model record fields survive carry"
    );
    assert_eq!(
        rooted.identity_row_selection.declarations,
        expected_declarations
    );
    assert_eq!(rooted.identity_row_selection.fields, expected_fields);
    assert_ne!(
        base.package().bytes(),
        rooted.package().bytes(),
        "the actual writer changed its package"
    );
    let original_pair = base
        .selected_source_record_rows()
        .iter()
        .find(|row| {
            row.qualified_declaration
                .iter()
                .map(AsRef::as_ref)
                .eq(["Pair"])
        })
        .expect("original authored record");

    let pair = rooted
        .selected_source_record_rows()
        .iter()
        .find(|row| {
            row.qualified_declaration
                .iter()
                .map(AsRef::as_ref)
                .eq(["Pair"])
        })
        .expect("real renamed source record");
    assert_eq!(pair.members[0].original_member_identity.as_ref(), "fst");
    assert_eq!(pair.members[0].admitted_name.as_ref(), "one");
    assert_eq!(pair.record_key, original_pair.record_key);
    assert_eq!(pair.source_authority, original_pair.source_authority);
    assert_eq!(pair.source_identity, original_pair.source_identity);
    assert_eq!(
        pair.structural_node, original_pair.structural_node,
        "the named declaration retains its original structural identity"
    );
    let before_value = base
        .wire_nodes
        .iter()
        .find(|node| {
            node.semantic_form.as_ref() == "record_value"
                && node.semantic_type == original_pair.structural_node
        })
        .expect("the real authored mkPair value");
    let after_value = rooted
        .wire_nodes
        .iter()
        .find(|node| {
            node.semantic_form.as_ref() == "record_value"
                && node.semantic_type == pair.structural_node
        })
        .expect("the actual writer carried the renamed Pair value");
    assert_ne!(
        before_value.node_id, after_value.node_id,
        "renaming the value's binding really rekeys its content node"
    );
    assert!(
        !rooted
            .wire_nodes
            .iter()
            .any(|node| node.node_id == before_value.node_id),
        "the stale value id is absent from the final graph"
    );
}

#[test]
#[trace("TC-1331", "FR-531-AC-31")]
#[trace("TC-1331", "FR-531-AC-33")]
fn later_authentic_record_member_copy_denial_retains_an_observed_admitted_prefix() {
    use crate::emit::identity_rows::copy_source_record;
    use quire_contract_model::{NativeJoinLocus, NativeJoinLocusSource};
    let (emission, _) = fixture();
    let records = emission.selected_source_record_rows();
    assert_eq!(records.len(), 2);
    assert_eq!(
        records[0]
            .qualified_declaration
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        ["Point"]
    );
    assert_eq!(
        records[1]
            .qualified_declaration
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        ["Tagged"]
    );
    let point_index = records
        .iter()
        .position(|row| row.qualified_declaration.last().map(AsRef::as_ref) == Some("Point"))
        .expect("original Point row");
    let tagged_index = records
        .iter()
        .position(|row| row.qualified_declaration.last().map(AsRef::as_ref) == Some("Tagged"))
        .expect("original Tagged row");
    assert_ne!(point_index, tagged_index);
    let point_locus = NativeJoinLocus {
        source: NativeJoinLocusSource::AdapterRow(point_index),
        form: "identity_record_output",
    };
    let tagged_locus = NativeJoinLocus {
        source: NativeJoinLocusSource::AdapterRow(tagged_index),
        form: "identity_record_output",
    };
    let mut measure = ample();
    let first = copy_source_record(&records[point_index], &mut measure, point_locus)
        .expect("genuine first record conversion");
    let first_need = measure.consumed();
    let second = copy_source_record(&records[tagged_index], &mut measure, tagged_locus)
        .expect("genuine second record conversion");
    assert_eq!(
        second
            .members
            .iter()
            .map(|member| member.original_member_identity.as_ref())
            .collect::<Vec<_>>(),
        ["p", "note", "maybe"]
    );
    assert_eq!(
        second.members[0].value_type,
        records[tagged_index].members[0].value_type
    );
    assert_eq!(
        second.members[1].presence,
        records[tagged_index].members[1].presence
    );
    assert_eq!(
        second.members[2].value_type,
        records[tagged_index].members[2].value_type
    );
    let need = measure.consumed();
    for (work, storage, kind) in [
        (need.work - 1, need.storage, NativeJoinChargeKind::Work),
        (need.work, need.storage - 1, NativeJoinChargeKind::Storage),
    ] {
        let mut budget = NativeJoinBudget::new(work, storage);
        let _ = allocation_counter::measure(|| {});
        let mut prefix = None;
        let observed_prefix = allocation_counter::measure(|| {
            prefix = Some(copy_source_record(
                &records[point_index],
                &mut budget,
                point_locus,
            ))
        });
        let prefix = prefix
            .expect("prefix call")
            .expect("already paid first record");
        assert_eq!(prefix, first);
        assert_eq!(budget.consumed(), first_need);
        assert!(observed_prefix.count_total > 0);
        let mut failure = None;
        let observed_denial = allocation_counter::measure(|| {
            failure = Some(copy_source_record(
                &records[tagged_index],
                &mut budget,
                tagged_locus,
            ))
        });
        let failure = failure
            .expect("later call")
            .expect_err("unpaid Tagged member returns no copied record");
        assert_eq!(failure.denied().expect("first unpaid resource").kind, kind);
        assert_eq!(
            failure.locus(),
            Some(NativeJoinLocus {
                source: NativeJoinLocusSource::AdapterRow(tagged_index),
                form: "identity_record_member_output",
            })
        );
        assert_eq!(
            failure.configured_bound(),
            Some(if kind == NativeJoinChargeKind::Work {
                work
            } else {
                storage
            })
        );
        assert!(budget.consumed().storage >= first_need.storage);
        assert_eq!(
            observed_denial.count_total, 0,
            "neither denied record payload nor diagnostic allocates"
        );
        assert_eq!(observed_denial.bytes_total, 0);
        assert_eq!(
            prefix, first,
            "earlier admitted real copy survives the later refusal"
        );
    }
}

#[test]
#[trace("TC-1331", "FR-531-AC-31")]
#[trace("TC-1331", "FR-531-AC-33")]
fn later_authentic_union_payload_copy_denial_keeps_the_paid_record_prefix() {
    use crate::emit::identity_rows::{copy_source_record, copy_source_union};
    use quire_contract_model::{NativeJoinLocus, NativeJoinLocusSource};
    let (emission, _) = fixture();
    let records = emission.selected_source_record_rows();
    let point_index = records
        .iter()
        .position(|row| row.qualified_declaration.last().map(AsRef::as_ref) == Some("Point"))
        .expect("original Point row");
    let record = &records[point_index];
    let unions = emission.selected_source_union_rows();
    assert_eq!(unions.len(), 1);
    let shape_index = unions
        .iter()
        .position(|row| row.qualified_declaration.last().map(AsRef::as_ref) == Some("Shape"))
        .expect("original Shape row");
    let union = &unions[shape_index];
    assert_eq!(
        union
            .qualified_declaration
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        ["Shape"]
    );
    assert_eq!(
        union
            .members
            .iter()
            .map(|member| member.member.identifier())
            .collect::<Vec<_>>(),
        ["Empty", "Circle", "Rect"]
    );
    let record_locus = NativeJoinLocus {
        source: NativeJoinLocusSource::AdapterRow(point_index),
        form: "identity_record_output",
    };
    let union_locus = NativeJoinLocus {
        source: NativeJoinLocusSource::AdapterRow(shape_index),
        form: "identity_union_output",
    };
    let mut measure = ample();
    let first = copy_source_record(record, &mut measure, record_locus).expect("real first output");
    let first_need = measure.consumed();
    let original = copy_source_union(union, &mut measure, union_locus).expect("genuine union copy");
    assert_eq!(original, *union);
    assert_eq!(original.members[2].payload_nodes.len(), 2);
    assert_eq!(
        original.members[2].payload,
        [ValueType::Integer, ValueType::Integer]
    );
    let need = measure.consumed();
    for (work, storage, kind) in [
        (need.work - 1, need.storage, NativeJoinChargeKind::Work),
        (need.work, need.storage - 1, NativeJoinChargeKind::Storage),
    ] {
        let mut budget = NativeJoinBudget::new(work, storage);
        let _ = allocation_counter::measure(|| {});
        let mut prefix = None;
        let observed_prefix = allocation_counter::measure(|| {
            prefix = Some(copy_source_record(record, &mut budget, record_locus))
        });
        let prefix = prefix.expect("prefix call").expect("admitted real record");
        assert_eq!(prefix, first);
        assert_eq!(budget.consumed(), first_need);
        assert!(observed_prefix.bytes_total > 0);
        let mut failure = None;
        let observed_denial = allocation_counter::measure(|| {
            failure = Some(copy_source_union(union, &mut budget, union_locus))
        });
        let failure = failure
            .expect("union call")
            .expect_err("unpaid union payload returns no row");
        assert_eq!(failure.denied().expect("original unpaid charge").kind, kind);
        assert_eq!(
            failure.locus(),
            Some(NativeJoinLocus {
                source: NativeJoinLocusSource::AdapterRow(shape_index),
                form: "identity_union_member_output",
            })
        );
        assert_eq!(
            failure.configured_bound(),
            Some(if kind == NativeJoinChargeKind::Work {
                work
            } else {
                storage
            })
        );
        assert_eq!(observed_denial.count_total, 0);
        assert_eq!(observed_denial.bytes_total, 0);
        assert_eq!(prefix, first);
    }
}

#[test]
#[trace("TC-1331", "FR-531-AC-31")]
fn output_adapter_denials_use_the_original_row_channel_indices() {
    use quire_contract_model::{NativeJoinLocus, NativeJoinLocusSource};

    let (emission, package) = fixture();
    let records = emission.selected_source_record_rows();
    let tagged_index = records
        .iter()
        .position(|row| row.qualified_declaration.last().map(AsRef::as_ref) == Some("Tagged"))
        .expect("retained Tagged row");
    assert!(
        tagged_index > 0,
        "the later record distinguishes a constant-zero locus"
    );
    let shape_index = emission
        .selected_source_union_rows()
        .iter()
        .position(|row| row.qualified_declaration.last().map(AsRef::as_ref) == Some("Shape"))
        .expect("retained Shape row");
    let mut native_budget = ample();
    let (declarations, fields) = emission
        .identity_input_rows(&mut native_budget)
        .expect("authentic producer input copies");
    let joined = SelectedExactIdentities::from_stage_rows(
        &package,
        declarations,
        fields,
        &mut native_budget,
    )
    .expect("the native join precedes output materialization");
    let mut complete_budget = ample();
    let complete = emission
        .identity_output_rows(&package, &joined, &mut complete_budget)
        .expect("complete owned output");
    assert_eq!(complete.source_records().len(), records.len());
    let need = complete_budget.consumed();

    for expected in [
        NativeJoinLocus {
            source: NativeJoinLocusSource::AdapterRow(tagged_index),
            form: "identity_record_member_output",
        },
        NativeJoinLocus {
            source: NativeJoinLocusSource::AdapterRow(shape_index),
            form: "identity_union_member_output",
        },
    ] {
        let (mut budget, refusal) = (0..need.work)
            .find_map(|limit| {
                let mut budget = NativeJoinBudget::new(limit, need.storage);
                let refusal = adapter_error(
                    emission
                        .identity_output_rows(&package, &joined, &mut budget)
                        .expect_err("the complete output needs more work"),
                );
                (refusal.locus() == Some(expected)).then_some((budget, refusal))
            })
            .expect("the actual output loop reaches this original row and channel");
        assert_eq!(
            refusal.denied().expect("native unpaid charge").kind,
            NativeJoinChargeKind::Work
        );
        assert_eq!(refusal.configured_bound(), Some(budget.work_limit()));
        let consumed = budget.consumed();
        assert!(
            consumed.storage > 0,
            "the genuine earlier output prefix was admitted"
        );
        let repeated = adapter_error(
            emission
                .identity_rows(&package, &mut budget)
                .expect_err("a later complete adapter cannot resume over denial"),
        );
        assert_eq!(repeated, refusal);
        assert_eq!(budget.first_unpaid_locus(), Some(expected));
        assert_eq!(budget.consumed(), consumed);
    }
}

#[test]
#[trace("TC-1331", "FR-531-AC-31")]
fn sparse_producer_input_selection_keeps_its_retained_field_index() {
    use quire_contract_model::{NativeJoinLocus, NativeJoinLocusSource};

    let (mut emission, _) = fixture();
    let original_index = emission
        .identity_row_selection
        .fields
        .iter()
        .copied()
        .find(|index| *index > 0)
        .expect("the authentic retained field population has a later row");
    let expected = emission.selected_exact_field_rows()[original_index].clone();
    // Exercise the producer-copy phase only. A subset is not claimed to be
    // a complete or authoritative native identity join.
    emission.identity_row_selection.declarations.clear();
    emission.identity_row_selection.fields = vec![original_index];
    let mut complete = ample();
    let (declarations, fields) = emission
        .identity_input_rows(&mut complete)
        .expect("the original retained field copies");
    assert!(declarations.is_empty());
    assert_eq!(fields, vec![expected]);
    let need = complete.consumed();
    for kind in [NativeJoinChargeKind::Work, NativeJoinChargeKind::Storage] {
        let mut budget = NativeJoinBudget::new(
            if kind == NativeJoinChargeKind::Work {
                need.work - 1
            } else {
                need.work
            },
            if kind == NativeJoinChargeKind::Storage {
                need.storage - 1
            } else {
                need.storage
            },
        );
        let refusal = adapter_error(
            emission
                .identity_input_rows(&mut budget)
                .expect_err("the selected retained field must be paid before copying"),
        );
        assert_eq!(refusal.denied().expect("first unpaid charge").kind, kind);
        assert_eq!(
            refusal.locus(),
            Some(NativeJoinLocus {
                source: NativeJoinLocusSource::AdapterRow(original_index),
                form: "identity_field_input",
            })
        );
    }
}
