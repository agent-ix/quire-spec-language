// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251/FR-252 monitor plan producer tests over QSL-580's checked timed
//! front end.

use ix_trace_rs::trace;
use qsl_analyze::monitor_plan::produce;
use qsl_foundation::{SourceIdentity, Span};
use qsl_package::CheckedPackage;
use qsl_replay::{MonitorPlanRefusal, MonitorTarget, TimingObligation, TimingObligationSettings};
use qsl_semantics::check::{ModelTimeSource, PackageDeclarations, TemporalProfile, TimeSourceKind};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::admit_unit;
use quire_exact::{Integer, Rational};

use crate::units::Domain;

pub(crate) const TIME_UNITS: &str = "dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    unit ms : Time = rational(1, 1000) * s;\n\
    unit us : Time = rational(1, 1000000) * s;\n";

pub(crate) fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

/// The text of a unit holding one timed clause `C` over a `ConfigVersion`,
/// its time source declared in the unit when `real_time` (so the package
/// emits), and `extra` declarations appended.
pub(crate) fn unit_source(formula: &str, extra: &str, real_time: bool, activation: &str) -> String {
    let domain = Domain::config_version();
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t = \"quire.temporal.timed/v1\";\n\
         {complete}\
         model M = \"{}\";\n\
         {TIME_UNITS}\
         {time_decl}\
         temporal C using t over (c: M::ConfigVersion) clock \"model-time\" {activation} {{ {formula} }}\n\
         {extra}\n",
        domain.package,
        complete = if real_time {
            "profile tc = \"quire.temporal.complete/v1\";\n"
        } else {
            ""
        },
        time_decl = if real_time { "time M dense unit ms;\n" } else { "" },
    )
}

pub(crate) fn checked_clause(formula: &str) -> (CheckedPackage, usize) {
    checked_clause_with(formula, "")
}

/// [`checked_clause`] with `extra` declarations appended to the unit.
pub(crate) fn checked_clause_with(formula: &str, extra: &str) -> (CheckedPackage, usize) {
    checked_clause_in(formula, extra, false)
}

/// [`checked_clause_with`], the model's time source declared in the unit
/// (`time M dense unit ms`) when `real_time`, so the package emits.
pub(crate) fn checked_clause_in(
    formula: &str,
    extra: &str,
    real_time: bool,
) -> (CheckedPackage, usize) {
    checked_clause_on(formula, extra, real_time, "on origin")
}

/// [`checked_clause_in`] with the clause's `activation` spelled out.
pub(crate) fn checked_clause_on(
    formula: &str,
    extra: &str,
    real_time: bool,
    activation: &str,
) -> (CheckedPackage, usize) {
    let domain = Domain::config_version();
    let packages =
        qsl_semantics::model::intake::package_input([domain.document.as_slice()]).unwrap();
    let source = SourceIdentity::new("agent-ix", "monitor-plan", "git", "1");
    let unit = unit_source(formula, extra, real_time, activation);
    let parsed = qsl_cst::parse(
        source.clone(),
        "monitor.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 parses");
    assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model admission")
    .with_parsed_operation_clauses(|text| {
        qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
    })
    .expect("operation clauses parse");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "package".to_owned(),
        built,
        models,
        Vec::new(),
    )
    .expect("S3 assembly");
    if !real_time {
        declarations.time_sources.push(ModelTimeSource {
            model: "M".to_owned(),
            span: Span { start: 0, end: 1 },
            model_node: crate::fixture_ids::node_key([0]),
            tick_operation_key: None,
            tick_operation_span: None,
            tick_operation_anchor: None,
            source: TimeSourceKind::Dense,
            unit: "ms".to_owned(),
            unit_node: crate::fixture_ids::node_key([0]),
            time_dimension: crate::fixture_ids::node_key([0]),
            nominal_nodes: Vec::new(),
        });
    }
    let graph = declarations
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("S3: {refusals:?}"));
    assert_eq!(graph.temporal_clauses().len(), 1);
    let atom_count = graph.temporal_clauses()[0]
        .clause()
        .formula()
        .nodes()
        .iter()
        .filter(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
        .count();
    (CheckedPackage::link(graph), atom_count)
}

#[trace("TC-706", "FR-251")]
#[test]
fn producer_keeps_checked_formula_atoms_and_converts_intervals_to_ticks() {
    let (package, atom_count) = checked_clause(
        "always (holds(c.versionNumber = 2) implies eventually[0 ms, 3 ms] holds(c.versionNumber = 3))",
    );
    assert_eq!(atom_count, 2);
    let clause = package.graph().temporal_clauses()[0].clause();
    assert_eq!(clause.profile(), TemporalProfile::Timed);
    let target = MonitorTarget {
        counter: "counter".to_owned(),
        unit: "ms".to_owned(),
        width_bits: 20,
        uncertainty: ratio(0, 1),
        max_event_rate: Some(ratio(2, 1)),
        max_reading_gap: 100,
        max_activations: 8,
    };
    let (plan, obligations) = produce(
        &package,
        qsl_semantics::library::LibraryName::new("fixture").expect("a library name"),
        clause,
        &target,
        &TimingObligationSettings::default(),
    )
    .expect("checked timed clause derives a plan");

    assert_eq!(plan.program().atoms().len(), 2);
    assert_eq!(plan.intervals().len(), 1);
    assert_eq!(plan.intervals()[0].1.lower(), &ratio(0, 1));
    assert_eq!(plan.intervals()[0].1.upper(), &ratio(3, 1));
    assert_eq!(plan.buffers(), &[(plan.intervals()[0].0, 9)]);
    assert!(matches!(
        obligations[0],
        TimingObligation::MonitorAgreement { events: 8, .. }
    ));
    assert!(matches!(
        &obligations[1],
        TimingObligation::TickArithmetic { width_bits: 20, rate, .. } if rate == &ratio(2, 1)
    ));
}

#[trace("TC-706", "FR-251")]
#[test]
fn producer_sizes_the_ten_millisecond_past_window_as_twenty_one_events() {
    let (package, _) = checked_clause("once[0 ms, 10 ms] holds(c.versionNumber = 2)");
    let clause = package.graph().temporal_clauses()[0].clause();
    let target = MonitorTarget {
        counter: "counter".to_owned(),
        unit: "ms".to_owned(),
        width_bits: 16,
        uncertainty: ratio(0, 1),
        max_event_rate: Some(ratio(2, 1)),
        max_reading_gap: 100,
        max_activations: 8,
    };
    let (plan, _) = produce(
        &package,
        qsl_semantics::library::LibraryName::new("fixture").expect("a library name"),
        clause,
        &target,
        &TimingObligationSettings::default(),
    )
    .expect("checked timed clause derives a plan");
    assert_eq!(plan.buffers().len(), 1);
    assert_eq!(plan.buffers()[0].1, 23);
}

#[trace("TC-706", "FR-251")]
#[test]
fn producer_converts_the_default_one_per_microsecond_rate_to_target_units() {
    let (package, _) = checked_clause("once[0 ms, 1 ms] holds(c.versionNumber = 2)");
    let clause = package.graph().temporal_clauses()[0].clause();
    let target = MonitorTarget {
        counter: "counter".to_owned(),
        unit: "ms".to_owned(),
        width_bits: 16,
        uncertainty: ratio(0, 1),
        max_event_rate: None,
        max_reading_gap: 100,
        max_activations: 8,
    };
    let (plan, _) = produce(
        &package,
        qsl_semantics::library::LibraryName::new("fixture").expect("a library name"),
        clause,
        &target,
        &TimingObligationSettings::default(),
    )
    .expect("checked timed clause derives a plan");
    assert_eq!(plan.rate_limit(), &ratio(1_000, 1));
    assert_eq!(plan.buffers()[0].1, 2_001);
}

#[trace("TC-706", "FR-251")]
#[test]
fn producer_refuses_a_gap_at_least_one_full_counter_wrap() {
    let (package, _) = checked_clause("always holds(c.versionNumber = 2)");
    let clause = package.graph().temporal_clauses()[0].clause();
    let target = MonitorTarget {
        counter: "counter".to_owned(),
        unit: "ms".to_owned(),
        width_bits: 16,
        uncertainty: ratio(0, 1),
        max_event_rate: None,
        max_reading_gap: 65_536,
        max_activations: 8,
    };
    assert_eq!(
        produce(
            &package,
            qsl_semantics::library::LibraryName::new("fixture").expect("a library name"),
            clause,
            &target,
            &TimingObligationSettings::default()
        ),
        Err(MonitorPlanRefusal::GapExceedsWidth {
            max_reading_gap: 65_536,
            width_bits: 16,
        })
    );
}

pub(crate) fn premise_plan(formula: &str, target: &MonitorTarget) -> qsl_replay::TimedMonitorPlan {
    premise_plan_on(formula, "on origin", target)
}

/// [`premise_plan`] for a clause activated by `activation`.
pub(crate) fn premise_plan_on(
    formula: &str,
    activation: &str,
    target: &MonitorTarget,
) -> qsl_replay::TimedMonitorPlan {
    let (package, _) = checked_clause_on(formula, "", false, activation);
    let clause = package.graph().temporal_clauses()[0].clause();
    produce(
        &package,
        qsl_semantics::library::LibraryName::new("fixture").expect("a library name"),
        clause,
        target,
        &TimingObligationSettings::default(),
    )
    .expect("plan")
    .0
}

pub(crate) fn base_target() -> MonitorTarget {
    MonitorTarget {
        counter: "counter".to_owned(),
        unit: "ms".to_owned(),
        width_bits: 20,
        uncertainty: ratio(0, 1),
        max_event_rate: Some(ratio(2, 1)),
        max_reading_gap: 100,
        max_activations: 8,
    }
}

/// FR-487-AC-6 (MP-02, MP-33, MP-43, MP-44): each identity-bearing premise
/// alone changes the plan identity, and so does the formula's interval.
#[trace("TC-1081", "FR-251-AC-5")]
#[test]
fn each_premise_alone_changes_the_complete_plan_identity() {
    let formula = "always (holds(c.versionNumber = 2) implies eventually[0 ms, 3 ms] holds(c.versionNumber = 3))";
    let base = premise_plan(formula, &base_target());
    assert_eq!(base.id(), premise_plan(formula, &base_target()).id());
    let changed: Vec<MonitorTarget> = vec![
        MonitorTarget {
            counter: "counter2".to_owned(),
            ..base_target()
        },
        MonitorTarget {
            width_bits: 24,
            ..base_target()
        },
        MonitorTarget {
            uncertainty: ratio(1, 2),
            ..base_target()
        },
        MonitorTarget {
            max_event_rate: Some(ratio(1, 10)),
            ..base_target()
        },
        MonitorTarget {
            max_reading_gap: 101,
            ..base_target()
        },
        MonitorTarget {
            max_event_rate: Some(ratio(3, 1)),
            ..base_target()
        },
    ];
    for target in changed {
        assert_ne!(base.id(), premise_plan(formula, &target).id(), "{target:?}");
    }
    let other = premise_plan(
        "always (holds(c.versionNumber = 2) implies eventually[0 ms, 4 ms] holds(c.versionNumber = 3))",
        &base_target(),
    );
    assert_ne!(base.id(), other.id());
    let swapped = premise_plan(
        "always (eventually[0 ms, 3 ms] holds(c.versionNumber = 3) implies holds(c.versionNumber = 2))",
        &base_target(),
    );
    assert_ne!(base.id(), swapped.id());
}

/// FR-251-AC-2 and AC-13: on a 1 MHz counter `once[0 ms, 10 ms]` and
/// `once[5 ms, 10 ms]` both get capacity 22 (upper bound, not width).
#[trace("TC-706", "FR-251-AC-13")]
#[test]
fn capacity_uses_the_upper_bound_plus_the_tick_cover() {
    for formula in [
        "once[0 ms, 10 ms] holds(c.versionNumber = 2)",
        "once[5 ms, 10 ms] holds(c.versionNumber = 2)",
    ] {
        let target = MonitorTarget {
            unit: "us".to_owned(),
            max_event_rate: Some(ratio(2, 1000)),
            ..base_target()
        };
        assert_eq!(
            premise_plan(formula, &target).buffers()[0].1,
            22,
            "{formula}"
        );
    }
}

/// FR-487-AC-4 premise ranges (MP-18, MP-21, MP-34, MP-36, MP-38): the
/// producer refuses what the reader would.
#[trace("TC-978", "FR-487-AC-4")]
#[test]
fn producer_premises_meet_the_plan_root_reader_ranges() {
    let premises = premise_plan(
        "once[0 ms, 10 ms] holds(c.versionNumber = 2)",
        &base_target(),
    )
    .root_premises();
    assert_eq!(premises.validate(), Ok(()));
    assert_eq!(
        qsl_replay::canonical_rational(&premises.max_event_rate),
        "2"
    );
    assert_eq!(qsl_replay::canonical_rational(&ratio(1, 10)), "1/10");
    for (edit, defect) in [
        (
            qsl_replay::PlanRootPremises {
                counter_width_bits: 0,
                ..premises.clone()
            },
            qsl_replay::PremiseDefect::CounterWidth,
        ),
        (
            qsl_replay::PlanRootPremises {
                counter_width_bits: 65,
                ..premises.clone()
            },
            qsl_replay::PremiseDefect::CounterWidth,
        ),
        (
            qsl_replay::PlanRootPremises {
                max_reading_gap: 1 << 20,
                ..premises.clone()
            },
            qsl_replay::PremiseDefect::MaxReadingGap,
        ),
        (
            qsl_replay::PlanRootPremises {
                max_event_rate: ratio(0, 1),
                ..premises.clone()
            },
            qsl_replay::PremiseDefect::MaxEventRate,
        ),
        (
            qsl_replay::PlanRootPremises {
                max_event_rate: ratio(-1, 2),
                ..premises.clone()
            },
            qsl_replay::PremiseDefect::MaxEventRate,
        ),
        (
            qsl_replay::PlanRootPremises {
                counter: String::new(),
                ..premises.clone()
            },
            qsl_replay::PremiseDefect::Counter,
        ),
        (
            qsl_replay::PlanRootPremises {
                uncertainty: ratio(-1, 2),
                ..premises.clone()
            },
            qsl_replay::PremiseDefect::Uncertainty,
        ),
    ] {
        assert_eq!(edit.validate(), Err(defect));
    }
}

/// FR-251-AC-5 (TC-1081 step 2): a plan with any one component replaced is
/// refused at admission, and the plan for another target is a distinct,
/// admitted plan.
#[trace("TC-1081", "FR-251-AC-5")]
#[test]
fn admission_refuses_a_plan_with_one_component_replaced() {
    use qsl_analyze::monitor_plan::{admit, PlanAdmissionRefusal};
    use qsl_replay::TimedMonitorPlan;
    let formula = "always (holds(c.versionNumber = 2) implies eventually[0 ms, 3 ms] holds(c.versionNumber = 3))";
    let (package, _) = checked_clause(formula);
    let clause = package.graph().temporal_clauses()[0].clause();
    let library = || qsl_semantics::library::LibraryName::new("fixture").expect("a library name");
    let target = base_target();
    let plan = premise_plan(formula, &target);
    let id = admit(&package, library(), clause, &target, &plan).expect("derived plan admits");
    assert_eq!(id, plan.id());

    let rebuild = |buffers: Option<Vec<_>>, gap: Option<u64>, unc: Option<Rational>| {
        TimedMonitorPlan::new(
            plan.subject().clone(),
            plan.binding().clone(),
            unc.unwrap_or_else(|| plan.uncertainty().clone()),
            gap.unwrap_or(plan.max_reading_gap()),
            plan.program().clone(),
            plan.intervals().to_vec(),
            buffers.unwrap_or_else(|| plan.buffers().to_vec()),
            plan.rate_limit().clone(),
            plan.activation_capacity(),
        )
    };
    let tampered = [
        rebuild(Some(vec![(plan.buffers()[0].0, 99)]), None, None),
        rebuild(None, Some(7), None),
        rebuild(None, None, Some(ratio(3, 1))),
    ];
    for candidate in &tampered {
        assert_ne!(candidate.id(), plan.id());
        assert_eq!(
            admit(&package, library(), clause, &target, candidate),
            Err(PlanAdmissionRefusal::NotDerivedFromSelection)
        );
    }
    let other_target = MonitorTarget {
        max_reading_gap: 7,
        ..base_target()
    };
    let other = premise_plan(formula, &other_target);
    assert_ne!(other.id(), plan.id());
    assert!(admit(&package, library(), clause, &other_target, &other).is_ok());
}

/// FR-251-AC-5 (TC-1081 step 2), every component a plan holds: an atom, an
/// operand reference, an interval association, a buffer owner, the activation
/// binding, the counter, the tick unit, the event rate and the activation
/// capacity are each replaced alone with one the selection does not derive,
/// and each candidate is refused at admission. The entry and init bodies are
/// not part of a `TimedMonitorPlan`; the plan-root content key carries them
/// (`changing_any_one_entry_body_changes_the_plan_root_key`).
#[trace("TC-1081", "FR-251-AC-5")]
#[test]
fn admission_refuses_each_component_replaced_alone() {
    use qsl_analyze::monitor_plan::{admit, PlanAdmissionRefusal};
    use qsl_replay::{
        ClockBindingKey, MonitorAtom, MonitorProgram, SubformulaId, TickInterval, TimedMonitorPlan,
    };
    use qsl_semantics::check::Activation;
    let formula = "always (holds(c.versionNumber = 2) implies eventually[0 ms, 3 ms] holds(c.versionNumber = 3))";
    let (package, _) = checked_clause(formula);
    let clause = package.graph().temporal_clauses()[0].clause();
    let library = || qsl_semantics::library::LibraryName::new("fixture").expect("a library name");
    let target = base_target();
    let plan = premise_plan(formula, &target);
    assert!(admit(&package, library(), clause, &target, &plan).is_ok());
    let program = plan.program();
    let atoms = program.atoms();
    assert_eq!(atoms.len(), 2);
    let interval = plan.intervals()[0].0;
    let other_node = (0..program.formula().nodes().len())
        .find(|index| *index != program.root().index() && *index != interval.index())
        .expect("another formula node");
    let with_program = |root: SubformulaId, atoms: Vec<MonitorAtom>, activation: Activation| {
        MonitorProgram::new(
            root,
            program.formula().clone(),
            activation,
            atoms,
            program.guard().map(|(body, slots)| (body.clone(), slots)),
        )
    };
    let rebuild = |program: MonitorProgram,
                   binding: ClockBindingKey,
                   intervals: Vec<(SubformulaId, TickInterval)>,
                   buffers: Vec<(SubformulaId, u64)>,
                   rate: Rational,
                   activations: u64| {
        TimedMonitorPlan::new(
            plan.subject().clone(),
            binding,
            plan.uncertainty().clone(),
            plan.max_reading_gap(),
            program,
            intervals,
            buffers,
            rate,
            activations,
        )
    };
    let same = |program: MonitorProgram| {
        rebuild(
            program,
            plan.binding().clone(),
            plan.intervals().to_vec(),
            plan.buffers().to_vec(),
            plan.rate_limit().clone(),
            plan.activation_capacity(),
        )
    };
    let swapped_atoms = vec![
        MonitorAtom::new(atoms[0].id(), atoms[1].body().clone(), atoms[0].slots()),
        atoms[1].clone(),
    ];
    let binding = plan.binding();
    let candidates: Vec<(&str, TimedMonitorPlan)> = vec![
        (
            "atom",
            same(with_program(
                program.root(),
                swapped_atoms,
                program.activation(),
            )),
        ),
        (
            "operand reference",
            same(with_program(
                SubformulaId::new(other_node),
                atoms.to_vec(),
                program.activation(),
            )),
        ),
        (
            "activation binding",
            same(with_program(
                program.root(),
                atoms.to_vec(),
                Activation::Each { guarded: false },
            )),
        ),
        (
            "interval association",
            rebuild(
                program.clone(),
                binding.clone(),
                vec![(
                    interval,
                    TickInterval::new(ratio(0, 1), true, ratio(4, 1), true),
                )],
                plan.buffers().to_vec(),
                plan.rate_limit().clone(),
                plan.activation_capacity(),
            ),
        ),
        (
            "buffer owner",
            rebuild(
                program.clone(),
                binding.clone(),
                plan.intervals().to_vec(),
                vec![(SubformulaId::new(other_node), plan.buffers()[0].1)],
                plan.rate_limit().clone(),
                plan.activation_capacity(),
            ),
        ),
        (
            "counter",
            rebuild(
                program.clone(),
                ClockBindingKey::new(
                    "other".to_owned(),
                    binding.unit().to_owned(),
                    binding.unit_seconds().clone(),
                    binding.width_bits(),
                ),
                plan.intervals().to_vec(),
                plan.buffers().to_vec(),
                plan.rate_limit().clone(),
                plan.activation_capacity(),
            ),
        ),
        (
            "tick unit",
            rebuild(
                program.clone(),
                ClockBindingKey::new(
                    binding.counter().to_owned(),
                    "us".to_owned(),
                    ratio(1, 1_000_000),
                    binding.width_bits(),
                ),
                plan.intervals().to_vec(),
                plan.buffers().to_vec(),
                plan.rate_limit().clone(),
                plan.activation_capacity(),
            ),
        ),
        (
            "event rate",
            rebuild(
                program.clone(),
                binding.clone(),
                plan.intervals().to_vec(),
                plan.buffers().to_vec(),
                ratio(3, 1),
                plan.activation_capacity(),
            ),
        ),
        (
            "activation capacity",
            rebuild(
                program.clone(),
                binding.clone(),
                plan.intervals().to_vec(),
                plan.buffers().to_vec(),
                plan.rate_limit().clone(),
                plan.activation_capacity() + 1,
            ),
        ),
    ];
    for (what, candidate) in &candidates {
        assert_ne!(candidate, &plan, "{what} makes a different plan");
        assert_eq!(
            admit(&package, library(), clause, &target, candidate),
            Err(PlanAdmissionRefusal::NotDerivedFromSelection),
            "{what}"
        );
    }
}

/// FR-251 "Fault boundary": production and admission both refuse a clause
/// whose program cannot be generated, so no plan is derived or admitted for
/// it.
#[trace("TC-706", "FR-251")]
#[test]
fn production_and_admission_refuse_a_clause_with_no_complete_program() {
    use qsl_analyze::monitor_plan::{admit, PlanAdmissionRefusal};
    let library = || qsl_semantics::library::LibraryName::new("fixture").expect("a library name");
    let target = base_target();
    for (shape, past) in [
        ("once holds(c.versionNumber = 2)", true),
        ("always eventually holds(c.versionNumber = 2)", false),
    ] {
        let (package, _) = checked_clause(shape);
        let clause = package.graph().temporal_clauses()[0].clause();
        let produced = produce(
            &package,
            library(),
            clause,
            &target,
            &TimingObligationSettings::default(),
        )
        .map(|_| ())
        .unwrap_err();
        let admitted = admit(
            &package,
            library(),
            clause,
            &target,
            &premise_plan("holds(c.versionNumber = 2)", &target),
        );
        assert_eq!(
            admitted,
            Err(PlanAdmissionRefusal::Underivable(produced.clone())),
            "{shape}"
        );
        if past {
            assert!(
                matches!(produced, MonitorPlanRefusal::UnboundedPast { .. }),
                "{produced:?}"
            );
        } else {
            assert!(
                matches!(produced, MonitorPlanRefusal::UnboundedOverUnbounded { .. }),
                "{produced:?}"
            );
        }
    }
}
