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
    SourceProvision,
};
use qsl_eval::simulation::Limits;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::{
    CounterexampleKind, IncompleteCause, ModelCheckLimit, ModelTraceSettlement, ModelTraceValue,
    TerminalValue,
};
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
        AnalyzeLimits::unbounded(),
        &world.provision,
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
        AnalyzeLimits::unbounded(),
        &world.provision,
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

/// A complete source provision preserves real replay for a package assembled
/// from two source units; no single-unit restriction discards the extra unit.
#[test]
fn a_package_of_two_source_units_replays_from_its_complete_provision() {
    use qsl_replay::spine::{
        admit_parsed_unit_list, check_units, parse, select_units, DependencyInput, ParseRequest,
        SpineLimits,
    };
    use qsl_semantics::model::accounting::ModelNormalizationLimits;
    use qsl_semantics::model::intake::package_input;

    let world = world();
    let text = String::from_utf8(world.unit.clone()).expect("a text unit");
    let extra = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
                 profile v = \"quire.value.complete/v1\";\n\
                 profile t = \"{INFINITE}\";\n\
                 function Extra using v(): Boolean pure {{ true }}\n"
    );
    let units: Vec<_> = [("safety", text.as_str()), ("extra", extra.as_str())]
        .into_iter()
        .map(|(identity, source)| {
            let identity = qsl_foundation::SourceIdentity::new("agent-ix", identity);
            parse(
                &ParseRequest {
                    source: &identity,
                    path: "unit.native",
                    bytes: source.as_bytes(),
                },
                qsl_cst::Limits::default(),
                &Cancel::new(),
            )
            .expect("S1 and S2 parse the unit")
            .into_value()
        })
        .collect();
    let dependencies = DependencyInput::default();
    let units =
        admit_parsed_unit_list(units, &dependencies).expect("both headers and source labels admit");
    let packages = package_input([world.domain.document.as_slice()]).expect("admits");
    let selected = select_units(
        &units,
        &packages,
        ModelNormalizationLimits::default(),
        qsl_cst::Limits::default(),
        &Cancel::new(),
    )
    .expect("the model selection admits")
    .into_value();
    let checked = check_units(
        &units,
        "package".to_owned(),
        &selected,
        &dependencies,
        SpineLimits::default(),
        &Cancel::new(),
    )
    .expect("the qualified frontend checks both source units")
    .into_value();
    let package = checked.first().expect("the checked aggregate").package();
    assert_eq!(package.graph().sources().len(), 2);
    let node = &package.graph().temporal_clauses()[0];
    let key = OccurrenceKey::new(node.identity(), node.claim().clone());
    let mut provision = SourceProvision::default();
    for source in package.graph().sources() {
        provision.insert(
            source,
            match source.identity() {
                "safety" => text.as_bytes().to_vec(),
                "extra" => extra.as_bytes().to_vec(),
                other => panic!("unexpected source {other}"),
            },
        );
    }
    let subject = ClauseSubject::new(
        package,
        world.binding(&["c"]),
        AnalyzeLimits::unbounded(),
        &provision,
        &key,
        ClauseLimits::default(),
    )
    .expect("the complete multi-unit subject is accepted");
    let mut raw_subject = world.subject(&["c"], crate::counter::UNLIMITED);
    raw_subject.simulation.package = package;
    let system = qsl_eval::simulation::model::ModelSystem::new(raw_subject.simulation.clone())
        .expect("the multi-unit system admits");
    let letters = system
        .clause_letters(node)
        .expect("the multi-unit clause letters");
    let raw = qsl_analyze::model_check::check_model(
        qsl_analyze::model_check::ModelCheckRequest {
            subject: raw_subject,
            temporal_clock_selections: &[],
            item: qsl_replay::ModelCheckItem::Clause {
                clause: node.clause(),
                letters: &letters,
            },
            max_depth: None,
            limits: qsl_replay::ModelCheckLimits::default(),
            reductions: Default::default(),
        },
        &Cancel::new(),
    )
    .expect("the model checker produces a real candidate");
    let qsl_replay::ModelCheckOutcome::Violated { counterexample, .. } = raw else {
        panic!("the same counterexample exists")
    };
    subject
        .replay_counterexample(&counterexample)
        .expect("the operation-built multi-unit request replays the native candidate");
    let records = analyze_items(
        &Engines::qsl().expect("the engines register"),
        vec![subject.item(RequestIndex::new(0))],
        &Cancel::new(),
    );
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].terminal().expect("terminal record").value(),
        &TerminalValue::Refuted
    );
    let counterexample = records[0]
        .model_check
        .as_ref()
        .and_then(|model| model.counterexample.as_ref())
        .expect("a real counterexample");
    let replayed = subject
        .replay_counterexample(counterexample)
        .expect("the supplied sources replay");
    assert!(matches!(
        replayed.settlement,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            ..
        }
    ));
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
    let other_provision = SourceProvision::single(world.package.graph().source(), other_unit);
    let wrong = ClauseSubject::new(
        &world.package,
        world.binding(&["c"]),
        AnalyzeLimits::unbounded(),
        &other_provision,
        &key,
        ClauseLimits::default(),
    )
    .expect("construction does not compare the bytes");
    assert!(wrong.replay_counterexample(&counterexample).is_err());
}
