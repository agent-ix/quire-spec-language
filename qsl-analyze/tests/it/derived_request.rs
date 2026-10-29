// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124, FR-232 and FR-243 with QSpec FR-368 end to end: the request
//! writer's derived items. A request built by `SubjectContext` and finished
//! by `ModelRequest::derived` carries the caller's items first and then the
//! subject's derived items; each derived item is anchored at its subject's
//! `model_root`, routed by its dense mark, and settles through `analyze` on
//! the engine it is routed to: a dense subject's deadlock-freedom and
//! time-lock-freedom items on EN-6, an untimed subject's deadlock-freedom
//! item on EN-1.

use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze, derived_model_item, derived_zone_item, AnalyzeItem, AnalyzeRecord,
    DerivedItemMismatch, Engines, TimedItem,
};
use qsl_analyze::model_check::manifest as en1;
use qsl_analyze::model_check::SubjectContext;
use qsl_analyze::zone_check::manifest as en6;
use qsl_analyze::zone_check::{ZoneCheckItem, ZoneCheckLimits, ZoneCheckRequest};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{PopulationUniverse, TimedSystem, DEFAULT_MAX_CLOCKS};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::CheckedPackage;
use qsl_replay::node_occurrence;
use qsl_replay::SubjectIdentity;
use qsl_replay::{
    CertificateCheckLimits, CertifiedItem, IncompleteCause, ModelCheckLimit, TerminalValue,
    ZoneCertificateRequest,
};
use qsl_route::deadlock::{
    ItemObligation, ModelRefinementItem, ModelStateGraphItem, ModelTemporalItem, NoAnchor,
    ProtocolTemporalItem,
};
use qsl_route::model_request::{ModelRequest, WrittenRequest};
use qsl_route::reductions::Reductions;
use qsl_route::request::items_from_requirements;
use qsl_route::request::{
    AbstractSubject, DerivedLimits, ExtentClassification, ItemKind, RequestItem,
};
use qsl_semantics::check::Capability;
use qsl_semantics::family::{ClaimExtent, Requirements};
use qsl_semantics::library::PackageId;
use quire_exact::Cancel;
use quire_semantic_value::catalog::CatalogCode;
use std::collections::BTreeSet;
use std::num::NonZeroU64;

use crate::counter::{clause_text, CONTRACT};
use crate::dense_route::{ids, registry};
use crate::model_settle::case;
use crate::timed_world::{World, PACKAGE};
use crate::zone_search::{rpc, rpc_claim, INFINITE};

/// The derived items, each with its obligation, that
/// `ModelRequest::derived` writes, with EN-1 and EN-6 registered, after
/// `temporal`, `refinements` and `state_graph` in that caller order. Each
/// temporal and state-graph item's subject reads the selections
/// `selections`; a refinement's reads its concrete model. A record's
/// requirements are not read by the derivation, so each caller item is
/// written with a bounded `temporal-satisfaction` record.
pub(crate) fn derived_items<S: Clone + Eq>(
    temporal: &[ModelTemporalItem<'_, S>],
    refinements: &[ModelRefinementItem<'_, S>],
    state_graph: &[ModelStateGraphItem<'_, S>],
    selections: &[&str],
) -> Vec<(RequestItem, ItemObligation<S>)> {
    let selections: BTreeSet<&str> = selections.iter().copied().collect();
    let record = Requirements::new(Capability::TemporalSatisfaction, ClaimExtent::Bounded);
    let mut request = ModelRequest::new();
    for item in temporal {
        request.authored(item, &record, &selections);
    }
    for item in refinements {
        let abstract_subject = AbstractSubject {
            package_id: PackageId::of_preimage(b"abstract"),
            initial_states: item.abstract_initial.to_vec(),
        };
        request
            .refinement(item, &record, abstract_subject)
            .expect("the refinement has a record");
    }
    for item in state_graph {
        request.state_graph(item, &selections);
    }
    let written = request.derived(&registry(), None, &DerivedLimits::default());
    written
        .items()
        .iter()
        .filter(|item| {
            matches!(
                item.item_kind(),
                Some(ItemKind::DeadlockFreedom | ItemKind::TimeLockFreedom)
            )
        })
        .map(|item| {
            let obligation = written
                .obligation(item.index())
                .expect("its obligation")
                .clone();
            (item.clone(), obligation)
        })
        .collect()
}

/// The request `ModelRequest::derived` writes, with EN-1 and EN-6
/// registered, after the temporal items `items` over protocol subjects, in
/// that order, each written with a bounded `temporal-satisfaction` record.
pub(crate) fn protocol_request<S: Clone + Eq>(
    items: &[ProtocolTemporalItem<'_, S>],
) -> WrittenRequest<S> {
    let record = Requirements::new(Capability::TemporalSatisfaction, ClaimExtent::Bounded);
    let mut request = ModelRequest::new();
    for item in items {
        request.protocol(item, &record);
    }
    request.derived(&registry(), None, &DerivedLimits::default())
}

/// The items of `written` of kind `kind`, each with its obligation.
pub(crate) fn items_of_kind<S: Clone + Eq>(
    written: &WrittenRequest<S>,
    kind: ItemKind,
) -> Vec<(RequestItem, ItemObligation<S>)> {
    written
        .items()
        .iter()
        .filter(|item| item.item_kind() == Some(kind))
        .map(|item| {
            let obligation = written.obligation(item.index()).expect("its obligation");
            (item.clone(), obligation.clone())
        })
        .collect()
}

/// The subject of each derived item of kind `kind` that `written`
/// derives, in derivation order among the written ones and then among the
/// ones settled `unsupported` for want of an anchor.
pub(crate) fn derived_subjects<S: Clone + Eq>(
    written: &WrittenRequest<S>,
    kind: ItemKind,
) -> Vec<S> {
    let unanchored = written
        .unanchored()
        .iter()
        .filter(|item| item.kind == kind)
        .map(|item| item.subject.clone());
    items_of_kind(written, kind)
        .into_iter()
        .map(|(_, obligation)| obligation.subject)
        .chain(unanchored)
        .collect()
}

/// The claim QSpec FR-368 gives a derived item over a subject reading the
/// selection `identity` of `package`: its `model_root` node's `generated`
/// occurrence at ordinal 0.
fn root_claim(package: &CheckedPackage, identity: &str) -> OccurrenceKey {
    let (_, key) = package
        .graph()
        .model_roots()
        .find(|(selection, _)| selection.identity == identity)
        .unwrap_or_else(|| panic!("a model_root for {identity}"));
    OccurrenceKey::generated(WireNodeId::from_digest(*key.as_bytes()))
}

/// The request over `subject` holding the item of its package's first
/// temporal clause, finished with its derived items under `limits`.
fn written(subject: &ModelSubject<'_>, limits: &DerivedLimits) -> WrittenRequest<SubjectIdentity> {
    let context = SubjectContext::new(subject).expect("the package emits");
    let mut request = ModelRequest::new();
    let clause = &subject.simulation.package.graph().temporal_clauses()[0];
    let authored = context
        .author_clause(&mut request, clause)
        .expect("the clause has a record");
    assert_eq!(authored.get(), 0, "the caller's item comes first");
    request.derived(&registry(), None, limits)
}

/// The kinds of `written`'s items, in index order.
fn kinds(written: &WrittenRequest<SubjectIdentity>) -> Vec<Option<ItemKind>> {
    written.items().iter().map(RequestItem::item_kind).collect()
}

/// The backend and value `record` settled.
fn settled(record: &AnalyzeRecord) -> (String, TerminalValue) {
    let terminal = record.terminal().expect("a terminal record");
    (
        terminal
            .backend()
            .map(|backend| backend.as_str().to_owned())
            .expect("a backend"),
        terminal.value().clone(),
    )
}

/// QSpec FR-368-AC-17 and AC-18 with FR-243, and the producer half of
/// AC-14 (the model subject's `model_root` anchor; the protocol anchor is
/// `race_freedom`'s) and of AC-16 (`unbounded`, no finite bound, no
/// `extent.bounds`; QSL writes no extent domain list): over the dense
/// `Rpc`, the request holds the authored clause at index 0, then the
/// deadlock-freedom and then the time-lock-freedom item. Both derived
/// items are `temporal-satisfaction`, unbounded with no finite bound, carry
/// no `hybrid-dynamics` requirement, name the same `model_root` claim, and
/// route to EN-6 alone.
#[trace("TC-698", "FR-243-AC-1")]
#[trace("TC-687", "FR-232-AC-1")]
#[trace("QSpec-TC-431")]
#[trace("QSpec-TC-302", "QSpec-FR-368-AC-17")]
#[trace("QSpec-TC-302", "QSpec-FR-368-AC-18")]
#[test]
fn a_dense_subjects_request_carries_its_derived_items_after_the_callers() {
    let world = rpc(4);
    let subject = ModelSubject {
        simulation: world.request(),
        proof_bounds: Vec::new(),
    };
    let written = written(&subject, &DerivedLimits::default());
    assert_eq!(
        kinds(&written),
        [
            Some(ItemKind::Formula),
            Some(ItemKind::DeadlockFreedom),
            Some(ItemKind::TimeLockFreedom)
        ]
    );
    let claim = root_claim(&world.package, PACKAGE);
    for (position, item) in written.items().iter().enumerate().skip(1) {
        assert_eq!(item.index().get(), position);
        assert_eq!(item.occurrence(), &claim);
        assert_eq!(item.kind(), Capability::TemporalSatisfaction);
        assert_eq!(
            item.extent(),
            ExtentClassification::Unbounded {
                finite_bound_available: false
            }
        );
        assert!(item.requirements().is_empty());
        assert!(item.domains().is_empty());
        assert!(item.abstract_subject().is_none());
        assert_eq!(item.derived_limits(), Some(&DerivedLimits::default()));
        assert_eq!(
            ids(item.candidates().expect("a derived item's candidates")),
            [en6::BACKEND_ID]
        );
    }
    let authored = &written.items()[0];
    assert_eq!(authored.candidates(), None);
    assert_ne!(authored.occurrence(), &claim);
    assert!(written.unanchored().is_empty());
}

/// The EN-6 run of `written`'s derived item at `at` over `timed`, deciding
/// `item` and checked as `certified` under the clock bound `max_clocks`,
/// through `derived_zone_item`.
#[allow(clippy::too_many_arguments)] // one run's subject, check and item
fn zone_run<'w>(
    world: &'w World,
    written: &WrittenRequest<SubjectIdentity>,
    timed: &'w TimedSystem<'w, 'w>,
    reductions: &'w Reductions,
    max_clocks: NonZeroU64,
    at: usize,
    item: ZoneCheckItem<'w>,
    certified: CertifiedItem<'w>,
) -> Result<AnalyzeItem<'w>, DerivedItemMismatch> {
    let derived = &written.items()[at];
    let timed_item = TimedItem {
        request: ZoneCheckRequest {
            subject: timed,
            item,
            obligation: written
                .obligation(derived.index())
                .expect("its obligation")
                .clone(),
            limits: ZoneCheckLimits::default(),
            reductions,
        },
        certificate: ZoneCertificateRequest {
            simulation: world.request(),
            model: "M",
            max_clocks,
            item: certified,
        },
        certificate_limits: CertificateCheckLimits::default(),
        package_id: PackageId::of_preimage(b"derived-timed").record(),
        replay: Box::new(|_| panic!("a proof has no counterexample to replay")),
    };
    derived_zone_item(derived, timed_item)
}

/// The records `analyze` settles for `items`.
fn analyzed(items: Vec<AnalyzeItem<'_>>) -> Vec<AnalyzeRecord> {
    analyze(
        &Engines::qsl().expect("the layer-A manifests register"),
        items,
        &Cancel::new(),
    )
}

/// FR-243-AC-1 and FR-232 end to end: the dense `Rpc`'s derived
/// deadlock-freedom and time-lock-freedom items, run through `analyze` by
/// `derived_zone_item` at their own request indices, settle on EN-6 with
/// FR-245's certificate check; an adapter given a run of another item
/// builds nothing.
#[trace("TC-698", "FR-243-AC-1")]
#[trace("TC-687", "FR-232-AC-1")]
#[test]
fn a_dense_subjects_derived_items_settle_on_en6() {
    let world = rpc(4);
    let subject = ModelSubject {
        simulation: world.request(),
        proof_bounds: Vec::new(),
    };
    let written = written(&subject, &DerivedLimits::default());
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let reductions = Reductions::default();
    let run = |at, item, certified| {
        zone_run(
            &world,
            &written,
            &timed,
            &reductions,
            DEFAULT_MAX_CLOCKS,
            at,
            item,
            certified,
        )
    };
    let mismatch = run(
        1,
        ZoneCheckItem::TimeLockFreedom,
        CertifiedItem::TimeLockFreedom,
    )
    .err()
    .expect("a time-lock run is not the deadlock-freedom item's");
    assert_eq!(
        mismatch,
        DerivedItemMismatch::OtherItem {
            index: written.items()[1].index(),
            kind: ItemKind::DeadlockFreedom
        }
    );
    let records = analyzed(vec![
        run(
            1,
            ZoneCheckItem::DeadlockFreedom,
            CertifiedItem::DeadlockFreedom,
        )
        .expect("the deadlock-freedom run"),
        run(
            2,
            ZoneCheckItem::TimeLockFreedom,
            CertifiedItem::TimeLockFreedom,
        )
        .expect("the time-lock-freedom run"),
    ]);
    assert_eq!(
        records
            .iter()
            .map(|record| record.request_index.get())
            .collect::<Vec<_>>(),
        [1, 2]
    );
    for record in &records {
        let (backend, value) = settled(record);
        assert_eq!(backend, en6::BACKEND_ID);
        assert!(
            matches!(value, TerminalValue::Proved { .. }),
            "{:?}: {value:?}",
            record.request_index
        );
    }
}

/// FR-243 over a `time tick` subject (SR-1786001 FND-001): a timed subject
/// whose source is a tick, not `time dense`, routes its authored clause's
/// record and both derived items to EN-6 alone, never to EN-1, which has no
/// time-lock-freedom item; its deadlock-freedom item settles proved on
/// EN-6.
#[trace("TC-698", "FR-243-AC-1")]
#[trace("TC-687", "FR-232-AC-1")]
#[test]
fn a_tick_subjects_derived_items_route_to_en6_alone() {
    let world = crate::timed_world::world(
        &crate::timed_world::document(&[], &[("go", &["phase"]), ("tick", &["phase"])]),
        INFINITE,
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         time M tick M::Call::tick period 5 ms unit ms;\n\
         pre Go using v on M::Call::go { self.phase = 0 }\n\
         post Gone using v on M::Call::go { self.phase = 1 }\n\
         pre Tick using v on M::Call::tick { self.phase = 1 }\n\
         post Ticked using v on M::Call::tick { self.phase = 0 }\n\
         temporal Quiet using inf over (c: M::Call) clock \"model-steps\" on origin \
         { always holds(not c.retried) }\n",
    );
    let subject = ModelSubject {
        simulation: world.request(),
        proof_bounds: Vec::new(),
    };
    // FR-243: the authored clause over the tick subject is a timed item,
    // so its record routes to EN-6 alone too.
    let graph = world.package.graph();
    let occurrence = node_occurrence(&graph.temporal_clauses()[0]);
    assert!(graph
        .requirements()
        .get(&occurrence)
        .expect("the clause's record")
        .requirements()
        .timed());
    let records = items_from_requirements(graph.requirements(), &registry(), None);
    let clause = records
        .iter()
        .find(|item| *item.occurrence() == occurrence)
        .expect("the clause's item");
    assert_eq!(ids(clause.candidates()), [en6::BACKEND_ID]);

    let written = written(&subject, &DerivedLimits::default());
    assert_eq!(
        kinds(&written),
        [
            Some(ItemKind::Formula),
            Some(ItemKind::DeadlockFreedom),
            Some(ItemKind::TimeLockFreedom)
        ]
    );
    for item in &written.items()[1..] {
        assert_eq!(
            ids(item.candidates().expect("its candidates")),
            [en6::BACKEND_ID],
            "{:?}",
            item.item_kind()
        );
    }
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let reductions = Reductions::default();
    let records = analyzed(vec![zone_run(
        &world,
        &written,
        &timed,
        &reductions,
        DEFAULT_MAX_CLOCKS,
        1,
        ZoneCheckItem::DeadlockFreedom,
        CertifiedItem::DeadlockFreedom,
    )
    .expect("the deadlock-freedom run")]);
    let (backend, value) = settled(&records[0]);
    assert_eq!(backend, en6::BACKEND_ID);
    assert!(matches!(value, TerminalValue::Proved { .. }), "{value:?}");
}

/// QSpec FR-368 "Negotiation" (SR-1786001 FND-002, SR-1786002 FND-003):
/// the caller's limits reach EN-6. With `max_states` 1 the dense `Rpc`'s
/// deadlock-freedom item stops `incomplete`; with `max_clocks` 2 a run over
/// a subject built under EN-6's default clock bound is refused, naming
/// both bounds, and one built and checked under 2 settles proved.
#[trace("TC-698", "FR-243-AC-1")]
#[test]
fn a_derived_items_caller_limits_reach_en6() {
    let world = rpc(4);
    let subject = ModelSubject {
        simulation: world.request(),
        proof_bounds: Vec::new(),
    };
    let system = world.system();
    let reductions = Reductions::default();
    let deadlock = |written: &WrittenRequest<SubjectIdentity>, max_clocks: NonZeroU64| {
        let timed =
            TimedSystem::with_max_clocks(&system, "M", max_clocks).expect("a timed subject");
        let run = zone_run(
            &world,
            written,
            &timed,
            &reductions,
            max_clocks,
            1,
            ZoneCheckItem::DeadlockFreedom,
            CertifiedItem::DeadlockFreedom,
        );
        run.map(|item| settled(&analyzed(vec![item])[0]).1)
    };

    let one_state = written(
        &subject,
        &DerivedLimits::from_settings([("max_states", 1)]).expect("a member"),
    );
    let stopped = deadlock(&one_state, DEFAULT_MAX_CLOCKS).expect("the run");
    assert!(
        matches!(stopped, TerminalValue::Incomplete(_)),
        "{stopped:?}"
    );

    let two_clocks = written(
        &subject,
        &DerivedLimits::from_settings([("max_clocks", 2)]).expect("a member"),
    );
    assert_eq!(
        deadlock(&two_clocks, DEFAULT_MAX_CLOCKS).err(),
        Some(DerivedItemMismatch::ClockBound {
            index: two_clocks.items()[1].index(),
            expected: 2,
            run: DEFAULT_MAX_CLOCKS.get(),
        })
    );
    let two = NonZeroU64::new(2).expect("nonzero");
    let proved = deadlock(&two_clocks, two).expect("the run");
    assert!(matches!(proved, TerminalValue::Proved { .. }), "{proved:?}");
}

/// The Counter unit with `terminal Done` at 3 and one clause over it.
fn counter_body(terminal: &str) -> String {
    format!(
        "{CONTRACT}{terminal}{}",
        clause_text("Bounded", "", "always holds(c.value <= 3)")
    )
}

const DONE: &str = "terminal Done using v on M when { self.value = 3 };\n";

/// FR-124-AC-2, QSpec FR-368-AC-17 and the producer half of AC-20 (no
/// `max_depth`, provider defaults for unset limits; the reader's refusal
/// of a `max_depth` is not QSL's) end to end: the untimed
/// Counter's request holds the clause, then one deadlock-freedom item and
/// no time-lock-freedom item, routed to both engines; run on EN-1 by
/// `derived_model_item`, it settles on EN-1 under EN-1's default limits,
/// and `limit-reached` at `max_states` when the caller sets
/// `max_states` 1. With `terminal any` there is no derived item, and the
/// clause keeps its index.
#[trace("TC-519", "FR-124-AC-2")]
#[trace("QSpec-TC-302", "QSpec-FR-368-AC-17")]
#[trace("QSpec-TC-431")]
#[test]
fn an_untimed_subjects_deadlock_item_settles_on_en1() {
    let settle = |limits: &DerivedLimits| {
        let case = case(&counter_body(DONE), &[0]);
        let item = case.model_item();
        let written = written(&item.request.subject, limits);
        assert_eq!(
            kinds(&written),
            [Some(ItemKind::Formula), Some(ItemKind::DeadlockFreedom)]
        );
        let derived = &written.items()[1];
        assert_eq!(
            derived.occurrence(),
            &root_claim(item.request.subject.simulation.package, "test/counter")
        );
        assert_eq!(
            ids(derived.candidates().expect("its candidates")),
            [en1::BACKEND_ID, en6::BACKEND_ID]
        );
        let mut records = analyze(
            &Engines::qsl().expect("the layer-A manifests register"),
            vec![derived_model_item(derived, item).expect("the deadlock-freedom run")],
            &Cancel::new(),
        );
        let record = records.remove(0);
        assert_eq!(record.request_index.get(), 1);
        settled(&record)
    };
    let (backend, value) = settle(&DerivedLimits::default());
    assert_eq!(backend, en1::BACKEND_ID);
    assert!(matches!(value, TerminalValue::Proved { .. }), "{value:?}");
    let one = DerivedLimits::from_settings([("max_states", 1)]).expect("a member");
    assert_eq!(
        settle(&one),
        (
            en1::BACKEND_ID.to_owned(),
            TerminalValue::Incomplete(IncompleteCause::LimitReached {
                limit: ModelCheckLimit::MaxStates,
                value: 1,
            })
        )
    );

    let opted_out = case(&counter_body("terminal Halt using v on M any;\n"), &[0]);
    let item = opted_out.model_item();
    let written = written(&item.request.subject, &DerivedLimits::default());
    assert_eq!(kinds(&written), [Some(ItemKind::Formula)]);
    let formula = &written.items()[0];
    assert_eq!(
        derived_model_item(formula, item).err(),
        Some(DerivedItemMismatch::NotDerived {
            index: formula.index()
        })
    );
}

/// QSpec FR-368-AC-22: a subject that reads the one selection `M` of a
/// package selecting `A` and `M` is anchored at `M`'s `model_root`; the
/// same subject reading both selections has no anchor, so its derived
/// items are settled `unsupported`, `unsupported_projection`/
/// `unsupported-requested-capability`, and no request item is written for
/// them.
#[trace("QSpec-TC-431", "QSpec-FR-368-AC-22")]
#[test]
fn a_subject_reading_several_model_selections_has_no_derived_request_item() {
    use crate::timed_refinement::{refinement, req, treq, CONTRACTS};
    use crate::timed_world::{refining, DENSE};
    let clause = "temporal DenseClause using inf over (c: M::Call) clock \"model-steps\" on \
         origin { always holds(not c.late) }\n";
    let world = refining(
        &req(),
        &treq(),
        &format!("{DENSE}{CONTRACTS}{}{clause}", refinement("self.inflight")),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    assert_eq!(world.package.graph().model_selections().len(), 2);
    let one = ModelSubject {
        simulation: world.concrete_request(),
        proof_bounds: Vec::new(),
    };
    let anchored = written(&one, &DerivedLimits::default());
    assert_eq!(
        kinds(&anchored),
        [
            Some(ItemKind::Formula),
            Some(ItemKind::DeadlockFreedom),
            Some(ItemKind::TimeLockFreedom)
        ]
    );
    assert_eq!(
        anchored.items()[1].occurrence(),
        &root_claim(&world.package, PACKAGE)
    );

    let abstract_identity = serde_json::from_slice::<serde_json::Value>(&req()).expect("JSON")
        ["package"]["identity"]
        .as_str()
        .expect("an identity")
        .to_owned();
    let mut both = world.concrete_request();
    let mut abstract_universe: PopulationUniverse = both.universes[0].clone();
    abstract_universe.population.package = abstract_identity;
    both.universes.push(abstract_universe);
    let both = ModelSubject {
        simulation: both,
        proof_bounds: Vec::new(),
    };
    let unanchored = written(&both, &DerivedLimits::default());
    assert_eq!(kinds(&unanchored), [Some(ItemKind::Formula)]);
    let settled: Vec<_> = unanchored
        .unanchored()
        .iter()
        .map(|item| (item.kind, item.cause, item.catalog_code()))
        .collect();
    let code = CatalogCode::new("unsupported_projection", "unsupported-requested-capability");
    let several = NoAnchor::SeveralSelections { rows: 2 };
    assert_eq!(
        settled,
        [
            (ItemKind::DeadlockFreedom, several, code),
            (ItemKind::TimeLockFreedom, several, code)
        ]
    );
    assert!(unanchored.unanchored()[0]
        .detail()
        .contains("deadlock-freedom"));
}

/// QSpec FR-361-AC-17 (STD-560) and FR-368-AC-22's "or none" (TC-290 step
/// 10): a subject with no universe reads no `model_selections` row, so it
/// has no anchor. Its deadlock-freedom item is settled `unsupported`,
/// `unsupported_projection`/`unsupported-requested-capability`, with no
/// request item, and the authored clause over it is written as before.
#[trace("QSpec-TC-290", "QSpec-FR-361-AC-17")]
#[trace("QSpec-TC-290", "QSpec-FR-368-AC-22")]
#[test]
fn a_subject_reading_no_model_selection_has_no_derived_request_item() {
    let world = rpc(4);
    let mut simulation = world.request();
    simulation.universes.clear();
    let subject = ModelSubject {
        simulation,
        proof_bounds: Vec::new(),
    };
    let written = written(&subject, &DerivedLimits::default());
    assert_eq!(kinds(&written), [Some(ItemKind::Formula)]);
    let [unanchored] = written.unanchored() else {
        panic!("one unanchored item: {:?}", written.unanchored());
    };
    assert_eq!(unanchored.kind, ItemKind::DeadlockFreedom);
    assert_eq!(unanchored.cause, NoAnchor::NoSelection);
    assert_eq!(
        unanchored.catalog_code(),
        CatalogCode::new("unsupported_projection", "unsupported-requested-capability")
    );
    assert!(unanchored.detail().contains("reads no model selection"));
}

/// QSpec FR-368-AC-25 over a timed model: in the dense `Rpc` unit, which
/// selects no protocol profile, the protocol `P` over `M::Call` is a protocol
/// subject whose checked clause has no node. A request with an item over the
/// model subject and one over `P` gives the model subject its
/// deadlock-freedom and time-lock-freedom items, and `P` only its
/// deadlock-freedom item, settled `unsupported` with no anchor
/// (FR-368-AC-24): `P` has no time-lock-freedom item, written or settled.
#[trace("QSpec-TC-431", "QSpec-FR-368-AC-25")]
#[test]
fn a_protocol_subject_over_a_timed_model_gains_no_time_lock_freedom_item() {
    let world = rpc_claim(
        4,
        INFINITE,
        "temporal NoLateReply using inf over (c: M::Call) clock \"model-steps\" on origin \
         { always holds(not c.late) }\n\
         protocol P using v over (c: M::Call) on origin {\n\
         role w on M::Call;\n\
         run sequence Main { }\n\
         finish Done as (d: Boolean) { true };\n\
         }\n",
    );
    let graph = world.package.graph();
    let protocol = graph.protocol("P").expect("P");
    assert_eq!(protocol.node(), None);
    let clause = node_occurrence(&graph.temporal_clauses()[0]);
    let record = Requirements::new(Capability::TemporalSatisfaction, ClaimExtent::Bounded);
    let selections: BTreeSet<&str> = [PACKAGE].into_iter().collect();
    let mut request = ModelRequest::new();
    request.authored(
        &ModelTemporalItem {
            occurrence: &clause,
            subject: &"Rpc",
            package: graph,
        },
        &record,
        &selections,
    );
    request.protocol(
        &ProtocolTemporalItem {
            occurrence: &clause,
            subject: &"P",
            protocol,
            models: &[],
            non_atomic_access: false,
        },
        &record,
    );
    let written = request.derived(&registry(), None, &DerivedLimits::default());
    let items: Vec<_> = written
        .items()
        .iter()
        .map(|item| {
            let obligation = written.obligation(item.index()).expect("its obligation");
            (obligation.subject, item.item_kind())
        })
        .collect();
    assert_eq!(
        items,
        [
            ("Rpc", Some(ItemKind::Formula)),
            ("P", Some(ItemKind::Formula)),
            ("Rpc", Some(ItemKind::DeadlockFreedom)),
            ("Rpc", Some(ItemKind::TimeLockFreedom)),
        ]
    );
    let unanchored: Vec<_> = written
        .unanchored()
        .iter()
        .map(|item| (item.subject, item.kind, item.cause))
        .collect();
    assert_eq!(
        unanchored,
        [("P", ItemKind::DeadlockFreedom, NoAnchor::NoProtocolProfile)]
    );
}
