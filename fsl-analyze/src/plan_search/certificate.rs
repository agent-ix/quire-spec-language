// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-650's `potential` and `closure` certificates: the writer the
//! search calls after a complete exploration, and the core checker
//! ([`check`]), which re-derives every entry's successors through
//! [`ModelSystem`] and re-evaluates goal, policy and objective itself.

use filament_exact::{Cancel, Rational};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::TransitionSystem;

use super::{
    limit_reached, zero, MonitorStep, NodeKey, ObjectiveKind, PlanLengths, PlanLimit, PlanRules,
    PlanSearchLimits, PlanStopCause, PlanView, Search,
};

/// Which certificate a result carries (FSpec FR-650, FR-802).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificateKind {
    /// A lower-bound map for a non-negative per-step sum.
    Potential,
    /// The complete set of admissible-prefix nodes.
    Closure,
}

impl CertificateKind {
    /// The wire name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Potential => "potential",
            Self::Closure => "closure",
        }
    }
}

/// A certificate rule, by FSpec FR-650's rule names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificateRule {
    /// Every re-derived step has a record naming the right node.
    Closure,
    /// `pi(n) <= cost(a) + pi(n')` for every record.
    Descent,
    /// `pi(n) <= 0` at every goal node whose length is in the domain.
    Goal,
    /// Entry 0 is the initial node and `pi` of it is the plan's value.
    Initial,
    /// No goal node improves on the reported plan.
    Improvement,
    /// No node reaches the goal.
    GoalFree,
}

impl CertificateRule {
    /// The wire name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Closure => "closure",
            Self::Descent => "descent",
            Self::Goal => "goal",
            Self::Initial => "initial",
            Self::Improvement => "improvement",
            Self::GoalFree => "goal-free",
        }
    }
}

/// The core checker's verdict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CertificateOutcome {
    /// Every rule held at every entry.
    Accepted,
    /// The first failing rule and the entry it failed at.
    Rejected {
        /// The rule.
        rule: CertificateRule,
        /// The zero-based entry index.
        entry: usize,
    },
}

/// One certificate entry: its search node, value and successor records.
#[derive(Clone, Debug)]
pub struct Entry {
    key: NodeKey,
    state: ModelSimState,
    monitors: Vec<Vec<u8>>,
    /// The entry that admitted this node and the step, for re-reading the
    /// node's representative prefix.
    parent: Option<(usize, ModelTransition)>,
    /// `pi` for a potential certificate (`None` is no reachable goal); the
    /// goal node's objective value for a closure certificate.
    value: Option<Rational>,
    /// One record per enabled, prefix-admissible step, in step order: the
    /// step and the successor's entry index.
    successors: Vec<(ModelTransition, usize)>,
}

/// A written certificate.
#[derive(Clone, Debug)]
pub struct Certificate {
    kind: CertificateKind,
    objective: ObjectiveKind,
    merges: bool,
    entries: Vec<Entry>,
}

impl Certificate {
    /// The number of entries.
    pub fn entries(&self) -> u64 {
        u64::try_from(self.entries.len()).unwrap_or(u64::MAX)
    }

    /// FR-755's tamper seam: remove one successor record before the core
    /// checker reads the certificate. Out-of-range indices change nothing.
    pub(crate) fn drop_successor(&mut self, entry: usize, successor: usize) {
        if let Some(entry) = self.entries.get_mut(entry) {
            if successor < entry.successors.len() {
                entry.successors.remove(successor);
            }
        }
    }

    /// A library test seam: give entry `entry` the state of entry `source`
    /// while its key stays, before the core checker reads the certificate.
    pub(crate) fn forge_state(&mut self, entry: usize, source: usize) {
        if let Some(state) = self.entries.get(source).map(|source| source.state.clone()) {
            if let Some(entry) = self.entries.get_mut(entry) {
                entry.state = state;
            }
        }
    }

    /// A library test seam: point entry `entry`'s parent record at the
    /// entry itself before the core checker reads the certificate.
    pub(crate) fn parent_cycle(&mut self, entry: usize) {
        if let Some(found) = self.entries.get_mut(entry) {
            if let Some((parent, _)) = &mut found.parent {
                *parent = entry;
            }
        }
    }
}

/// Write the certificate of a complete exploration, entries in the
/// canonical exploration order (layers by step count, each in FSpec
/// FR-801's within-layer order).
pub(crate) fn write<R: PlanRules>(
    search: &mut Search<'_, '_, R>,
    kind: CertificateKind,
) -> Result<Certificate, PlanStopCause> {
    let deepest = search
        .nodes
        .iter()
        .map(|node| node.depth)
        .max()
        .unwrap_or(0);
    let mut order = Vec::with_capacity(search.nodes.len());
    for depth in 0..=deepest {
        let mut layer: Vec<usize> = (0..search.nodes.len())
            .filter(|index| search.nodes[*index].depth == depth)
            .collect();
        search.order_layer(&mut layer);
        order.extend(layer);
    }
    let bound = search.request.limits.get(PlanLimit::MaxCertificateEntries);
    if u64::try_from(order.len()).map_or(true, |entries| entries > bound) {
        return Err(limit_reached(PlanLimit::MaxCertificateEntries, bound));
    }
    let mut position = vec![0; search.nodes.len()];
    for (entry, node) in order.iter().enumerate() {
        position[*node] = entry;
    }
    let potential = match kind {
        CertificateKind::Potential => Some(potentials(search)?),
        CertificateKind::Closure => None,
    };
    let entries = order
        .iter()
        .map(|&index| {
            let node = &search.nodes[index];
            Entry {
                key: node.key.clone(),
                state: node.state.clone(),
                monitors: node.monitors.clone(),
                parent: node
                    .parent
                    .as_ref()
                    .map(|(parent, step)| (position[*parent], step.clone())),
                value: match &potential {
                    Some(pi) => pi[index].clone(),
                    None => node.goal.clone(),
                },
                successors: node
                    .successors
                    .iter()
                    .map(|(step, target, _)| (step.clone(), position[*target]))
                    .collect(),
            }
        })
        .collect();
    Ok(Certificate {
        kind,
        objective: search.kind,
        merges: search.rules.merges_prefixes(),
        entries,
    })
}

/// `pi(n)`: the least remaining cost to a goal node, `None` when none is
/// reachable within the length bound.
fn potentials<R: PlanRules>(
    search: &mut Search<'_, '_, R>,
) -> Result<Vec<Option<Rational>>, PlanStopCause> {
    let mut pi: Vec<Option<Rational>> = vec![None; search.nodes.len()];
    let deepest = search
        .nodes
        .iter()
        .map(|node| node.depth)
        .max()
        .unwrap_or(0);
    for depth in (0..=deepest).rev() {
        for index in 0..search.nodes.len() {
            let node = &search.nodes[index];
            if node.depth != depth {
                continue;
            }
            let mut best = node.goal.as_ref().map(|_| zero());
            for (_, target, cost) in &node.successors {
                if let Some(rest) = &pi[*target] {
                    if let Some(through) = search
                        .rules
                        .add_objective_values(cost, rest)
                        .map_err(super::stop_cause)?
                    {
                        if best.as_ref().is_none_or(|kept| through < *kept) {
                            best = Some(through);
                        }
                    }
                }
            }
            pi[index] = best;
        }
    }
    Ok(pi)
}

/// The checker's own meter: one `max_check_work` unit per entry read and
/// per successor re-derived.
struct CheckMeter<'c> {
    used: u64,
    bound: u64,
    cancel: &'c Cancel,
}

impl CheckMeter<'_> {
    fn charge(&mut self) -> Result<(), PlanStopCause> {
        if let Some(cause) = self.cancel.cause() {
            return Err(PlanStopCause::Cancelled(cause));
        }
        if self.used >= self.bound {
            return Err(limit_reached(PlanLimit::MaxCheckWork, self.bound));
        }
        self.used += 1;
        Ok(())
    }
}

/// The core checker (FSpec FR-650): re-derive every entry's successors by
/// exact evaluation of the model's operations and the policy monitors,
/// and check the certificate's rules. `plan_value` is the reported plan's
/// objective value, `None` for a certificate of infeasibility.
///
/// # Errors
/// [`PlanStopCause`] when `max_check_work`, the evaluation meter or a
/// cancellation stops the check before a verdict.
pub(crate) fn check<R: PlanRules>(
    system: &ModelSystem<'_>,
    rules: &mut R,
    certificate: &Certificate,
    lengths: PlanLengths,
    limits: PlanSearchLimits,
    cancel: &Cancel,
    plan_value: Option<&Rational>,
) -> Result<CertificateOutcome, PlanStopCause> {
    let mut meter = CheckMeter {
        used: 0,
        bound: limits.get(PlanLimit::MaxCheckWork),
        cancel,
    };
    let reject = |rule, entry| Ok(CertificateOutcome::Rejected { rule, entry });
    let entries = &certificate.entries;
    // Rule `initial`: entry 0 is the initial node.
    let Some(start) = system.initial().into_iter().next() else {
        return reject(CertificateRule::Initial, 0);
    };
    let Some(start_admissible) = invariants_hold(system, &start)? else {
        return reject(CertificateRule::Initial, 0);
    };
    let mut monitors = Vec::new();
    let mut start_rejected = !start_admissible;
    if start_admissible {
        for policy in 0..rules.policy_count() {
            match rules
                .policy_start(policy, &start)
                .map_err(super::stop_cause)?
            {
                MonitorStep::Continue(monitor) => monitors.push(monitor),
                MonitorStep::Reject => {
                    start_rejected = true;
                    break;
                }
            }
        }
    }
    if start_rejected {
        // A start that breaks an invariant or a safety policy admits no
        // prefix: the only closed set is the empty one.
        return if entries.is_empty() && plan_value.is_none() {
            Ok(CertificateOutcome::Accepted)
        } else {
            reject(CertificateRule::Initial, 0)
        };
    }
    let Some(first) = entries.first() else {
        return reject(CertificateRule::Initial, 0);
    };
    let start_key = system.state_key(&start).map(|(key, _)| key).ok();
    if start_key.as_ref() != Some(&first.key.state)
        || monitors != first.monitors
        || first.key.depth != 0
        || first.parent.is_some()
    {
        return reject(CertificateRule::Initial, 0);
    }
    for (index, entry) in entries.iter().enumerate() {
        meter.charge()?;
        // The entry's stored state is the state its key names, and (past
        // entry 0) its parent record names an earlier entry that records
        // the step into it.
        match system.state_key(&entry.state) {
            Ok((key, _)) if key == entry.key.state => {}
            _ => return reject(CertificateRule::Closure, index),
        }
        if index > 0 && !parent_records(entries, index) {
            return reject(CertificateRule::Closure, index);
        }
        let (states, steps) = prefix(entries, index);
        if entry.key.depth >= lengths.max() {
            continue;
        }
        let expansion = match system.successors(&entry.state) {
            Ok(expansion) => expansion,
            Err(_) => return reject(CertificateRule::Closure, index),
        };
        let mut derived: Vec<_> = expansion
            .successors
            .into_iter()
            .map(|(step, state)| (rules.step_key(&step), step, state))
            .collect();
        derived.sort_by(|left, right| left.0.cmp(&right.0));
        let mut records = 0_usize;
        for (_, step, state) in derived {
            meter.charge()?;
            match invariants_hold(system, &state)? {
                Some(true) => {}
                Some(false) => continue,
                None => return reject(CertificateRule::Closure, index),
            }
            let mut next_monitors = Vec::with_capacity(entry.monitors.len());
            let mut rejected = false;
            for (policy, monitor) in entry.monitors.iter().enumerate() {
                match rules
                    .policy_advance(policy, monitor, &state)
                    .map_err(super::stop_cause)?
                {
                    MonitorStep::Continue(next) => next_monitors.push(next),
                    MonitorStep::Reject => {
                        rejected = true;
                        break;
                    }
                }
            }
            if rejected {
                continue;
            }
            let mut extended_states = states.clone();
            extended_states.push(state.clone());
            let mut extended_steps = steps.clone();
            extended_steps.push(step.clone());
            let view = PlanView {
                states: &extended_states,
                steps: &extended_steps,
            };
            let cost = match certificate.objective {
                ObjectiveKind::StepSum => {
                    match rules
                        .step_cost(&view, extended_steps.len() - 1)
                        .map_err(super::stop_cause)?
                    {
                        Some(cost) => cost,
                        // An undefined cost admits no completion: no record.
                        None => continue,
                    }
                }
                ObjectiveKind::General => zero(),
            };
            let accumulated = if certificate.objective == ObjectiveKind::StepSum {
                let total = step_total(rules, &states, &steps)?;
                let Some(total) = total else {
                    return reject(CertificateRule::Closure, index);
                };
                match rules
                    .accumulate_step_cost(&view, &total, &cost)
                    .map_err(super::stop_cause)?
                {
                    Some(total) => Some(total),
                    None => continue,
                }
            } else {
                None
            };
            records += 1;
            let Some((_, target)) = entry
                .successors
                .iter()
                .find(|(recorded, _)| *recorded == step)
            else {
                return reject(CertificateRule::Closure, index);
            };
            let Some(successor) = entries.get(*target) else {
                return reject(CertificateRule::Closure, index);
            };
            let Ok((state_key, _)) = system.state_key(&state) else {
                return reject(CertificateRule::Closure, index);
            };
            // With no cost dominance the node keeps its accumulated value,
            // which the re-derived step must reproduce.
            let accumulated = entry.key.accumulated.as_ref().and(accumulated);
            let names_node = successor.key.state == state_key
                && successor.key.monitors == next_monitors
                && successor.key.depth == entry.key.depth + 1
                && successor.key.accumulated == accumulated
                && (certificate.merges
                    || successor.parent.as_ref().map(|(parent, _)| *parent) == Some(index));
            if !names_node {
                return reject(CertificateRule::Closure, index);
            }
            if certificate.kind == CertificateKind::Potential {
                // Rule `descent`: pi(n) <= cost(a) + pi(n').
                let holds = match (&entry.value, &successor.value) {
                    (_, None) => true,
                    (None, Some(_)) => false,
                    (Some(pi), Some(rest)) => rules
                        .add_objective_values(&cost, rest)
                        .map_err(super::stop_cause)?
                        .is_some_and(|through| *pi <= through),
                };
                if !holds {
                    return reject(CertificateRule::Descent, index);
                }
            }
        }
        // Every record names a re-derived, admissible step.
        if records != entry.successors.len() {
            return reject(CertificateRule::Closure, index);
        }
    }
    // The goal-dependent rules re-evaluate the goal at every entry whose
    // step count is in the domain.
    for (index, entry) in entries.iter().enumerate() {
        let (states, steps) = prefix(entries, index);
        let mut goal = lengths.contains(entry.key.depth)
            && goal_holds(rules, &states, &steps, &entry.monitors)?;
        let value = if goal {
            objective_value(rules, certificate.objective, &states, &steps)?
        } else {
            None
        };
        goal = value.is_some();
        match certificate.kind {
            CertificateKind::Potential => {
                // Rule `goal`: pi(n) <= 0 at every node whose length is in
                // `D` and at which the goal holds.
                let at_most_zero = entry.value.as_ref().is_some_and(|pi| *pi <= zero());
                if goal && !at_most_zero {
                    return reject(CertificateRule::Goal, index);
                }
            }
            CertificateKind::Closure => {
                // The checker re-evaluates the objective itself; a plan
                // whose objective is undefined is not a goal (FSpec FR-802).
                match plan_value {
                    None if goal => return reject(CertificateRule::GoalFree, index),
                    Some(plan) if goal => {
                        let improves = value.is_some_and(|value| match rules.direction() {
                            super::Direction::Minimize => value < *plan,
                            super::Direction::Maximize => value > *plan,
                        });
                        if improves {
                            return reject(CertificateRule::Improvement, index);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    if certificate.kind == CertificateKind::Potential {
        let offset = rules
            .objective_offset(&PlanView {
                states: std::slice::from_ref(&start),
                steps: &[],
            })
            .map_err(super::stop_cause)?;
        let initial = match first.value.as_ref().zip(offset) {
            Some((value, offset)) => rules
                .add_objective_values(value, &offset)
                .map_err(super::stop_cause)?,
            None => None,
        };
        if initial.as_ref() != plan_value {
            return reject(CertificateRule::Initial, 0);
        }
    }
    Ok(CertificateOutcome::Accepted)
}

/// Whether every state clause and invariant holds at `state`; `None` when
/// the evaluation fails for a reason other than a limit. A state whose
/// invariant cannot be decided ends the check with
/// [`PlanStopCause::InvariantUndetermined`]: a derivation resting on it is
/// not a proof, and the cause is distinct from a rejected certificate.
fn invariants_hold(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
) -> Result<Option<bool>, PlanStopCause> {
    match super::admission(system, state) {
        Ok(super::Admission::Admitted) => Ok(Some(true)),
        Ok(super::Admission::Violated) => Ok(Some(false)),
        Ok(super::Admission::Undetermined(invariant)) => {
            let key = system
                .state_key(state)
                .map(|(key, _)| key)
                .unwrap_or_default();
            Err(PlanStopCause::InvariantUndetermined {
                invariant,
                state: key,
            })
        }
        Err(stop) => match super::expansion_stop(stop, system) {
            super::Stop::Cause(cause) => Err(cause),
            super::Stop::Negative(_) | super::Stop::Error(_) => Ok(None),
        },
    }
}

/// The objective value of the plan `states`/`steps`, recomputed: the step
/// cost sum or the objective expression; `None` when it is undefined.
fn objective_value<R: PlanRules>(
    rules: &mut R,
    kind: ObjectiveKind,
    states: &[ModelSimState],
    steps: &[ModelTransition],
) -> Result<Option<Rational>, PlanStopCause> {
    match kind {
        ObjectiveKind::General => rules
            .objective(&PlanView { states, steps })
            .map_err(super::stop_cause),
        ObjectiveKind::StepSum => {
            let Some(sum) = step_total(rules, states, steps)? else {
                return Ok(None);
            };
            rules
                .step_sum_objective(&PlanView { states, steps }, &sum)
                .map_err(super::stop_cause)
        }
    }
}

/// Recompute the ordered sum independently of the recorded node values.
fn step_total<R: PlanRules>(
    rules: &mut R,
    states: &[ModelSimState],
    steps: &[ModelTransition],
) -> Result<Option<Rational>, PlanStopCause> {
    let mut sum = zero();
    for index in 0..steps.len() {
        let view = PlanView {
            states: &states[..=index + 1],
            steps: &steps[..=index],
        };
        let Some(cost) = rules.step_cost(&view, index).map_err(super::stop_cause)? else {
            return Ok(None);
        };
        let Some(total) = rules
            .accumulate_step_cost(&view, &sum, &cost)
            .map_err(super::stop_cause)?
        else {
            return Ok(None);
        };
        sum = total;
    }
    Ok(Some(sum))
}

/// Whether entry `index`'s parent record names an earlier entry, one step
/// shallower, that records the step into `index`.
fn parent_records(entries: &[Entry], index: usize) -> bool {
    let entry = &entries[index];
    let Some((parent, step)) = &entry.parent else {
        return false;
    };
    *parent < index
        && entries[*parent].key.depth + 1 == entry.key.depth
        && entries[*parent]
            .successors
            .iter()
            .any(|(recorded, target)| recorded == step && *target == index)
}

fn goal_holds<R: PlanRules>(
    rules: &mut R,
    states: &[ModelSimState],
    steps: &[ModelTransition],
    monitors: &[Vec<u8>],
) -> Result<bool, PlanStopCause> {
    let view = PlanView { states, steps };
    if !rules.goal(&view).map_err(super::stop_cause)? {
        return Ok(false);
    }
    let Some(last) = states.last() else {
        return Ok(false);
    };
    for (policy, monitor) in monitors.iter().enumerate() {
        if !rules
            .policy_accepts(policy, monitor, last)
            .map_err(super::stop_cause)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The representative prefix of entry `index`, read from the entries. The
/// walk is bounded by the entry's step count, so a parent cycle cannot loop.
fn prefix(entries: &[Entry], mut index: usize) -> (Vec<ModelSimState>, Vec<ModelTransition>) {
    let mut states = vec![entries[index].state.clone()];
    let mut steps = Vec::new();
    for _ in 0..entries[index].key.depth {
        let Some((parent, step)) = &entries[index].parent else {
            break;
        };
        steps.push(step.clone());
        index = *parent;
        states.push(entries[index].state.clone());
    }
    states.reverse();
    steps.reverse();
    (states, steps)
}
