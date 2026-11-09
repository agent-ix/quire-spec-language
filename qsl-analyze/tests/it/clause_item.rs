// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-281-AC-2 (TC-762) over a real authored clause, with supplementary EN-1
//! state-budget and refusal cases (FR-281-AC-6 and TC-763 are the EN-5
//! item's, tagged in `exact_probabilistic_item.rs`): the
//! EN-1 clause item (`ClauseSubject`) is built from the compiled unit's
//! bytes, the subject binding and the clause's occurrence, and runs through
//! `analyze_items`. No closure, letters or verdict is supplied by the test:
//! the counterexample replays through the production `replay_model_trace`.

use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze_items, AnalyzeLimits, ClauseItemRefusal, ClauseLimits, ClauseSubject, Engine, Engines,
};
use qsl_eval::simulation::Limits;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::{
    CounterexampleKind, IncompleteCause, ModelCheckLimit, ModelTraceSettlement, ModelTraceValue,
    TerminalValue,
};
use qsl_semantics::check::PackageDeclarations;
use quire_exact::{Cancel, Origin, Role};
use quire_semantic_value::catalog::Category;
use serde_json::json;

use crate::counter::CONTRACT;
use crate::model_world::{self, Clause, World, INFINITE};

/// The Counter world with `always holds(c.value <= 1)`: `inc` breaks it at
/// value 2.
pub(crate) fn world() -> World {
    World::new(
        model_world::counter(),
        CONTRACT,
        INFINITE,
        &[Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value <= 1)",
        }],
        &[("c", json!({"value": {"integer": "0"}}))],
    )
}

pub(crate) fn subject(world: &World, limits: ClauseLimits) -> ClauseSubject<'_> {
    let node = &world.package.graph().temporal_clauses()[0];
    let key = OccurrenceKey::new(node.identity(), node.claim().clone());
    ClauseSubject::new(
        &world.package,
        world.binding(&["c"]),
        AnalyzeLimits::default(),
        &world.unit,
        &key,
        limits,
    )
    .expect("the package holds the clause")
}

fn run(world: &World, limits: ClauseLimits) -> qsl_analyze::analyze::AnalyzeRecord {
    let subject = subject(world, limits);
    let mut engines = Engines::default();
    engines
        .with(Engine::model_check().expect("EN-1's own manifest is admitted"))
        .expect("one engine");
    let mut records = analyze_items(
        &engines,
        vec![subject.item(RequestIndex::new(0))],
        &Cancel::new(),
    );
    assert_eq!(records.len(), 1);
    records.remove(0)
}

/// FR-281-AC-2: the invariant a transition breaks settles `refuted`, the
/// record carries the counterexample, and the production replay reproduced
/// it before the item settled (a replay that disagreed or refused would not
/// settle `refuted`).
#[trace("TC-762", "FR-281-AC-2")]
#[test]
fn a_clause_a_transition_breaks_settles_refuted_with_a_replayed_counterexample() {
    let world = world();
    let record = run(&world, ClauseLimits::default());
    let terminal = record.terminal().expect("a terminal record");
    assert_eq!(terminal.value(), &TerminalValue::Refuted);
    assert_eq!(terminal.value().category(), Category::Violation);
    assert_eq!(terminal.backend().map(BackendId::as_str), Some("en-1"));
    let counterexample = record
        .model_check
        .as_ref()
        .and_then(|model| model.counterexample.clone())
        .expect("the record carries the counterexample");
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    let replay = subject(&world, ClauseLimits::default())
        .replay_counterexample(&counterexample)
        .expect("the counterexample replays");
    assert!(
        matches!(
            replay.settlement,
            ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Truth(false),
                ..
            }
        ),
        "{:?}",
        replay.settlement
    );
}

/// Supplementary EN-1 case (FR-281-AC-6's budget row is the EN-5 item's): the
/// same clause under `max_states` 1 settles `incomplete` with the
/// state-budget cause, not a verdict.
#[test]
fn the_same_clause_under_one_state_settles_incomplete_naming_the_state_budget() {
    let record = run(
        &world(),
        ClauseLimits {
            limits: Limits {
                max_states: 1,
                ..Limits::default()
            },
            ..ClauseLimits::default()
        },
    );
    let terminal = record.terminal().expect("a terminal record");
    assert_eq!(
        terminal.value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 1,
        })
    );
}

fn refused(world: &World, key: &OccurrenceKey) -> ClauseItemRefusal {
    ClauseSubject::new(
        &world.package,
        world.binding(&["c"]),
        AnalyzeLimits::default(),
        &world.unit,
        key,
        ClauseLimits::default(),
    )
    .err()
    .expect("the subject is refused")
}

/// A key naming the clause's node under a claim occurrence it does not have
/// is no clause of the package.
#[test]
fn a_key_with_another_claim_occurrence_is_an_unknown_clause() {
    let world = world();
    let node = &world.package.graph().temporal_clauses()[0];
    let key = OccurrenceKey::new(node.identity(), Origin::new(Role::new("claim"), 99));
    assert_eq!(refused(&world, &key), ClauseItemRefusal::UnknownClause);
}

/// A package assembled from two source units has no one `unit` byte string
/// to replay: construction refuses typed.
#[test]
fn a_package_of_two_source_units_is_refused() {
    use qsl_semantics::check::AssemblyLimits;
    use qsl_semantics::model::accounting::ModelNormalizationLimits;
    use qsl_semantics::model::intake::{admit_units_with_cancel, package_input};
    use quire_semantic_value::checking::CheckingLimits;

    let world = world();
    let text = String::from_utf8(world.unit.clone()).expect("a text unit");
    let extra = "language \"ix:native\" edition \"1-draft\";\n\
                 profile w = \"quire.value.complete/v1\";\n\
                 function Extra using w(): Boolean pure { true }\n";
    let units: Vec<_> = [("safety", text.as_str()), ("extra", extra)]
        .into_iter()
        .map(|(identity, source)| {
            let parsed = qsl_cst::parse(
                qsl_foundation::SourceIdentity::new("agent-ix", identity),
                "unit.native",
                source.as_bytes(),
                qsl_cst::Limits::default(),
            )
            .expect("S1 parses the unit");
            assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
            let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
            (parsed.source().reference().clone(), unit)
        })
        .collect();
    let packages = package_input([world.domain.document.as_slice()]).expect("admits");
    let selections: Vec<_> = units
        .iter()
        .map(|(_, unit)| unit.selections().models.as_slice())
        .collect();
    let selected = admit_units_with_cancel(
        &selections,
        &packages,
        ModelNormalizationLimits::default(),
        &Cancel::new(),
    )
    .expect("the model selection admits");
    let graph = PackageDeclarations::assemble_units(
        "package".to_owned(),
        units,
        selected,
        Vec::new(),
        AssemblyLimits::default(),
        &Cancel::new(),
    )
    .expect("the units assemble")
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("the units check: {refusals:?}"));
    assert_eq!(graph.sources().len(), 2);
    let package = qsl_package::CheckedPackage::link(graph);
    let node = &package.graph().temporal_clauses()[0];
    let key = OccurrenceKey::new(node.identity(), node.claim().clone());
    let refusal = ClauseSubject::new(
        &package,
        world.binding(&["c"]),
        AnalyzeLimits::default(),
        &world.unit,
        &key,
        ClauseLimits::default(),
    )
    .err()
    .expect("refused");
    assert_eq!(refusal, ClauseItemRefusal::MultiUnit);
}

/// A counterexample outside the subject, or replayed over other source bytes
/// than the package was checked from, is refused by the production replay:
/// no `Reproduced` result exists for `settle_model_check` to settle `refuted`
/// from. This drives `replay_counterexample`, not the item's own closure
/// (the item settles through the same replay). `UnnamedClause` has no test:
/// `Identifier` is ASCII, and whether S1 admits a non-ASCII clause name is
/// not checked here.
#[test]
fn a_tampered_counterexample_or_another_unit_is_refused_by_the_replay() {
    let world = world();
    let record = run(&world, ClauseLimits::default());
    let counterexample = record
        .model_check
        .as_ref()
        .and_then(|model| model.counterexample.clone())
        .expect("the record carries the counterexample");

    let mut outside = counterexample.clone();
    outside.initial_state = 1;
    assert!(subject(&world, ClauseLimits::default())
        .replay_counterexample(&outside)
        .is_err());

    let node = &world.package.graph().temporal_clauses()[0];
    let key = OccurrenceKey::new(node.identity(), node.claim().clone());
    let other_unit = b"language \"ix:native\" edition \"1-draft\";\n";
    let wrong = ClauseSubject::new(
        &world.package,
        world.binding(&["c"]),
        AnalyzeLimits::default(),
        other_unit,
        &key,
        ClauseLimits::default(),
    )
    .expect("construction does not compare the bytes");
    assert!(wrong.replay_counterexample(&counterexample).is_err());
}
