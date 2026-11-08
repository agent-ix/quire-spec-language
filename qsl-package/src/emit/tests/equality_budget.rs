// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531 shared equality admission on producer and bytes-only paths.

use super::*;
use crate::equality_file::EqualityDecisionFailure;
use crate::{BarePackage, BareSupply, NativeJoinFailure, ReadEqualityRows};
use quire_contract_model::{
    AdmittedEqualityReadErrorCode as Code, NativeJoinBudget, NativeJoinChargeKind,
};

fn emission() -> Emission {
    converted_equality::emitted(
        "function same using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure { a = b }",
    )
}

#[trace("TC-1331", "FR-531-AC-28")]
#[test]
fn producer_and_bytes_equality_share_exact_owner_charges_and_denials() {
    let emission = emission();
    let Read::Verified { admitted, .. } = read_back(&emission) else {
        panic!("native reader admits")
    };
    let [row] = emission.equality_producer_identity_rows() else {
        panic!("one equality")
    };
    let witness = emission.equality_witness(&admitted);
    let bytes = witness
        .to_bytes(&admitted)
        .expect("authentic equality bytes");
    let read = ReadEqualityRows::read(&bytes, &admitted).expect("bytes read");
    let mut measure = NativeJoinBudget::new(1_000_000, 1_000_000);
    let expected = emission
        .admitted_equality(
            &admitted,
            row.checked_node(),
            row.occurrence(),
            &mut measure,
        )
        .expect("producer decision");
    assert_eq!(
        expected.schedule(),
        quire_semantic_value::declaration::EqualitySchedule::Text
    );
    assert_eq!(
        expected.operator(),
        quire_semantic_value::declaration::EqualityOperator::Equal
    );
    let need = measure.consumed();
    assert!(need.work > 0 && need.storage > 0);
    let mut exact = NativeJoinBudget::new(need.work, need.storage);
    let actual = read
        .decision(&admitted, row.checked_node(), &mut exact)
        .unwrap_or_else(|_| panic!("exact bytes budget admits"));
    assert_eq!(actual, expected);
    assert_eq!(exact.consumed(), need);
    for (work, storage, kind, code) in [
        (
            need.work - 1,
            need.storage,
            NativeJoinChargeKind::Work,
            Code::WorkLimit,
        ),
        (
            need.work,
            need.storage - 1,
            NativeJoinChargeKind::Storage,
            Code::StorageLimit,
        ),
    ] {
        let mut producer = NativeJoinBudget::new(work, storage);
        let failure = emission
            .admitted_equality(
                &admitted,
                row.checked_node(),
                row.occurrence(),
                &mut producer,
            )
            .expect_err("one short refuses producer");
        assert_eq!(failure.code, code);
        assert_eq!(producer.first_unpaid().expect("denial").kind, kind);
        let mut bytes = NativeJoinBudget::new(work, storage);
        let Err(EqualityDecisionFailure::Exhausted(failure)) =
            read.decision(&admitted, row.checked_node(), &mut bytes)
        else {
            panic!("one short is typed native exhaustion")
        };
        assert_eq!(failure.code, code);
        assert_eq!(bytes.consumed(), producer.consumed());
        assert_eq!(bytes.first_unpaid(), producer.first_unpaid());
    }
    let mut shared = NativeJoinBudget::new(need.work * 2 - 1, need.storage * 2);
    emission
        .admitted_equality(&admitted, row.checked_node(), row.occurrence(), &mut shared)
        .expect("first read");
    let first = shared.consumed();
    let error = emission
        .admitted_equality(&admitted, row.checked_node(), row.occurrence(), &mut shared)
        .expect_err("second read uses remaining allowance");
    assert_eq!(error.code, Code::WorkLimit);
    assert!(shared.consumed().work >= first.work);
    assert_eq!(
        shared.first_unpaid().expect("unpaid work").kind,
        NativeJoinChargeKind::Work
    );
}

#[trace("TC-1331", "FR-531-AC-28")]
#[test]
fn bare_admission_retains_equality_and_reports_its_denial_after_projection() {
    let emission = emission();
    let Read::Verified { admitted, .. } = read_back(&emission) else {
        panic!("native reader admits")
    };
    let identities = emission
        .identity_rows(&admitted, &mut NativeJoinBudget::new(1_000_000, 1_000_000))
        .expect("original identities");
    let witness = emission.equality_witness(&admitted);
    let bytes = witness.to_bytes(&admitted).expect("owner bytes");
    let equality = ReadEqualityRows::read(&bytes, &admitted).expect("read owner bytes");
    let supply = BareSupply {
        identities: &identities,
        equality: Some(&equality),
    };
    let mut measure = NativeJoinBudget::new(1_000_000, 1_000_000);
    let bare = BarePackage::new(&admitted)
        .with_supply(supply, &mut measure)
        .expect("full admission");
    let full = measure.consumed();
    let declarations = declaration_cost(&admitted, supply);
    assert!(declarations.work > 0 && declarations.storage > 0);
    // Equality is admitted before the declarations, so its denials are the
    // budgets short of what precedes them.
    let need = quire_contract_model::NativeJoinCharge {
        work: full.work - declarations.work,
        storage: full.storage - declarations.storage,
    };
    let [row] = equality.applications() else {
        panic!("one original row")
    };
    let expected = bare
        .equality_decisions()
        .get(&row.application)
        .expect("retained equality")
        .as_ref()
        .expect("decision")
        .clone();
    assert_eq!(
        expected.schedule(),
        quire_semantic_value::declaration::EqualitySchedule::Text
    );
    bare.world().expect("world uses admitted projections");
    assert_eq!(measure.consumed(), full);
    let mut exact = NativeJoinBudget::new(full.work, full.storage);
    let exact_bare = BarePackage::new(&admitted)
        .with_supply(supply, &mut exact)
        .expect("exact complete pass");
    assert_eq!(
        exact_bare
            .equality_decisions()
            .get(&row.application)
            .expect("retained")
            .as_ref()
            .expect("decision"),
        &expected
    );
    assert_eq!(exact.consumed(), full);
    // One short of the whole pass is denied by the declaration projection.
    for (work, storage) in [(full.work - 1, full.storage), (full.work, full.storage - 1)] {
        let error = BarePackage::new(&admitted)
            .with_supply(supply, &mut NativeJoinBudget::new(work, storage))
            .expect_err("one short in the declarations");
        assert!(matches!(error.native(), NativeJoinFailure::Projection(_)));
    }
    for (work, storage, kind, code) in [
        (
            need.work - 1,
            full.storage,
            NativeJoinChargeKind::Work,
            Code::WorkLimit,
        ),
        (
            full.work,
            need.storage - 1,
            NativeJoinChargeKind::Storage,
            Code::StorageLimit,
        ),
    ] {
        let mut budget = NativeJoinBudget::new(work, storage);
        let error = BarePackage::new(&admitted)
            .with_supply(supply, &mut budget)
            .expect_err("one short in equality");
        let NativeJoinFailure::Equality(native) = error.native() else {
            panic!("actual equality cause after identity/type prefix")
        };
        assert_eq!(native.code, code);
        let diagnostic = error
            .limit()
            .expect("bare entry retains the stage diagnostic");
        let (bound, consumed) = if kind == NativeJoinChargeKind::Work {
            (work, budget.consumed().work)
        } else {
            (storage, budget.consumed().storage)
        };
        assert_eq!(diagnostic.configured_bound(), bound);
        assert_eq!(
            diagnostic.actual(),
            u128::from(consumed) + budget.first_unpaid().expect("unpaid").amount
        );
        assert_eq!(native.consumed_work, budget.consumed().work);
        assert!(budget.consumed().work > 0 && budget.consumed().storage > 0);
        assert_eq!(budget.first_unpaid().expect("first unpaid").kind, kind);
    }
}

#[trace("TC-1331", "FR-531-AC-28")]
#[test]
fn source_evaluation_view_preserves_the_equality_denial_and_exact_stage_limit() {
    use quire_semantic_value::catalog::CatalogCoded;
    let package = CheckedPackage::link(converted_equality::checked_graph(
        "function same using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure { a = b }",
    ));
    let emission = emit(&package);
    let Read::Verified { admitted, .. } = read_back(&emission) else {
        panic!("owner admits")
    };
    let mut measure = NativeJoinBudget::new(1_000_000, 1_000_000);
    let identities = emission
        .identity_rows(&admitted, &mut measure)
        .expect("producer row join");
    let equality = emission.equality_witness(&admitted);
    let supply = BareSupply {
        identities: &identities,
        equality: Some(&equality),
    };
    BarePackage::new(&admitted)
        .with_supply(supply, &mut measure)
        .expect("same complete native pass");
    // View creation owns the pass up to equality; the declarations
    // `with_supply` ends with are the bare route's alone.
    let declarations = declaration_cost(&admitted, supply);
    let need = quire_contract_model::NativeJoinCharge {
        work: measure.consumed().work - declarations.work,
        storage: measure.consumed().storage - declarations.storage,
    };
    let domains = BTreeMap::new();
    let limits = |work, storage| {
        V2ReadLimits::fixture()
            .with_work(work)
            .with_storage(storage)
    };
    let read = crate::read_evaluation_emission(
        &package,
        &emission,
        library(),
        &domains,
        limits(need.work, need.storage),
        crate::EmitLimits::fixture(),
    )
    .expect("public source reader admits");
    read.evaluation_view()
        .expect("the exact native allowance admits the public view");
    for (work, storage, kind, code, setting) in [
        (
            need.work - 1,
            need.storage,
            NativeJoinChargeKind::Work,
            Code::WorkLimit,
            "i2.work_units",
        ),
        (
            need.work,
            need.storage - 1,
            NativeJoinChargeKind::Storage,
            Code::StorageLimit,
            "i2.native_join_storage",
        ),
    ] {
        // The same original row-production prefix that evaluation_view owns.
        let mut expected_budget = NativeJoinBudget::new(work, storage);
        emission
            .identity_rows(&admitted, &mut expected_budget)
            .expect("prefix admits");
        let error = BarePackage::new(&admitted)
            .with_supply(supply, &mut expected_budget)
            .expect_err("equality is the denied tail");
        let NativeJoinFailure::Equality(expected) = error.native() else {
            panic!("owner equality denial")
        };
        assert_eq!(expected.code, code);
        let unpaid = expected_budget
            .first_unpaid()
            .expect("original first unpaid charge");
        assert_eq!(unpaid.kind, kind);
        let bound = if kind == NativeJoinChargeKind::Work {
            work
        } else {
            storage
        };
        let consumed = if kind == NativeJoinChargeKind::Work {
            expected_budget.consumed().work
        } else {
            expected_budget.consumed().storage
        };
        let short = crate::read_evaluation_emission(
            &package,
            &emission,
            library(),
            &domains,
            limits(work, storage),
            crate::EmitLimits::fixture(),
        )
        .expect("I2 read still admits");
        let Err(crate::ViewFailure::Exhausted(denial)) = short.evaluation_view() else {
            panic!("public view must expose exhaustion")
        };
        assert_eq!(denial.native, NativeJoinFailure::Equality(expected.clone()));
        assert_eq!(denial.limit.configured_bound(), bound);
        assert_eq!(denial.limit.setting().name(), setting);
        assert_eq!(denial.limit.actual(), u128::from(consumed) + unpaid.amount);
        assert_eq!(denial.catalog_code().code(), "stage_limit_exceeded");
    }
}

/// What the selected-object declaration projection, the last charge of
/// `with_supply` (FR-533 "Observation admission"), costs on its own.
fn declaration_cost(
    admitted: &quire_contract_model::CheckedPackageV2,
    supply: BareSupply<'_>,
) -> quire_contract_model::NativeJoinCharge {
    let bare = BarePackage::new(admitted)
        .with_supply(supply, &mut NativeJoinBudget::new(1_000_000, 1_000_000))
        .expect("full admission");
    let mut budget = NativeJoinBudget::new(1_000_000, 1_000_000);
    admitted
        .project_selected_type_declarations(
            bare.identities().expect("joined identities"),
            &[],
            supply.equality.map_or(&[], |rows| rows.source_units()),
            &mut budget,
        )
        .expect("the declarations project");
    budget.consumed()
}
