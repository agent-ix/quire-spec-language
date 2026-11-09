// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-281-AC-2 and AC-6 (TC-762, TC-763) over a real authored clause: the
//! EN-1 clause item (`ClauseSubject`) is built from the compiled unit's
//! bytes, the subject binding and the clause's occurrence, and runs through
//! `analyze_items`. No closure, letters or verdict is supplied by the test:
//! the counterexample replays through the production `replay_model_trace`.

use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze_items, AnalyzeLimits, ClauseLimits, ClauseSubject, Engine, Engines,
};
use qsl_eval::simulation::Limits;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::{
    CounterexampleKind, IncompleteCause, ModelCheckLimit, ModelTraceSettlement, ModelTraceValue,
    TerminalValue,
};
use quire_exact::Cancel;
use quire_semantic_value::catalog::Category;
use serde_json::json;

use crate::counter::CONTRACT;
use crate::model_world::{self, Clause, World, INFINITE};

/// The Counter world with `always holds(c.value <= 1)`: `inc` breaks it at
/// value 2.
fn world() -> World {
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

fn subject(world: &World, limits: ClauseLimits) -> ClauseSubject<'_> {
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

/// FR-281-AC-6: the same clause under `max_states` 1 settles `incomplete`
/// with the state-budget cause, not a verdict.
#[trace("TC-763", "FR-281-AC-6")]
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
