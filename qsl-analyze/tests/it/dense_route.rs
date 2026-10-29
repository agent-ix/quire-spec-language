// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-243 (TC-698): every non-probabilistic timed item over a `time dense`
//! subject routes to EN-6 alone, with EN-1 and EN-6 both registered: the
//! timed clauses TT-1 to TT-4, an infinite-trace clause over the dense
//! subject, and the deadlock-freedom and time-lock-freedom items derived
//! from them. An item over an untimed subject keeps its candidates.

use ix_trace_rs::trace;
use qsl_analyze::model_check::manifest as en1;
use qsl_analyze::zone_check::manifest as en6;
use qsl_replay::node_occurrence;
use qsl_route::deadlock::ModelTemporalItem;
use qsl_route::request::items_from_requirements;
use qsl_route::request::ItemKind;
use qsl_route::{CandidateOutcome, ManifestDigest, ProviderOrigin, Registry};
use qsl_semantics::check::TimedPropertyForm;

use crate::derived_request::derived_items;
use crate::timed_world::{document, world, World, PACKAGE};
use crate::zone_search::{rpc_claim, INFINITE, TIMED};

/// EN-1 and EN-6, each registered from its own manifest.
pub(crate) fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register(
            en1::descriptor(ProviderOrigin::Linked, ManifestDigest::from_digest([1; 32]))
                .expect("EN-1 admits"),
        )
        .expect("registered");
    registry
        .register(
            en6::descriptor(ProviderOrigin::Linked, ManifestDigest::from_digest([6; 32]))
                .expect("EN-6 admits"),
        )
        .expect("registered");
    registry
}

/// The backend identities of a computed candidate set.
pub(crate) fn ids(outcome: &CandidateOutcome) -> Vec<String> {
    let CandidateOutcome::Candidates(set) = outcome else {
        panic!("a computed candidate set: {outcome:?}");
    };
    set.candidates()
        .iter()
        .map(|candidate| candidate.id().as_str().to_owned())
        .collect()
}

/// How `world`'s one temporal clause and the items derived from it route.
struct Routes {
    /// The clause item's candidates.
    candidates: Vec<String>,
    /// Whether the clause's record is marked dense.
    dense: bool,
    /// The kind and candidates of each derived item, in request order.
    derived: Vec<(ItemKind, Vec<String>)>,
}

fn routes(world: &World) -> Routes {
    let registry = registry();
    let graph = world.package.graph();
    let [clause] = graph.temporal_clauses() else {
        panic!("one temporal clause");
    };
    let occurrence = node_occurrence(clause);
    let records = graph.requirements();
    let dense = records
        .get(&occurrence)
        .expect("the clause's record")
        .requirements()
        .dense_time();
    let items = items_from_requirements(records, &registry, None);
    let item = items
        .iter()
        .find(|item| *item.occurrence() == occurrence)
        .expect("the clause's item");
    let subject = "rpc";
    let temporal = [ModelTemporalItem {
        occurrence: &occurrence,
        subject: &subject,
        package: graph,
    }];
    let derived = derived_items(&temporal, &[], &[], &[PACKAGE])
        .iter()
        .map(|(item, _)| {
            (
                item.item_kind().expect("a derived kind"),
                ids(item.candidates().expect("its candidates")),
            )
        })
        .collect();
    Routes {
        candidates: ids(item.candidates()),
        dense,
        derived,
    }
}

/// FR-243-AC-1 and AC-3 (TC-698): over the dense `Rpc`, `NoLateReply` under
/// the infinite-trace profile and each timed form TT-1 to TT-4 route to
/// EN-6 alone, and so do the deadlock-freedom and time-lock-freedom items
/// over the same subject.
#[trace("TC-698", "FR-243-AC-1")]
#[trace("TC-698", "FR-243-AC-3")]
#[test]
fn timed_items_over_a_dense_subject_route_to_en6_alone() {
    let clause = |profile: &str, body: &str| {
        format!(
            "temporal P using {profile} over (c: M::Call) clock \"{clock}\" on origin {{ {body} }}\n",
            clock = if profile == "tt" { "model-time" } else { "model-steps" },
        )
    };
    let infinite = rpc_claim(4, INFINITE, &clause("inf", "always holds(not c.late)"));
    let timed = [
        (
            "always holds(not c.late)",
            TimedPropertyForm::TimedInvariant,
        ),
        (
            "eventually[0 ms, 3 ms] holds(c.inflight)",
            TimedPropertyForm::BoundedWindow {
                horizon: quire_exact::Rational::from_integer(quire_exact::Integer::from(3u64)),
            },
        ),
        (
            "always (holds(c.inflight) implies eventually[0 ms, 3 ms] holds(not c.inflight))",
            TimedPropertyForm::TimedSafety,
        ),
        (
            "always eventually holds(c.inflight)",
            TimedPropertyForm::TimedLiveness,
        ),
    ];
    let mut worlds = vec![(None, infinite)];
    for (body, form) in timed {
        worlds.push((Some(form), rpc_claim(4, TIMED, &clause("tt", body))));
    }
    for (form, world) in &worlds {
        let [node] = world.package.graph().temporal_clauses() else {
            panic!("one temporal clause");
        };
        assert_eq!(node.clause().timed_form(), form.as_ref());
        let Routes {
            candidates,
            dense,
            derived,
        } = routes(world);
        assert!(dense, "{form:?}");
        assert_eq!(candidates, [en6::BACKEND_ID], "{form:?}");
        assert_eq!(
            derived,
            [
                (ItemKind::DeadlockFreedom, vec![en6::BACKEND_ID.to_owned()]),
                (ItemKind::TimeLockFreedom, vec![en6::BACKEND_ID.to_owned()])
            ],
            "{form:?}: the deadlock-freedom and time-lock-freedom items"
        );
    }
}

/// FR-243 / FR-430: an item over an untimed subject is not marked dense and
/// keeps both candidates; it derives a deadlock-freedom item that is not
/// dense, and no time-lock-freedom item.
#[trace("TC-698")]
#[test]
fn an_item_over_an_untimed_subject_keeps_its_candidates() {
    let untimed = world(
        &document(&[], &[("send", &["inflight"])]),
        INFINITE,
        "temporal P using inf over (c: M::Call) clock \"model-steps\" on origin \
         { always holds(not c.late) }\n",
    );
    let Routes {
        candidates,
        dense,
        derived,
    } = routes(&untimed);
    assert!(!dense);
    assert_eq!(candidates, [en1::BACKEND_ID, en6::BACKEND_ID]);
    assert_eq!(
        derived,
        [(
            ItemKind::DeadlockFreedom,
            vec![en1::BACKEND_ID.to_owned(), en6::BACKEND_ID.to_owned()]
        )]
    );
}

/// The record `analyze` settles for `world`'s one clause on EN-6, through
/// `zone_item`: the zone search, FR-245's certificate check and FR-237's
/// replay, under the package identity `package`.
fn analyzed(world: &World) -> qsl_analyze::analyze::AnalyzeRecord {
    use qsl_analyze::analyze::{analyze, zone_item, Engines, TimedItem};
    use qsl_analyze::zone_check::{ZoneCheckItem, ZoneCheckLimits, ZoneCheckRequest};
    use qsl_eval::simulation::model::{TimedSystem, DEFAULT_MAX_CLOCKS};
    use qsl_replay::{CertificateCheckLimits, CertifiedItem, ModelTraceReplayResult};
    use qsl_route::deadlock::{ItemObligation, ObligationKind};
    use qsl_route::reductions::{ObligationReductions, Reductions};

    let package = qsl_semantics::library::PackageId::of_preimage(b"analyze-dense");
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("letters");
    let reductions = Reductions::default();
    let item = TimedItem {
        request: ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::Clause {
                clause: node.clause(),
                letters: &letters,
            },
            obligation: ItemObligation {
                subject: "c",
                kind: ObligationKind::Authored(node_occurrence(node)),
                reductions: ObligationReductions::default(),
            },
            limits: ZoneCheckLimits::default(),
            reductions: &reductions,
        },
        certificate: qsl_replay::ZoneCertificateRequest {
            simulation: world.request(),
            model: "M",
            max_clocks: DEFAULT_MAX_CLOCKS,
            item: CertifiedItem::Clause {
                clause: node.clause(),
                letters: &letters,
            },
        },
        certificate_limits: CertificateCheckLimits::default(),
        package_id: package.record(),
        replay: Box::new(|payload| {
            qsl_replay::replay_timed_counterexample(
                &timed,
                payload,
                Some(node.clause()),
                &letters,
                system.meter_limits(),
                &qsl_eval::simulation::Limits::default(),
            )
            .map(|(settlement, hints)| ModelTraceReplayResult {
                settlement,
                hints,
                package_id: package,
            })
        }),
    };
    let mut records = analyze(
        &Engines::qsl().expect("the layer-A manifests register"),
        vec![zone_item(qsl_foundation::RequestIndex::new(0), item)],
        &quire_exact::Cancel::new(),
    );
    assert_eq!(records.len(), 1);
    records.remove(0)
}

/// FR-243-AC-1 end to end (TC-698): `NoLateReply` over the dense `Rpc` runs
/// on EN-6 through `analyze` and settles `Proved{ZoneCertified, Certified}`
/// with `T = 4 ms`, once FR-245 accepts its certificate, and `refuted` with
/// `T = 3 ms`, once its counterexample replays (FR-239-AC-1).
#[trace("TC-698", "FR-243-AC-1")]
#[test]
fn a_dense_clause_settles_on_en6_through_analyze() {
    use qsl_replay::{Certification, ProofBasis, TerminalValue};
    let claim = "temporal NoLateReply using inf over (c: M::Call) clock \"model-steps\" on origin \
         { always holds(not c.late) }\n";
    for (t, value) in [
        (
            4,
            TerminalValue::Proved {
                basis: ProofBasis::ZoneCertified,
                certification: Certification::Certified,
            },
        ),
        (3, TerminalValue::Refuted),
    ] {
        let record = analyzed(&rpc_claim(t, INFINITE, claim));
        let terminal = record.terminal().expect("a terminal record");
        assert_eq!(
            terminal.backend().map(qsl_foundation::BackendId::as_str),
            Some(en6::BACKEND_ID),
            "T = {t}"
        );
        assert_eq!(*terminal.value(), value, "T = {t}");
    }
}

/// FR-144-AC-1 with FR-430 (TC-549, SR-1751002 FND-001): with EN-1 and EN-6
/// both registered, the untimed `CasRefinesCounter` record routes to the
/// explicit-state engine alone, while an untimed clause keeps both.
#[trace("TC-549", "FR-144-AC-1")]
#[test]
fn an_untimed_refinement_record_routes_to_explicit_state_alone() {
    use crate::refinement_fixture::{cas_refines_counter, World as Cas};
    let world = Cas::new(&cas_refines_counter());
    let occurrence = world
        .refinement()
        .requirement_occurrence()
        .expect("the refinement has a record");
    let records = world.package().graph().requirements();
    assert!(!records
        .get(occurrence)
        .expect("its record")
        .requirements()
        .dense_time());
    let items = items_from_requirements(records, &registry(), None);
    let item = items
        .iter()
        .find(|item| item.occurrence() == occurrence)
        .expect("the record's item");
    assert_eq!(ids(item.candidates()), [en1::BACKEND_ID]);
}

/// FR-243 with FR-144 (TC-698, SR-1753002 FND-002): one package holding the
/// dense `M` and the untimed `A` routes each record by its own subject, in
/// one `items_from_requirements` call: the clause over `M` and the timed
/// refinement of `A` by `M` to EN-6 alone, the clause over `A` to both
/// engines, and the untimed refinement of `A` by `A` to explicit-state
/// alone.
#[trace("TC-698", "FR-243-AC-1")]
#[trace("TC-549", "FR-144-AC-1")]
#[test]
fn a_mixed_package_routes_each_record_by_its_own_subject() {
    use crate::timed_refinement::{refinement, req, treq, CONTRACTS};
    use crate::timed_world::{refining, DENSE};
    let clauses = "temporal DenseClause using inf over (c: M::Call) clock \"model-steps\" on \
         origin { always holds(not c.late) }\n\
         temporal UntimedClause using inf over (c: A::Call) clock \"model-steps\" on \
         origin { always holds(not c.late) }\n";
    let untimed = "refinement Same using inf abstract A concrete A {\n\
         population A::calls from A::calls;\n\
         object A::Call from A::Call { phase = self.phase; inflight = self.inflight; late = self.late; retried = self.retried; }\n\
         step A::Call::send -> A::Call::send(self);\n\
         }\n";
    let world = refining(
        &req(),
        &treq(),
        &format!(
            "{DENSE}{CONTRACTS}{}{untimed}{clauses}",
            refinement("self.inflight")
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let graph = world.package.graph();
    let records = graph.requirements();
    let items = items_from_requirements(records, &registry(), None);
    let route = |occurrence: &qsl_foundation::source::provenance::OccurrenceKey| {
        let dense = records
            .get(occurrence)
            .expect("its record")
            .requirements()
            .dense_time();
        let item = items
            .iter()
            .find(|item| item.occurrence() == occurrence)
            .expect("its item");
        (dense, ids(item.candidates()))
    };
    let clause = |name: &str| {
        let node = graph
            .temporal_clauses()
            .iter()
            .find(|node| node.clause().name() == name)
            .unwrap_or_else(|| panic!("the clause {name}"));
        route(&node_occurrence(node))
    };
    let refined = |name: &str| {
        let refinement = graph
            .refinements()
            .iter()
            .find(|refinement| refinement.name() == name)
            .unwrap_or_else(|| panic!("the refinement {name}"));
        route(
            refinement
                .requirement_occurrence()
                .expect("the refinement has a record"),
        )
    };
    let both = vec![en1::BACKEND_ID.to_owned(), en6::BACKEND_ID.to_owned()];
    assert_eq!(
        clause("DenseClause"),
        (true, vec![en6::BACKEND_ID.to_owned()])
    );
    assert_eq!(clause("UntimedClause"), (false, both));
    assert_eq!(refined("Timed"), (true, vec![en6::BACKEND_ID.to_owned()]));
    assert_eq!(refined("Same"), (false, vec![en1::BACKEND_ID.to_owned()]));
}

/// FR-235-AC-5 and FR-281 end to end (TC-710, SR-1755002 FND-001):
/// FR-239-AC-5's `Ticker` clause, whose division by zero is undefined at
/// position 2, settles `refuted` on EN-6 through `analyze`; the record
/// carries EN-6's counterexample, and its outcome item names the cause
/// `undefined-evaluation` (`division-by-zero` at position 2), never the
/// `undefined` label.
#[trace("TC-710", "FR-235-AC-5")]
#[test]
fn an_undefined_timed_evaluation_settles_refuted_through_analyze() {
    use qsl_replay::{CounterexampleKind, TerminalValue};
    use quire_semantic_value::catalog::Category;
    let record = analyzed(&crate::zone_search::ticker(false));
    let terminal = record.terminal().expect("a terminal record");
    assert_eq!(*terminal.value(), TerminalValue::Refuted);
    assert_eq!(
        terminal.backend().map(qsl_foundation::BackendId::as_str),
        Some(en6::BACKEND_ID)
    );
    let counterexample = record
        .counterexample()
        .expect("the record carries EN-6's counterexample");
    let CounterexampleKind::UndefinedEvaluation(undefined) = &counterexample.kind else {
        panic!("an undefined evaluation: {:?}", counterexample.kind);
    };
    assert_eq!(undefined.site.position, 2);
    let item = record.outcome_item().expect("an item");
    assert_eq!(item.category(), Category::Violation);
    let written = serde_json::to_value(&item).expect("writes");
    assert_eq!(written["result"], "refuted");
    assert_eq!(written["cause"]["kind"], "undefined-evaluation");
    assert_eq!(written["cause"]["cause"], "division-by-zero");
    assert!(
        written["cause"]["where"]
            .as_str()
            .is_some_and(|at| at.starts_with("position 2, state ")),
        "{written}"
    );
    assert!(!written.to_string().contains("\"undefined\""));
}
