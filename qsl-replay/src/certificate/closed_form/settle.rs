// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-248's verdict map: what a closed-form outcome settles as once the
//! checker has answered.

use quire_exact::Cancel;
use quire_semantic_value::catalog::Category;

use crate::certificate::closed_form::outcome::ClosedFormOutcome;
use crate::certificate::closed_form::recompute::{recompute, ClosedFormCheck};
use crate::certificate::closed_form::task_set::ScheduleSubject;
use crate::proof_result::{
    Certification, IncompleteCause, InconclusiveCause, ProofBasis, TerminalValue,
};
use crate::result::{DisagreementCause, Verdict};

/// The terminal value `outcome` settles as once [`recompute`] has checked
/// it (FR-248), so no caller can settle an outcome the checker did not see:
/// `Proved{ClosedForm{analysis}, Certified}` for a schedule the checker
/// agrees with, `Refuted` for a miss it agrees with, `Inconclusive(
/// SufficientTestFailed)` for a failed sufficient test it agrees with,
/// `Inconclusive(ReplayParity)` for any outcome it disagrees with, and
/// `Incomplete` for a stop of EN-7's or of the check.
pub fn settle(
    subject: &ScheduleSubject,
    outcome: &ClosedFormOutcome,
    cancel: &Cancel,
) -> TerminalValue {
    let (claimed, agreed) = match outcome {
        ClosedFormOutcome::Stopped(cause, _) => return TerminalValue::Incomplete(*cause),
        ClosedFormOutcome::Schedulable(_) => (
            Category::Success,
            TerminalValue::Proved {
                basis: ProofBasis::ClosedForm {
                    analysis: subject.analysis(),
                },
                certification: Certification::Certified,
            },
        ),
        ClosedFormOutcome::Miss(_) => (Category::Violation, TerminalValue::Refuted),
        ClosedFormOutcome::SufficientTestFailed(_) => (
            Category::Inconclusive,
            TerminalValue::Inconclusive(InconclusiveCause::SufficientTestFailed),
        ),
    };
    match recompute(subject, outcome, cancel) {
        ClosedFormCheck::Agrees => agreed,
        ClosedFormCheck::Cancelled(source) => {
            TerminalValue::Incomplete(IncompleteCause::Cancelled { source })
        }
        // The checker reproduced no value for the evidence, so the replayed
        // verdict is the inconclusive one.
        ClosedFormCheck::Disagrees(_) => TerminalValue::Inconclusive(
            InconclusiveCause::ReplayParity(DisagreementCause::NoValue {
                proved: Verdict::from_category(claimed),
                replayed: Verdict::from_category(Category::Inconclusive),
            }),
        ),
    }
}
