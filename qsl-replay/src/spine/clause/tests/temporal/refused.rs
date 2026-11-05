// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1160 step 4 (FR-327-AC-6): a `holds` atom whose S6a result is a
//! kernel refusal stops the run as a refused condition under every profile
//! and trace shape, from source through `run_clause`.
//!
//! The atom counts the elements of `c.value` greater than zero into
//! `type Zero = Int[0, 0]`: `c.value` 0 counts 0 and is true, `c.value` 1
//! counts 1, which the kernel refuses `integer_out_of_domain`/
//! `outside-domain` with `expected` `Int[0, 0]`.

use super::*;
use crate::spine::{CallOutcome, CallRefusal};

/// The refusal record the clause run reported, asserted to be the kernel's
/// `integer_out_of_domain` record for `Int[0, 0]` with the atom's locus, and
/// the position `at` the refused condition was read at.
fn assert_refused_condition(report: &ClauseRunReport, at: u64) {
    assert_eq!(
        report.refused_at,
        Some(at),
        "the refused condition's position"
    );
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
    let ClauseDisposition::Evaluate(CallOutcome::Refused(CallRefusal::Record {
        code,
        fields,
        locus,
        ..
    })) = &report.disposition
    else {
        panic!("expected a refused condition, got {:?}", report.disposition);
    };
    assert_eq!(
        (code.code(), code.cause()),
        ("integer_out_of_domain", "outside-domain")
    );
    assert_eq!(
        fields.get("expected").map(String::as_str),
        Some("Int[0, 0]")
    );
    assert!(locus.is_some(), "the atom's locus");
    assert_eq!(report.disposition.category(), Category::Refusal);
    assert_eq!(report.disposition.truth(), None);
    assert!(report.counterexample.is_none());
}

/// FR-327-AC-6, the bounded profile over the finite trace `c.value` 0, 1,
/// 2: the atom is true at the first position and its S6a result at the
/// second is the kernel's `Outcome::Refused(IntegerOutOfDomain)`. The run
/// stops there and reports the refused condition: the record
/// `integer_out_of_domain`/`outside-domain`, its `expected` field and the
/// atom's locus, category refusal, no truth value, at position 1. The same
/// clause over values that never leave the domain holds, so the refusal is
/// the atom's, and a refusal at position 0 is reported at position 0.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn a_refused_holds_condition_stops_a_bounded_run_with_its_record() {
    assert_refused_condition(
        &report(finite(Unit::Refusing, "RefusedAfterTrue", &[0, 1, 2])),
        1,
    );
    assert_refused_condition(
        &report(finite(Unit::Refusing, "RefusedAfterTrue", &[1, 0, 0])),
        0,
    );
    let within = report(finite(Unit::Refusing, "RefusedAfterTrue", &[0, 0, 0]));
    assert_eq!(outcome(&within), &TemporalVerdict::Holds);
}

/// FR-327-AC-6, the infinite-trace profile: the same atom over the prefix
/// `c.value` 0, 1, 2 (FR-328) and over the lasso with an empty prefix and
/// loop 0, 1 (FR-329) returns the refused condition.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn a_refused_holds_condition_stops_an_infinite_trace_run_over_a_prefix_and_a_lasso() {
    assert_refused_condition(
        &report(finite(Unit::RefusingInfinite, "RefusedWhole", &[0, 1, 2])),
        1,
    );
    assert_refused_condition(
        &report(lasso(Unit::RefusingInfinite, "RefusedWhole", 0, &[0, 1])),
        1,
    );
    let within = report(lasso(Unit::RefusingInfinite, "RefusedWhole", 0, &[0, 0]));
    assert_eq!(outcome(&within), &TemporalVerdict::Holds);
}

/// FR-327-AC-6, evaluation order: `always[0,2]` over `c.value = 1 and
/// <refused at 1>` with `c.value` 0, 1, 2 is false at position 0 and refused
/// at position 1. The run stops at the first decisive position, 0, and
/// reports `Completed(false)` there, category violation, with no refused
/// condition: the evaluator never needed position 1's result. Refusing at
/// position 1 alone, with the atom true at 0 (`RefusedAfterTrue`), is the
/// refused condition.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn a_false_activation_before_a_refused_condition_is_the_result() {
    let report = report(finite(Unit::Refusing, "FalseThenRefused", &[0, 1, 2]));
    assert_eq!(outcome(&report), &TemporalVerdict::Violated { at: 0 });
    assert_eq!(report.category(), Category::Violation);
    assert_eq!(report.refused_at, None);
    assert!(report.counterexample.is_some());
}

/// FR-327-AC-6, FR-330: the evaluator's result for a `holds` condition whose
/// S6a result is the kernel `Refusal::CheckedInvariant` at position 1 is the
/// refused condition at position 1 (`TemporalRefusal::ConditionRefused`, with
/// the refusal kept in `ObservedRun::stopped`), and the run reports it as
/// FR-100 maps it: the internal failure, stage `evaluate`, category
/// `internal-failure`, exit 30, no truth value, not a refusal, carrying the
/// refused condition's position. The observed run is built as
/// `evaluate_observed` returns it, as the kernel reserves
/// `CheckedInvariant` for a broken evaluator invariant and no checked source
/// reaches it.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn a_checked_invariant_refused_condition_reports_the_internal_failure() {
    let compiled = compiled();
    let evaluation = quire_eval::Evaluation {
        outcome: quire_eval::FamilyOutcome::Evaluated(quire_exact::Outcome::Refused(
            quire_exact::Refusal::CheckedInvariant {
                cause: quire_exact::CheckedInvariantCause::EqualityOperandSourceNotAdmitted,
            },
        )),
        location: None,
        losses: Vec::new(),
    };
    let at = qsl_eval::temporal::TemporalPosition::new(1);
    let observed = qsl_eval::temporal::ObservedRun {
        result: Err(qsl_eval::temporal::TemporalRefusal::ConditionRefused { at }),
        stopped: Some(qsl_eval::temporal::HoldsStop::Refused { at, evaluation }),
        undefined: None,
    };
    let (disposition, refused_at) = crate::spine::clause::temporal::observed_disposition(
        observed.stopped,
        observed.result,
        compiled.package.graph(),
        std::slice::from_ref(&compiled.source),
    );
    assert_eq!(refused_at, Some(1));
    let ClauseDisposition::EvaluateFault(fault) = &disposition else {
        panic!("expected the internal failure, got {disposition:?}");
    };
    assert_eq!(fault.invariant(), "checked-program-invariant");
    assert_eq!(disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(disposition.category(), Category::InternalFailure);
    assert_eq!(disposition.category().exit_code(), 30);
    assert_eq!(disposition.truth(), None);
}

/// FR-501, FR-327 "Refused condition": under the finite-window profile an
/// `on each` activation that is definitely false at stamp 0 is returned
/// before a condition refused at the later stamp 10, in evaluation order,
/// as under every other profile.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn a_false_activation_before_a_refused_condition_is_the_result_under_the_finite_window_profile() {
    let report = report(timestamped_with(
        Unit::TimestampedRefusing,
        &[0, 10],
        &[0, 1],
        12,
        Some(12),
    ));
    assert_eq!(outcome(&report), &TemporalVerdict::Violated { at: 0 });
    assert_eq!(report.refused_at, None);
}

/// FR-327 "Refused condition": an atom undefined at position 0 is returned
/// before the condition refused at position 1. The report is the undefined
/// evaluation, category violation, and carries no refused condition.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn an_undefined_atom_before_a_refused_condition_is_the_result_and_no_refusal_is_reported() {
    let report = report(finite(Unit::Refusing, "UndefinedThenRefused", &[2, 1, 0]));
    assert!(
        matches!(outcome(&report), TemporalVerdict::Undefined { at: 0, .. }),
        "{:?}",
        report.disposition
    );
    assert_eq!(report.category(), Category::Violation);
    assert_eq!(report.refused_at, None);
    // Two atoms at one position: the first undefined, the second refused. The
    // undefined atom is earlier in clause-node order, so it is the result and
    // the refusal the second atom read is not reported.
    let beside = self::report(finite(Unit::Refusing, "UndefinedBesideRefused", &[2, 0, 0]));
    assert!(
        matches!(outcome(&beside), TemporalVerdict::Undefined { at: 0, .. }),
        "{:?}",
        beside.disposition
    );
    assert_eq!(beside.refused_at, None);
    // The refusal is the clause's own: with the undefined position first
    // read as a value, the refused one is returned.
    let refused = self::report(finite(Unit::Refusing, "UndefinedThenRefused", &[0, 1, 0]));
    assert_eq!(refused.refused_at, Some(1));
}

/// FR-501, FR-327: under the finite-window profile an atom undefined at
/// stamp 0 is returned before a condition refused at stamp 10, and the
/// refused condition is reported when the undefined atom is a value.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn an_undefined_atom_before_a_refused_condition_is_the_result_under_the_finite_window_profile() {
    let run = |values: &[i64]| {
        report(timestamped_with(
            Unit::TimestampedUndefinedThenRefused,
            &[0, 10],
            values,
            12,
            Some(12),
        ))
    };
    let undefined = run(&[2, 1]);
    assert!(
        matches!(
            outcome(&undefined),
            TemporalVerdict::Undefined { at: 0, .. }
        ),
        "{:?}",
        undefined.disposition
    );
    assert_eq!(undefined.refused_at, None);
    assert_eq!(run(&[0, 1]).refused_at, Some(1));
}
