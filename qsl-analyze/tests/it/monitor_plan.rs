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
use quire_exact::{Integer, NodeKey, Rational};

use crate::units::Domain;

const TIME_UNITS: &str = "dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    unit ms : Time = rational(1, 1000) * s;\n\
    unit us : Time = rational(1, 1000000) * s;\n";

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn checked_clause(formula: &str) -> (CheckedPackage, usize) {
    let domain = Domain::config_version();
    let packages =
        qsl_semantics::model::intake::package_input([domain.document.as_slice()]).unwrap();
    let source = SourceIdentity::new("agent-ix", "monitor-plan", "git", "1");
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t = \"quire.temporal.timed/v1\";\n\
         model M = \"{}\";\n\
         {TIME_UNITS}\
         temporal C using t over (c: M::ConfigVersion) clock \"model-time\" on origin {{ {formula} }}\n",
        domain.package,
    );
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
        "fixture".to_owned(),
        built,
        models,
        Vec::new(),
    )
    .expect("S3 assembly");
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
    assert_eq!(plan.buffers(), &[(plan.intervals()[0].0, 7)]);
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
    assert_eq!(plan.buffers()[0].1, 21);
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
    assert_eq!(plan.buffers()[0].1, 1_001);
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
