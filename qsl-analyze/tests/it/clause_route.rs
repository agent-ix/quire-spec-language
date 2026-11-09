// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-281-AC-2 and AC-11 through the real byte-driven entries (QSL-1819 PR B):
//! an authored-clause item carries the clause's occurrence and budgets, and
//! its operation owns the complete source provision, so
//! `analyze` / `provider::run` build the EN-1 item over their own package and
//! binding. Packages, units and snapshots are bytes; no closure, letters or
//! verdict is supplied.

use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze, AnalyzeLimits, AnalyzeRequest, ClaimItem, ClauseLimits, Engines, ItemForm,
    ItemOptions, NegotiatedItem, SubjectBinding, TemporalItem,
};
use qsl_analyze::provider;
use qsl_eval::simulation::Limits;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::{
    DeclineCode, IncompleteCause, ModelCheckLimit, ProofRefusalCause, TerminalRecord,
    TerminalValue, UnavailabilityCause,
};
use qsl_route::{request::ExtentClassification, routing::Disposition, Candidate};
use qsl_semantics::check::Capability;
use quire_exact::{Cancel, Origin, Role};
use quire_semantic_value::catalog::{CatalogCode, Category};
use serde_json::json;

use crate::counter::CONTRACT;
use crate::model_world::{self, Clause, Domain, World, INFINITE};

fn world(domain: Domain) -> World {
    World::new(
        domain,
        CONTRACT,
        INFINITE,
        &[Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value <= 1)",
        }],
        &[("c", json!({"value": {"integer": "0"}}))],
    )
}

fn work(world: &World, backend: &str, run: ClauseLimits) -> NegotiatedItem {
    let node = &world.package.graph().temporal_clauses()[0];
    let candidate = Candidate::new(BackendId::new(backend));
    NegotiatedItem {
        item: ClaimItem {
            request_index: RequestIndex::new(0),
            claim: OccurrenceKey::new(node.identity(), node.claim().clone()),
            kind: Capability::TemporalSatisfaction,
            extent: ExtentClassification::Bounded,
            backend: Some(BackendId::new(backend)),
            candidates: vec![candidate.clone()],
            options: ItemOptions {
                required: true,
                ..ItemOptions::default()
            },
            form: Some(ItemForm::Temporal(TemporalItem {
                run,
                ..TemporalItem::default()
            })),
        },
        disposition: Disposition::Supported(candidate),
        diagnostic: None,
    }
}

fn through_analyze(world: &World, item: NegotiatedItem) -> TerminalRecord {
    let binding: SubjectBinding<'_> = world.binding(&["c"]);
    let outcome = analyze(
        &world.package,
        &AnalyzeRequest {
            binding,
            provision: world.provision.clone(),
            items: vec![item],
        },
        &Engines::qsl().expect("the layer-A manifests register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("the binding is admitted")
    .into_value();
    assert_eq!(outcome.records.len(), 1);
    outcome.records.into_iter().next().expect("one record")
}

fn through_run(world: &World, item: NegotiatedItem) -> TerminalRecord {
    let mut records = provider::run(
        &world.package,
        &world.binding(&["c"]),
        &world.provision,
        &[item],
        &Engines::qsl().expect("the layer-A manifests register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .records;
    assert_eq!(records.len(), 1);
    records.remove(0)
}

/// FR-281-AC-2 through `analyze` and `provider::run`: the invariant a
/// transition breaks settles `refuted` on EN-1. FR-281-AC-2 is split: the
/// record settles `Refuted` here, through the real entries, while the
/// assertion that replaying the counterexample reproduces it lives in
/// `clause_item.rs` (a terminal record keeps no counterexample; the item
/// settles `refuted` only after the production replay reproduced it).
#[trace("TC-762", "FR-281-AC-2")]
#[test]
fn analyze_routes_an_authored_clause_to_en1_and_settles_refuted_with_replay() {
    let world = world(model_world::counter());
    for record in [
        through_analyze(&world, work(&world, "en-1", ClauseLimits::default())),
        through_run(&world, work(&world, "en-1", ClauseLimits::default())),
    ] {
        assert_eq!(record.value(), &TerminalValue::Refuted);
        assert_eq!(record.value().category(), Category::Violation);
        assert_eq!(record.backend().map(BackendId::as_str), Some("en-1"));
    }
}

/// Supplementary EN-1 case (not AC-6, which is the EN-5 item's): the same
/// clause under `max_states` 1 settles `incomplete` naming the state budget.
#[test]
fn analyze_settles_an_authored_clause_under_one_state_incomplete() {
    let world = world(model_world::counter());
    let run = ClauseLimits {
        limits: Limits {
            max_states: 1,
            ..Limits::default()
        },
        ..ClauseLimits::default()
    };
    let record = through_analyze(&world, work(&world, "en-1", run));
    assert_eq!(
        record.value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 1,
        })
    );
}

/// FR-281-AC-11 / TC-1393 step 1 (the EN-1 `RequiresBound` item): over a
/// model whose operation modifies an unbounded `Integer`, the EN-1 clause
/// item on `Engines::qsl()` settles `unsupported` with
/// `unsupported_projection`/`unbounded-extent`. The record-level payload
/// (`temporal-satisfaction`, EN-1's `BackendId`, `bounded`) is covered by the
/// unit test `engine_refusals_settle_by_the_refusal_table` in
/// `src/analyze.rs`, because `terminal_records` drops it.
#[trace("TC-1393", "FR-281-AC-11")]
#[test]
fn an_unbounded_extent_clause_settles_unsupported_through_the_provider_entry() {
    let world = world(Domain {
        document: crate::counter::document(true),
        ..model_world::counter()
    });
    for record in [
        through_run(&world, work(&world, "en-1", ClauseLimits::default())),
        through_analyze(&world, work(&world, "en-1", ClauseLimits::default())),
    ] {
        assert_eq!(
            record.value(),
            &TerminalValue::Unsupported(UnavailabilityCause::BackendAbsent)
        );
        assert_eq!(
            record.diagnostic(),
            Some(CatalogCode::new(
                "unsupported_projection",
                "unbounded-extent"
            ))
        );
    }
}

/// A clause negotiated to a backend other than EN-1 is not run: it settles
/// `unsupported`, and a clause the package does not hold settles
/// `invalid-request` without an engine run.
#[test]
fn a_clause_routed_off_en1_or_absent_from_the_package_is_not_run() {
    let world = world(model_world::counter());
    let off = through_analyze(&world, work(&world, "en-4", ClauseLimits::default()));
    assert!(
        matches!(off.value(), TerminalValue::Unsupported(_)),
        "{off:?}"
    );

    let mut absent = work(&world, "en-1", ClauseLimits::default());
    absent.item.claim = OccurrenceKey::new(
        world.package.graph().temporal_clauses()[0].identity(),
        Origin::new(Role::new("claim"), 99),
    );
    let record = through_analyze(&world, absent);
    assert_eq!(
        record.value(),
        &TerminalValue::Declined {
            cause: ProofRefusalCause::InvalidInput,
            code: DeclineCode::Qsl(Code::InvalidRequest),
        }
    );
    assert_eq!(
        record.diagnostic(),
        Some(CatalogCode::new("invalid_capability", "inapplicable-kind"))
    );
}
