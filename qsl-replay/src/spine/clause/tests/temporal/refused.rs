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
/// `integer_out_of_domain` record for `Int[0, 0]` with the atom's locus.
fn assert_refused_condition(report: &ClauseRunReport) {
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
/// atom's locus, category refusal, no truth value. The same clause over
/// values that never leave the domain holds, so the refusal is the atom's.
///
/// Not asserted here: the refused condition's position (the report carries
/// none) and the `FalseThenRefused` ordering (see the PR description).
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn a_refused_holds_condition_stops_a_bounded_run_with_its_record() {
    assert_refused_condition(&report(finite(
        Unit::Refusing,
        "RefusedAfterTrue",
        &[0, 1, 2],
    )));
    let within = report(finite(Unit::Refusing, "RefusedAfterTrue", &[0, 0, 0]));
    assert_eq!(outcome(&within), &TemporalVerdict::Holds);
}

/// FR-327-AC-6, the infinite-trace profile: the same atom over the prefix
/// `c.value` 0, 1, 2 (FR-328) and over the lasso with an empty prefix and
/// loop 0, 1 (FR-329) returns the refused condition.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn a_refused_holds_condition_stops_an_infinite_trace_run_over_a_prefix_and_a_lasso() {
    assert_refused_condition(&report(finite(
        Unit::RefusingInfinite,
        "RefusedWhole",
        &[0, 1, 2],
    )));
    assert_refused_condition(&report(lasso(
        Unit::RefusingInfinite,
        "RefusedWhole",
        0,
        &[0, 1],
    )));
    let within = report(lasso(Unit::RefusingInfinite, "RefusedWhole", 0, &[0, 0]));
    assert_eq!(outcome(&within), &TemporalVerdict::Holds);
}
