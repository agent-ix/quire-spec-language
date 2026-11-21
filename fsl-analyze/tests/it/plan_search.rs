// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1978: the plan search engine (FR-755, FR-1011, FR-1012) over FSpec
//! FR-650's published counter fixture: model `M`, `Counter.value` in
//! `Int[0, 20]`, `inc` then `double`, one counter `c` with `value = 1`.
//!
//! The goal, objective and policy rules are written here as the checked
//! expressions of the fixture's unit would evaluate them; the search,
//! certificates, checker and replay are the engine's.

use std::collections::BTreeMap;

use filament_exact::{Cancel, CancelCause, Integer, Rational, ScalarLimits};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_analyze::plan_search::{
    search, CertificateKind, CertificateOutcome, CertificateRule, Direction, MonitorStep,
    ObjectiveKind, PlanFault, PlanLengths, PlanLimit, PlanResult, PlanRules, PlanSearchError,
    PlanSearchLimits, PlanSearchRequest, PlanStatus, PlanStopCause, PlanView, RuleStop,
};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use ix_trace_rs::trace;
use serde_json::json;

use crate::counter::UNLIMITED;
use crate::model_world::{self, Domain, World};

const FIXTURE: &str = "proposals/planning-v1/fixtures/counter/";

/// The fixture unit's contracts on `inc` and `double`.
const CONTRACTS: &str = "pre CanInc using v on M::Counter::inc { self.value < 20 }\n\
     post Inc using v on M::Counter::inc { self.value = pre(self.value) + 1 }\n\
     pre CanDouble using v on M::Counter::double { self.value <= 10 }\n\
     post Double using v on M::Counter::double { self.value = 2 * pre(self.value) }\n";

fn counter_world() -> World {
    counter_world_with(1, "")
}

/// The counter at `initial`, its contracts followed by `extra` clauses.
fn counter_world_with(initial: i64, extra: &str) -> World {
    let document =
        filament_specification::published_file(&format!("{FIXTURE}model.semantic-ir.json"))
            .expect("FSpec publishes the counter model")
            .to_vec();
    World::new(
        Domain {
            package: "example/plan-counter",
            object: "Counter",
            population: "counters",
            document,
        },
        &format!("{CONTRACTS}{extra}"),
        model_world::INFINITE,
        &[],
        &[("c", json!({"value": {"integer": initial.to_string()}}))],
    )
}

/// The counter's policy clauses, compiled by hand to their monitors.
#[derive(Clone, Copy)]
enum Policy {
    /// `always holds(c.value != 4)`: a safety monitor.
    NotFour,
    /// `always holds(c.value != 8)`: a safety monitor.
    NotEight,
    /// `eventually holds(c.value = 3)`: a non-safety automaton.
    SomeTime,
    /// `eventually[0, 2] holds(c.value >= 4)`: a bounded safety monitor.
    SoonFour,
}

#[derive(Clone, Copy, PartialEq)]
enum Cost {
    /// The default objective, `length`.
    Length,
    /// `sum<Cost>(s in Reach.steps : if s.operation = M::Op::double then 5 else 1)`.
    DoubleCostsFive,
    /// `maximize Reach.final.value`.
    MaximizeFinal,
    /// `sum<Cost>(s in Reach.steps : if s.operation = M::Op::double then 0 else 1)`.
    FreeDouble,
    /// `maximize sum<Cost>(s in Reach.steps : 1)`: a per-step sum that is
    /// maximized, so it settles with a closure certificate.
    MaximizeLength,
    /// An objective that is undefined for every plan.
    Undefined,
}

struct Counter<'w> {
    world: &'w World,
    names: BTreeMap<String, String>,
    goal: i64,
    cost: Cost,
    policies: Vec<Policy>,
    /// An evaluation meter of `work_units` charged once per goal evaluation.
    work_units: Option<u64>,
    used: u64,
    /// Cancelled (requested) at the goal evaluation after one held.
    trip: Option<Cancel>,
    found: bool,
}

impl<'w> Counter<'w> {
    fn new(world: &'w World, system: &ModelSystem<'_>, goal: i64) -> Self {
        let names = system
            .operations()
            .into_iter()
            .map(|(_, name, text)| (text.to_owned(), name.to_owned()))
            .collect();
        Self {
            world,
            names,
            goal,
            cost: Cost::Length,
            policies: Vec::new(),
            work_units: None,
            used: 0,
            trip: None,
            found: false,
        }
    }

    fn value(&self, state: &ModelSimState) -> i64 {
        self.world.domain.integer(state, "c", "value")
    }

    fn operation(&self, step: &ModelTransition) -> &str {
        self.names.get(step.operation()).map_or("", String::as_str)
    }
}

fn integer(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

impl PlanRules for Counter<'_> {
    type StepKey = (u8, Vec<String>);

    fn direction(&self) -> Direction {
        match self.cost {
            Cost::MaximizeFinal | Cost::MaximizeLength => Direction::Maximize,
            Cost::Length | Cost::DoubleCostsFive | Cost::FreeDouble | Cost::Undefined => {
                Direction::Minimize
            }
        }
    }

    fn objective_kind(&self) -> ObjectiveKind {
        match self.cost {
            Cost::MaximizeFinal | Cost::Undefined => ObjectiveKind::General,
            Cost::Length | Cost::DoubleCostsFive | Cost::FreeDouble | Cost::MaximizeLength => {
                ObjectiveKind::StepSum
            }
        }
    }

    fn merges_prefixes(&self) -> bool {
        true
    }

    /// FSpec FR-804: `inc` is declared before `double`.
    fn step_key(&self, step: &ModelTransition) -> Self::StepKey {
        let order = match self.operation(step) {
            "inc" => 0,
            "double" => 1,
            _ => 2,
        };
        (order, step.arguments().to_vec())
    }

    fn policy_count(&self) -> usize {
        self.policies.len()
    }

    fn policy_start(
        &mut self,
        policy: usize,
        state: &ModelSimState,
    ) -> Result<MonitorStep, RuleStop> {
        self.policy_advance(policy, &[0], state)
    }

    fn policy_advance(
        &mut self,
        policy: usize,
        monitor: &[u8],
        state: &ModelSimState,
    ) -> Result<MonitorStep, RuleStop> {
        let value = self.value(state);
        let seen = monitor.first().copied().unwrap_or(0);
        Ok(match self.policies[policy] {
            Policy::NotFour if value == 4 => MonitorStep::Reject,
            Policy::NotEight if value == 8 => MonitorStep::Reject,
            Policy::NotFour | Policy::NotEight => MonitorStep::Continue(vec![0]),
            Policy::SomeTime => MonitorStep::Continue(vec![u8::from(seen == 1 || value == 3)]),
            // The monitor counts positions up to the deadline; 255 is "met".
            Policy::SoonFour if seen == 255 || value >= 4 => MonitorStep::Continue(vec![255]),
            Policy::SoonFour if seen >= 2 => MonitorStep::Reject,
            Policy::SoonFour => MonitorStep::Continue(vec![seen + 1]),
        })
    }

    fn policy_accepts(
        &mut self,
        policy: usize,
        monitor: &[u8],
        last: &ModelSimState,
    ) -> Result<bool, RuleStop> {
        let seen = monitor.first().copied().unwrap_or(0);
        Ok(match self.policies[policy] {
            Policy::NotFour | Policy::NotEight => true,
            Policy::SomeTime => seen == 1,
            // The terminal stutter repeats `last` up to the deadline.
            Policy::SoonFour => seen == 255 || self.value(last) >= 4,
        })
    }

    fn goal(&mut self, plan: &PlanView<'_>) -> Result<bool, RuleStop> {
        if let Some(bound) = self.work_units {
            if self.used >= bound {
                return Err(RuleStop::Accounting {
                    counter: "work_units",
                    value: bound,
                });
            }
            self.used += 1;
        }
        let holds = plan
            .states
            .last()
            .is_some_and(|state| self.value(state) == self.goal);
        // Cancel on the evaluation after the first plan was found.
        if self.found {
            if let Some(cancel) = &self.trip {
                cancel.cancel(CancelCause::Requested);
            }
        }
        self.found |= holds;
        Ok(holds)
    }

    fn step_cost(
        &mut self,
        plan: &PlanView<'_>,
        index: usize,
    ) -> Result<Option<Rational>, RuleStop> {
        Ok(Some(match self.cost {
            Cost::DoubleCostsFive if self.operation(&plan.steps[index]) == "double" => integer(5),
            Cost::FreeDouble if self.operation(&plan.steps[index]) == "double" => integer(0),
            _ => integer(1),
        }))
    }

    fn objective(&mut self, plan: &PlanView<'_>) -> Result<Option<Rational>, RuleStop> {
        if self.cost == Cost::Undefined {
            return Ok(None);
        }
        Ok(plan.states.last().map(|state| integer(self.value(state))))
    }

    fn begin_replay(&mut self) {
        // The replay runs under its own allowance.
        self.used = 0;
    }
}

fn run(
    rules: &mut Counter<'_>,
    system: &ModelSystem<'_>,
    lengths: (u64, u64),
    limits: PlanSearchLimits,
    fault: Option<&PlanFault>,
) -> Result<PlanResult, PlanSearchError> {
    run_with(rules, system, lengths, limits, fault, &Cancel::new())
}

fn run_with(
    rules: &mut Counter<'_>,
    system: &ModelSystem<'_>,
    lengths: (u64, u64),
    limits: PlanSearchLimits,
    fault: Option<&PlanFault>,
    cancel: &Cancel,
) -> Result<PlanResult, PlanSearchError> {
    search(
        system,
        rules,
        PlanSearchRequest {
            lengths: PlanLengths::new(lengths.0, lengths.1).expect("a length range"),
            limits,
            cancel,
            fault,
        },
    )
}

fn operations(rules: &Counter<'_>, result: &PlanResult) -> Vec<String> {
    result
        .plan
        .iter()
        .flat_map(|plan| plan.steps.iter())
        .map(|step| rules.operation(step).to_owned())
        .collect()
}

fn limited(limit: PlanLimit, value: u64) -> PlanSearchLimits {
    let mut limits = PlanSearchLimits::default();
    limits.set(limit, value.try_into().expect("a positive limit"));
    limits
}

/// FSpec FR-650-AC-1, FR-804-AC-1 and FR-755-AC-1: `length` gives `optimal`
/// `inc, double, double` at 3, the first of the two 3-step plans, with a
/// `potential` certificate the core accepts.
#[test]
#[trace("TC-2105", "FR-755-AC-1")]
#[trace("TC-2862", "FR-1012-AC-2")]
fn the_counter_plans_by_length_with_a_potential_certificate() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let mut rules = Counter::new(&world, &system, 8);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(operations(&rules, &result), ["inc", "double", "double"]);
    assert_eq!(result.objective, Some(integer(3)));
    let certificate = result.certificate.expect("a certificate");
    assert_eq!(certificate.kind, CertificateKind::Potential);
    assert_eq!(certificate.outcome, CertificateOutcome::Accepted);
    assert!(result.certified);
    let plan = result.plan.expect("a plan");
    assert_eq!(plan.states.len(), 4);
    let values: Vec<i64> = plan.states.iter().map(|state| rules.value(state)).collect();
    assert_eq!(values, [1, 2, 4, 8]);
    // FSpec FR-804-AC-4: the same request gives the same counts and plan.
    let again = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(again.explored, result.explored);
    assert_eq!(operations(&rules, &again), ["inc", "double", "double"]);
}

/// FSpec FR-650-AC-2 and FR-755-AC-1: the `double`-costs-five sum gives
/// `inc, inc, inc, double` at 8.
#[test]
#[trace("TC-2105", "FR-755-AC-1")]
fn the_counter_plans_by_step_cost() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let mut rules = Counter::new(&world, &system, 8);
    rules.cost = Cost::DoubleCostsFive;
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(operations(&rules, &result), ["inc", "inc", "inc", "double"]);
    assert_eq!(result.objective, Some(integer(8)));
    let certificate = result.certificate.expect("a certificate");
    assert_eq!(certificate.kind, CertificateKind::Potential);
    assert!(result.certified);
}

/// FSpec FR-650-AC-3, FR-755-AC-2: goal 21 and `domain Int[0, 2]` are each
/// `infeasible` with an accepted closure certificate.
#[test]
#[trace("TC-2105", "FR-755-AC-2")]
fn unreachable_goals_are_infeasible_by_a_closure_certificate() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    for (goal, lengths) in [(21, (0, 5)), (8, (0, 2))] {
        let mut rules = Counter::new(&world, &system, goal);
        let result = run(
            &mut rules,
            &system,
            lengths,
            PlanSearchLimits::default(),
            None,
        )
        .unwrap();
        assert_eq!(result.status, PlanStatus::Infeasible);
        assert!(result.plan.is_none());
        let certificate = result.certificate.expect("a certificate");
        assert_eq!(certificate.kind, CertificateKind::Closure);
        assert_eq!(certificate.outcome, CertificateOutcome::Accepted);
        assert!(result.certified);
    }
}

/// FSpec FR-650-AC-4, FR-803-AC-1/2/3/5 and FR-755-AC-3: a safety policy
/// prunes, a non-safety policy is decided on each completed plan, a bounded
/// one prunes at its deadline, and a policy every goal plan breaks is
/// `infeasible`.
#[test]
#[trace("TC-2105", "FR-755-AC-3")]
#[trace("TC-2862", "FR-1012-AC-1")]
fn plans_are_held_to_their_policies() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let plan = |policy: Policy| {
        let mut rules = Counter::new(&world, &system, 8);
        rules.policies = vec![policy];
        let result = run(
            &mut rules,
            &system,
            (0, 5),
            PlanSearchLimits::default(),
            None,
        )
        .unwrap();
        let values: Vec<i64> = result
            .plan
            .iter()
            .flat_map(|plan| plan.states.iter())
            .map(|state| rules.value(state))
            .collect();
        (
            result.status,
            operations(&rules, &result),
            values,
            result.certified,
        )
    };
    let (status, _, values, certified) = plan(Policy::NotFour);
    assert_eq!(status, PlanStatus::Optimal);
    assert!(certified);
    assert!(!values.contains(&4), "{values:?}");
    let (status, steps, values, _) = plan(Policy::SomeTime);
    assert_eq!(status, PlanStatus::Optimal);
    assert_eq!(steps, ["inc", "inc", "inc", "double"]);
    assert!(values.contains(&3));
    let (status, steps, _, _) = plan(Policy::SoonFour);
    assert_eq!(status, PlanStatus::Optimal);
    assert_eq!(steps, ["inc", "double", "double"]);
    let (status, _, _, certified) = plan(Policy::NotEight);
    assert_eq!(status, PlanStatus::Infeasible);
    assert!(certified);
}

/// FSpec FR-801-AC-8 and FR-1012-AC-6: the canonical exploration pins the
/// counts of a stopped search, with no plan.
#[test]
#[trace("TC-2862", "FR-1012-AC-6")]
#[trace("TC-2861", "FR-1011-AC-1")]
fn the_canonical_exploration_pins_stopped_counts() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    for (limit, value, states, work) in [
        (PlanLimit::MaxStates, 2, 2, 6),
        (PlanLimit::MaxWork, 1, 1, 1),
    ] {
        let mut rules = Counter::new(&world, &system, 8);
        let result = run(&mut rules, &system, (0, 5), limited(limit, value), None).unwrap();
        assert_eq!(result.status, PlanStatus::Incomplete);
        assert_eq!(
            (result.explored.states, result.explored.work),
            (states, work)
        );
        assert!(result.plan.is_none());
        assert_eq!(
            result.cause,
            Some(PlanStopCause::LimitReached {
                limit: limit.member(),
                value,
                setting: limit.setting().to_owned(),
            })
        );
    }
}

/// FSpec FR-801-AC-2 and FR-1011-AC-2: a denied evaluation-meter charge
/// names its `accounting` setting, not a `plan_search` one.
#[test]
#[trace("TC-2861", "FR-1011-AC-2")]
fn an_exhausted_evaluation_meter_names_its_accounting_setting() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let mut rules = Counter::new(&world, &system, 8);
    rules.work_units = Some(1);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    let Some(PlanStopCause::LimitReached { setting, .. }) = result.cause else {
        panic!("a limit stop: {:?}", result.cause);
    };
    assert_eq!(setting, "accounting.work_units");
}

/// FSpec FR-650-AC-5/AC-7 and FR-755-AC-2: the certificate tamper seam makes
/// the core reject `closure` at entry 0; a rejected potential demotes the
/// plan to `feasible`, and a rejected closure for `infeasible` leaves the
/// run `incomplete` with no plan.
#[test]
#[trace("TC-2105", "FR-755-AC-2")]
fn a_tampered_certificate_is_rejected_by_the_core() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let rejected = CertificateOutcome::Rejected {
        rule: CertificateRule::Closure,
        entry: 0,
    };
    let mut rules = Counter::new(&world, &system, 8);
    let fault = PlanFault::DropSuccessor {
        entry: 0,
        successor: 1,
    };
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        Some(&fault),
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Feasible);
    assert_eq!(operations(&rules, &result), ["inc", "double", "double"]);
    assert_eq!(result.certificate.expect("a certificate").outcome, rejected);
    assert!(!result.certified);

    let mut rules = Counter::new(&world, &system, 21);
    let fault = PlanFault::DropSuccessor {
        entry: 0,
        successor: 0,
    };
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        Some(&fault),
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert!(result.plan.is_none());
    assert_eq!(
        result.cause,
        Some(PlanStopCause::CertificateRejected {
            rule: CertificateRule::Closure,
            entry: 0,
        })
    );
}

/// FSpec FR-650-AC-6 and FR-755-AC-4: a substituted plan whose fifth
/// `double` is disabled is a method failure naming step 4.
#[test]
#[trace("TC-2105", "FR-755-AC-4")]
fn a_substituted_plan_that_fails_replay_is_a_method_failure() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let mut rules = Counter::new(&world, &system, 8);
    let fault = PlanFault::ReportPlan(vec!["double".to_owned(); 5]);
    let error = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        Some(&fault),
    )
    .expect_err("replay fails");
    assert_eq!(
        error,
        PlanSearchError::ReplayFailed {
            step: 4,
            operation: "double".to_owned(),
        }
    );
}

/// FSpec FR-800-AC-3 and FR-802-AC-6: a goal true at the initial state is
/// the empty plan; `maximize Reach.final.value` is general and settles with
/// a `closure` certificate.
#[test]
#[trace("TC-2862", "FR-1012-AC-2")]
fn the_empty_plan_and_a_general_objective() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let mut rules = Counter::new(&world, &system, 1);
    let result = run(
        &mut rules,
        &system,
        (0, 3),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert!(result
        .plan
        .as_ref()
        .is_some_and(|plan| plan.steps.is_empty()));
    assert_eq!(result.objective, Some(integer(0)));

    let mut rules = Counter::new(&world, &system, 8);
    rules.cost = Cost::MaximizeFinal;
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(result.objective, Some(integer(8)));
    let certificate = result.certificate.expect("a certificate");
    assert_eq!(certificate.kind, CertificateKind::Closure);
    assert_eq!(certificate.outcome, CertificateOutcome::Accepted);
}

fn world_system(world: &World) -> ModelSystem<'_> {
    ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits")
}

/// FSpec FR-650 "admissible": an invariant that excludes the otherwise-best
/// plan changes the answer, and an invariant is evaluated at the states at
/// the maximum plan length too.
#[test]
#[trace("TC-2105", "FR-755-AC-1")]
fn a_violated_invariant_makes_a_state_inadmissible() {
    let world = counter_world_with(
        1,
        "invariant NotFour using v on M::Counter at current { self.value != 4 }\n",
    );
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    // `inc, double, double` and `inc, inc, inc, double` pass through 4.
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(
        operations(&rules, &result),
        ["inc", "inc", "double", "inc", "inc"]
    );
    assert_eq!(result.objective, Some(integer(5)));
    assert!(result.certified);

    // The same invariant at the maximum length: no plan of at most 3 steps.
    let world = counter_world_with(
        1,
        "invariant NotEight using v on M::Counter at current { self.value != 8 }\n",
    );
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    let result = run(
        &mut rules,
        &system,
        (0, 3),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Infeasible);
    assert!(result.certified);
}

/// FSpec FR-650: replay rejects a plan through a state that breaks an
/// invariant, which the substitution seam can build.
#[test]
#[trace("TC-2105", "FR-755-AC-4")]
fn replay_rejects_a_plan_through_a_violated_invariant() {
    let world = counter_world();
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    // Without the invariant the plan is a good plan; with it, it is not.
    let plain = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(plain.status, PlanStatus::Optimal);
    let world = counter_world_with(
        1,
        "invariant NotFour using v on M::Counter at current { self.value != 4 }\n",
    );
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    let fault = PlanFault::ReportPlan(vec![
        "inc".to_owned(),
        "double".to_owned(),
        "double".to_owned(),
    ]);
    let error = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        Some(&fault),
    )
    .expect_err("replay rejects");
    assert!(
        matches!(error, PlanSearchError::ReplayRejected(_)),
        "{error:?}"
    );
}

/// FSpec FR-801-AC-2/AC-3: a stop by the evaluation meter or a cancellation
/// is `incomplete` carrying the best plan found, never a method failure.
#[test]
#[trace("TC-1951", "FR-801-AC-3")]
fn a_stopped_search_still_reports_its_best_plan() {
    let world = counter_world();
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 2);
    rules.work_units = Some(2);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert_eq!(operations(&rules, &result), ["inc"]);
    assert!(matches!(
        result.cause,
        Some(PlanStopCause::LimitReached { .. })
    ));

    let cancel = Cancel::new();
    let mut rules = Counter::new(&world, &system, 2);
    rules.trip = Some(cancel.clone());
    let result = run_with(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
        &cancel,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert_eq!(operations(&rules, &result), ["inc"]);
    assert_eq!(
        result.cause,
        Some(PlanStopCause::Cancelled(CancelCause::Requested))
    );
}

/// FSpec FR-802: when every goal plan has an undefined objective, the space
/// holds no admissible plan and the closure certificate is accepted.
#[test]
#[trace("TC-2105", "FR-755-AC-2")]
fn an_undefined_objective_everywhere_is_infeasible() {
    let world = counter_world();
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    rules.cost = Cost::Undefined;
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Infeasible);
    assert!(result.certified);
    assert!(result.explored.undefined_objectives > 0);
}

/// FSpec FR-650's potential `goal` rule is `pi <= 0` at goal nodes only in
/// one direction: a zero-cost step into the goal from a non-goal node is
/// still `optimal`.
#[test]
#[trace("TC-2105", "FR-755-AC-1")]
fn a_zero_cost_step_into_the_goal_is_still_optimal() {
    let world = counter_world();
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    rules.cost = Cost::FreeDouble;
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(operations(&rules, &result), ["double", "double", "double"]);
    assert_eq!(result.objective, Some(integer(0)));
    let certificate = result.certificate.expect("a certificate");
    assert_eq!(certificate.kind, CertificateKind::Potential);
    assert_eq!(certificate.outcome, CertificateOutcome::Accepted);
}

/// FSpec FR-650: a per-step sum settled by a closure certificate has its
/// improvement rule checked against the checker's own cost sum.
#[test]
#[trace("TC-2105", "FR-755-AC-1")]
fn a_maximized_step_sum_settles_by_closure() {
    let world = counter_world();
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    rules.cost = Cost::MaximizeLength;
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(result.objective, Some(integer(5)));
    let certificate = result.certificate.expect("a certificate");
    assert_eq!(certificate.kind, CertificateKind::Closure);
    assert_eq!(certificate.outcome, CertificateOutcome::Accepted);
}

/// FSpec FR-650: a forged entry state, and a parent record that points at
/// the entry itself, are each rejected at that entry (and terminate).
#[test]
#[trace("TC-2105", "FR-755-AC-2")]
fn a_forged_state_and_a_parent_cycle_are_rejected() {
    let world = counter_world();
    let system = world_system(&world);
    for fault in [
        PlanFault::ForgeEntryState {
            entry: 1,
            source: 0,
        },
        PlanFault::ParentCycle { entry: 1 },
    ] {
        let mut rules = Counter::new(&world, &system, 8);
        let result = run(
            &mut rules,
            &system,
            (0, 5),
            PlanSearchLimits::default(),
            Some(&fault),
        )
        .unwrap();
        assert_eq!(result.status, PlanStatus::Feasible, "{fault:?}");
        assert_eq!(
            result.certificate.expect("a certificate").outcome,
            CertificateOutcome::Rejected {
                rule: CertificateRule::Closure,
                entry: 1,
            },
            "{fault:?}"
        );
    }
}

/// FSpec FR-650: replay recomputes the objective, so a substituted plan
/// that is admissible but worse than the certified value is rejected.
#[test]
#[trace("TC-2105", "FR-755-AC-4")]
fn a_substituted_worse_plan_is_rejected() {
    let world = counter_world();
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    let fault = PlanFault::ReportPlan(["inc", "inc", "inc", "double"].map(str::to_owned).to_vec());
    let error = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        Some(&fault),
    )
    .expect_err("replay rejects");
    assert!(
        matches!(error, PlanSearchError::ReplayRejected(_)),
        "{error:?}"
    );
}

/// FSpec FR-801 "Charge order" step 4: a disabled step is charged one
/// `max_work` and yields no successor. From `value = 11`, `double` is
/// disabled; the run charges the initial admission and goal, `inc` applied,
/// its node admitted and its goal and objective, then the disabled `double`.
#[test]
#[trace("TC-1951", "FR-801-AC-8")]
fn a_disabled_step_is_charged() {
    let world = counter_world_with(11, "");
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 12);
    let result = run(
        &mut rules,
        &system,
        (0, 1),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(
        (result.explored.states, result.explored.work),
        (2, 5),
        "{:?}",
        result.explored
    );
    // The fifth unit is the disabled `double`: one fewer stops there.
    let mut rules = Counter::new(&world, &system, 12);
    let result = run(
        &mut rules,
        &system,
        (0, 1),
        limited(PlanLimit::MaxWork, 4),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert_eq!(
        (result.explored.states, result.explored.work),
        (2, 4),
        "{:?}",
        result.explored
    );
    assert_eq!(operations(&rules, &result), ["inc"]);
}

/// A meter on which the cube invariant below is undetermined from
/// `value = 4` up: its product needs more than 5 integer bits there, while
/// the counter's own operations (results of at most 20) stay within 5.
const SMALL_INTEGERS: ScalarLimits = ScalarLimits {
    integer_bits: 5,
    ..UNLIMITED
};

/// The cube invariant that the `SMALL_INTEGERS` meter cannot decide from
/// `value = 4` up (an evaluation stopped by the meter: FR-120 "Invariants").
const CUBE: &str =
    "invariant Cube using v on M::Counter at current { self.value * self.value * self.value >= 0 }\n";

fn undecided_system(world: &World) -> ModelSystem<'_> {
    ModelSystem::new(world.subject(&["c"], SMALL_INTEGERS).simulation).expect("admits")
}

/// An undetermined invariant is not a violation: when the only goal path
/// crosses such a state the result is `incomplete` naming the invariant,
/// never `infeasible`.
#[test]
#[trace("TC-2105", "FR-755-AC-2")]
#[trace("TC-2862", "FR-1012-AC-7")]
fn an_undetermined_invariant_on_the_only_path_is_incomplete() {
    let world = counter_world_with(1, CUBE);
    let system = undecided_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert!(!result.certified);
    assert!(result.plan.is_none());
    assert!(matches!(
        &result.cause,
        Some(PlanStopCause::InvariantUndetermined { invariant, .. }) if invariant == "Cube"
    ));
}

/// A found plan plus an undecided prune elsewhere is `incomplete` carrying
/// the plan, never `optimal`; a replay of a plan through the undecided state
/// is not admissible evidence.
#[test]
#[trace("TC-2105", "FR-755-AC-2")]
#[trace("TC-2862", "FR-1012-AC-7")]
fn an_undecided_prune_caps_a_found_plan_at_incomplete() {
    let world = counter_world_with(1, CUBE);
    let system = undecided_system(&world);
    let mut rules = Counter::new(&world, &system, 2);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert!(!result.certified);
    assert!(result.certificate.is_none());
    assert_eq!(operations(&rules, &result), ["inc"]);
    assert_eq!(result.objective, Some(integer(1)));
    assert!(matches!(
        &result.cause,
        Some(PlanStopCause::InvariantUndetermined { invariant, .. }) if invariant == "Cube"
    ));

    // `inc, double` crosses the undecided state 4.
    let fault = PlanFault::ReportPlan(["inc", "double"].map(str::to_owned).to_vec());
    let mut rules = Counter::new(&world, &system, 2);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        Some(&fault),
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert!(result.plan.is_none());
}

/// A source plan with one non-safety policy whose truth needs continuation.
fn source_policy_world(start: &[(&str, serde_json::Value)]) -> World {
    World::new(
        model_world::counter(),
        "type Lengths = Int[0, 1];\n\
         synthesis Reach using t grammar M::Counter domain Lengths satisfies { true };\n",
        model_world::INFINITE,
        &[model_world::Clause {
            activation: "on origin",
            formula: "eventually holds(c.value = 3)",
        }],
        start,
    )
}

/// Policy prefixes, separate policies and terminal reads all use the
/// request meter; a successful exact-bound first read leaves no second budget.
#[test]
#[trace("FR-1011-AC-2", "FSpec-FR-801-AC-2")]
fn source_policy_reads_share_the_request_work_counter() {
    use filament_exact::{LimitKind, Meter};
    use fsl_analyze::plan_source::SourcePlan;
    use fsl_eval::simulation::model::{read_clause, ClauseReading};
    use fsl_eval::simulation::TransitionSystem;

    let world = source_policy_world(&[("c", json!({"value": {"integer": "0"}}))]);
    let system = world_system(&world);
    let state = system
        .initial()
        .into_iter()
        .next()
        .expect("one initial state");
    let node = &world.package.graph().temporal_clauses()[0];
    let mut calibration = Meter::new(UNLIMITED);
    assert_eq!(
        read_clause(
            &system,
            node,
            std::slice::from_ref(&state),
            false,
            &mut calibration
        )
        .expect("the policy reads the initial prefix"),
        ClauseReading::Undecided,
    );
    let one_read = calibration.consumed(LimitKind::WorkUnits);
    assert!(one_read > 0, "the policy read must charge work");
    let limits = ScalarLimits {
        work_units: one_read,
        ..UNLIMITED
    };
    let synthesis = &world.package.graph().syntheses()[0];
    for next in ["other-policy", "advance", "terminal"] {
        let mut rules =
            SourcePlan::new(&world.package, synthesis, &system, vec![node, node], limits)
                .expect("the source plan and policies are checked");
        let MonitorStep::Continue(monitor) = rules
            .policy_start(0, &state)
            .expect("the exact-bound first policy read completes")
        else {
            panic!("an unsettled prefix is retained");
        };
        let stopped = match next {
            "other-policy" => rules.policy_start(1, &state).map(|_| ()),
            "advance" => rules.policy_advance(0, &monitor, &state).map(|_| ()),
            "terminal" => rules.policy_accepts(0, &monitor, &state).map(|_| ()),
            _ => unreachable!("the listed read modes are exhaustive"),
        };
        assert!(
            matches!(stopped,
                Err(RuleStop::Accounting { counter: "work_units", value }) if value == one_read
            ),
            "the next policy read uses the exhausted request budget: {stopped:?}"
        );
    }
}

/// A policy quantified over two objects shares one budget across instances.
#[test]
#[trace("FR-1011-AC-2", "FSpec-FR-801-AC-2")]
fn source_policy_instances_share_the_request_work_counter() {
    use filament_exact::{LimitKind, Meter};
    use fsl_eval::simulation::model::{read_clause, ClauseReading};
    use fsl_eval::simulation::TransitionSystem;

    let both = source_policy_world(&[
        ("c", json!({"value": {"integer": "0"}})),
        ("d", json!({"value": {"integer": "0"}})),
    ]);
    let system = ModelSystem::new(both.subject(&["c", "d"], UNLIMITED).simulation)
        .expect("both objects are in the universe");
    let state = system
        .initial()
        .into_iter()
        .next()
        .expect("one initial state");
    let node = &both.package.graph().temporal_clauses()[0];
    let mut calibration = Meter::new(UNLIMITED);
    assert_eq!(
        read_clause(
            &system,
            node,
            std::slice::from_ref(&state),
            false,
            &mut calibration,
        )
        .expect("both instances read with an unlimited budget"),
        ClauseReading::Undecided
    );
    let total_work = calibration.consumed(LimitKind::WorkUnits);
    assert!(
        total_work > 1,
        "two nontrivial policy instances charge work"
    );
    let budget = total_work - 1;
    let mut meter = Meter::new(ScalarLimits {
        work_units: budget,
        ..UNLIMITED
    });
    assert_eq!(
        read_clause(
            &system,
            &both.package.graph().temporal_clauses()[0],
            std::slice::from_ref(&state),
            false,
            &mut meter,
        )
        .expect("accounting is a typed policy reading"),
        ClauseReading::Incomplete {
            counter: "work_units",
            value: budget
        }
    );
}

/// Closing an unmet eventual policy rejects the candidate; quantifying the
/// same policy over an empty population holds vacuously.
#[test]
#[trace("FSpec-FR-803-AC-2", "FSpec-FR-803-AC-5")]
fn source_policy_acceptance_requires_closed_truth_and_preserves_vacuity() {
    use filament_exact::Meter;
    use fsl_analyze::plan_source::SourcePlan;
    use fsl_eval::simulation::model::{read_clause, ClauseReading};
    use fsl_eval::simulation::TransitionSystem;

    for empty in [false, true] {
        let objects = if empty {
            Vec::new()
        } else {
            vec![("c", json!({"value": {"integer": "0"}}))]
        };
        let world = source_policy_world(&objects);
        let keys = if empty { vec![] } else { vec!["c"] };
        let system = ModelSystem::new(world.subject(&keys, UNLIMITED).simulation)
            .expect("the declared population is complete");
        let state = system
            .initial()
            .into_iter()
            .next()
            .expect("one initial state");
        let node = &world.package.graph().temporal_clauses()[0];
        let mut rules = SourcePlan::new(
            &world.package,
            &world.package.graph().syntheses()[0],
            &system,
            vec![node],
            UNLIMITED,
        )
        .expect("the source plan is checked");
        let MonitorStep::Continue(monitor) = rules
            .policy_start(0, &state)
            .expect("the initial policy prefix reads")
        else {
            panic!("the initial prefix cannot refute eventual truth");
        };
        assert_eq!(
            rules
                .policy_accepts(0, &monitor, &state)
                .expect("the closed policy reads"),
            empty
        );
        assert_eq!(
            read_clause(
                &system,
                node,
                std::slice::from_ref(&state),
                true,
                &mut Meter::new(UNLIMITED),
            )
            .expect("the closed policy is readable"),
            if empty {
                ClauseReading::Holds
            } else {
                ClauseReading::Violated
            }
        );
    }
}
