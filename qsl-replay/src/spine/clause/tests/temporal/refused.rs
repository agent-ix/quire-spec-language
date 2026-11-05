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

/// FR-327-AC-6, FR-330: the kernel `Refusal::CheckedInvariant` as a `holds`
/// condition's S6a result at position 1 is kept as a refused condition at
/// position 1 (`a_checked_invariant_refusal_is_a_refused_condition_at_its_position`
/// in `qsl-eval`) and the run reports it as FR-100 maps it: the internal
/// failure, stage `evaluate`, category `internal-failure`, exit 30, no truth
/// value, and not a refusal.
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
    let disposition = crate::spine::clause::temporal::refused_disposition(
        evaluation,
        compiled.package.graph(),
        std::slice::from_ref(&compiled.source),
    );
    let ClauseDisposition::EvaluateFault(fault) = &disposition else {
        panic!("expected the internal failure, got {disposition:?}");
    };
    assert_eq!(fault.invariant(), "checked-program-invariant");
    assert_eq!(disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(disposition.category(), Category::InternalFailure);
    assert_eq!(disposition.category().exit_code(), 30);
    assert_eq!(disposition.truth(), None);
}
