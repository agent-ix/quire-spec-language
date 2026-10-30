// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251/FR-252 tick-monitor plan production.

use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{One, ToPrimitive};
use quire_exact::{Integer as ExactInteger, Rational};

use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::CheckedPackage;
use qsl_replay::{
    plan_obligations, AtomId, ClockBindingKey, MonitorAtom, MonitorPlanRefusal, MonitorProgram,
    MonitorSubject, MonitorTarget, SubformulaId, TickInterval, TimedMonitorPlan, TimingObligation,
    TimingObligationSettings,
};
use qsl_semantics::check::{CheckedInterval, CheckedTemporalClause, FormulaNode};
use qsl_semantics::library::LibraryName;

/// The FR-251 default event rate, `one event / microsecond`, expressed per
/// target unit whose exact duration is `unit_seconds`.
fn default_event_rate(unit_seconds: &Rational) -> Rational {
    unit_seconds.mul(&Rational::from_integer(ExactInteger::from(1_000_000_i64)))
}

/// FR-251's per-window capacity: `ceil(rate * window) + 1`.
fn buffer_capacity(rate: &Rational, window: &Rational) -> Result<u64, MonitorPlanRefusal> {
    let zero = Rational::from_integer(ExactInteger::from(0_i64));
    if rate <= &zero || window < &zero {
        return Err(MonitorPlanRefusal::InvalidBufferInputs);
    }
    let product = rate.mul(window);
    let numerator = product.numerator().as_big();
    let denominator = product.denominator().as_big();
    let capacity: BigInt = numerator.div_ceil(denominator) + BigInt::one();
    capacity
        .to_u64()
        .ok_or(MonitorPlanRefusal::BufferCapacityOverflow)
}

/// Refuse a target whose maximum reading gap could span a full counter wrap.
fn validate_reading_gap(max_reading_gap: u64, width_bits: u32) -> Result<(), MonitorPlanRefusal> {
    if width_bits == 0 {
        return Err(MonitorPlanRefusal::ZeroCounterWidth);
    }
    let wrap = BigInt::one() << usize::try_from(width_bits).unwrap_or(usize::MAX);
    if BigInt::from(max_reading_gap) >= wrap {
        return Err(MonitorPlanRefusal::GapExceedsWidth {
            max_reading_gap,
            width_bits,
        });
    }
    Ok(())
}

/// Derive FR-251's tick plan and FR-252's per-plan timing obligations from a
/// clause in its checked package and a target premise.
pub fn produce(
    package: &CheckedPackage,
    library: LibraryName,
    clause: &CheckedTemporalClause,
    target: &MonitorTarget,
    settings: &TimingObligationSettings,
) -> Result<(TimedMonitorPlan, [TimingObligation; 2]), MonitorPlanRefusal> {
    validate_reading_gap(target.max_reading_gap, target.width_bits)?;
    let zero = Rational::from_integer(ExactInteger::from(0_i64));
    if target.uncertainty < zero {
        return Err(MonitorPlanRefusal::InvalidBufferInputs);
    }

    let node = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause() == clause)
        .ok_or(MonitorPlanRefusal::NotTimedClause)?;
    let binding = clause
        .time_binding()
        .ok_or(MonitorPlanRefusal::NotTimedClause)?;
    let source_scale = package
        .graph()
        .time_unit_scale(&binding.unit)
        .map_err(|_| MonitorPlanRefusal::InvalidTimeUnit {
            unit: binding.unit.clone(),
        })?;
    let target_scale = package.graph().time_unit_scale(&target.unit).map_err(|_| {
        MonitorPlanRefusal::InvalidTimeUnit {
            unit: target.unit.clone(),
        }
    })?;
    let rate_limit = target
        .max_event_rate
        .clone()
        .unwrap_or_else(|| default_event_rate(target_scale));
    if rate_limit <= zero {
        return Err(MonitorPlanRefusal::InvalidBufferInputs);
    }

    let clause_node = node.identity().clone();
    let subject = MonitorSubject::new(
        library,
        clause_node.clone(),
        OccurrenceKey::new(clause_node, node.claim().clone()),
        clause.name(),
    );
    let clock_binding = ClockBindingKey::new(
        target.counter.clone(),
        target.unit.clone(),
        target_scale.clone(),
        target.width_bits,
    );

    let convert = |value: &Rational| {
        value
            .mul(source_scale)
            .div(target_scale)
            .ok_or(MonitorPlanRefusal::IntervalConversion)
    };
    let mut intervals = Vec::new();
    let mut buffers = Vec::new();
    for operator in clause.operators() {
        let Some(CheckedInterval::Timed(interval)) = &operator.interval else {
            continue;
        };
        let lower = convert(interval.lower())?;
        let upper = convert(interval.upper())?;
        let id = SubformulaId::new(operator.node.index());
        let tick_interval = TickInterval::new(
            lower.clone(),
            interval.lower_closed(),
            upper.clone(),
            interval.upper_closed(),
        );
        let window = upper.sub(&lower);
        intervals.push((id, tick_interval));
        buffers.push((id, buffer_capacity(&rate_limit, &window)?));
    }

    let formula = clause.formula();
    let mut atoms = Vec::new();
    for (index, formula_node) in formula.nodes().iter().enumerate() {
        if matches!(formula_node, FormulaNode::Holds { .. }) {
            let id = formula
                .node_id(index)
                .expect("the iterated formula node has an arena id");
            let (body, slots) = node
                .condition(id)
                .ok_or(MonitorPlanRefusal::MissingAtomBody { index })?;
            atoms.push(MonitorAtom::new(
                AtomId::new(index),
                body.body().clone(),
                slots,
            ));
        }
    }
    let guard = node
        .guard()
        .map(|(body, slots)| (body.body().clone(), slots));
    let program = MonitorProgram::new(
        SubformulaId::new(formula.root().index()),
        formula.clone(),
        clause.activation(),
        atoms,
        guard,
    );
    let plan = TimedMonitorPlan::new(
        subject,
        clock_binding,
        target.uncertainty.clone(),
        target.max_reading_gap,
        program,
        intervals,
        buffers,
        rate_limit,
    );
    let obligations = plan_obligations(&plan, settings);
    Ok((plan, obligations))
}

#[cfg(test)]
mod tests {
    use super::*;
    use quire_exact::{Integer, Rational};

    fn ratio(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator))
            .expect("nonzero denominator")
    }

    #[test]
    fn default_event_rate_is_one_event_per_microsecond_in_target_units() {
        // A target unit is measured in seconds. One event per microsecond
        // therefore becomes 1,000 events per millisecond.
        assert_eq!(default_event_rate(&ratio(1, 1_000)), ratio(1_000, 1));
        assert_eq!(default_event_rate(&ratio(1, 1)), ratio(1_000_000, 1));
    }

    #[test]
    fn event_buffer_capacity_rounds_up_then_adds_one() {
        assert_eq!(buffer_capacity(&ratio(2, 1), &ratio(10, 1)), Ok(21));
        assert_eq!(buffer_capacity(&ratio(1, 3), &ratio(10, 1)), Ok(5));
    }

    #[test]
    fn event_buffer_capacity_rejects_negative_values_and_overflow() {
        assert!(buffer_capacity(&ratio(-1, 1), &ratio(10, 1)).is_err());
        assert!(buffer_capacity(&ratio(1, 1), &ratio(-1, 1)).is_err());
        assert!(buffer_capacity(&ratio(i64::MAX, 1), &ratio(i64::MAX, 1)).is_err());
    }

    #[test]
    fn counter_gap_must_be_decidable_from_the_counter_width() {
        assert_eq!(validate_reading_gap(100, 16), Ok(()));
        assert_eq!(
            validate_reading_gap(65_536, 16),
            Err(MonitorPlanRefusal::GapExceedsWidth {
                max_reading_gap: 65_536,
                width_bits: 16,
            })
        );
    }

    #[test]
    fn timing_obligation_settings_use_fr252_defaults() {
        let settings = TimingObligationSettings::default();
        assert_eq!(settings.agreement_horizon, 8);
        assert!(settings.timestamp_parameters.is_empty());
    }
}
