// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-450 "Static replay": QSpec FR-193 "Trace replay" static checks 1 to 5
//! over a trace's intervals, in exact rational interval arithmetic widened
//! by the trace's `delta`. A check fails only when its value is definitely
//! false, and the outcome names the first failing check in trace order.

use qsl_foundation::diagnostic::JsonPointer;
use qsl_semantics::check::{CheckedHybridModel, CompareOp, HybridTerm};
use quire_exact::Rational;

use super::expand::Expander;
use super::interval::{compare, Interval, IntervalEvaluator, Tri};
use super::trace::{before_pointer, jump_pointer, loop_pointer, state_pointer, ExactInterval};
use super::{HybridReplayMeter, HybridReplayRefusal, HybridTrace, StaticCheck, StaticReplay};

fn intervals(enclosure: &[ExactInterval]) -> Vec<Interval> {
    enclosure
        .iter()
        .map(|interval| Interval {
            lower: Some(interval.lower.clone()),
            upper: Some(interval.upper.clone()),
        })
        .collect()
}

fn interval(time: &ExactInterval) -> Interval {
    Interval {
        lower: Some(time.lower.clone()),
        upper: Some(time.upper.clone()),
    }
}

/// Whether `a op b`, widened by `delta`, is at least possibly true.
fn possibly(op: CompareOp, a: &Interval, b: &Interval, delta: &Rational) -> bool {
    compare(op, a, b, delta) != Tri::False
}

struct Checker<'m, 'e, 'g, 'b> {
    model: &'m CheckedHybridModel,
    evaluator: IntervalEvaluator<'e, 'g, 'b>,
}

impl Checker<'_, '_, '_, '_> {
    /// Whether `term` is at least possibly true over `variables`.
    fn possible(
        &mut self,
        term: &HybridTerm,
        variables: &[Interval],
    ) -> Result<bool, HybridReplayRefusal> {
        let exp = self.evaluator.expand(term)?;
        Ok(self.evaluator.truth(exp, variables)? != Tri::False)
    }

    fn invariant(
        &mut self,
        mode: usize,
        variables: &[Interval],
    ) -> Result<bool, HybridReplayRefusal> {
        let model = self.model;
        self.possible(&model.modes[mode].invariant, variables)
    }
}

/// Static replay of an admitted `trace` over `model`.
pub(super) fn replay(
    model: &CheckedHybridModel,
    expander: &mut Expander<'_>,
    trace: &HybridTrace,
    meter: &mut HybridReplayMeter,
) -> Result<StaticReplay, HybridReplayRefusal> {
    let delta = trace.tolerance();
    let mut checker = Checker {
        model,
        evaluator: IntervalEvaluator::new(expander, delta.clone(), meter),
    };
    let inconsistent =
        |at: JsonPointer, check: StaticCheck| Ok(StaticReplay::Inconsistent { at, check });
    let zero = Interval::point(super::trace::zero());

    // Check 1: `start` meets an initial entry of its mode, and its time
    // contains 0.
    let start = &trace.start;
    let start_variables = intervals(&start.enclosure);
    let mut meets_initial = possibly(CompareOp::Eq, &interval(&start.time), &zero, &delta);
    if meets_initial {
        meets_initial = false;
        for entry in model
            .initial
            .iter()
            .filter(|entry| entry.mode == start.mode)
        {
            if checker.possible(&entry.condition, &start_variables)?
                && checker.invariant(start.mode.get(), &start_variables)?
            {
                meets_initial = true;
                break;
            }
        }
    }
    if !meets_initial {
        return inconsistent(state_pointer(0), StaticCheck::Initial);
    }
    // Check 2 at `start`.
    if !checker.invariant(start.mode.get(), &start_variables)? {
        return inconsistent(state_pointer(0), StaticCheck::Invariant);
    }
    let mut previous = start;
    for (j, jump) in trace.jumps.iter().enumerate() {
        let previous_lower = Interval::point(previous.time.lower.clone());
        let reached = &jump.state;
        let reached_time = interval(&reached.time);
        // Check 3: no time runs backwards, and a jump takes no time.
        if let Some(before) = &jump.before {
            let before_time = interval(&before.time);
            if !possibly(
                CompareOp::Ge,
                &Interval::point(before.time.upper.clone()),
                &previous_lower,
                &delta,
            ) || !possibly(CompareOp::Eq, &before_time, &reached_time, &delta)
            {
                return inconsistent(before_pointer(j).key("time"), StaticCheck::Time);
            }
        }
        if !possibly(
            CompareOp::Ge,
            &Interval::point(reached.time.upper.clone()),
            &previous_lower,
            &delta,
        ) {
            return inconsistent(jump_pointer(j).key("state").key("time"), StaticCheck::Time);
        }
        // Check 4: the source invariant and the guard before the jump, and
        // the reset between the two states.
        if let Some(before) = &jump.before {
            let before_variables = intervals(&before.enclosure);
            let reached_variables = intervals(&reached.enclosure);
            let source = previous.mode.get();
            let transition = usize::try_from(jump.transition)
                .ok()
                .and_then(|t| model.modes[source].transitions.get(t))
                .unwrap_or_else(|| unreachable!("an admitted trace names a transition"));
            let mut holds = checker.invariant(source, &before_variables)?
                && checker.possible(&transition.guard, &before_variables)?;
            let mut constrained = vec![false; model.variables.len()];
            for reset in &transition.resets {
                if !holds {
                    break;
                }
                let variable = reset.variable.get();
                constrained[variable] = true;
                let exp = checker.evaluator.expand(&reset.value)?;
                let value = match checker.evaluator.evaluate(exp, &before_variables)? {
                    super::interval::IValue::Number(value) => value,
                    super::interval::IValue::Boolean(_) => Interval::whole(),
                };
                holds = possibly(reset.relation, &reached_variables[variable], &value, &delta);
            }
            for (variable, constrained) in constrained.iter().enumerate() {
                if holds && !constrained {
                    holds = possibly(
                        CompareOp::Eq,
                        &reached_variables[variable],
                        &before_variables[variable],
                        &delta,
                    );
                }
            }
            if !holds {
                return inconsistent(jump_pointer(j), StaticCheck::Jump);
            }
        }
        // Check 2 at the reached state.
        if !checker.invariant(reached.mode.get(), &intervals(&reached.enclosure))? {
            return inconsistent(jump_pointer(j).key("state"), StaticCheck::Invariant);
        }
        previous = reached;
    }
    // Check 5: a lasso's last state meets the state it returns to.
    if let Some(loop_to) = trace.loop_to {
        let target = usize::try_from(loop_to)
            .ok()
            .and_then(|k| trace.state(k))
            .unwrap_or_else(|| unreachable!("an admitted loop names a state"));
        let last = previous;
        let meets = last.mode == target.mode
            && intervals(&last.enclosure)
                .iter()
                .zip(intervals(&target.enclosure))
                .all(|(a, b)| possibly(CompareOp::Eq, a, &b, &delta));
        if !meets {
            return inconsistent(loop_pointer(), StaticCheck::Loop);
        }
    }
    Ok(StaticReplay::Consistent)
}
