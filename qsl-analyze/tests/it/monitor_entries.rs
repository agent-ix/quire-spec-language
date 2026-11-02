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
        root: None,
        clause: None,
        unprojectable: Vec::new(),
        tick_unit: None,
    }
}

pub(crate) fn package_of(declarations: &str) -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         {units}{declarations}\n",
        units = crate::monitor_plan::TIME_UNITS
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
        TickOutcome::ActivationLimit { .. } => 7,
        TickOutcome::AtomProjection { atom } => 100 + i64::try_from(atom).unwrap(),
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
        activation_capacity: plan.activation_capacity(),
        unprojectable: Default::default(),
    }
}

/// Per input, the sorted `(origin, code)` results of the interval node.
pub(crate) fn run_reference(
    plan: &TimedMonitorPlan,
    nodes: &[usize],
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
                .filter(|result| nodes.contains(&result.subformula))
                .map(|result| {
                    (
                        result.origin * 1000 + i128::try_from(result.subformula).unwrap(),
                        reference_code(result.outcome),
                    )
                })
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
    run_entries_with(plan, &spec, inputs)
}

/// [`run_entries`] over `spec`, which may evaluate its atoms from admitted
/// values (`spec.clause`).
pub(crate) fn run_entries_with(
    plan: &TimedMonitorPlan,
    spec: &qsl_analyze::monitor_entries::EntrySpec,
    inputs: &[Input],
) -> Vec<Vec<(i128, i64)>> {
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
                        None => spec
                            .atoms
                            .iter()
                            .map(|atom| format!("a{atom}: {}", truth(atom)))
                            .collect::<Vec<_>>()
                            .join(", "),
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
        encoded.push(format!("dump(s{step_no})"));
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
        function code using v(o: Outcome): Integer pure { case o { Outcome::Failed(f): 6; Outcome::False: 2; Outcome::Incomplete(l): if l.limit = \"activation\" then 7 else if l.limit = \"atom_projection\" then 100 + l.amount else 5; Outcome::Indeterminate: 4; Outcome::Pending: 3; Outcome::True: 1; } }\n\
        function dump using v(s: State): Sequence<Integer> pure { map(x in s.buffer_0: 1000000000000 + x.tFirst * 10000 + x.tLast * 10 + (if x.lost then 1 else 0)) }\n\
        function encode using v(rs: Results): Sequence<Integer> pure { map(r in rs: ((r.instance.origin_tick * 1000 + r.instance.subformula) * 1000) + code(r.outcome)) }\n";
    let package = package_of(&format!("{}\n{helpers}\n{body}", render(spec)));
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
        if number >= 1_000_000_000_000 {
            if std::env::var("QSL_DUMP").is_ok() {
                eprintln!("  entries cell {}", number - 1_000_000_000_000);
            }
        } else if number < 0 {
            steps.push(Vec::new());
        } else {
            steps
                .last_mut()
                .unwrap()
                .push((number / 1000, i64::try_from(number % 1000).unwrap()));
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
    let node = [spec_from_plan(&plan).unwrap().node.subformula];
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
        run_reference(&plan, &node, &inputs)
    );
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
        let entries = run_entries(&plan, &inputs);
        let mut nodes = vec![spec.node.subformula];
        nodes.extend(spec.root.as_ref().map(|root| root.index));
        let reference = run_reference(&plan, &nodes, &inputs);
        assert_eq!(
            entries, reference,
            "{formula} {target:?} seed {seed}\n{inputs:#?}"
        );
    }
}

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

fn microsecond_target(
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

use quire_exact::Rational;

#[test]
fn every_operator_agrees_with_the_reference_without_uncertainty() {
    for shape in SHAPES {
        differential(
            shape,
            microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600),
            25,
        );
    }
}

#[test]
fn every_operator_agrees_with_the_reference_under_uncertainty() {
    for shape in SHAPES {
        differential(
            shape,
            microsecond_target(crate::monitor_plan::ratio(1, 2), 2, 12, 3600),
            25,
        );
    }
}

#[test]
fn small_buffers_overflow_the_same_way() {
    for shape in SHAPES {
        differential(
            shape,
            microsecond_target(crate::monitor_plan::ratio(0, 1), 1, 12, 3600),
            25,
        );
    }
}

#[test]
fn reading_gaps_fail_the_open_instances_the_same_way() {
    for shape in SHAPES {
        differential(
            shape,
            microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 1500),
            25,
        );
    }
}

const ROOT_SHAPES: [&str; 5] = [
    "always (holds(c.versionNumber = 2) implies eventually[0 ms, 3 ms] holds(c.versionNumber = 3))",
    "always once[0 ms, 3 ms] holds(c.versionNumber = 3)",
    "eventually (holds(c.versionNumber = 2) and always[0 ms, 3 ms] holds(c.versionNumber = 3))",
    "always (eventually[0 ms, 3 ms] holds(c.versionNumber = 3) implies holds(c.versionNumber = 2))",
    "always historically[1 ms, 3 ms] holds(c.versionNumber = 3)",
];

#[test]
fn unbounded_roots_fold_one_outcome_per_activation_like_the_reference() {
    for shape in ROOT_SHAPES {
        differential(
            shape,
            microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600),
            25,
        );
        differential(
            shape,
            microsecond_target(crate::monitor_plan::ratio(1, 2), 2, 12, 3600),
            25,
        );
        differential(
            shape,
            microsecond_target(crate::monitor_plan::ratio(0, 1), 1, 12, 3600),
            25,
        );
        differential(
            shape,
            microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 1500),
            25,
        );
    }
}

#[test]
fn a_zero_activation_capacity_never_starts_the_root() {
    let mut target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600);
    target.max_activations = 0;
    for shape in ROOT_SHAPES {
        differential(shape, target.clone(), 10);
    }
}

#[test]
fn dump_entries_wire() {
    let package = package_of(&render(&deadline_spec()));
    let emission = qsl_package::emit_checked(&package).expect("emits");
    std::fs::write("/tmp/q1463-wire.json", emission.package().bytes()).unwrap();
}

/// The plan-root wire (QSpec FR-487, FR-322-AC-308/309): the generated
/// entries become declaration-less function nodes and one
/// `temporal`/`monitor_plan` node names them with the plan's premises.
#[test]
fn the_plan_root_is_written_with_seventeen_bindings() {
    use qsl_package::{emit_checked, emit_monitor_plan_over, MonitorPlanWire};
    let formula = ROOT_SHAPES[0];
    let target = microsecond_target(crate::monitor_plan::ratio(0, 1), 2, 12, 3600);
    let plan = premise_plan(formula, &target);
    let spec = spec_from_plan(&plan).unwrap();
    let (package, _) = crate::monitor_plan::checked_clause_in(formula, &render(&spec), true);
    let emission = emit_checked(&package).expect("the unit emits");
    let name = qsl_semantics::library::LibraryName::new(emission.package().name()).unwrap();
    let domain = crate::units::Domain::config_version();
    let mut evidence = emission.evidence().clone();
    evidence
        .insert_domain_package_document(domain.package, domain.document.clone())
        .unwrap();
    let premises = plan.root_premises();
    let buffers: Vec<(usize, u64)> = plan
        .buffers()
        .iter()
        .map(|(id, c)| (id.index(), *c))
        .collect();
    let wire = MonitorPlanWire {
        formula: plan.program().formula(),
        counter: &premises.counter,
        counter_width_bits: premises.counter_width_bits,
        max_reading_gap: premises.max_reading_gap,
        max_event_rate: &premises.max_event_rate,
        uncertainty: &premises.uncertainty,
        buffers: &buffers,
        tick_unit: plan.binding().unit(),
        activation_capacity: premises.activation_capacity,
        state: "State",
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
    let rooted = emit_monitor_plan_over(&emission, name, &evidence, &wire)
        .unwrap_or_else(|refusal| panic!("{refusal:?}"));
    let json: serde_json::Value = serde_json::from_slice(rooted.package().bytes()).unwrap();
    let nodes = json["semantic_graph"]["nodes"].as_array().unwrap();
    let roots: Vec<&serde_json::Value> = nodes
        .iter()
        .filter(|n| n["node_tag"] == "temporal" && n["semantic_form"] == "monitor_plan")
        .collect();
    assert_eq!(roots.len(), 1);
    let members = roots[0]["body"]["members"].as_array().unwrap();
    let names: Vec<&str> = members
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, qsl_replay::PLAN_ROOT_BINDINGS.to_vec());
    // The entries are declaration-less function nodes.
    for name in ["init", "on_event", "on_watermark", "on_gap"] {
        let member = members.iter().find(|m| m["name"] == name).unwrap();
        let target = &member["value"]["target"];
        let entry = nodes.iter().find(|n| &n["node_id"] == target).unwrap();
        assert_eq!(entry["node_tag"], "function");
        assert!(entry.get("declaration").is_none() && entry.get("owner").is_none());
    }
    // FR-487 field spellings.
    let record = |name: &str| {
        nodes
            .iter()
            .find(|n| n["declaration"]["qualified_name"] == serde_json::json!([name]))
            .unwrap()
    };
    let fields = |name: &str| -> Vec<String> {
        record(name)["body"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["name"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(fields("Instance"), ["subject", "subformula", "origin"]);
    assert_eq!(fields("Activation"), ["subformula", "origin", "status"]);
    assert_eq!(fields("LimitReached"), ["limit", "value", "setting"]);
    assert_eq!(
        fields("State"),
        [
            "now",
            "history_start",
            "buffer_0",
            "activations",
            "event_position"
        ]
    );
}

/// The wire of the plan root for `formula` under `target`, with the root's
/// own identity-bearing premises replaced by `edit`.
fn root_wire(
    formula: &str,
    target: &qsl_replay::MonitorTarget,
    edit: impl Fn(&mut Premises),
) -> serde_json::Value {
    use qsl_package::{emit_checked, emit_monitor_plan_over, MonitorPlanWire};
    let plan = premise_plan(formula, target);
    let spec = spec_from_plan(&plan).unwrap();
    let (package, _) = crate::monitor_plan::checked_clause_in(formula, &render(&spec), true);
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
    };
    edit(&mut premises);
    let buffers: Vec<(usize, u64)> = plan
        .buffers()
        .iter()
        .map(|(id, c)| (id.index(), *c + premises.capacity_delta))
        .collect();
    let root = plan.root_premises();
    let wire = MonitorPlanWire {
        formula: plan.program().formula(),
        counter: &premises.counter,
        counter_width_bits: root.counter_width_bits,
        max_reading_gap: root.max_reading_gap,
        max_event_rate: &premises.rate,
        uncertainty: &premises.uncertainty,
        buffers: &buffers,
        tick_unit: &premises.tick_unit,
        activation_capacity: premises.activation_capacity,
        state: "State",
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
    let rooted = emit_monitor_plan_over(&emission, name, &evidence, &wire)
        .unwrap_or_else(|refusal| panic!("{refusal:?}"));
    serde_json::from_slice(rooted.package().bytes()).unwrap()
}

struct Premises {
    counter: String,
    tick_unit: String,
    rate: Rational,
    uncertainty: Rational,
    activation_capacity: u64,
    capacity_delta: u64,
}

fn root_key(wire: &serde_json::Value) -> serde_json::Value {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["node_tag"] == "temporal" && n["semantic_form"] == "monitor_plan")
        .unwrap()["node_id"]
        .clone()
}

/// FR-487-AC-6 (MP-02, MP-33, MP-43, MP-44, MP-45, MP-58): each
/// identity-bearing premise alone gives a different root key.
#[test]
fn each_root_premise_alone_changes_the_root_key() {
    let formula = ROOT_SHAPES[0];
    let target = microsecond_target(crate::monitor_plan::ratio(1, 2), 2, 12, 3600);
    let base = root_key(&root_wire(formula, &target, |_| {}));
    let variants: Vec<(&str, serde_json::Value)> = vec![
        (
            "MP-02 capacity",
            root_key(&root_wire(formula, &target, |p| p.capacity_delta = 1)),
        ),
        (
            "MP-33 uncertainty",
            root_key(&root_wire(formula, &target, |p| {
                p.uncertainty = crate::monitor_plan::ratio(0, 1)
            })),
        ),
        (
            "MP-43 counter",
            root_key(&root_wire(formula, &target, |p| {
                p.counter = "ticks2".to_owned()
            })),
        ),
        (
            "MP-44 rate",
            root_key(&root_wire(formula, &target, |p| {
                p.rate = crate::monitor_plan::ratio(1, 10)
            })),
        ),
        (
            "MP-45 tick unit",
            root_key(&root_wire(formula, &target, |p| {
                p.tick_unit = "ms".to_owned()
            })),
        ),
        (
            "MP-58 activations",
            root_key(&root_wire(formula, &target, |p| p.activation_capacity += 1)),
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
    let entries = run_entries(&plan, &inputs);
    let reference = run_reference(&plan, &[spec.node.subformula], &inputs);
    assert_eq!(entries, reference);
    (spec.node.subformula, entries)
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
        profile: "v".to_owned(),
    });
    let mut nodes = vec![spec.node.subformula];
    nodes.extend(spec.root.as_ref().map(|root| root.index));
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
            run_entries_with(&plan, &spec, &inputs),
            run_reference(&plan, &nodes, &inputs),
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
        let mut nodes = vec![spec.node.subformula];
        nodes.extend(spec.root.as_ref().map(|root| root.index));
        let reference = |unprojectable: bool| {
            let mut ref_spec = reference_spec(&plan);
            if unprojectable {
                ref_spec.unprojectable.insert(atom);
            }
            ref_spec
        };
        let mut saw_projection = false;
        for seed in 0..15 {
            let inputs = trace(seed * 7919 + 13, &spec.atoms, target.width_bits, 14);
            let entries = run_entries_with(&plan, &spec, &inputs);
            let mut monitor = TickMonitorReference::new(reference(true));
            let expected: Vec<Vec<(i128, i64)>> = inputs
                .iter()
                .map(|input| {
                    let step = match input {
                        Input::Reading(reading, atoms) => TickStep::Reading {
                            reading: *reading,
                            atoms: atoms.iter().copied().collect(),
                            guard: true,
                        },
                        Input::Watermark(boundary) => TickStep::Watermark {
                            boundary: *boundary,
                        },
                        Input::Poll(reading) => TickStep::Poll { reading: *reading },
                    };
                    let mut results: Vec<(i128, i64)> = monitor
                        .step(&step)
                        .unwrap()
                        .into_iter()
                        .filter(|result| nodes.contains(&result.subformula))
                        .map(|result| {
                            (
                                result.origin * 1000 + i128::try_from(result.subformula).unwrap(),
                                reference_code(result.outcome),
                            )
                        })
                        .collect();
                    results.sort_unstable();
                    results
                })
                .collect();
            saw_projection |= expected
                .iter()
                .flatten()
                .any(|(_, code)| *code == 100 + i64::try_from(atom).unwrap());
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
