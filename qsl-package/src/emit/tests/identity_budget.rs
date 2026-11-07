// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531 identity producer inputs, native admission and owned output charges.

use super::*;
use crate::{IdentityFileRefusal, NativeRowError, NativeRowErrorCode, ReadIdentityRows};
use quire_contract_model::{NativeJoinBudget, NativeJoinChargeKind, SelectedExactIdentities};

fn fixture() -> (Emission, Box<quire_contract_model::CheckedPackageV2>) {
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
    assert!(!emission.identity_row_selection.declarations.is_empty());
    assert!(!emission.identity_row_selection.fields.is_empty());
    assert!(!emission.selected_source_record_rows.is_empty());
    assert!(!emission.selected_source_union_rows.is_empty());
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
    let native = phases.last_success_delta();
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
