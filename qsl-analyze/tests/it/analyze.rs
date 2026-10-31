// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-281 (TC-762, TC-763) and FR-280-AC-1 (TC-761): the `analyze`
//! operation over the layer-A engines it has item adapters for, EN-1 and
//! EN-4. Every verdict comes from the engine's own run, certificate checker
//! and counterexample replay; no test hands `analyze` a verdict.
//!
//! Open: FR-281-AC-2 for an invariant one transition breaks. The authored
//! clause's counterexample now replays (`replay_model_trace` reproduces an
//! authored clause's counterexample, QSL-1096), but no test here builds that
//! item over a real clause yet. The deadlock case here is TC-762 step 2's
//! refuted item, not AC-2's clause.
//!
//! Not covered, because EN-5 has no solver or certificate checker yet
//! (QSL-574, QSL-575): FR-281-AC-1's EN-5 item, and so the EN-5 half of
//! FR-281-AC-3 and of FR-281-AC-6 (EN-1 runs those here). Not covered:
//! FR-281-AC-7's replay of the undefined value through a real authored
//! clause (the replay exists, QSL-1096, but the test supplies the replay
//! result by hand); its settlement from a replay result that reproduces the
//! undefined value is.

use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze, model_item, AnalyzeItem, DispositionKind, Engine, Engines, Payload, Settlement,
};
use qsl_eval::simulation::Limits;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::{
    settle_model_check, CertificateRule, Certification, CounterexampleKind, IncompleteCause,
    InconclusiveCause, ModelCheckItem, ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome,
    ModelTraceSettlement, ModelTraceValue, ProofBasis, RunStats, TemporalCounterexample,
    TerminalValue, UndefinedEvaluation, UndefinedLocus, UndefinedSite,
};
use qsl_route::{Mode, ProviderOrigin};
use qsl_semantics::check::Capability;
use quire_exact::{Cancel, CancelCause, Undefined};
use quire_semantic_value::catalog::Category;

use crate::closure_certificate::world;
use crate::counter::{CONTRACT, UNLIMITED};
use crate::model_settle::{case, replay};

fn index(at: usize) -> RequestIndex {
    RequestIndex::new(at)
}

fn en1() -> Engines {
    let mut engines = Engines::default();
    engines
        .with(Engine::model_check().expect("EN-1's own manifest is admitted"))
        .expect("one engine");
    engines
}

fn only(
    mut records: Vec<qsl_analyze::analyze::AnalyzeRecord>,
) -> qsl_analyze::analyze::AnalyzeRecord {
    assert_eq!(records.len(), 1);
    records.remove(0)
}

fn value(record: &qsl_analyze::analyze::AnalyzeRecord) -> TerminalValue {
    record
        .terminal()
        .expect("a terminal record")
        .value()
        .clone()
}

/// TC-762 step 2 for EN-1's deadlock-freedom item, which the Counter subject
/// breaks at its last state: the item settles `refuted`, the record carries
/// the counterexample, and `replay` reproduces it. FR-281-AC-2 stays open
/// (see the module header).
#[trace("TC-762")]
#[test]
fn a_broken_item_settles_refuted_and_its_counterexample_replays() {
    let case = case(CONTRACT, &[0]);
    let record = only(analyze(
        &en1(),
        vec![model_item(index(0), case.model_item())],
        &Cancel::new(),
    ));
    assert_eq!(value(&record), TerminalValue::Refuted);
    let terminal = record.terminal().expect("a terminal record");
    assert_eq!(terminal.backend().map(BackendId::as_str), Some("en-1"));
    assert_eq!(terminal.value().category(), Category::Violation);
    let counterexample = record
        .model_check
        .as_ref()
        .and_then(|model| model.counterexample.clone())
        .expect("the record carries the counterexample");
    assert_eq!(counterexample.kind, CounterexampleKind::Deadlock);
    let replayed = replay(&case, counterexample).expect("the counterexample replays");
    assert!(
        matches!(
            replayed.settlement,
            ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Truth(false),
                ..
            }
        ),
        "{:?}",
        replayed.settlement
    );
}

/// FR-281-AC-3 (TC-762 step 3), with EN-1 for the item an engine proves: a
/// request of three items, one whose claim kind no engine advertises, one
/// EN-1 proves and one it refutes, returns three records in request order,
/// the first `unsupported` with a cause naming the claim kind.
#[trace("TC-762", "FR-281-AC-3")]
#[test]
fn three_items_give_three_records_the_first_unsupported_naming_its_kind() {
    let world = world("", &[("c", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let broken = case(CONTRACT, &[0]);
    let records = analyze(
        &Engines::qsl().expect("the layer-A manifests register"),
        vec![
            AnalyzeItem::unadapted(index(0), Capability::Monitorability),
            model_item(
                index(1),
                invariant.model_item(&world, ModelCheckLimits::default(), |_| {
                    panic!("a proved item replays nothing")
                }),
            ),
            model_item(index(2), broken.model_item()),
        ],
        &Cancel::new(),
    );
    assert_eq!(
        records.iter().map(|r| r.request_index).collect::<Vec<_>>(),
        [index(0), index(1), index(2)]
    );
    let Settlement::Disposed(unsupported) = &records[0].settlement else {
        panic!("expected a disposition: {:?}", records[0].settlement);
    };
    assert_eq!(unsupported.kind, DispositionKind::Unsupported);
    assert_eq!(unsupported.backend, None);
    let diagnostic = unsupported.diagnostic.as_ref().expect("a diagnostic");
    assert_eq!(
        diagnostic.code.to_string(),
        "unsupported_projection/unsupported-requested-capability"
    );
    assert_eq!(
        diagnostic.payload,
        Payload::Kind(Capability::Monitorability)
    );
    assert_eq!(
        value(&records[1]),
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified,
        }
    );
    assert_eq!(value(&records[2]), TerminalValue::Refuted);
}

/// FR-281-AC-4 (TC-763 steps 1 and 2), over EN-1: the invariant item proves
/// and its closure certificate is accepted (`certified`); offered a
/// certificate request for another item, the core checker rejects the
/// certificate and the item settles `inconclusive` with `CertificateRejected`.
#[trace("TC-763", "FR-281-AC-4")]
#[test]
fn a_rejected_certificate_is_inconclusive_and_an_accepted_one_certified() {
    let world = world(CONTRACT, &[("c", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let run = |item| {
        only(analyze(
            &en1(),
            vec![model_item(index(0), item)],
            &Cancel::new(),
        ))
    };
    let accepted = run(
        invariant.model_item(&world, ModelCheckLimits::default(), |_| {
            panic!("a proved item replays nothing")
        }),
    );
    assert_eq!(
        value(&accepted),
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified,
        }
    );

    let mut foreign = invariant.model_item(&world, ModelCheckLimits::default(), |_| {
        panic!("a proved item replays nothing")
    });
    foreign.certificate =
        invariant.request_with(&world, ModelCheckItem::DeadlockFreedom, UNLIMITED);
    let rejected = run(foreign);
    assert!(
        matches!(
            value(&rejected),
            TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
                rule: CertificateRule::ShapeMismatch,
                ..
            })
        ),
        "{:?}",
        value(&rejected)
    );
}

/// FR-281-AC-4 (TC-763 step 2): a proof from an engine whose result no core
/// checker covers settles `proved`, category success, `uncertified`, naming
/// that engine. EN-1 under partial-order reduction is such an engine: a
/// reduced graph has no core checker (ADR-021 RV-1, FR-160), so its proof
/// carries no certificate to check. The same item unreduced settles
/// `certified`, so the label comes from the missing checker and not from the
/// engine.
#[trace("TC-763", "FR-281-AC-4")]
#[test]
fn a_proof_with_no_core_checker_settles_proved_uncertified_naming_its_engine() {
    let world = world(CONTRACT, &[("c", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let run = |reduced: bool| {
        let mut item = invariant.model_item(&world, ModelCheckLimits::default(), |_| {
            panic!("a proved item replays nothing")
        });
        item.request.reductions.partial_order = reduced;
        only(analyze(
            &en1(),
            vec![model_item(index(0), item)],
            &Cancel::new(),
        ))
    };
    let reduced = run(true);
    let TerminalValue::Proved {
        basis: ProofBasis::Reduced { .. },
        certification,
    } = value(&reduced)
    else {
        panic!("expected a reduced proof: {:?}", value(&reduced));
    };
    assert_eq!(certification, Certification::Uncertified);
    let terminal = reduced.terminal().expect("a terminal record");
    assert_eq!(terminal.value().category(), Category::Success);
    assert_eq!(terminal.backend().map(BackendId::as_str), Some("en-1"));
    assert_eq!(
        value(&run(false)),
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified,
        }
    );
}

/// FR-281-AC-5 (TC-763 step 3), over EN-1's refuted deadlock item: a
/// counterexample the real replay does not reproduce (its prefix truncated)
/// settles `inconclusive` with `replay_parity`; the unaltered one settles
/// `refuted`.
#[trace("TC-763", "FR-281-AC-5")]
#[test]
fn a_counterexample_settles_refuted_only_when_replay_reproduces_it() {
    let case = case(CONTRACT, &[0]);
    let run = |alter: fn(&mut TemporalCounterexample)| {
        let mut item = case.model_item();
        item.replay = Box::new(|payload| {
            let mut payload = payload.clone();
            alter(&mut payload);
            replay(&case, payload)
        });
        value(&only(analyze(
            &en1(),
            vec![model_item(index(0), item)],
            &Cancel::new(),
        )))
    };

    assert!(matches!(
        run(|payload| {
            payload.prefix.pop();
        }),
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
    ));
    assert_eq!(run(|_| {}), TerminalValue::Refuted);
}

/// FR-281-AC-6 (TC-763 step 4), over EN-1: the proved item with its
/// engine's state budget set to 1 settles `incomplete` with the state-budget
/// cause. A cancelled handle settles each routed item `incomplete`, keeping
/// the cancellation cause.
#[trace("TC-763", "FR-281-AC-6")]
#[test]
fn a_state_budget_of_one_and_a_cancelled_handle_settle_incomplete() {
    let world = world("", &[("c", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let limits = ModelCheckLimits {
        limits: Limits {
            max_states: 1,
            ..Limits::default()
        },
        ..ModelCheckLimits::default()
    };
    let record = only(analyze(
        &en1(),
        vec![model_item(
            index(0),
            invariant.model_item(&world, limits, |_| panic!("nothing to replay")),
        )],
        &Cancel::new(),
    ));
    assert_eq!(
        value(&record),
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 1,
        })
    );

    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let records = analyze(
        &en1(),
        vec![model_item(
            index(0),
            invariant.model_item(&world, ModelCheckLimits::default(), |_| {
                panic!("a cancelled item runs nothing")
            }),
        )],
        &cancel,
    );
    assert_eq!(
        value(&records[0]),
        TerminalValue::Incomplete(IncompleteCause::Cancelled {
            source: CancelCause::Requested
        })
    );
}

/// FR-281-AC-7 (TC-763 step 5): an EN-1 refutation by an undefined claim
/// evaluation. The counterexample is the Counter subject's deadlock
/// counterexample with its kind set to the undefined evaluation at its last
/// state, and its replay is the real replay of that deadlock with the
/// reproduced value replaced by the undefined one: the test does not build a
/// real authored clause whose replay reproduces the undefined evaluation
/// (QSL-1096 makes that replay possible). The item settles `refuted`, category
/// violation, with cause `undefined-evaluation` whose `where` names the state
/// and whose `cause` is the undefined reason, and the document writes no
/// `undefined` label.
#[trace("TC-763", "FR-281-AC-7")]
#[test]
fn an_undefined_claim_evaluation_settles_refuted_with_the_undefined_cause() {
    let case = case(CONTRACT, &[0]);
    let mut counterexample = crate::model_settle::counterexample(&case);
    let last = counterexample.prefix.len() as u64;
    let evaluation = UndefinedEvaluation {
        site: UndefinedSite {
            position: last,
            state_key: {
                let request = case.request();
                let system =
                    qsl_eval::simulation::model::ModelSystem::new(request.subject.simulation)
                        .expect("the model admits");
                let (bytes, _) = system
                    .state_key(&system.initial_states()[0])
                    .expect("encodes");
                qsl_eval::simulation::StateKey::from_canonical_bytes(bytes)
            },
            locus: UndefinedLocus::Contract("CanInc".to_owned()),
            transition: None,
            time: None,
        },
        cause: UndefinedCause::Kernel(Undefined::DivisionByZero),
    };
    counterexample.kind = CounterexampleKind::UndefinedEvaluation(evaluation.clone());
    let settled = settle_model_check(
        &ModelCheckOutcome::Violated {
            counterexample: Box::new(counterexample.clone()),
            stats: RunStats::default(),
        },
        &ModelCheckLimits::default(),
        &case.request(),
        &Cancel::new(),
        |payload| {
            let mut result = replay(&case, crate::model_settle::counterexample(&case))?;
            result.settlement = ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Undefined(evaluation.clone()),
                trace_position: payload.trace_position(),
            };
            Ok(result)
        },
    );
    assert_eq!(settled.value, TerminalValue::Refuted);
    // The undefined counterexample rides the record `analyze` settles for an
    // EN-1 item whose run violated it and whose replay reproduced it.
    let mut item = case.model_item();
    item.replay = Box::new(|payload| {
        let mut result = replay(&case, crate::model_settle::counterexample(&case))?;
        result.settlement = ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Undefined(evaluation.clone()),
            trace_position: payload.trace_position(),
        };
        Ok(result)
    });
    let mut record = only(analyze(
        &en1(),
        vec![model_item(index(0), item)],
        &Cancel::new(),
    ));
    assert_eq!(value(&record), TerminalValue::Refuted);
    record
        .model_check
        .as_mut()
        .and_then(|model| model.counterexample.as_mut())
        .expect("the record carries the counterexample")
        .kind = CounterexampleKind::UndefinedEvaluation(evaluation.clone());
    let item = record.outcome_item().expect("an item");
    assert_eq!(item.category(), Category::Violation);
    let written = serde_json::to_value(&item).expect("writes");
    assert_eq!(written["result"], "refuted");
    assert_eq!(written["category"], "violation");
    assert_eq!(written["cause"]["kind"], "undefined-evaluation");
    assert_eq!(written["cause"]["cause"], "division-by-zero");
    assert!(
        written["cause"]["where"]
            .as_str()
            .is_some_and(|at| at.starts_with(&format!("position {last}, state "))),
        "{written}"
    );
    assert!(!written.to_string().contains("\"undefined\""));
}

/// FR-280-AC-1 (TC-761 step 1) for EN-1: its manifest converts into a
/// descriptor advertising exactly the claim kind it settles in TC-762,
/// `temporal-satisfaction` over a bounded domain, and the two layer-A
/// engines register together.
#[trace("TC-761", "FR-280-AC-1")]
#[test]
fn en1_advertises_exactly_the_temporal_satisfaction_pair() {
    use qsl_analyze::model_check::manifest;
    let descriptor =
        manifest::descriptor(ProviderOrigin::Linked).expect("EN-1's own manifest is admitted");
    assert_eq!(descriptor.id().as_str(), manifest::BACKEND_ID);
    assert_eq!(
        descriptor.advertises().collect::<Vec<_>>(),
        [(Capability::TemporalSatisfaction, Mode::Bounded)]
    );
}

/// FR-226-AC-2 (TC-671 step 2): EN-1's registered provider manifest
/// descriptor carries `memory_models` `sc`, `tso` and `ra`.
#[trace("TC-671", "FR-226-AC-2")]
#[test]
fn en1_advertises_the_memory_models_sc_tso_and_ra() {
    use qsl_analyze::model_check::manifest;
    use qsl_foundation::MemoryModelName::{Ra, Sc, Tso};
    let descriptor =
        manifest::descriptor(ProviderOrigin::Linked).expect("EN-1's own manifest is admitted");
    assert_eq!(
        descriptor.memory_models(),
        &std::collections::BTreeSet::from([Sc, Tso, Ra])
    );
}

fn service_request<'w>(
    world: &'w crate::prob_world::World,
) -> qsl_analyze::statistical::StatisticalRequest<'w> {
    qsl_analyze::statistical::StatisticalRequest {
        subject: world.subject(),
        claim: world.claim("Two"),
        method: qsl_replay::statistical::StatisticalMethod::Okamoto,
        seed: 7,
        sampler: qsl_foundation::selection::DefinitionRef::new(
            "agent-ix",
            "quire.simulation.sampler/v1",
        )
        .expect("a valid DefinitionRef"),
        limits: qsl_replay::statistical::StatisticalLimits::default(),
        max_witnesses: 3,
        symmetry: Vec::new(),
    }
}

/// The `analyze` half of FR-280-AC-5 (TC-761 step 4); FR-280-AC-2 stays
/// open for EN-1, which has no provider entry yet: over
/// `Service`, whose checked package S4 refuses to emit, the record `analyze`
/// settles for the `2 ms` claim (EN-4, `statistical`, Okamoto, seed 7,
/// `max_witnesses` 3) is `measured` and rejected with its three witnesses,
/// and equals the record the provider entry returns for the same item,
/// package, binding and limits. Both reach one `check_statistical` and one
/// settlement; the witnesses replay `Reproduced` in
/// `statistical_witness_replay`.
#[trace("TC-761", "FR-280-AC-5")]
#[test]
fn analyze_and_the_provider_entry_settle_one_record_for_one_en4_item() {
    use crate::prob_world::{self, World};
    use qsl_analyze::analyze::statistical_item;
    use qsl_analyze::statistical::provider::serve_items;
    use qsl_replay::statistical::Decision;

    let claim = "probabilistic Two using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 \
         { quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
         until holds(s.phase = 2) within 10) <= 2 }\n";
    let world = World::new(&prob_world::service(claim));
    qsl_package::emit_checked(&world.package).expect_err("S4 refuses Service");

    let analyzed = only(analyze(
        &Engines::qsl().expect("they register"),
        vec![statistical_item(index(0), service_request(&world))],
        &Cancel::new(),
    ))
    .terminal()
    .cloned()
    .expect("the item settles a record");
    let served = serve_items(
        vec![qsl_analyze::statistical::provider::RoutedItem {
            index: index(0),
            request: service_request(&world),
        }],
        &Cancel::new(),
    )
    .remove(0)
    .expect("the item settles a record");

    assert_eq!(analyzed, served);
    assert_eq!(analyzed.backend().map(BackendId::as_str), Some("en-4"));
    let TerminalValue::Measured(verdict) = analyzed.value() else {
        panic!("expected measured, got {:?}", analyzed.value());
    };
    assert_eq!(verdict.decision, Decision::Rejected);
    assert_eq!(verdict.witnesses.len(), 3);
    qsl_package::emit_checked(&world.package).expect_err("S4 still refuses Service");
}
