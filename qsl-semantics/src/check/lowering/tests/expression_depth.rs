// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-415 step 10 (FR-093-AC-14): a function body of any depth checks, or
//! refuses on its own unproved obligation, the same way at 1,000 levels as
//! at 2; no limit names a depth and nothing overflows the stack. Each check
//! runs on a spawned thread with a 512 KiB stack, in whatever profile the
//! suite runs in. A debug build once aborted at 20 nested `a and (…)`:
//! typing, the definedness walk and lowering each recursed once per level,
//! and measuring the declaration recursed once per level before any limit
//! was checked.

use super::*;
use crate::check::depth_forms::{declarations, Form, FORMS};
use crate::check::{Obligation, WrongSnapshotCause};
use quire_semantic_value::checking::CheckMode;

/// The stack each check runs on.
const STACK: usize = 512 * 1024;

/// How many levels the deep bodies nest.
const DEEP: usize = 1_000;

/// Check `form` nested `levels` times under `limits` on a [`STACK`]-byte
/// thread. The body is built and dropped on that thread too.
pub(super) fn check_on_small_stack(
    form: Form,
    levels: usize,
    limits: CheckingLimits,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(move || declarations(form, levels).check(limits))
        .expect("the check thread spawns")
        .join()
        .expect("the check completes on a 512 KiB stack")
}

/// What a check gives: checked, or the distinct causes it refused on.
fn outcome(checked: Result<CheckedGraph, Vec<CheckRefusal>>) -> Result<(), Vec<CheckCause>> {
    checked.map(|_| ()).map_err(|refusals| {
        let mut causes: Vec<CheckCause> = Vec::new();
        for refusal in refusals {
            if !causes.contains(&refusal.cause) {
                causes.push(refusal.cause);
            }
        }
        causes
    })
}

/// Every nested form, nested 1,000 deep, checks or refuses on its own
/// unproved obligation exactly as it does nested 2 deep, through typing,
/// the definedness walk and lowering: the unguarded field chain refuses on
/// its unproved `value` both times, and every other form checks.
#[trace("FR-093-AC-14", "TC-415")]
#[test]
fn every_nested_form_checks_at_1000_levels_as_at_2_on_a_small_stack() {
    for form in FORMS {
        let shallow = outcome(check_on_small_stack(form, 2, CheckingLimits::default()));
        match (form, &shallow) {
            (Form::Field, Err(causes)) => {
                assert_eq!(causes, &[CheckCause::Unproved(Obligation::Presence)]);
            }
            (Form::Field, Ok(())) => panic!("an unguarded `value` is unproved"),
            (_, Ok(())) => {}
            (_, Err(causes)) => panic!("{form:?} at 2 levels checks: {causes:?}"),
        }
        let deep = outcome(check_on_small_stack(form, DEEP, CheckingLimits::default()));
        assert_eq!(deep, shallow, "{form:?} at {DEEP} levels");
    }
}

/// `pre(…)`'s syntactic checks walk the whole operand before typing enters
/// it: a postcondition `pre` over 1,000 nested `a and (…)` with no eligible
/// read refuses as a forbidden pre-read.
#[trace("FR-093-AC-14", "TC-415")]
#[test]
fn a_postcondition_pre_over_1000_levels_refuses_on_a_small_stack() {
    let graph =
        PackageDeclarations::new(fixture_source(), qsl_foundation::IdentityLimits::default())
            .check(CheckingLimits::default())
            .expect("an empty package checks");
    let refused = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn_scoped(scope, || {
                let parameters = vec![("a".to_owned(), ValueType::Boolean)];
                let mut unread = name_expr("a");
                for _ in 0..DEEP {
                    unread = binary(BinaryOperator::And, name_expr("a"), unread);
                }
                graph.check_postcondition_expression(
                    parameters,
                    &Expression::pre(unread),
                    Some(&ValueType::Boolean),
                    CheckMode::Linked,
                    CheckingLimits::default(),
                )
            })
            .expect("the check thread spawns")
            .join()
            .expect("the check completes on a 512 KiB stack")
    });
    assert_eq!(
        refused.expect_err("no eligible read").cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
}
