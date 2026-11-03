// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251 "Transitions" (TC-1081 steps 4, 6-8, 10-14): the state-passing
//! entries generated as complete-V1 source are checked by the one checker and
//! run by `qsl-eval`'s function evaluator against the tick-trace reference.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use qsl_analyze::monitor_entries::{render, spec_from_plan, EntrySpec, Expr};
use qsl_eval::temporal::{TickBound, TickMonitorReference, TickMonitorSpec, TickOutcome, TickStep};
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_replay::TimedMonitorPlan;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{Meter, Outcome, Rational, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;

use crate::monitor_plan::{base_target, premise_plan};

pub(crate) fn package_of(declarations: &str) -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         {units}{declarations}\n",
        units = crate::monitor_plan::TIME_UNITS
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u"),
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
    let spec = spec_from_plan(&deadline_plan()).expect("the plan is generated");
    let _ = package_of(&render(&spec));
}

// ---------------------------------------------------------------------
// The differential (FR-251-AC-7): the generated entries, run by qsl-eval's
// function evaluator under a simulated runtime, against the tick-trace
// reference evaluator.

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
pub(crate) fn reference_code(outcome: TickOutcome) -> i64 {
    match outcome {
        TickOutcome::True => 1,
        TickOutcome::False => 2,
        TickOutcome::Pending => 3,
        TickOutcome::Indeterminate => 4,
        TickOutcome::Failed { .. } => 6,
        TickOutcome::ActivationLimit { .. } => 7,
        TickOutcome::AtomProjection { atom } => 1000 + i64::try_from(atom).unwrap(),
        TickOutcome::Incomplete { capacity } => 2000 + i64::try_from(capacity).unwrap(),
    }
}

pub(crate) fn reference_spec<'p>(plan: &'p TimedMonitorPlan) -> TickMonitorSpec<'p> {
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
        activation_capacity: plan.activation_capacity(),
        unprojectable: Default::default(),
    }
}

/// Per input, the `(origin * 1000 + subformula, code)` results of the
/// reference, in their reported order.
pub(crate) fn run_reference_with(
    spec: TickMonitorSpec<'_>,
    inputs: &[Input],
) -> Vec<Vec<(i128, i64)>> {
    let mut monitor = TickMonitorReference::new(spec);
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
            monitor
                .step(&step)
                .expect("the step applies")
                .into_iter()
                .map(|result| {
                    (
                        result.origin * 1000 + i128::try_from(result.subformula).unwrap(),
                        reference_code(result.outcome),
                    )
                })
                .collect()
        })
        .collect()
}

pub(crate) fn run_reference(plan: &TimedMonitorPlan, inputs: &[Input]) -> Vec<Vec<(i128, i64)>> {
    run_reference_with(reference_spec(plan), inputs)
}

/// Per input, the `(origin * 1000 + subformula, code)` results of the
/// generated entries, with this function playing the runtime: the baseline,
/// the modular delta and the reading-gap fault (RT FR-276, QSL FR-251
/// "Counter faults").
pub(crate) fn run_entries(plan: &TimedMonitorPlan, inputs: &[Input]) -> Vec<Vec<(i128, i64)>> {
    let spec = spec_from_plan(plan).expect("the plan is in the generated fragment");
    run_entries_with(plan, &spec, inputs)
}

/// [`run_entries`] over `spec`, which may evaluate its atoms from admitted
/// values (`spec.clause`).
pub(crate) fn run_entries_with(
    plan: &TimedMonitorPlan,
    spec: &EntrySpec,
    inputs: &[Input],
) -> Vec<Vec<(i128, i64)>> {
    run_entries_core(plan, spec, inputs, None).0
}

/// [`run_entries_with`], also evaluating `probe` (a `Sequence<Integer>`
/// expression over the state `s`) after every input that calls an entry; the
/// probe values of an input that only polls are those of the previous call.
pub(crate) fn run_entries_probe(
    plan: &TimedMonitorPlan,
    spec: &EntrySpec,
    inputs: &[Input],
    probe: &str,
) -> (Vec<Vec<(i128, i64)>>, Probes) {
    run_entries_core(plan, spec, inputs, Some(probe))
}

/// The probe values read after each input.
type Probes = Vec<Vec<i128>>;

const PROBE_BASE: i128 = 1_000_000_000_000_000_000;

fn run_entries_core(
    plan: &TimedMonitorPlan,
    spec: &EntrySpec,
    inputs: &[Input],
    probe: Option<&str>,
) -> (Vec<Vec<(i128, i64)>>, Probes) {
    let mut body = String::from(
        "function scenario using v(): Sequence<Integer> pure { let s0 = monitorInit() in ",
    );
    let mut baseline: Option<u64> = None;
    let mut encoded = Vec::new();
    let mut step_no = 0;
    for input in inputs {
        let call = match input {
            Input::Poll(_) => {
                if probe.is_some() {
                    encoded.push(format!("probe(s{step_no})"));
                }
                encoded.push("marker()".to_owned());
                continue;
            }
            Input::Watermark(boundary) => {
                format!("onWatermarkR(s{step_no}, {boundary}, Scope {{ subject: 0 }})")
            }
            Input::Reading(reading, atoms) => {
                let (call, next) = rt_call(
                    baseline,
                    *reading,
                    plan.binding().width_bits(),
                    plan.max_reading_gap(),
                );
                baseline = next;
                let delta = match call {
                    RtCall::OnEvent(delta) | RtCall::OnGap(delta) => delta,
                    RtCall::Baseline => 0,
                };
                if matches!(call, RtCall::OnGap(_)) {
                    format!("onGapR(s{step_no}, {delta})")
                } else {
                    let truth = |atom: &usize| {
                        atoms
                            .iter()
                            .find(|(a, _)| a == atom)
                            .is_some_and(|(_, v)| *v)
                    };
                    let fields = match &spec.clause {
                        // The admitted value the atoms read: the first atom
                        // holds at 2, the second at 3, else 1.
                        Some(clause) => {
                            let value = if truth(&spec.atoms[0]) {
                                2
                            } else if truth(&spec.atoms[1]) {
                                3
                            } else {
                                1
                            };
                            format!("{}: {value}", clause.parameters[0].0)
                        }
                        None => {
                            let mut fields = spec
                                .atoms
                                .iter()
                                .map(|atom| format!("a{atom}: {}", truth(atom)))
                                .collect::<Vec<_>>();
                            if spec.activation
                                == qsl_analyze::monitor_entries::ActivationKind::EachGuarded
                            {
                                fields.push("guard: true".to_owned());
                            }
                            fields.join(", ")
                        }
                    };
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
        if probe.is_some() {
            encoded.push(format!("probe(s{step_no})"));
        }
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
        function ints1 using v(a: Integer): Sequence<Integer> pure { sequence[a] }\n\
        function ints4 using v(a: Integer, b: Integer, c: Integer, d: Integer): Sequence<Integer> pure { sequence[a, b, c, d] }\n\
        function code using v(o: Outcome): Integer pure { case o { Outcome::Failed(f): 6; Outcome::False: 2; Outcome::Incomplete(l): if l.limit = \"activation\" then 7 else if l.limit = \"atom_projection\" then 1000 + l.amount else 2000 + l.amount; Outcome::Indeterminate: 4; Outcome::Pending: 3; Outcome::True: 1; } }\n\
        function encode using v(rs: Results): Sequence<Integer> pure { map(r in rs: ((r.instance.origin_tick * 1000 + r.instance.subformula) * 100000) + code(r.outcome)) }\n";
    let probe_fn = probe.map_or_else(String::new, |probe| {
        format!(
            "function probe using v(s: State): Sequence<Integer> pure {{ map(x in {probe}: x + {PROBE_BASE}) }}\n"
        )
    });
    let package = package_of(&format!("{}\n{helpers}{probe_fn}\n{body}", render(spec)));
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
    let mut probes: Vec<Vec<i128>> = vec![Vec::new()];
    for element in flat.elements() {
        let Value::Integer(value) = element else {
            panic!("integers")
        };
        let number: i128 = value.as_big().try_into().expect("small");
        if number < 0 {
            steps.push(Vec::new());
            probes.push(Vec::new());
        } else if number >= PROBE_BASE / 2 {
            probes.last_mut().unwrap().push(number - PROBE_BASE);
        } else {
            steps
                .last_mut()
                .unwrap()
                .push((number / 100_000, i64::try_from(number % 100_000).unwrap()));
        }
    }
    steps.pop();
    probes.pop();
    (steps, probes)
}

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

/// The first interval operator of the plan's formula.
fn interval_node(plan: &TimedMonitorPlan) -> usize {
    plan.intervals()[0].0.index()
}

fn sorted(mut steps: Vec<Vec<(i128, i64)>>) -> Vec<Vec<(i128, i64)>> {
    steps.iter_mut().for_each(|step| step.sort_unstable());
    steps
}

#[trace("TC-1081", "FR-251-AC-7")]
#[test]
fn the_entries_agree_with_the_reference_on_a_deadline_trace() {
    let plan = deadline_plan();
    let atom = spec_from_plan(&plan).unwrap().atoms[0];
    let inputs = vec![
        Input::Reading(0, vec![(atom, false)]),
        Input::Reading(1000, vec![(atom, false)]),
        Input::Reading(2998, vec![(atom, true)]),
        Input::Watermark(4000),
    ];
    assert_eq!(run_entries(&plan, &inputs), run_reference(&plan, &inputs));
}

/// A deterministic generator of traces (a fixed linear congruential stream,
/// so a failure reproduces).
struct Stream(u64);

impl Stream {
    fn next(&mut self, bound: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % bound
    }
}

fn trace(seed: u64, atoms: &[usize], width_bits: u32, length: usize) -> Vec<Input> {
    let mut stream = Stream(seed);
    let modulus = 1_u64 << width_bits;
    let (mut raw, mut now, mut watermark) = (stream.next(modulus), 0_u64, 0_u64);
    let mut inputs = vec![Input::Reading(
        raw,
        atoms
            .iter()
            .map(|atom| (*atom, stream.next(2) == 1))
            .collect(),
    )];
    for _ in 0..length {
        match stream.next(10) {
            0 => {
                let boundary = watermark.max(now) + stream.next(2500);
                watermark = boundary;
                inputs.push(Input::Watermark(boundary));
            }
            1 => inputs.push(Input::Poll(stream.next(modulus))),
            _ => {
                let floor = if watermark >= now {
                    watermark - now + 1
                } else {
                    0
                };
                let delta = [0, 1, 300, 700, 1200, 2999, 3000, 3001, 3500]
                    [usize::try_from(stream.next(9)).unwrap()]
                .max(floor)
                .min(modulus - 1);
                raw = (raw + delta) % modulus;
                now += delta;
                let truths = atoms
                    .iter()
                    .map(|atom| (*atom, stream.next(2) == 1))
                    .collect();
                inputs.push(Input::Reading(raw, truths));
            }
        }
    }
    inputs
}

fn differential(formula: &str, target: qsl_replay::MonitorTarget, traces: u64) {
    let plan = premise_plan(formula, &target);
    let spec = spec_from_plan(&plan).unwrap_or_else(|refusal| panic!("{formula}: {refusal}"));
    for seed in 0..traces {
        let inputs = trace(seed * 7919 + 13, &spec.atoms, target.width_bits, 14);
        let entries = sorted(run_entries(&plan, &inputs));
        let reference = sorted(run_reference(&plan, &inputs));
        assert_eq!(
            entries, reference,
            "{formula} {target:?} seed {seed}\n{inputs:#?}"
        );
    }
}

const P: &str = "holds(c.versionNumber = 3)";
const Q: &str = "holds(c.versionNumber = 2)";

const SHAPES: [&str; 9] = [
    "eventually[0 ms, 3 ms] holds(c.versionNumber = 3)",
    "always[0 ms, 3 ms] holds(c.versionNumber = 3)",
    "once[0 ms, 3 ms] holds(c.versionNumber = 3)",
    "historically[0 ms, 3 ms] holds(c.versionNumber = 3)",
    "holds(c.versionNumber = 2) until[0 ms, 3 ms] holds(c.versionNumber = 3)",
    "holds(c.versionNumber = 2) release[0 ms, 3 ms] holds(c.versionNumber = 3)",
    "holds(c.versionNumber = 2) since[0 ms, 3 ms] holds(c.versionNumber = 3)",
    "holds(c.versionNumber = 2) triggered[0 ms, 3 ms] holds(c.versionNumber = 3)",
    "eventually[1 ms, 3 ms] (holds(c.versionNumber = 2) and not holds(c.versionNumber = 3))",
];

/// Nested interval operators at any depth, temporal operands of the binary
/// operators, Boolean combinations and two interval operators joined at one
/// position.
fn nested_shapes() -> Vec<String> {
    vec![
        format!("once[0 ms, 3 ms] once[0 ms, 2 ms] {P}"),
        format!("eventually[0 ms, 3 ms] eventually[0 ms, 2 ms] {P}"),
        format!("once[0 ms, 3 ms] eventually[0 ms, 2 ms] {P}"),
        format!("eventually[0 ms, 3 ms] once[0 ms, 2 ms] {P}"),
        format!("always[0 ms, 3 ms] historically[0 ms, 2 ms] {P}"),
        format!("historically[0 ms, 3 ms] always[1 ms, 2 ms] {P}"),
        format!("once[0 ms, 3 ms] once[0 ms, 2 ms] once[0 ms, 1 ms] {P}"),
        format!("eventually[0 ms, 3 ms] once[0 ms, 2 ms] eventually[0 ms, 1 ms] {P}"),
        format!("{Q} until[0 ms, 3 ms] (eventually[0 ms, 2 ms] {P})"),
        format!("(eventually[0 ms, 2 ms] {Q}) release[0 ms, 3 ms] {P}"),
        format!("(historically[0 ms, 2 ms] {Q}) since[0 ms, 3 ms] {P}"),
        format!("{Q} triggered[0 ms, 3 ms] (once[0 ms, 2 ms] {P})"),
        format!("once[0 ms, 3 ms] {Q} and eventually[0 ms, 2 ms] {P}"),
        format!("once[0 ms, 3 ms] {Q} or not eventually[0 ms, 1 ms] {P}"),
        format!("{Q} implies eventually[0 ms, 3 ms] once[0 ms, 2 ms] {P}"),
        format!("eventually[0 ms, 3 ms] ({Q} and once[1 ms, 2 ms] {P})"),
        format!("once[0 ms, 3 ms] eventually {P}"),
        format!("eventually[0 ms, 3 ms] always {P}"),
        format!("once[0 ms, 3 ms] ({Q} and eventually {P})"),
    ]
}

pub(crate) fn microsecond_target(
    uncertainty: Rational,
    rate: i64,
    width_bits: u32,
    gap: u64,
) -> qsl_replay::MonitorTarget {
    qsl_replay::MonitorTarget {
        unit: "us".to_owned(),
        width_bits,
        max_reading_gap: gap,
        uncertainty,
        max_event_rate: Some(crate::monitor_plan::ratio(rate, 1000)),
        ..base_target()
    }
}

/// The trace families of FR-251-AC-7: no uncertainty, uncertainty 1/2, a
/// small buffer that overflows, and a small reading gap.
fn families() -> [qsl_replay::MonitorTarget; 4] {
    [
        microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600),
        microsecond_target(crate::monitor_plan::ratio(1, 2), 2, 12, 3600),
        microsecond_target(crate::monitor_plan::ratio(0, 1), 1, 12, 3600),
        microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 1500),
    ]
}

#[trace("TC-1081", "FR-251-AC-7")]
#[test]
fn every_operator_agrees_with_the_reference_without_uncertainty() {
    for shape in SHAPES {
        differential(shape, families()[0].clone(), 25);
    }
}

#[trace("TC-1081", "FR-251-AC-7")]
#[test]
fn every_operator_agrees_with_the_reference_under_uncertainty() {
    for shape in SHAPES {
        differential(shape, families()[1].clone(), 25);
    }
}

#[trace("TC-1081", "FR-251-AC-7")]
#[test]
fn small_buffers_overflow_the_same_way() {
    for shape in SHAPES {
        differential(shape, families()[2].clone(), 25);
    }
}

#[trace("TC-1081", "FR-251-AC-7")]
#[test]
fn reading_gaps_fail_the_open_instances_the_same_way() {
    for shape in SHAPES {
        differential(shape, families()[3].clone(), 25);
    }
}

pub(crate) const ROOT_SHAPES: [&str; 5] = [
    "always (holds(c.versionNumber = 2) implies eventually[0 ms, 3 ms] holds(c.versionNumber = 3))",
    "always once[0 ms, 3 ms] holds(c.versionNumber = 3)",
    "eventually (holds(c.versionNumber = 2) and always[0 ms, 3 ms] holds(c.versionNumber = 3))",
    "always (eventually[0 ms, 3 ms] holds(c.versionNumber = 3) implies holds(c.versionNumber = 2))",
    "always historically[1 ms, 3 ms] holds(c.versionNumber = 3)",
];

#[trace("TC-1081", "FR-251-AC-7")]
#[test]
fn unbounded_roots_fold_one_outcome_per_activation_like_the_reference() {
    for shape in ROOT_SHAPES {
        for target in families() {
            differential(shape, target, 25);
        }
    }
}

#[trace("TC-1081", "FR-251-AC-7")]
#[test]
fn nested_operators_agree_with_the_reference_on_every_trace_family() {
    for shape in nested_shapes() {
        for target in families() {
            differential(&shape, target, 12);
        }
    }
}

#[trace("TC-1081", "FR-251-AC-7")]
#[test]
fn a_zero_activation_capacity_never_starts_the_root() {
    let mut target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600);
    target.max_activations = 0;
    for shape in ROOT_SHAPES {
        differential(shape, target.clone(), 10);
    }
}

/// The wire of the plan root for `formula` under `target`, with the root's
/// own identity-bearing premises replaced by `edit`.
fn root_wire(
    formula: &str,
    target: &qsl_replay::MonitorTarget,
    edit: impl Fn(&mut Premises),
) -> serde_json::Value {
    root_wire_with(formula, target, edit, |_| {}, |source| source)
}

/// [`root_wire`] over the entries of `spec` as edited by `edit_spec`, with
/// their rendered source passed through `tweak`.
fn root_wire_with(
    formula: &str,
    target: &qsl_replay::MonitorTarget,
    edit: impl Fn(&mut Premises),
    edit_spec: impl Fn(&mut EntrySpec),
    tweak: impl Fn(String) -> String,
) -> serde_json::Value {
    let (bytes, _) = root_bytes_with(formula, target, edit, edit_spec, tweak);
    serde_json::from_slice(&bytes).unwrap()
}

/// The bytes of the plan-root package of [`root_wire_with`], and the evidence
/// its reader needs.
fn root_bytes_with(
    formula: &str,
    target: &qsl_replay::MonitorTarget,
    edit: impl Fn(&mut Premises),
    edit_spec: impl Fn(&mut EntrySpec),
    tweak: impl Fn(String) -> String,
) -> (Vec<u8>, quire_contract_model::CheckedPackageEvidence) {
    try_root_bytes_with(formula, target, edit, edit_spec, tweak)
        .unwrap_or_else(|refusal| panic!("{refusal:?}"))
}

/// [`root_bytes_with`], the writer's refusal returned.
fn try_root_bytes_with(
    formula: &str,
    target: &qsl_replay::MonitorTarget,
    edit: impl Fn(&mut Premises),
    edit_spec: impl Fn(&mut EntrySpec),
    tweak: impl Fn(String) -> String,
) -> Result<(Vec<u8>, quire_contract_model::CheckedPackageEvidence), qsl_package::EmitRefusal> {
    use qsl_package::{emit_checked, emit_monitor_plan_over, MonitorPlanWire};
    let plan = premise_plan(formula, target);
    let mut spec = spec_from_plan(&plan).unwrap();
    edit_spec(&mut spec);
    let (package, _) = crate::monitor_plan::checked_clause_in(formula, &tweak(render(&spec)), true);
    let emission = emit_checked(&package).expect("the unit emits");
    let name = qsl_semantics::library::LibraryName::new(emission.package().name()).unwrap();
    let domain = crate::units::Domain::config_version();
    let mut evidence = emission.evidence().clone();
    evidence
        .insert_domain_package_document(domain.package, domain.document.clone())
        .unwrap();
    let mut premises = Premises {
        counter: plan.root_premises().counter,
        tick_unit: plan.binding().unit().to_owned(),
        rate: plan.root_premises().max_event_rate,
        uncertainty: plan.root_premises().uncertainty,
        activation_capacity: plan.root_premises().activation_capacity,
        capacity_delta: 0,
        state: "State",
    };
    edit(&mut premises);
    let buffers: Vec<(usize, u64)> = plan
        .buffers()
        .iter()
        .map(|(id, c)| (id.index(), *c + premises.capacity_delta))
        .collect();
    let root = plan.root_premises();
    let clause_key = package.graph().temporal_clauses()[0].identity();
    let wire = MonitorPlanWire {
        clause: &clause_key,
        formula: plan.program().formula(),
        counter: &premises.counter,
        counter_width_bits: root.counter_width_bits,
        max_reading_gap: root.max_reading_gap,
        max_event_rate: &premises.rate,
        uncertainty: &premises.uncertainty,
        buffers: &buffers,
        tick_unit: &premises.tick_unit,
        activation_capacity: premises.activation_capacity,
        state: premises.state,
        inputs: "Inputs",
        scope: "Scope",
        result: "Result",
        entries: [
            qsl_analyze::monitor_entries::names::INIT,
            qsl_analyze::monitor_entries::names::ON_EVENT,
            qsl_analyze::monitor_entries::names::ON_WATERMARK,
            qsl_analyze::monitor_entries::names::ON_GAP,
        ],
        renamed_records: &["Instance", "Activation", "LimitReached"],
        renames: &qsl_analyze::monitor_entries::WIRE_FIELD_NAMES,
    };
    let rooted = emit_monitor_plan_over(emission.package(), name, &evidence, &wire)?;
    Ok((rooted.package().bytes().to_vec(), rooted.evidence().clone()))
}

struct Premises {
    counter: String,
    tick_unit: String,
    rate: Rational,
    uncertainty: Rational,
    activation_capacity: u64,
    capacity_delta: u64,
    /// The declared name the root binds as its `state`.
    state: &'static str,
}

/// The content key the strict reader derives for the plan root of `formula`
/// under `target` with the premises edited by `edit`.
fn root_key(
    formula: &str,
    target: &qsl_replay::MonitorTarget,
    edit: impl Fn(&mut Premises),
) -> quire_contract_model::CheckedNodeId {
    let (bytes, evidence) = root_bytes_with(formula, target, edit, |_| {}, |source| source);
    crate::monitor_root::keys(&crate::monitor_root::read_back(&bytes, &evidence)).root
}

/// FR-487-AC-6 (MP-02, MP-33, MP-43, MP-44, MP-45, MP-58): each
/// identity-bearing premise alone gives a different root key, as the strict
/// reader derives it.
#[test]
fn each_root_premise_alone_changes_the_root_key() {
    let formula = ROOT_SHAPES[0];
    let target = microsecond_target(crate::monitor_plan::ratio(1, 2), 2, 12, 3600);
    let base = root_key(formula, &target, |_| {});
    assert_eq!(base, root_key(formula, &target, |_| {}));
    let variants: Vec<(&str, quire_contract_model::CheckedNodeId)> = vec![
        (
            "MP-02 capacity",
            root_key(formula, &target, |p| p.capacity_delta = 1),
        ),
        (
            "MP-33 uncertainty",
            root_key(formula, &target, |p| {
                p.uncertainty = crate::monitor_plan::ratio(0, 1)
            }),
        ),
        (
            "MP-43 counter",
            root_key(formula, &target, |p| p.counter = "ticks2".to_owned()),
        ),
        (
            "MP-44 rate",
            root_key(formula, &target, |p| {
                p.rate = crate::monitor_plan::ratio(1, 10)
            }),
        ),
        (
            "MP-45 tick unit",
            root_key(formula, &target, |p| p.tick_unit = "ms".to_owned()),
        ),
        (
            "MP-58 activations",
            root_key(formula, &target, |p| p.activation_capacity += 1),
        ),
    ];
    for (id, key) in &variants {
        assert_ne!(&base, key, "{id}");
    }
    for (a, (_, left)) in variants.iter().enumerate() {
        for (_, right) in &variants[a + 1..] {
            assert_ne!(left, right);
        }
    }
}

/// A name nothing declares cannot be bound as the root's `state`.
#[test]
fn the_writer_refuses_a_name_nothing_declares() {
    let target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600);
    let refusal = try_root_bytes_with(
        ROOT_SHAPES[0],
        &target,
        |p| p.state = "NoSuchType",
        |_| {},
        |source| source,
    )
    .map(|_| ())
    .unwrap_err();
    assert!(
        matches!(refusal, qsl_package::EmitRefusal::MonitorPlanInvariant(_)),
        "{refusal:?}"
    );
}

/// FR-322-AC-309: a declaration-less function node is keyed
/// `[function, form, parameter count, parameter operands, body root]`.
#[test]
fn the_entries_carry_the_declaration_less_function_key() {
    let wire = root_wire(
        ROOT_SHAPES[0],
        &microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600),
        |_| {},
    );
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let functions: Vec<&serde_json::Value> = nodes
        .iter()
        .filter(|n| n["node_tag"] == "function" && n.get("declaration").is_none())
        .collect();
    assert_eq!(functions.len(), 4);
    let counts: Vec<String> = functions
        .iter()
        .map(|n| n["node_id"]["path"][2].as_str().unwrap().to_owned())
        .collect();
    let mut sorted = counts.clone();
    sorted.sort();
    // init 0, on_gap 2, on_event 3, on_watermark 3 parameters.
    assert_eq!(sorted, ["0", "2", "3", "3"]);
    for function in functions {
        let path = function["node_id"]["path"].as_array().unwrap();
        assert_eq!(path[0], "function");
        assert_eq!(path[1], "pure_function");
        let operands = &path[3..];
        assert_eq!(
            operands.len(),
            path[2].as_str().unwrap().parse::<usize>().unwrap() + 1
        );
        assert!(operands.iter().all(|segment| segment.get("node").is_some()));
    }
}
/// QSpec FR-487 "Counter-gap path" (MP-54, MP-55, MP-56, MP-57): on a gap
/// every open instance fails and leaves the state, a decided one and the
/// results already emitted are untouched, and the faulting reading is the
/// new baseline.
fn gap_results(acks: &[bool]) -> (usize, Vec<Vec<(i128, i64)>>) {
    let target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 16, 100);
    let plan = premise_plan("eventually[0 ms, 3 ms] holds(c.versionNumber = 3)", &target);
    let spec = spec_from_plan(&plan).unwrap();
    let atom = spec.atoms[0];
    let mut inputs: Vec<Input> = acks
        .iter()
        .enumerate()
        .map(|(n, ack)| Input::Reading(10 * n as u64, vec![(atom, *ack)]))
        .collect();
    inputs.push(Input::Reading(5000, vec![(atom, false)]));
    inputs.push(Input::Reading(5010, vec![(atom, false)]));
    let entries = sorted(run_entries(&plan, &inputs));
    let reference = sorted(run_reference(&plan, &inputs));
    assert_eq!(entries, reference);
    (interval_node(&plan), entries)
}

#[test]
fn a_gap_fails_the_open_instance_and_leaves_the_decided_one() {
    // MP-54: one decided (ack at its own reading) and one open instance.
    let (node, steps) = gap_results(&[true, false]);
    let sub = i128::try_from(node).unwrap();
    assert_eq!(steps[2], [(10 * 1000 + sub, 6)]);
    // The reading after the fault is differenced from the faulting reading.
    assert!(steps[3].iter().all(|(_, code)| *code != 6));
}

#[test]
fn a_gap_with_no_open_instance_changes_nothing() {
    // MP-55.
    let (_, steps) = gap_results(&[true]);
    assert_eq!(steps[1], []);
}

#[test]
fn a_gap_fails_every_open_instance_in_instance_order() {
    // MP-56.
    let (node, steps) = gap_results(&[false, false]);
    let sub = i128::try_from(node).unwrap();
    assert_eq!(steps[2], [(sub, 6), (10 * 1000 + sub, 6)]);
}

/// QSpec FR-487 "Counter-gap path", the activation-held half of MP-54: an
/// instance of an unbounded operator that is still open when the counter
/// faults settles `Failed(ReadingGapExceeded)` and is dropped, exactly as an
/// open buffered instance does, and the reading after the fault starts clean.
#[trace("TC-1081", "FR-251-AC-11")]
#[test]
fn a_gap_fails_the_open_activation_held_instance() {
    let target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 16, 100);
    // The truth that keeps the instance open: `eventually` is never met,
    // `always` is never violated.
    for (shape, truth) in [
        ("eventually holds(c.versionNumber = 3)", false),
        ("always holds(c.versionNumber = 3)", true),
    ] {
        let plan = premise_plan(shape, &target);
        let atom = spec_from_plan(&plan).unwrap().atoms[0];
        // The first reading is the baseline, the second an event, the third
        // jumps past `max_reading_gap`, the fourth is differenced from it.
        let inputs = vec![
            Input::Reading(0, vec![(atom, truth)]),
            Input::Reading(10, vec![(atom, truth)]),
            Input::Reading(5000, vec![(atom, truth)]),
            Input::Reading(5010, vec![(atom, truth)]),
        ];
        let entries = sorted(run_entries(&plan, &inputs));
        assert_eq!(entries, sorted(run_reference(&plan, &inputs)), "{shape}");
        let failed = |step: usize| entries[step].iter().filter(|(_, code)| *code == 6).count();
        assert_eq!(
            failed(0) + failed(1),
            0,
            "{shape}: nothing fails before the gap"
        );
        assert_eq!(failed(2), 1, "{shape}: the open instance fails at the gap");
        assert_eq!(
            failed(3),
            0,
            "{shape}: the dropped instance reports no more"
        );
    }
}

/// An aggregate atom is unknown at every position, as the reference
/// evaluator has it: an operator over it never decides true or false.
#[trace("TC-1081", "FR-251-AC-7")]
#[test]
fn an_aggregate_atom_is_unknown_in_the_entries() {
    let target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600);
    let plan = premise_plan("eventually[0 ms, 3 ms] holds(c.versionNumber = 3)", &target);
    let mut spec = spec_from_plan(&plan).unwrap();
    let atom = spec.atoms[0];
    let node = spec
        .nodes
        .iter_mut()
        .find(|node| node.interval.is_some())
        .unwrap();
    node.psi = Expr::Unknown;
    let inputs = vec![
        Input::Reading(0, vec![(atom, true)]),
        Input::Reading(1000, vec![(atom, true)]),
        Input::Reading(4000, vec![(atom, true)]),
        Input::Watermark(9000),
    ];
    let codes: Vec<i64> = run_entries_with(&plan, &spec, &inputs)
        .into_iter()
        .flatten()
        .map(|(_, code)| code)
        .collect();
    assert!(
        codes.contains(&4),
        "the settled instance is indeterminate: {codes:?}"
    );
    assert!(
        codes.iter().all(|code| *code == 3 || *code == 4),
        "never decided true or false: {codes:?}"
    );
}

/// A plan whose interval operator has no buffer row is refused rather than
/// given a capacity of one.
#[test]
fn an_interval_operator_without_a_buffer_row_is_refused() {
    let target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600);
    let plan = premise_plan("eventually[0 ms, 3 ms] holds(c.versionNumber = 3)", &target);
    let node = interval_node(&plan);
    let without = TimedMonitorPlan::new(
        plan.subject().clone(),
        plan.binding().clone(),
        plan.uncertainty().clone(),
        plan.max_reading_gap(),
        plan.program().clone(),
        plan.intervals().to_vec(),
        Vec::new(),
        plan.rate_limit().clone(),
        plan.activation_capacity(),
    );
    assert_eq!(
        spec_from_plan(&without),
        Err(qsl_analyze::monitor_entries::EntryRefusal::MissingBuffer { node })
    );
}

/// QSL FR-251 "Entry bindings": the `Inputs` fields are the already-read
/// values the atoms depend on, and each atom is a pure helper over them
/// (the authored expression over those values). The entries then agree with
/// the reference on traces whose atoms come from the admitted value.
#[test]
fn the_entries_evaluate_atoms_from_admitted_values() {
    let formula = ROOT_SHAPES[0];
    let target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600);
    let plan = premise_plan(formula, &target);
    let mut spec = spec_from_plan(&plan).unwrap();
    spec.clause = Some(qsl_analyze::monitor_entries::ClauseSource {
        parameters: vec![("versionNumber".to_owned(), "Integer".to_owned())],
        atoms: spec
            .atoms
            .iter()
            .zip(["versionNumber = 2", "versionNumber = 3"])
            .map(|(atom, text)| (*atom, text.to_owned()))
            .collect(),
        guard: None,
        profile: "v".to_owned(),
    });
    for seed in 0..25 {
        let mut inputs = trace(seed * 31 + 5, &spec.atoms, target.width_bits, 14);
        // The atoms are functions of one admitted value.
        for input in &mut inputs {
            if let Input::Reading(_, truths) = input {
                let value = if truths[0].1 {
                    2
                } else if truths[1].1 {
                    3
                } else {
                    1
                };
                *truths = vec![(spec.atoms[0], value == 2), (spec.atoms[1], value == 3)];
            }
        }
        assert_eq!(
            sorted(run_entries_with(&plan, &spec, &inputs)),
            sorted(run_reference(&plan, &inputs)),
            "seed {seed}"
        );
    }
}

/// QSpec FR-487-AC-12, QSL FR-251-AC-17 / TC-1081 step 12: an atom with no
/// admitted projection makes every instance that reads it settle
/// `Incomplete(LimitReached{atom_projection, <AtomId>})`, entries and
/// reference alike.
#[test]
fn an_atom_without_a_projection_settles_incomplete_atom_projection() {
    for shape in [
        "eventually[0 ms, 3 ms] holds(c.versionNumber = 3)",
        "holds(c.versionNumber = 2) since[0 ms, 3 ms] holds(c.versionNumber = 3)",
        ROOT_SHAPES[0],
    ] {
        let target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600);
        let plan = premise_plan(shape, &target);
        let mut spec = spec_from_plan(&plan).unwrap();
        let atom = *spec.atoms.last().unwrap();
        spec.unprojectable = vec![atom];
        let mut saw_projection = false;
        for seed in 0..15 {
            let inputs = trace(seed * 7919 + 13, &spec.atoms, target.width_bits, 14);
            let entries = sorted(run_entries_with(&plan, &spec, &inputs));
            let mut ref_spec = reference_spec(&plan);
            ref_spec.unprojectable.insert(atom);
            let expected = sorted(run_reference_with(ref_spec, &inputs));
            saw_projection |= expected
                .iter()
                .flatten()
                .any(|(_, code)| *code == 1000 + i64::try_from(atom).unwrap());
            assert_eq!(entries, expected, "{shape} seed {seed}");
        }
        assert!(saw_projection, "{shape}: some instance reads the atom");
    }
}

/// What the runtime does with one reading (RT FR-276; QSL FR-251 "Counter
/// faults"; QSpec FR-487 "Counter-gap path"): the first reading is the
/// baseline, a later one is differenced modulo `2^width` and passed to
/// `on_event`, or, above `max_reading_gap`, to `on_gap`; the reading then
/// becomes the new baseline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RtCall {
    Baseline,
    OnEvent(u64),
    OnGap(u64),
}

pub(crate) fn rt_call(
    baseline: Option<u64>,
    reading: u64,
    width_bits: u32,
    max_gap: u64,
) -> (RtCall, Option<u64>) {
    let Some(base) = baseline else {
        return (RtCall::Baseline, Some(reading));
    };
    let modulus = 1_u128 << width_bits;
    let delta =
        u64::try_from((u128::from(reading) + modulus - u128::from(base)) % modulus).unwrap();
    let call = if delta > max_gap {
        RtCall::OnGap(delta)
    } else {
        RtCall::OnEvent(delta)
    };
    (call, Some(reading))
}

/// QSpec FR-487 MP-57 and QSL FR-251-AC-3/AC-10/AC-11: readings 65530, 4,
/// 65500, 65510 on a 16-bit counter with `max_reading_gap` 100 call
/// baseline, `on_event(10)`, `on_gap(65496)`, `on_event(10)`; and 65530 then
/// 65500 calls `on_gap(65506)`.
#[test]
fn the_runtime_calls_baseline_event_gap_event() {
    let mut baseline = None;
    let mut calls = Vec::new();
    for reading in [65530_u64, 4, 65500, 65510] {
        let (call, next) = rt_call(baseline, reading, 16, 100);
        baseline = next;
        calls.push(call);
    }
    assert_eq!(
        calls,
        [
            RtCall::Baseline,
            RtCall::OnEvent(10),
            RtCall::OnGap(65496),
            RtCall::OnEvent(10)
        ]
    );
    let (second, _) = rt_call(Some(65530), 65500, 16, 100);
    assert_eq!(second, RtCall::OnGap(65506));
}

// ---------------------------------------------------------------------
// FR-251 capacity, overflow-order and retention vectors on a 1 MHz counter.

use ix_trace_rs::trace;

/// A 1 MHz-counter target at `rate` events per millisecond.
fn tick_target(rate: i64, activations: u64) -> qsl_replay::MonitorTarget {
    qsl_replay::MonitorTarget {
        unit: "us".to_owned(),
        width_bits: 20,
        max_reading_gap: 100_000,
        max_activations: activations,
        max_event_rate: Some(crate::monitor_plan::ratio(rate, 1000)),
        ..base_target()
    }
}

fn capacities(plan: &TimedMonitorPlan) -> Vec<u64> {
    let mut buffers: Vec<(usize, u64)> = plan
        .buffers()
        .iter()
        .map(|(id, capacity)| (id.index(), *capacity))
        .collect();
    buffers.sort_unstable();
    buffers.into_iter().map(|(_, capacity)| capacity).collect()
}

fn evenly(count: u64, spacing: u64, atom: usize, truth: impl Fn(u64) -> bool) -> Vec<Input> {
    (0..count)
        .map(|n| Input::Reading(n * spacing, vec![(atom, truth(n))]))
        .collect()
}

/// FR-251-AC-18: the inner operator of `once[0 ms, 10 ms] once[0 ms, 5 ms]`
/// keeps its entries for the sum of the bounds plus a cover per level, and
/// its 33rd entry overflows.
#[trace("TC-706", "FR-251-AC-18")]
#[test]
fn a_nested_once_is_retained_for_the_sum_of_the_bounds_plus_a_cover_per_level() {
    let plan = premise_plan(
        &format!("once[0 ms, 10 ms] once[0 ms, 5 ms] {P}"),
        &tick_target(2, 8),
    );
    assert_eq!(capacities(&plan), [22, 32]);
    let spec = spec_from_plan(&plan).unwrap();
    let inner = i128::try_from(plan.intervals()[1].0.index()).unwrap();
    let inputs = evenly(34, 100, spec.atoms[0], |_| false);
    let entries = run_entries(&plan, &inputs);
    assert_eq!(entries, run_reference(&plan, &inputs));
    let overflowed = |step: usize| {
        entries[step]
            .iter()
            .any(|(key, code)| key % 1000 == inner && *code == 2032)
    };
    assert!(!overflowed(31), "the 32nd entry still fits");
    assert!(overflowed(32), "the 33rd entry overflows");
}

/// FR-251-AC-20: two sibling operators that both overflow at one position
/// are reported lowest `subformula` first, each with its own result.
#[trace("TC-706", "FR-251-AC-20")]
#[test]
fn simultaneous_overflows_are_reported_lowest_subformula_first() {
    let plan = premise_plan(
        &format!("once[0 ms, 10 ms] {P} and once[0 ms, 10 ms] {Q}"),
        &tick_target(2, 8),
    );
    assert_eq!(capacities(&plan), [22, 22]);
    let spec = spec_from_plan(&plan).unwrap();
    let nodes: Vec<i128> = plan
        .intervals()
        .iter()
        .map(|(id, _)| i128::try_from(id.index()).unwrap())
        .collect();
    let inputs: Vec<Input> = (0..23)
        .map(|n| Input::Reading(n * 100, spec.atoms.iter().map(|a| (*a, false)).collect()))
        .collect();
    let entries = run_entries(&plan, &inputs);
    assert_eq!(entries, run_reference(&plan, &inputs));
    let overflow: Vec<i128> = entries[22]
        .iter()
        .filter(|(_, code)| *code == 2022)
        .map(|(key, _)| key % 1000)
        .collect();
    assert_eq!(overflow, nodes);
    assert!(nodes[0] < nodes[1]);
}

/// FR-251-AC-21: the retention rule holds for mixed-direction nesting, and
/// the tick-0 entry is still read at tick 14999.
#[trace("TC-1081", "FR-251-AC-21")]
#[test]
fn mixed_direction_nesting_keeps_the_inner_entry_until_every_reader_settled() {
    for (formula, outer) in [
        (format!("once[0 ms, 10 ms] eventually[0 ms, 5 ms] {P}"), 22),
        (format!("eventually[0 ms, 5 ms] once[0 ms, 10 ms] {P}"), 12),
    ] {
        let plan = premise_plan(&formula, &tick_target(2, 8));
        assert_eq!(capacities(&plan), [outer, 32], "{formula}");
        let spec = spec_from_plan(&plan).unwrap();
        let atom = spec.atoms[0];
        let mut inputs: Vec<Input> = (0..30)
            .map(|n| Input::Reading(n * 517, vec![(atom, n == 0)]))
            .collect();
        inputs.push(Input::Reading(14_999, vec![(atom, false)]));
        // Declared progress, not events: more readings here would exceed the
        // stated maximum event rate inside the outer window.
        // Boundary 15_000 reads at horizon 15_001, boundary 15_002 at 15_003.
        inputs.push(Input::Watermark(15_000));
        inputs.push(Input::Watermark(15_002));
        let probe = "map(x in s.buffer_1: x.tf)";
        let (entries, buffer) = run_entries_probe(&plan, &spec, &inputs, probe);
        assert_eq!(entries, run_reference(&plan, &inputs), "{formula}");
        let incomplete: Vec<_> = entries
            .iter()
            .enumerate()
            .flat_map(|(step, results)| {
                results
                    .iter()
                    .filter(|(_, code)| *code >= 1000)
                    .map(move |result| (step, *result))
            })
            .collect();
        assert!(
            incomplete.is_empty(),
            "{formula}: Incomplete {incomplete:?}"
        );
        assert_eq!(
            buffer[30].first(),
            Some(&0),
            "tick 14999 still holds tick 0"
        );
        assert_eq!(
            buffer[31].first(),
            Some(&0),
            "tick 15001 still holds tick 0"
        );
        assert_ne!(
            buffer[32].first(),
            Some(&0),
            "tick 15003 has dropped tick 0"
        );
        // The decision that needs the tick-0 entry: some reading inside the
        // outer window of tick 0 decides true on it.
        assert!(entries.iter().flatten().any(|(_, code)| *code == 1));
    }
}

/// FR-251-AC-19: an unbounded `eventually` under `once[0 ms, 10 ms]` counts
/// against the shared activation capacity; the third start is refused.
#[trace("TC-1081", "FR-251-AC-19")]
#[test]
fn a_nested_unbounded_operator_shares_the_activation_capacity() {
    let plan = premise_plan(
        &format!("once[0 ms, 10 ms] eventually {P}"),
        &tick_target(2, 2),
    );
    let spec = spec_from_plan(&plan).unwrap();
    let eventually = spec.nodes.iter().find(|n| n.interval.is_none()).unwrap().id;
    let inputs: Vec<Input> = [0, 1000, 2000]
        .into_iter()
        .map(|tick| Input::Reading(tick, vec![(spec.atoms[0], false)]))
        .collect();
    let entries = run_entries(&plan, &inputs);
    assert_eq!(entries, run_reference(&plan, &inputs));
    let refused = |step: usize| {
        entries[step]
            .iter()
            .filter(|(key, code)| key % 1000 == i128::try_from(eventually).unwrap() && *code == 7)
    };
    assert_eq!(refused(0).count() + refused(1).count(), 0);
    assert_eq!(
        refused(2).map(|(key, _)| key / 1000).collect::<Vec<_>>(),
        [2000],
        "the third instance is not started"
    );
}

// ---------------------------------------------------------------------
// "On each" activations, instance limits, gap vectors and plan identity.

use crate::monitor_plan::premise_plan_on;
use qsl_analyze::monitor_entries::{ActivationKind, ClauseSource};

const ON_EACH: &str = "on each (q: M::ConfigVersion)";

/// FR-251-AC-17 / TC-1081 step 12 (QSpec FR-487 "status", MP-81): an "on
/// each" instance records the position it was created at in the `position`
/// field of its `status` record, read from `State.event_position`; two events
/// in one tick get distinct positions.
#[trace("TC-1081", "FR-251-AC-17")]
#[test]
fn an_on_each_instance_records_the_event_position_it_was_created_at() {
    let plan = premise_plan_on(ROOT_SHAPES[0], ON_EACH, &tick_target(2, 8));
    let spec = spec_from_plan(&plan).unwrap();
    assert_eq!(spec.activation, ActivationKind::Each);
    let truths = |req: bool| vec![(spec.atoms[0], req), (spec.atoms[1], false)];
    let inputs = vec![
        Input::Reading(0, truths(false)),
        Input::Reading(7, truths(false)),
        Input::Reading(7, truths(false)),
    ];
    let (entries, positions) = run_entries_probe(
        &plan,
        &spec,
        &inputs,
        "map(e in s.activations: e.status.position)",
    );
    assert_eq!(positions, [vec![1], vec![1, 2], vec![1, 2, 3]]);
    assert_eq!(sorted(entries), sorted(run_reference(&plan, &inputs)));
}

/// A guarded "on each" activation evaluates the clause's own guard text and
/// starts an instance only at events where it holds; the entries agree with
/// the reference given the same guard truths.
#[trace("TC-1081", "FR-251-AC-17")]
#[test]
fn a_guarded_on_each_activation_starts_instances_where_the_guard_holds() {
    let plan = premise_plan_on(
        ROOT_SHAPES[0],
        "on each (q: M::ConfigVersion) when (q.versionNumber = 2)",
        &tick_target(2, 8),
    );
    let mut spec = spec_from_plan(&plan).unwrap();
    assert_eq!(spec.activation, ActivationKind::EachGuarded);
    spec.clause = Some(ClauseSource {
        parameters: vec![("versionNumber".to_owned(), "Integer".to_owned())],
        atoms: spec
            .atoms
            .iter()
            .zip(["versionNumber = 2", "versionNumber = 3"])
            .map(|(atom, text)| (*atom, text.to_owned()))
            .collect(),
        guard: Some("versionNumber = 2".to_owned()),
        profile: "v".to_owned(),
    });
    let (first, second) = (spec.atoms[0], spec.atoms[1]);
    // The admitted value: 2 holds the guard and the first atom, 3 the second.
    let readings = [(0_u64, 2), (100, 1), (200, 2), (300, 3)];
    let mut inputs: Vec<Input> = readings
        .iter()
        .map(|(tick, value)| {
            Input::Reading(*tick, vec![(first, *value == 2), (second, *value == 3)])
        })
        .collect();
    inputs.push(Input::Watermark(1_000));
    let entries = run_entries_with(&plan, &spec, &inputs);
    let mut reference = TickMonitorReference::new(reference_spec(&plan));
    let expected: Vec<Vec<(i128, i64)>> = inputs
        .iter()
        .zip(readings.iter().map(Some).chain(std::iter::once(None)))
        .map(|(input, value)| {
            let step = match (input, value) {
                (Input::Reading(tick, atoms), Some((_, value))) => TickStep::Reading {
                    reading: *tick,
                    atoms: atoms.iter().copied().collect(),
                    guard: *value == 2,
                },
                (Input::Watermark(boundary), _) => TickStep::Watermark {
                    boundary: *boundary,
                },
                _ => unreachable!("the trace is readings then one watermark"),
            };
            reference
                .step(&step)
                .expect("the step applies")
                .into_iter()
                .map(|r| {
                    (
                        r.origin * 1000 + i128::try_from(r.subformula).unwrap(),
                        reference_code(r.outcome),
                    )
                })
                .collect()
        })
        .collect();
    assert_eq!(sorted(entries.clone()), sorted(expected));
    let root = i128::try_from(spec.root).unwrap();
    let live: Vec<i128> = entries[4]
        .iter()
        .filter(|(key, code)| key % 1000 == root && *code == 3)
        .map(|(key, _)| key / 1000)
        .collect();
    let mut live = live;
    live.sort_unstable();
    assert_eq!(
        live,
        [0, 200],
        "no instance starts where the guard is false"
    );
}

/// FR-251-AC-14 / TC-1081 step 10: with an activation capacity of 2 the
/// third live instance is not started and settles
/// `Incomplete(LimitReached{activation})`; the two live ones stay pending and
/// are reported at every watermark.
#[trace("TC-1081", "FR-251-AC-14")]
#[test]
fn the_third_live_instance_is_refused_by_the_activation_capacity() {
    let plan = premise_plan_on(ROOT_SHAPES[0], ON_EACH, &tick_target(2, 2));
    let spec = spec_from_plan(&plan).unwrap();
    let root = i128::try_from(spec.root).unwrap();
    let req = vec![(spec.atoms[0], true), (spec.atoms[1], false)];
    let inputs = vec![
        Input::Reading(0, req.clone()),
        Input::Reading(100, req.clone()),
        Input::Reading(200, req),
        Input::Watermark(300),
    ];
    let entries = run_entries(&plan, &inputs);
    assert_eq!(
        sorted(entries.clone()),
        sorted(run_reference(&plan, &inputs))
    );
    assert!(
        entries[2].contains(&(200 * 1000 + root, 7)),
        "{:?}",
        entries[2]
    );
    assert!(!entries[1].iter().any(|(_, code)| *code == 7));
    // The live ones stay pending and are reported at the watermark.
    let pending: Vec<_> = entries[3]
        .iter()
        .filter(|(key, code)| key % 1000 == root && *code == 3)
        .map(|(key, _)| key / 1000)
        .collect();
    assert_eq!(pending, [0, 100]);
}

/// FR-251-AC-11 / TC-1081 step 8: after a gap fault `now` has advanced by the
/// gap and is the history start; the next reading is differenced from the
/// faulting one.
#[trace("TC-1081", "FR-251-AC-11")]
#[test]
fn a_gap_advances_now_by_the_gap_and_the_next_reading_is_a_delta_of_ten() {
    let target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 16, 100);
    let plan = premise_plan("eventually[0 ms, 3 ms] holds(c.versionNumber = 3)", &target);
    let spec = spec_from_plan(&plan).unwrap();
    let truths = vec![(spec.atoms[0], false)];
    let inputs = vec![
        Input::Reading(65_530, truths.clone()),
        Input::Reading(65_500, truths.clone()),
        Input::Reading(65_510, truths),
    ];
    let (entries, probes) = run_entries_probe(
        &plan,
        &spec,
        &inputs,
        "ints4(s.now, s.history_start, s.event_position, size(s.buffer_0))",
    );
    assert_eq!(entries, run_reference(&plan, &inputs));
    assert_eq!(probes[0], [0, -1, 1, 1]);
    assert_eq!(probes[1], [65_506, 65_506, 1, 0]);
    assert_eq!(probes[2], [65_516, 65_506, 2, 1]);
}

/// FR-251-AC-15 / TC-1081 step 11: after a gap the buffers are empty and
/// `history_start` is the faulting reading's tick; `once[0 ms, 10 ms]` is
/// indeterminate until its window has refilled past the marker, then decides.
#[trace("TC-1081", "FR-251-AC-15")]
#[test]
fn a_past_window_is_indeterminate_until_it_has_refilled_past_the_marker() {
    let target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 16, 2000);
    let plan = premise_plan("once[0 ms, 10 ms] holds(c.versionNumber = 3)", &target);
    let spec = spec_from_plan(&plan).unwrap();
    let node = i128::try_from(interval_node(&plan)).unwrap();
    let truths = |p: bool| vec![(spec.atoms[0], p)];
    let mut inputs = vec![
        Input::Reading(65_530, truths(true)),
        Input::Reading(65_500, truths(false)),
        Input::Poll(1),
    ];
    let mut raw = 65_500_u64;
    for _ in 0..13 {
        raw = (raw + 1000) % 65_536;
        inputs.push(Input::Reading(raw, truths(false)));
    }
    let (entries, probes) = run_entries_probe(&plan, &spec, &inputs, "ints1(s.history_start)");
    assert_eq!(entries, run_reference(&plan, &inputs));
    assert_eq!(
        probes[1],
        [65_506],
        "the marker is the faulting reading's tick"
    );
    let codes: Vec<i64> = entries[3..]
        .iter()
        .map(|step| step.iter().find(|(key, _)| key % 1000 == node).unwrap().1)
        .collect();
    assert_eq!(codes[0], 4, "the first window reaches before the marker");
    assert_eq!(codes.last(), Some(&2), "the refilled window decides");
    assert!(codes.windows(2).all(|w| !(w[0] == 2 && w[1] == 4)));
}

/// FR-251-AC-6 / TC-1081 step 3, the isolation half: two monitors built from
/// one plan share no state, and each result keeps its subject and obligation
/// origin. Refusing to rebind a live monitor to another program or target is
/// RT's executor and is not covered here.
#[trace("TC-1081", "FR-251-AC-6")]
#[test]
fn two_monitors_from_one_plan_share_no_state() {
    let plan = deadline_plan();
    let spec = spec_from_plan(&plan).unwrap();
    let atom = spec.atoms[0];
    let source = format!(
        "{}\nfunction scenario using v(): Sequence<Integer> pure {{ \
            let a0 = monitorInit() in let b0 = monitorInit() in \
            let a1 = onEventR(a0, 0, Inputs {{ a{atom}: false }}) in \
            sequence[size(a1.state.buffer_0), size(b0.buffer_0), size(a1.state.activations), size(b0.activations), \
                     size(map(r in a1.results: r.instance.subject))] }}",
        render(&spec)
    );
    let package = package_of(&source);
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("scenario").unwrap(),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .unwrap();
    let FamilyOutcome::Evaluated(Outcome::Completed(Value::Collection(sizes))) = evaluation.outcome
    else {
        panic!("a completed sequence");
    };
    let sizes: Vec<i128> = sizes
        .elements()
        .iter()
        .map(|value| match value {
            Value::Integer(value) => value.as_big().try_into().unwrap(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(sizes[..4], [1, 0, 0, 0], "the second monitor holds nothing");
}

/// FR-251-AC-9 / TC-1081 step 6 and FR-251-AC-5 (the entry and init bodies,
/// which a `TimedMonitorPlan` does not hold): changing the body of any one of
/// the four entries changes that entry's content key and the plan root's, and
/// no other entry's. The keys are the ones the strict reader derives from the
/// package, read from the entries the root's own members name. An entry's key
/// carries the entry's own body; the named helper functions it calls are
/// keyed by name, so a change inside a helper is not covered by this key.
#[trace("TC-1081", "FR-251-AC-9")]
#[trace("TC-1081", "FR-251-AC-5")]
#[test]
fn changing_any_one_entry_body_changes_the_plan_root_key() {
    use crate::monitor_root::{keys, read_back};
    let formula = ROOT_SHAPES[0];
    let target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600);
    let keys_of = |tweak: &dyn Fn(String) -> String| {
        let (bytes, evidence) = root_bytes_with(formula, &target, |_| {}, |_| {}, tweak);
        keys(&read_back(&bytes, &evidence))
    };
    let base = keys_of(&|source| source);
    assert_eq!(
        base,
        keys_of(&|source| source),
        "the keys are deterministic"
    );
    // The four entries differ pairwise, so a swapped member would show.
    for (a, left) in base.entries.iter().enumerate() {
        for right in &base.entries[a + 1..] {
            assert_ne!(left, right);
        }
    }
    // (entry index, text changed, replacement): init, on_event,
    // on_watermark, on_gap. Each is text of the entry's own body.
    for (entry, from, to) in [
        (0, "history_start: 0 - 1", "history_start: 0 - 2"),
        (
            1,
            "onEventR(s, delta, i) in",
            "onEventR(s, delta + 0, i) in",
        ),
        (
            2,
            "onWatermarkR(s, boundary, scope_in) in",
            "onWatermarkR(s, boundary + 0, scope_in) in",
        ),
        (3, "onGapR(s, gap) in", "onGapR(s, gap + 0) in"),
    ] {
        let variant = keys_of(&|source| {
            assert!(source.contains(from), "{from}");
            source.replacen(from, to, 1)
        });
        assert_ne!(variant.root, base.root, "entry {entry}: the root key");
        assert_ne!(
            variant.entries[entry], base.entries[entry],
            "entry {entry}: its own key"
        );
        for other in (0..4).filter(|other| *other != entry) {
            assert_eq!(
                variant.entries[other], base.entries[other],
                "entry {entry} changed entry {other}"
            );
        }
    }
}

/// The closed program holds both atom bodies, the entries and the state
/// type as nodes of the emitted package. This is the node-presence half of
/// FR-251-AC-4 / TC-1081 step 1 only; building a monitor from the plan alone
/// and refusing a removed transition body need RT's executor, so the
/// criterion is not claimed here.
#[test]
fn the_emitted_program_holds_the_atom_bodies_the_entries_and_the_state_type() {
    let formula = ROOT_SHAPES[0];
    let target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600);
    let wire = root_wire_with(
        formula,
        &target,
        |_| {},
        |spec| {
            spec.clause = Some(ClauseSource {
                parameters: vec![("versionNumber".to_owned(), "Integer".to_owned())],
                atoms: spec
                    .atoms
                    .iter()
                    .zip(["versionNumber = 2", "versionNumber = 3"])
                    .map(|(atom, text)| (*atom, text.to_owned()))
                    .collect(),
                guard: None,
                profile: "v".to_owned(),
            });
        },
        |source| source,
    );
    let names: Vec<String> = wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|node| node["declaration"]["qualified_name"][0].as_str())
        .map(str::to_owned)
        .collect();
    for wanted in ["State", "Inputs", "Scope", "Result", "Outcome"] {
        assert!(names.iter().any(|n| n == wanted), "{wanted}");
    }
    let atoms = spec_from_plan(&premise_plan(formula, &target))
        .unwrap()
        .atoms;
    for atom in atoms {
        assert!(names.contains(&format!("atom{atom}")), "atom{atom}");
    }
}
