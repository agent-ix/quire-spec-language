// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-144 rows 2, 3, 4g and 9 (FR-052-AC-9 to AC-12), on the evidence route
//! (`qsl_eval::temporal::evidence`): the checked-package route owns temporal
//! truth, settlement and support; the caller supplies evidence, never a
//! verdict, and a correction chains to its exact direct predecessor.
//!
//! Fixture **T-E**: `Reach` = `eventually[0,2] holds(level = 2)` activated
//! `on origin`, one execution origin `O1` and a `trigger` naming it, one
//! correspondence held fixed across every request of a row. Trace **A** reads
//! `holds(level = 2)` false, false, true at positions 0, 1, 2. The clause
//! route's rows live in `qsl-replay`'s `checked_route_owner`.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{
    evaluate_temporal_evidence, ActivationState, EvidenceCounter, EvidenceRequest, EvidenceResult,
    FaultKind, InstanceState, NonValueKind, Relation, Truth,
};
use qsl_eval::temporal::Valuation;
use quire_eval::CallFailure;

use crate::checked_route_trigger::{refused, refused_on};
use crate::temporal_activation::{
    capture_named, delivery, env, id, node, origin, position, request, unlimited, Env, EACH,
};

const O1: &[u8] = b"O1";

fn reach() -> (Env, u64) {
    let env = env("on origin", "eventually[0, 2] holds(p)");
    let p = env.atom("p");
    (env, p)
}

/// Trace A, or the same trace with `level = 2` never reached.
fn over(env: &Env, p: u64, reached: Valuation) -> EvidenceRequest {
    let mut asked = request(
        env,
        vec![
            position(0, &[(p, Valuation::False)]),
            position(1, &[(p, Valuation::False)]),
            position(2, &[(p, reached)]),
        ],
    );
    asked.origins = vec![origin("O1", "payload")];
    asked.trigger = Some(id("O1"));
    asked
}

fn run(env: &Env, asked: &EvidenceRequest) -> Result<EvidenceResult, CallFailure> {
    evaluate_temporal_evidence(&env.subject(&[]), asked, &mut unlimited())
        .map(|issued| issued.result().clone())
}

fn correcting(env: &Env, p: u64, reached: Valuation, previous: &EvidenceResult) -> EvidenceRequest {
    let mut asked = over(env, p, reached);
    asked.relation = Relation::Superseding(previous.result_identity.clone());
    asked.ledger = Some(previous.ledger.clone());
    asked
}

/// Row 2: the request has no member for truth, settlement or support
/// (exhaustive destructuring: a new such member breaks the build), and a
/// superseding request's truth, settlement and support come from its own
/// evidence, not from the result it corrects, which stays unchanged.
#[trace("TC-144", "FR-052-AC-9")]
#[test]
fn a_correction_derives_truth_from_its_own_evidence_and_leaves_the_corrected_result() {
    let (env, p) = reach();
    let EvidenceRequest {
        trigger: _,
        correspondence: _,
        positions: _,
        anchor: _,
        deliveries: _,
        origins: _,
        trigger_evidence: _,
        trigger_scope: _,
        decision_progress: _,
        decision_closure: _,
        surrounding_progress: _,
        surrounding_closure: _,
        execution: _,
        completeness: _,
        authoritative_origin: _,
        evicted: _,
        cover: _,
        progress_ledger: _,
        ledger: _,
        relation: _,
        control: _,
        binding: _,
        occurrence: _,
        limits: _,
    } = over(&env, p, Valuation::True);

    let r1 = run(&env, &over(&env, p, Valuation::True)).expect("a result");
    // The leaf at position 0 is false; the formula is true.
    assert_eq!(r1.truth, Some(Truth::True));

    let r2 = run(&env, &correcting(&env, p, Valuation::False, &r1)).expect("a result");
    assert_eq!(r2.truth, Some(Truth::False));
    assert_eq!(r2.lineage, 1);
    assert_eq!(
        r2.relation,
        Relation::Superseding(r1.result_identity.clone())
    );
    // Derived from its own evidence: the same evidence as an original request.
    let fresh = run(&env, &over(&env, p, Valuation::False)).expect("a result");
    assert_eq!(
        (r2.truth, r2.settlement, r2.support.clone()),
        (fresh.truth, fresh.settlement, fresh.support.clone())
    );
    assert_eq!(
        r2.settlement,
        Some(qsl_eval::temporal::evidence::Settlement::ClosedScope)
    );
    // The corrected result is as it was: running its request again gives
    // the same result, and the new ledger still holds `r2` as the latest of
    // the chain, with `r1`'s ledger an unchanged earlier value.
    let again = run(&env, &over(&env, p, Valuation::True)).expect("a result");
    assert_eq!(again.result_identity, r1.result_identity);
    assert_eq!((again.truth, again.settlement), (r1.truth, r1.settlement));
    assert_eq!(
        r1.ledger
            .result(&env.obligation(), Some(&id("O1")))
            .map(|entry| entry.result_identity().clone()),
        Some(r1.result_identity.clone())
    );
    let held = r2
        .ledger
        .result(&env.obligation(), Some(&id("O1")))
        .expect("the latest result");
    assert_eq!(held.result_identity(), &r2.result_identity);
}

/// Row 3: the `Due` activation of `Reach` through the FR-449 map gives row 1's
/// truth, and the result keeps the binding, the trigger identity, the
/// instances with their states, the four progress axes, the completeness
/// record and the truth as separate members.
#[trace("TC-144", "FR-052-AC-9", "FR-052-AC-10")]
#[test]
fn the_due_activation_reports_each_fact_as_its_own_member() {
    let (base, p) = reach();
    let env = base.with_row("Due");
    let direct = run(&env, &over(&env, p, Valuation::True)).expect("a result");
    let mut asked = over(&env, p, Valuation::True);
    asked.control = Some(node("Due"));
    let due = run(&env, &asked).expect("a result");
    assert_eq!(due.truth, Some(Truth::True));
    assert_eq!(due.truth, direct.truth);
    assert_eq!(due.settlement, direct.settlement);
    assert_eq!(&*due.authenticated.package, "recipe");
    assert_eq!(due.authenticated.declaration, node("Due"));
    assert_eq!(&*due.authenticated.clock_name, "stamp-clock");
    assert_eq!(due.trigger, Some(id("O1")));
    assert_eq!(due.instances.len(), 1);
    assert_eq!(due.instances[0].identity.as_bytes(), O1);
    assert_eq!(due.instances[0].state, InstanceState::Active);
    assert_eq!(due.activation, ActivationState::Active);
    assert_eq!(due.decision_progress, Some(100));
    assert_eq!(due.surrounding_progress, Some(100));
    assert!(due.decision_closure.is_some() && due.surrounding_closure.is_some());
    assert!(due.completeness.is_some());
}

/// Row 4g: a protocol clause with no row in the activation map, then a row
/// naming another temporal clause, are refused with an empty charge log; a
/// delivery one capture fact short is an `Unavailable` instance naming the
/// capture, with no truth and no position evaluated.
#[trace("TC-144", "FR-052-AC-10", "FR-052-AC-11")]
#[test]
fn a_missing_map_row_or_capture_is_a_typed_outcome_with_no_truth() {
    let (base, p) = reach();
    let env = base.with_row("Due");
    let mut asked = over(&env, p, Valuation::True);
    asked.control = Some(node("Nope"));
    refused(&env, &asked, "activation-mapping-missing", "control", &[]);
    asked.control = Some(node("Due"));
    assert!(run(&env, &asked).is_ok());
    let mut subject = env.subject(&[]);
    let elsewhere = vec![qsl_eval::temporal::evidence::ActivationRow {
        control: node("Due"),
        clause: node("Else"),
    }];
    subject.activation_map = &elsewhere;
    refused_on(
        &subject,
        &asked,
        "activation-mapping-mismatch",
        "control",
        &[],
    );

    // The capture-bearing clause of fixture R: one capture fact fewer.
    let each = self::env(
        EACH,
        "capture amount: Int[0, 9] = q.level; eventually[0, 2] holds(p)",
    );
    let ep = each.atom("p");
    let mut short = request(
        &each,
        vec![position(0, &[(ep, Valuation::True)]), position(1, &[])],
    );
    short.deliveries = vec![delivery("t1", "r", "p", &[])];
    short.trigger = Some(id("t1"));
    let result = run(&each, &short).expect("a result");
    assert_eq!(result.activation, ActivationState::Unavailable);
    assert_eq!(result.truth, None);
    assert_eq!(result.settlement, None);
    let non_value = result.non_value.as_ref().expect("a non-value");
    assert_eq!(
        (non_value.kind, non_value.dimension.as_deref()),
        (NonValueKind::Missing, Some("amount"))
    );
    let InstanceState::Unavailable(fault) = &result.instances[0].state else {
        panic!("the instance is unavailable");
    };
    assert_eq!(
        (fault.subject.clone(), fault.kind),
        (capture_named("amount"), FaultKind::Missing)
    );
    assert_eq!(result.usage.get(EvidenceCounter::Positions), 0);
}

/// Row 9: the correction relation of fixture T-E. A correction with changed
/// evidence chains at lineage 1; naming `r1` again, another clause's `r1`,
/// another trigger's identity, an unchanged evidence, or no ledger is
/// refused with an empty charge log and the held results stay as they were.
#[trace("TC-144", "FR-052-AC-12", "FR-053-AC-9")]
#[test]
fn only_a_direct_changed_correction_of_the_latest_result_is_accepted() {
    let (env, p) = reach();
    let r1 = run(&env, &over(&env, p, Valuation::True)).expect("a result");
    let r2 = run(&env, &correcting(&env, p, Valuation::False, &r1)).expect("a result");
    assert_eq!(r2.lineage, 1);
    assert_eq!(
        r2.relation,
        Relation::Superseding(r1.result_identity.clone())
    );

    let cause = |asked: &EvidenceRequest, cause: &str, member: &str| {
        refused(&env, asked, cause, member, &[]);
    };
    // `r1` again, once `r2` is held.
    let mut again = correcting(&env, p, Valuation::Missing, &r1);
    again.ledger = Some(r2.ledger.clone());
    cause(&again, "predecessor-not-latest", "relation");
    // Another clause of the package.
    let other = reach().0.moved("recipe", "Other");
    let mut elsewhere = correcting(&other, p, Valuation::False, &r1);
    elsewhere.ledger = Some(r1.ledger.clone());
    refused(&other, &elsewhere, "predecessor-unknown", "relation", &[]);
    // Another trigger's identity: the ledger holds nothing under it.
    let mut foreign = correcting(&env, p, Valuation::False, &r1);
    foreign.origins = vec![origin("O2", "payload")];
    foreign.trigger = Some(id("O2"));
    cause(&foreign, "predecessor-unknown", "relation");
    // `r2`'s own evidence, unchanged.
    let mut same = correcting(&env, p, Valuation::False, &r2);
    same.ledger = Some(r2.ledger.clone());
    cause(&same, "unchanged-input", "relation");
    // No ledger.
    let mut bare = correcting(&env, p, Valuation::Missing, &r2);
    bare.ledger = None;
    cause(&bare, "no-ledger", "ledger");

    // The refusals changed no held result: `r2` is still the latest, so a
    // correction of `r2` is lineage 2 and `r1`, `r2` read as they did.
    let r3 = run(&env, &correcting(&env, p, Valuation::Missing, &r2)).expect("a result");
    assert_eq!(r3.lineage, 2);
    assert_eq!(
        r3.relation,
        Relation::Superseding(r2.result_identity.clone())
    );
    assert_eq!((r1.lineage, r1.truth), (0, Some(Truth::True)));
    assert_eq!((r2.lineage, r2.truth), (1, Some(Truth::False)));
}
