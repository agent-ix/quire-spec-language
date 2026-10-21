// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-178 (ADR-023 PA-2 to PA-6, HV-3): the run over an HP-6 item's projected
//! product, which shares the exploration, the first phase and the traces of
//! [`check_hyper`](super::check_hyper) and adds what only a projected product
//! has: the skipped and visible moves, the search horizon, the counterexample
//! of kind `Projected` and the three-valued confirmation of a rejecting state.

use qsl_eval::hyper::{ComponentStep, HyperTransition, ParameterBinding};
use qsl_eval::temporal::{evaluate_projected_prefix, TemporalOutcome};
use qsl_replay::{
    HyperCounterexample, HyperCounterexampleKind, HyperOutcome, ModelCheckLimit, StepMark,
};
use quire_exact::Meter;

use super::{
    exhausted, fault, tree_path, widen, ModelCheckRefusal, Positions, ProductStateId, RetainedEdge,
    Run, Totals,
};

impl Run<'_, '_> {
    /// Decide an HP-6 item (FR-178): for each instance, the first phase's
    /// rejecting monitor states, undefined evaluations and refused `match`es,
    /// ranked across instances as FR-126 ranks bindings. A run that finds none
    /// holds, or reaches its search horizon with no counterexample of at most
    /// `max_depth` moves.
    pub(super) fn decide_projected(
        &self,
        instances: &[Vec<ParameterBinding>],
        mut totals: Totals,
    ) -> Result<HyperOutcome, ModelCheckRefusal> {
        let horizon = self.request.max_depth;
        let ranked = match self.rank_instances(instances, horizon, &mut totals)? {
            Ok(ranked) => ranked,
            Err(outcome) => return Ok(outcome),
        };
        match (ranked.best, ranked.bounded, horizon) {
            (Some((_, hit)), _, _) => Self::finding(hit, &totals),
            (None, true, Some(depth)) => Ok(HyperOutcome::BoundReached {
                depth: widen(depth),
                stats: totals.stats(),
            }),
            (None, _, _) => Ok(HyperOutcome::Holds {
                stats: totals.stats(),
            }),
        }
    }

    /// Whether `transition` is a skip move: a component takes a skipped step.
    /// A visible move takes none.
    fn is_skip_move(run: &super::Explored<'_, '_>, transition: &HyperTransition) -> bool {
        transition.0.iter().enumerate().any(|(component, step)| {
            matches!(step, ComponentStep::Model(taken) if run.product.skipped(component, taken))
        })
    }

    /// The joint position the path `edges` ends at: its moves for a lockstep
    /// product, its visible moves for a projected one, since only a visible
    /// move enters a projected position (PA-2).
    pub(super) fn position_of(
        &self,
        run: &super::Explored<'_, '_>,
        edges: &[&RetainedEdge<HyperTransition>],
    ) -> u64 {
        if self.projected {
            widen(
                edges
                    .iter()
                    .filter(|edge| !Self::is_skip_move(run, &edge.transition))
                    .count(),
            )
        } else {
            widen(edges.len())
        }
    }

    /// The mark of `component`'s step in the move `transition`.
    pub(super) fn mark(
        run: &super::Explored<'_, '_>,
        component: usize,
        transition: &HyperTransition,
    ) -> StepMark {
        match transition.0.get(component) {
            Some(ComponentStep::Model(taken)) if run.product.skipped(component, taken) => {
                StepMark::Skipped
            }
            _ => StepMark::Visible,
        }
    }

    /// The monitor's rejection at `at` read back through the layer-5 evaluator
    /// (ADR-014 A-4): the joint projected prefix of the path to `at` is a bad
    /// prefix of the body. `Err(outcome)` is an evaluator that ran out of its
    /// meter.
    pub(super) fn confirmed_rejection(
        &self,
        run: &super::Explored<'_, '_>,
        at: ProductStateId,
        totals: &Totals,
    ) -> Result<Result<(), HyperOutcome>, ModelCheckRefusal> {
        let (root, path) = tree_path(&run.graph, at);
        let state = |id: ProductStateId| {
            run.graph
                .state(id)
                .map(|state| &state.state)
                .ok_or_else(|| fault("a-path-state-is-retained"))
        };
        // Projected position 0 is the initial state; each later one is the
        // state a visible move entered (PA-2).
        let mut sequence = vec![state(root)?];
        for edge in &path {
            if !Self::is_skip_move(run, &edge.transition) {
                sequence.push(state(edge.target)?);
            }
        }
        let len = widen(sequence.len());
        let mut letters = Positions {
            letters: self.request.letters,
            instance: run.instance,
            sequence,
        };
        let mut meter = Meter::new(self.request.meter).with_cancel(self.cancel.clone());
        let outcome = evaluate_projected_prefix(self.request.clause, len, &mut letters, &mut meter)
            .map_err(|_| fault("the-evaluator-reads-the-projected-prefix"))?;
        match outcome {
            TemporalOutcome::Violated { .. } => Ok(Ok(())),
            TemporalOutcome::Incomplete(_) | TemporalOutcome::WindowLimitReached(_) => {
                Ok(Err(exhausted(
                    ModelCheckLimit::EvaluationMeter,
                    self.request.meter.work_units,
                    totals,
                )))
            }
            _ => Err(fault("the-evaluator-confirms-the-monitor")),
        }
    }

    /// The `Projected` counterexample for the rejecting state `at` (ADR-023
    /// PA-6): the breadth-first tree path to it split into one trace per
    /// variable, each step marked skipped or visible, `trace_position` the
    /// projected position the visible steps reach.
    pub(super) fn projected_counterexample(
        &self,
        run: &super::Explored<'_, '_>,
        at: ProductStateId,
    ) -> Result<HyperCounterexample, ModelCheckRefusal> {
        let (root, path) = tree_path(&run.graph, at);
        let trace_position = self.position_of(run, &path);
        let lifted = self.lift(run, root, &path)?;
        let traces = self.traces(run, &lifted, None, None, true)?;
        Ok(HyperCounterexample {
            kind: HyperCounterexampleKind::Projected,
            instance: Self::instance_of(run.instance),
            traces,
            trace_position,
        })
    }
}
