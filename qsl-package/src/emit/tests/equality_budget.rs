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
    let need = measure.consumed();
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
    assert_eq!(measure.consumed(), need);
    let mut exact = NativeJoinBudget::new(need.work, need.storage);
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
        let mut budget = NativeJoinBudget::new(work, storage);
        let error = BarePackage::new(&admitted)
            .with_supply(supply, &mut budget)
            .expect_err("one short in equality");
        let NativeJoinFailure::Equality(native) = error else {
            panic!("actual equality cause after identity/type prefix")
        };
        assert_eq!(native.code, code);
        assert_eq!(native.consumed_work, budget.consumed().work);
        assert!(budget.consumed().work > 0 && budget.consumed().storage > 0);
        assert_eq!(budget.first_unpaid().expect("first unpaid").kind, kind);
    }
}
