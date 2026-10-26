// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-182 V-3 (TC-607): `settle_hyper_smt` over an EN-3 k-induction outcome.

use ix_trace_rs::trace;
use qsl_replay::{
    settle_hyper_smt, Certification, HyperProductIdentity, HyperQueryRequest, HyperSettleRefusal,
    HyperSmtOutcome, HyperSmtResult, InconclusiveCause, ModelCheckLimits, ProductMember,
    ProofBasis, SettlementBasis, SmtEncodingLimits, TerminalValue,
};
use quire_exact::Meter;
use quire_semantic_value::catalog::InternalFault;

use crate::vault_world::{non_interference, World, EQUAL_INPUTS, UNLIMITED};

fn instance(key: &str) -> Vec<(String, String)> {
    vec![("v".to_owned(), key.to_owned())]
}

/// Settle `result` for the item at instance `item_key`, the outcome carrying
/// the identity of instance `outcome_key`.
fn settle(
    world: &World,
    item_key: &str,
    outcome_key: &str,
    result: HyperSmtResult,
) -> Result<qsl_replay::HyperRecord, HyperSettleRefusal> {
    settle_with(world, item_key, Some(outcome_key), result)
}

/// As [`settle`]; `None` hands over an outcome whose `product` is `Err`.
fn settle_with(
    world: &World,
    item_key: &str,
    outcome_key: Option<&str>,
    result: HyperSmtResult,
) -> Result<qsl_replay::HyperRecord, HyperSettleRefusal> {
    let subjects = [world.subject()];
    let mut meter = Meter::new(UNLIMITED);
    let product = match outcome_key {
        Some(key) => {
            let named = instance(key);
            Ok(HyperProductIdentity::of(&HyperQueryRequest {
                clause: world.clause(),
                subjects: &subjects,
                instance: &named,
                meter: &mut meter,
                encoding: SmtEncodingLimits::default(),
            })
            .expect("the outcome's identity"))
        }
        None => Err(InternalFault::new("smt-encoding", "the identity faulted")),
    };
    let key = instance(item_key);
    let item = HyperQueryRequest {
        clause: world.clause(),
        subjects: &subjects,
        instance: &key,
        meter: &mut meter,
        encoding: SmtEncodingLimits::default(),
    };
    settle_hyper_smt(
        &item,
        &HyperSmtOutcome { result, product },
        &ModelCheckLimits::default(),
        &[],
    )
}

/// FR-182-AC-7: an `Inductive{depth: 1}` outcome with an empty `μ_U` settles
/// `proved`, `decisive-witness`, `Uncertified`, method `k-induction`.
#[trace("TC-607", "FR-182-AC-7")]
#[test]
fn an_inductive_outcome_with_an_empty_match_settles_proved_uncertified() {
    let world = World::new(&["v", "w"], false, &non_interference("", ""));
    let record = settle(&world, "v", "v", HyperSmtResult::Inductive { depth: 1 })
        .expect("the outcome answers the item");
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::Inductive { depth: 1 },
            certification: Certification::Uncertified,
        }
    );
    assert_eq!(record.basis, SettlementBasis::DecisiveWitness);
    assert_eq!(record.method, "k-induction");
}

/// An `Inductive` outcome whose clause has a non-empty `μ_U` and no accepted
/// witness settles `inconclusive`, `VacuousMatch`, never `proved`.
#[trace("TC-607", "FR-182-AC-1")]
#[test]
fn an_inductive_outcome_with_a_match_and_no_witness_is_vacuous() {
    let world = World::new(&["v", "w"], false, &non_interference(EQUAL_INPUTS, ""));
    let record = settle(&world, "v", "v", HyperSmtResult::Inductive { depth: 1 })
        .expect("the outcome answers the item");
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::VacuousMatch)
    );
}

/// An outcome carrying another instance's identity, or no identity, is no
/// evidence for the item and is refused.
#[trace("TC-607", "FR-182-AC-11")]
#[test]
fn an_outcome_for_another_product_is_refused() {
    let world = World::new(&["v", "w"], false, &non_interference("", ""));
    let result = HyperSmtResult::Inductive { depth: 1 };
    assert_eq!(
        settle(&world, "v", "w", result).unwrap_err(),
        HyperSettleRefusal::ProductMismatch {
            member: ProductMember::Instance
        }
    );
    assert_eq!(
        settle_with(&world, "v", None, result).unwrap_err(),
        HyperSettleRefusal::ProductMismatch {
            member: ProductMember::Clause
        }
    );
}

/// FR-182-AC-10: a subject with an empty initial list settles
/// `inconclusive`, `NoInitialState`, never `proved`.
#[trace("TC-607", "FR-182-AC-10")]
#[test]
fn an_empty_initial_list_settles_no_initial_state() {
    let world = World::new(&["v"], false, &non_interference("", "")).without_initial_states();
    let record = settle(&world, "v", "v", HyperSmtResult::Inductive { depth: 1 })
        .expect("the outcome answers the item");
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::NoInitialState)
    );
}

/// `InductionNotClosed{depth}` settles `inconclusive`, never `proved`.
#[trace("TC-607", "FR-182-AC-1")]
#[test]
fn an_unclosed_induction_is_inconclusive() {
    let world = World::new(&["v"], false, &non_interference("", ""));
    let record = settle(
        &world,
        "v",
        "v",
        HyperSmtResult::InductionNotClosed { depth: 1 },
    )
    .expect("the outcome answers the item");
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::InductionNotClosed { depth: 1 })
    );
}
