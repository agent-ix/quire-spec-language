// SPDX-License-Identifier: AGPL-3.0-or-later
//! Borrow the admitted clock; retain its exact IR rational only after payment.

use quire_contract_model::{
    CheckedMonitorClock, CheckedPackageV2, CheckedRational, CheckedTerm, LiteralKind,
};
use quire_observation::authority::progress;

use super::{charge::Charge, PlanAuthorityRefusal, ProgressSelectionRefusalCode};

pub(super) fn checked_period(
    package: &CheckedPackageV2,
    clock: CheckedMonitorClock<'_>,
    row: &progress::AssociationRow,
    charge: &mut Charge<'_>,
) -> Result<Option<CheckedRational>, PlanAuthorityRefusal> {
    // emit_plan_root retains the same combined CheckedUnit passed to
    // emit_checked. CheckedTimedModel::from_checked_time_source writes this
    // source's numerator/denominator directly into the period literal; the
    // monitor writer only rekeys references and generated record fields.
    // PlanRoot's private source/emission fields cannot be replaced. The
    // caller reached this clock through that emission's exact original
    // claim, so these canonical components are the original checked period,
    // without formatting or copying the kernel integer to compare it again.
    let CheckedMonitorClock::ModelTime { source, .. } = clock else {
        return Err(PlanAuthorityRefusal::FinalEmission);
    };
    charge.work(1)?;
    let Some(CheckedTerm::Aggregate { members }) = package.term(source) else {
        return Err(PlanAuthorityRefusal::FinalEmission);
    };
    // The strict reader admitted the closed dense/tick member order. Inspect
    // that actual term without constructing a second wire model or literal reader.
    charge.work(1)?;
    let Some(CheckedTerm::Binding { value: kind, .. }) = members.first() else {
        return Err(PlanAuthorityRefusal::FinalEmission);
    };
    charge.work(1)?;
    let CheckedTerm::Literal {
        value_kind: LiteralKind::Text,
        value,
        ..
    } = kind.as_ref()
    else {
        return Err(PlanAuthorityRefusal::FinalEmission);
    };
    let kind = value.as_str().ok_or(PlanAuthorityRefusal::FinalEmission)?;
    match &row.source {
        progress::TimeSource::Dense if charge.text(kind, "dense")? => Ok(None),
        progress::TimeSource::Tick {
            period_numerator,
            period_denominator,
            ..
        } if charge.text(kind, "tick")? => {
            charge.work(1)?;
            let Some(CheckedTerm::Binding { value, .. }) = members.get(4) else {
                return Err(PlanAuthorityRefusal::FinalEmission);
            };
            charge.work(1)?;
            let CheckedTerm::Literal {
                value_kind: LiteralKind::Rational,
                value,
                ..
            } = value.as_ref()
            else {
                return Err(PlanAuthorityRefusal::FinalEmission);
            };
            let text = value.as_str().ok_or(PlanAuthorityRefusal::FinalEmission)?;
            let mut bytes = text.bytes();
            let mut offset = 0;
            let slash = loop {
                charge.work(1)?;
                match bytes.next() {
                    Some(b'/') => break Some(offset),
                    Some(_) => offset += 1,
                    None => break None,
                }
            };
            // An absent denominator is exactly the admitted IR rational grammar.
            let (numerator, denominator) =
                slash.map_or((text, "1"), |at| (&text[..at], &text[at + 1..]));
            if !charge.text(numerator, period_numerator)?
                || !charge.text(denominator, period_denominator)?
            {
                return Err(ProgressSelectionRefusalCode::TimeSource.into());
            }
            copy_period(numerator, denominator, charge).map(Some)
        }
        _ => Err(ProgressSelectionRefusalCode::TimeSource.into()),
    }
}

fn copy_period(
    numerator: &str,
    denominator: &str,
    charge: &mut Charge<'_>,
) -> Result<CheckedRational, PlanAuthorityRefusal> {
    let bytes = numerator.len() as u128 + denominator.len() as u128;
    // One actual rational, two owned string containers and their copied bytes.
    // Both permissions precede either component allocation or copy.
    charge.storage(3 + bytes)?;
    charge.work(bytes)?;
    Ok(CheckedRational {
        numerator: numerator.into(),
        denominator: denominator.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use quire_contract_model::{NativeJoinBudget, NativeJoinLocus, NativeJoinLocusSource};

    #[test]
    #[trace("TC-1081", "FR-251-AC-50")]
    fn exact_ir_period_copy_is_paid_before_both_owned_components() {
        let locus = NativeJoinLocus {
            source: NativeJoinLocusSource::ReachedRow(37),
            form: "qsl_authority_selection",
        };
        let mut budget = NativeJoinBudget::new(2, 5);
        budget.resume();
        let value: CheckedRational = copy_period(
            "3",
            "2",
            &mut Charge {
                budget: &mut budget,
                locus,
            },
        )
        .unwrap();
        assert_eq!(value.numerator.as_ref(), "3");
        assert_eq!(value.denominator.as_ref(), "2");
        assert_eq!(budget.consumed().storage, 5);
        let mut short = NativeJoinBudget::new(2, 4);
        short.resume();
        assert!(matches!(
            copy_period(
                "3",
                "2",
                &mut Charge {
                    budget: &mut short,
                    locus
                }
            ),
            Err(PlanAuthorityRefusal::Budget { .. })
        ));
        assert_eq!(short.consumed().storage, 0);
        assert_eq!(short.consumed().work, 0);
    }
}
