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
    analyze, derived_model_item, derived_zone_item, AnalyzeRecord, DerivedItemMismatch, Engines,
    TimedItem,
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
use qsl_replay::SubjectIdentity;
use qsl_replay::{
    CertificateCheckLimits, CertifiedItem, IncompleteCause, ModelCheckLimit, TerminalValue,
    ZoneCertificateRequest,
};
use qsl_route::model_request::{ModelRequest, WrittenRequest};
use qsl_route::reductions::Reductions;
use qsl_route::request::{DerivedLimits, ExtentClassification, ItemKind, RequestItem};
use qsl_semantics::check::Capability;
use qsl_semantics::library::PackageId;
use quire_exact::Cancel;
use quire_semantic_value::catalog::CatalogCode;

use crate::counter::{clause_text, CONTRACT};
use crate::dense_route::{ids, registry};
use crate::model_settle::case;
use crate::timed_world::PACKAGE;
use crate::zone_search::rpc;

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

/// QSpec FR-368-AC-14, AC-16, AC-17 and AC-18 with FR-243: over the dense
/// `Rpc`, the request holds the authored clause at index 0, then the
/// deadlock-freedom and then the time-lock-freedom item. Both derived
/// items are `temporal-satisfaction`, unbounded with no finite bound, carry
/// no `hybrid-dynamics` requirement, name the same `model_root` claim, and
/// route to EN-6 alone.
#[trace("TC-698", "FR-243-AC-1")]
#[trace("TC-687", "FR-232-AC-1")]
#[trace("QSpec-TC-431", "QSpec-FR-368-AC-14")]
#[trace("QSpec-TC-431", "QSpec-FR-368-AC-16")]
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
    let package = PackageId::of_preimage(b"derived-dense");
    let reductions = Reductions::default();
    let run = |at: usize, item: ZoneCheckItem<'static>, certified: CertifiedItem<'static>| {
        let derived = &written.items()[at];
        let timed_item = TimedItem {
            request: ZoneCheckRequest {
                subject: &timed,
                item,
                obligation: written
                    .obligation(derived.index())
                    .expect("its obligation")
                    .clone(),
                limits: ZoneCheckLimits::default(),
                reductions: &reductions,
            },
            certificate: ZoneCertificateRequest {
                simulation: world.request(),
                model: "M",
                max_clocks: DEFAULT_MAX_CLOCKS,
                item: certified,
            },
            certificate_limits: CertificateCheckLimits::default(),
            package_id: package.record(),
            replay: Box::new(|_| panic!("a proof has no counterexample to replay")),
        };
        derived_zone_item(derived, timed_item)
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
    let records = analyze(
        &Engines::qsl().expect("the layer-A manifests register"),
        vec![
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
        ],
        &Cancel::new(),
    );
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

/// The Counter unit with `terminal Done` at 3 and one clause over it.
fn counter_body(terminal: &str) -> String {
    format!(
        "{CONTRACT}{terminal}{}",
        clause_text("Bounded", "", "always holds(c.value <= 3)")
    )
}

const DONE: &str = "terminal Done using v on M when { self.value = 3 };\n";

/// FR-124-AC-2 and QSpec FR-368-AC-17, AC-20 end to end: the untimed
/// Counter's request holds the clause, then one deadlock-freedom item and
/// no time-lock-freedom item, routed to both engines; run on EN-1 by
/// `derived_model_item`, it settles on EN-1 under EN-1's default limits,
/// and `limit-reached` at `max_states` when the caller sets
/// `max_states` 1. With `terminal any` there is no derived item, and the
/// clause keeps its index.
#[trace("TC-519", "FR-124-AC-2")]
#[trace("QSpec-TC-302", "QSpec-FR-368-AC-17")]
#[trace("QSpec-TC-431", "QSpec-FR-368-AC-20")]
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
        .map(|item| (item.kind, item.rows, item.catalog_code()))
        .collect();
    let code = CatalogCode::new("unsupported_projection", "unsupported-requested-capability");
    assert_eq!(
        settled,
        [
            (ItemKind::DeadlockFreedom, 2, code),
            (ItemKind::TimeLockFreedom, 2, code)
        ]
    );
    assert!(unanchored.unanchored()[0]
        .detail()
        .contains("deadlock-freedom"));
}
