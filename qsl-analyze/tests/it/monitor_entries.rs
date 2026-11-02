// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251 "Transitions" (TC-1081 steps 4, 6-8): the state-passing entries
//! generated as complete-V1 source are checked by the one checker and run by
//! `qsl-eval`'s function evaluator against the tick-trace reference.

use qsl_analyze::monitor_entries::{render, BoolExpr, EntrySpec, IntervalNode, Thresholds};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use quire_semantic_value::checking::CheckingLimits;

/// Eventually within [0, 3000] ticks at uncertainty 0 (cover width 1).
pub(crate) fn deadline_spec() -> EntrySpec {
    EntrySpec {
        atoms: vec![0, 1],
        node: IntervalNode {
            future: true,
            dual: false,
            phi: None,
            psi: BoolExpr::Atom(1),
            thresholds: Thresholds {
                all_lo: 1,
                all_hi: 2999,
                none_lo: -1,
                none_hi: 3001,
                zero_in: true,
            },
            capacity: 8,
            subformula: 3,
        },
        subject: "C".to_owned(),
        max_reading_gap: 100,
        activation_capacity: 4,
    }
}

pub(crate) fn package_of(declarations: &str) -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         {declarations}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    if !parsed.is_admissible() {
        let at = parsed
            .diagnostics()
            .iter()
            .filter_map(|d| {
                d.region
                    .as_ref()
                    .map(|r| (r.start() as usize, r.end() as usize))
            })
            .next()
            .unwrap_or((0, 0));
        let from = at.0.saturating_sub(60);
        panic!(
            "{:?}\n...{}[[{}]]{}",
            parsed
                .diagnostics()
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>(),
            &text[from..at.0],
            &text[at.0..at.1],
            &text[at.1..(at.1 + 60).min(text.len())]
        );
    }
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler builds the package declarations");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}\n{text}")),
    )
}

#[test]
fn the_generated_declarations_check() {
    let _ = package_of(&render(&deadline_spec()));
}

// ---------------------------------------------------------------------
// The differential (FR-251-AC-7): the generated entries, run by qsl-eval's
// function evaluator under a simulated runtime, against the tick-trace
// reference evaluator.

use std::collections::BTreeMap;

use qsl_analyze::monitor_entries::{spec_from_plan, EntryRefusal};
use qsl_eval::temporal::{TickBound, TickMonitorReference, TickMonitorSpec, TickOutcome, TickStep};
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_replay::TimedMonitorPlan;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{Meter, Outcome, ScalarLimits, Value};

use crate::monitor_plan::{base_target, checked_clause, premise_plan};

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

/// One input of a trace.
#[derive(Clone, Debug)]
pub(crate) enum Input {
    Reading(u64, Vec<(usize, bool)>),
    Watermark(u64),
    Poll(u64),
}

/// The code of an outcome, as the entries keep it.
fn reference_code(outcome: TickOutcome) -> i64 {
    match outcome {
        TickOutcome::True => 1,
        TickOutcome::False => 2,
        TickOutcome::Pending => 3,
        TickOutcome::Indeterminate => 4,
        TickOutcome::Incomplete { .. } => 5,
        TickOutcome::Failed { .. } => 6,
    }
}

fn reference_spec<'p>(plan: &'p TimedMonitorPlan) -> TickMonitorSpec<'p> {
    TickMonitorSpec {
        formula: plan.program().formula(),
        activation: plan.program().activation(),
        bounds: plan
            .intervals()
            .iter()
            .map(|(id, interval)| {
                (
                    id.index(),
                    TickBound {
                        lower: interval.lower().clone(),
                        lower_closed: interval.lower_closed(),
                        upper: interval.upper().clone(),
                        upper_closed: interval.upper_closed(),
                    },
                )
            })
            .collect(),
        uncertainty: plan.uncertainty().clone(),
        width_bits: plan.binding().width_bits(),
        max_reading_gap: plan.max_reading_gap(),
        capacities: plan
            .buffers()
            .iter()
            .map(|(id, c)| (id.index(), *c))
            .collect(),
    }
}

/// Per input, the sorted `(origin, code)` results of the interval node.
pub(crate) fn run_reference(
    plan: &TimedMonitorPlan,
    node: usize,
    inputs: &[Input],
) -> Vec<Vec<(i128, i64)>> {
    let mut monitor = TickMonitorReference::new(reference_spec(plan));
    inputs
        .iter()
        .map(|input| {
            let step = match input {
                Input::Reading(reading, atoms) => TickStep::Reading {
                    reading: *reading,
                    atoms: atoms.iter().copied().collect::<BTreeMap<_, _>>(),
                    guard: true,
                },
                Input::Watermark(boundary) => TickStep::Watermark {
                    boundary: *boundary,
                },
                Input::Poll(reading) => TickStep::Poll { reading: *reading },
            };
            let mut results: Vec<(i128, i64)> = monitor
                .step(&step)
                .expect("the step applies")
                .into_iter()
                .filter(|result| result.subformula == node)
                .map(|result| (result.origin, reference_code(result.outcome)))
                .collect();
            results.sort_unstable();
            results
        })
        .collect()
}

/// Per input, the sorted `(origin, code)` results of the generated entries,
/// with this function playing the runtime: the baseline, the modular delta
/// and the reading-gap fault (RT FR-276, QSL FR-251 "Counter faults").
pub(crate) fn run_entries(plan: &TimedMonitorPlan, inputs: &[Input]) -> Vec<Vec<(i128, i64)>> {
    let spec = spec_from_plan(plan).expect("the plan is in the generated fragment");
    let modulus = 1_u128 << plan.binding().width_bits();
    let mut body = String::from(
        "function scenario using v(): Sequence<Integer> pure { let s0 = monitorInit() in ",
    );
    let mut baseline: Option<u64> = None;
    let mut encoded = Vec::new();
    let mut step_no = 0;
    for input in inputs {
        let call = match input {
            Input::Poll(_) => {
                encoded.push("marker()".to_owned());
                continue;
            }
            Input::Watermark(boundary) => {
                format!("onWatermarkR(s{step_no}, {boundary}, Scope {{ subject: 0 }})")
            }
            Input::Reading(reading, atoms) => {
                let delta = baseline.map_or(0, |base| {
                    u64::try_from((u128::from(*reading) + modulus - u128::from(base)) % modulus)
                        .unwrap()
                });
                let fault = baseline.is_some() && delta > plan.max_reading_gap();
                baseline = Some(*reading);
                if fault {
                    format!("onGapR(s{step_no}, {delta})")
                } else {
                    let fields = spec
                        .atoms
                        .iter()
                        .map(|atom| {
                            let value = atoms
                                .iter()
                                .find(|(a, _)| a == atom)
                                .map_or(false, |(_, v)| *v);
                            format!("a{atom}: {value}")
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("onEventR(s{step_no}, {delta}, Inputs {{ {fields} }})")
                }
            }
        };
        step_no += 1;
        let _ = write!(
            body,
            "let t{step_no} = {call} in let s{step_no} = t{step_no}.state in "
        );
        encoded.push(format!("encode(t{step_no}.results)"));
        encoded.push("marker()".to_owned());
    }
    let joined = encoded
        .into_iter()
        .reduce(|acc, next| format!("cat({acc}, {next})"))
        .unwrap_or_else(|| "marker()".to_owned());
    let _ = write!(body, "{joined} }}");
    let helpers = "function marker using v(): Sequence<Integer> pure { sequence[0 - 1] }\n\
        function pairInts using v(a: Sequence<Integer>, b: Sequence<Integer>): Sequence<Sequence<Integer>> pure { sequence[a, b] }\n\
        function cat using v(a: Sequence<Integer>, b: Sequence<Integer>): Sequence<Integer> pure { flatten(pairInts(a, b)) }\n\
        function code using v(o: Outcome): Integer pure { case o { Outcome::Failed(f): 6; Outcome::False: 2; Outcome::Incomplete(l): 5; Outcome::Indeterminate: 4; Outcome::Pending: 3; Outcome::True: 1; } }\n\
        function encode using v(rs: Results): Sequence<Integer> pure { map(r in rs: r.instance.origin_tick * 8 + code(r.outcome)) }\n";
    let package = package_of(&format!("{}\n{helpers}\n{body}", render(&spec)));
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("scenario").expect("an identifier"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call runs");
    let FamilyOutcome::Evaluated(Outcome::Completed(Value::Collection(flat))) = evaluation.outcome
    else {
        panic!("a completed sequence, not {:?}", evaluation.outcome);
    };
    let mut steps: Vec<Vec<(i128, i64)>> = vec![Vec::new()];
    for element in flat.elements() {
        let Value::Integer(value) = element else {
            panic!("integers")
        };
        let number: i128 = value.as_big().try_into().expect("small");
        if number < 0 {
            steps.push(Vec::new());
        } else {
            steps
                .last_mut()
                .unwrap()
                .push((number / 8, i64::try_from(number % 8).unwrap()));
        }
    }
    steps.pop();
    steps.iter_mut().for_each(|step| step.sort_unstable());
    steps
}

use std::fmt::Write as _;

pub(crate) fn deadline_plan() -> TimedMonitorPlan {
    premise_plan(
        "eventually[0 ms, 3 ms] holds(c.versionNumber = 3)",
        &qsl_replay::MonitorTarget {
            unit: "us".to_owned(),
            max_reading_gap: 5000,
            max_event_rate: Some(crate::monitor_plan::ratio(2, 1000)),
            ..base_target()
        },
    )
}

#[test]
fn the_entries_agree_with_the_reference_on_a_deadline_trace() {
    let plan = deadline_plan();
    let node = spec_from_plan(&plan).unwrap().node.subformula;
    let atom = spec_from_plan(&plan).unwrap().atoms[0];
    let inputs = vec![
        Input::Reading(0, vec![(atom, false)]),
        Input::Reading(1000, vec![(atom, false)]),
        Input::Reading(2998, vec![(atom, true)]),
        Input::Watermark(4000),
    ];
    let _ = checked_clause;
    let _: Option<EntryRefusal> = None;
    assert_eq!(
        run_entries(&plan, &inputs),
        run_reference(&plan, node, &inputs)
    );
}
