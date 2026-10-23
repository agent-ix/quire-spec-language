// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-169's settlement map over state-graph certificates
//! (`qsl_replay::settle_state_graph`): an outcome whose certificate the core
//! accepts settles `proved`, `Certified`, and one whose certificate it
//! rejects or a limit stops never settles `proved`, whatever its evidence's
//! replay gives.
//!
//! The replay of an outcome's evidence (FR-170) is supplied to the map by
//! the caller. Unless a test says otherwise, these tests supply a reproduced
//! result against the world's own package, so every certificate rejection
//! below is the certificate's own.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::simulation::model::ModelSimState;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::{DigestDomain, DigestRecord};
use qsl_replay::{
    settle_state_graph, CertificateLocus, CertificateRule, Certification, DisagreementCause,
    Execution, GraphReplayResult, GraphReplaySettlement, GraphWitness, IncompleteCause,
    InconclusiveCause, ModelCheckLimit, ModelCheckLimits, ModelPath, OpenCause, ProofBasis,
    ReachedLimit, ResultLabel, RunEnd, SettlementBasis, StateGraphCertificate,
    StateGraphInstanceOutcome, StateGraphMethod, StateGraphOutcome, StateGraphRecord,
    TerminalValue, Truth, Verdict, WitnessSource, DEFAULT_WITNESS_SEED, EXPLICIT_STATE,
};
use qsl_semantics::library::PackageId;
use quire_exact::{Cancel, CancelCause};
use quire_semantic_value::catalog::Category;

use crate::counter::UNLIMITED;
use crate::graph_world::{
    config_version, config_world, config_world_over, game_world, job_world, World, CAN_STILL_WIN,
    CONFIG_CONTRACT, DIVIDES, REACHES_THREE, REACHES_TWO,
};
use crate::state_graph::{no_samples, reachable, run, AB};
use crate::state_graph_certificate::{config, ranks_of, state_where, with_ranks, GUARDED};

/// The method of a run under `limits` with no horizon and the default seed.
fn method(limits: ModelCheckLimits) -> StateGraphMethod {
    StateGraphMethod {
        limits,
        max_depth: None,
        seed: DEFAULT_WITNESS_SEED,
    }
}

/// How a test settles: the run's method and package, the cancellation
/// handle, and the replay result the caller hands the map.
struct Settling {
    method: StateGraphMethod,
    package_id: DigestRecord,
    cancel: Cancel,
    replay: GraphReplaySettlement,
    replayed: PackageId,
}

impl Settling {
    /// A run of `world`'s own package under `limits`, whose evidence
    /// replays as reproduced against that package.
    fn of(world: &World, limits: ModelCheckLimits) -> Self {
        Self {
            method: method(limits),
            package_id: world.package_id().record(),
            cancel: Cancel::new(),
            replay: GraphReplaySettlement::Reproduced,
            replayed: world.package_id(),
        }
    }
}

/// Settle `instance` of `claim` over `world` with universe `keys` as
/// `settling` states; `replays` counts the replays the map asked for.
fn settle_with(
    world: &World,
    keys: &[&str],
    claim: &str,
    instance: &StateGraphInstanceOutcome,
    settling: &Settling,
    replays: &mut u32,
) -> StateGraphRecord {
    settle_state_graph(
        instance,
        &settling.method,
        &settling.package_id,
        world.subject(keys, UNLIMITED),
        world.claim(claim),
        &settling.cancel,
        |_| {
            *replays += 1;
            Ok(GraphReplayResult {
                settlement: settling.replay.clone(),
                package_id: settling.replayed,
            })
        },
    )
}

/// Settle `instance` of `claim` over `world` with universe `keys` under
/// `limits`, against the world's own package, replaying its evidence as
/// reproduced; `replays` counts the replays the map asked for.
fn settle(
    world: &World,
    keys: &[&str],
    claim: &str,
    instance: &StateGraphInstanceOutcome,
    limits: ModelCheckLimits,
    replays: &mut u32,
) -> StateGraphRecord {
    settle_with(
        world,
        keys,
        claim,
        instance,
        &Settling::of(world, limits),
        replays,
    )
}

/// One stand-in witness per initial state, for a hand-built outcome whose
/// certificate is rejected before any witness is read.
fn stand_ins(initial: u32) -> Vec<GraphWitness> {
    (0..initial)
        .map(|index| GraphWitness {
            path: ModelPath {
                initial: index,
                steps: Vec::new(),
            },
            source: WitnessSource::Explored,
        })
        .collect()
}

/// The instance's outcome with its certificate replaced by `certificate`.
fn with_certificate(
    instance: &StateGraphInstanceOutcome,
    certificate: StateGraphCertificate,
) -> StateGraphInstanceOutcome {
    let outcome = match &instance.outcome {
        StateGraphOutcome::Witnessed { paths, .. } => StateGraphOutcome::Witnessed {
            paths: paths.clone(),
            certificate,
        },
        StateGraphOutcome::Holds { .. } => StateGraphOutcome::Holds { certificate },
        other => panic!("no certificate: {other:?}"),
    };
    StateGraphInstanceOutcome {
        outcome,
        ..instance.clone()
    }
}

/// Assert `record` settles `inconclusive`, `unsettled`,
/// `CertificateRejected{rule, at}`, never `proved`.
fn assert_rejected(record: &StateGraphRecord, rule: CertificateRule, at: DigestRecord) {
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule,
            at: CertificateLocus::ModelState(at),
        })
    );
    assert_eq!(
        (record.label, record.basis, record.execution, record.truth),
        (
            ResultLabel::Inconclusive,
            SettlementBasis::Unsettled,
            Execution::Completed,
            Truth::Pending
        )
    );
    assert_eq!(record.value.category(), Category::Inconclusive);
}

/// FR-169-AC-9 (settlement half): `ReachesTwo` for `c = a` under default
/// limits returns `Witnessed`; once `check_state_graph` accepts its
/// certificate and its witness replays, it settles `proved`,
/// `decisive-witness`, `Proved{Witness{sources}, Certified}`, success, each
/// source the sampled walk of its initial state, written `witnessed` with
/// each walk's seed and trace index, and its record names exploration,
/// the seed and the walks drawn. §7.2's `CanStillWin` `from (x.phase !=
/// Lost)` and §7.3's sequenced `InOneWay` settle `proved`, `closed-scope`,
/// `Proved{Exhaustive, Certified}` with nothing replayed.
#[trace("TC-643", "FR-169-AC-9")]
#[test]
fn accepted_certificates_settle_proved_certified() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let limits = ModelCheckLimits::default();
    let result = run(world.request(AB, vec![world.item("ReachesTwo")], limits));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Witnessed { paths, .. } = &instance.outcome else {
        panic!("witnessed: {:?}", instance.outcome);
    };
    let sampled: Vec<WitnessSource> = instance
        .sampled
        .iter()
        .map(|witness| witness.as_ref().expect("a sampled witness").source.clone())
        .collect();
    assert_eq!(sampled.len(), 1);
    assert_eq!(
        paths
            .iter()
            .map(|witness| witness.source.clone())
            .collect::<Vec<_>>(),
        sampled
    );
    let mut replays = 0;
    let record = settle(&world, AB, "ReachesTwo", instance, limits, &mut replays);
    assert_eq!(replays, 1, "the witness is replayed once");
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::Witness {
                sources: sampled.clone(),
            },
            certification: Certification::Certified,
        }
    );
    assert_eq!(
        (record.label, record.basis, record.execution, record.truth),
        (
            ResultLabel::Proved,
            SettlementBasis::DecisiveWitness,
            Execution::Completed,
            Truth::Satisfied
        )
    );
    assert_eq!(record.value.category(), Category::Success);
    let WitnessSource::Sampled(provenance) = &sampled[0] else {
        panic!("sampled: {sampled:?}");
    };
    let TerminalValue::Proved { basis, .. } = &record.value else {
        panic!("proved");
    };
    assert_eq!(
        serde_json::to_value(basis).expect("serializes"),
        serde_json::json!({
            "type": "witnessed",
            "sources": [{
                "type": "sampled",
                "seed": provenance.seed.to_string(),
                "trace_index": provenance.trace.to_string(),
            }],
        })
    );
    assert_eq!(record.method, EXPLICIT_STATE);
    assert_eq!(
        (record.seed, record.witness_samples, record.max_depth),
        (DEFAULT_WITNESS_SEED, limits.witness_samples, None)
    );
    assert_eq!(record.limits, limits.used());

    let exhaustive = TerminalValue::Proved {
        basis: ProofBasis::Exhaustive,
        certification: Certification::Certified,
    };
    let game = game_world(false, false, GUARDED);
    let result = run(game.request(&["g"], vec![game.item("Guarded")], no_samples()));
    let mut replays = 0;
    let record = settle(
        &game,
        &["g"],
        "Guarded",
        &result.items[0][0],
        no_samples(),
        &mut replays,
    );
    assert_eq!(record.value, exhaustive);
    assert_eq!(
        (record.label, record.basis),
        (ResultLabel::Proved, SettlementBasis::ClosedScope)
    );

    let job = job_world(true, false);
    let result = run(job.request(&["j"], vec![job.item("InOneWay")], no_samples()));
    let record = settle(
        &job,
        &["j"],
        "InOneWay",
        &result.items[0][0],
        no_samples(),
        &mut replays,
    );
    assert_eq!(record.value, exhaustive);
    assert_eq!(record.basis, SettlementBasis::ClosedScope);
    assert_eq!(replays, 0, "a `Holds` outcome carries no evidence");
}

/// FR-169-AC-10 (settlement half): AC-9's `ReachesTwo` outcome with its
/// certificate altered settles `inconclusive`, `CertificateRejected{rule,
/// at}`, never `proved`, and its witness is never replayed: `(2, 2)`
/// removed (`SuccessorMissing` at the first of `(1, 2)` and `(2, 1)`), the
/// rank of `(1, 0)` changed to 2 (`RankBroken` at `(0, 0)`), `(0, 0)`
/// unranked (`RankMissing` at `(0, 0)`); with `max_states` 2 it settles
/// `failed`, `resource-incomplete`, `Incomplete(LimitReached)`, naming
/// `max_states` and 2.
#[trace("TC-644", "FR-169-AC-10")]
#[test]
fn altered_rank_certificates_never_settle_proved() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let result = run(world.request(AB, vec![world.item("ReachesTwo")], no_samples()));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Witnessed { certificate, .. } = &instance.outcome else {
        panic!("witnessed: {:?}", instance.outcome);
    };
    let ranks = ranks_of(certificate);
    let mut replays = 0;
    let mut settled = |certificate: StateGraphCertificate, limits| {
        settle(
            &world,
            AB,
            "ReachesTwo",
            &with_certificate(instance, certificate),
            limits,
            &mut replays,
        )
    };

    let removed = config(&world, 2, 2);
    let closure = certificate
        .closure
        .iter()
        .copied()
        .filter(|state| *state != removed)
        .collect();
    assert_rejected(
        &settled(with_ranks(closure, &ranks), no_samples()),
        CertificateRule::SuccessorMissing,
        config(&world, 1, 2).min(config(&world, 2, 1)),
    );

    let mut changed = ranks.clone();
    changed.insert(config(&world, 1, 0), 2);
    assert_rejected(
        &settled(
            with_ranks(certificate.closure.clone(), &changed),
            no_samples(),
        ),
        CertificateRule::RankBroken,
        config(&world, 0, 0),
    );

    let mut unranked = ranks.clone();
    unranked.remove(&config(&world, 0, 0));
    assert_rejected(
        &settled(
            with_ranks(certificate.closure.clone(), &unranked),
            no_samples(),
        ),
        CertificateRule::RankMissing,
        config(&world, 0, 0),
    );

    let mut limits = no_samples();
    limits.limits.max_states = 2;
    let stopped = settled(certificate.clone(), limits);
    assert_eq!(
        stopped.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 2,
        })
    );
    assert_eq!(
        (
            stopped.label,
            stopped.basis,
            stopped.execution,
            stopped.truth
        ),
        (
            ResultLabel::Failed,
            SettlementBasis::Unavailable,
            Execution::ResourceIncomplete,
            Truth::Unavailable
        )
    );
    assert_eq!(stopped.value.category(), Category::Incomplete);
    let reached = stopped.limit_reached.expect("the record names its limit");
    assert_eq!((reached.limit.setting(), reached.value), ("max_states", 2));
    assert_eq!(replays, 0, "no witness of a rejected certificate replays");
}

/// FR-169-AC-10 (settlement half): a hand-built `Witnessed` outcome for
/// FR-168-AC-5's `Divides` whose certificate lists all 9 states settles
/// `CertificateRejected{PredicateUndefined, at}` at the first, in
/// certificate order, of the states where `va = 2`; §7.3's `InOneWay`
/// over the unsequenced job with every count 1 settles
/// `CertificateRejected{CountBroken, at}` at `000`, and the sequenced
/// certificate with its order reversed `CertificateRejected{OrderBroken,
/// at}` at the first node other than `to`. None settles `proved`.
#[trace("TC-644", "FR-169-AC-10")]
#[test]
fn undefined_and_altered_count_certificates_never_settle_proved() {
    let world = config_world(DIVIDES, &[(0, 0)]);
    let states = reachable(&world, AB).0;
    let va = |state: &ModelSimState| world.value(state, "a", "versionNumber");
    let ranks: BTreeMap<DigestRecord, u32> = states
        .iter()
        .map(|(digest, state)| {
            (
                *digest,
                [1, 0, 2][usize::try_from(va(state)).expect("fits")],
            )
        })
        .collect();
    let first_undefined = states
        .iter()
        .find(|(_, state)| va(state) == 2)
        .map(|(digest, _)| *digest)
        .expect("a state with va = 2");
    let hand_built = StateGraphInstanceOutcome {
        over: Some(world.reference("a")),
        outcome: StateGraphOutcome::Witnessed {
            paths: stand_ins(1),
            certificate: with_ranks(states.keys().copied().collect(), &ranks),
        },
        sampled: Vec::new(),
        walks_stopped: 0,
    };
    let mut replays = 0;
    assert_rejected(
        &settle(
            &world,
            AB,
            "Divides",
            &hand_built,
            no_samples(),
            &mut replays,
        ),
        CertificateRule::PredicateUndefined,
        first_undefined,
    );

    let unsequenced = job_world(false, false);
    let flags = |state: &ModelSimState| {
        (
            unsequenced.value(state, "j", "a"),
            unsequenced.value(state, "j", "b"),
            unsequenced.value(state, "j", "done"),
        )
    };
    let states = reachable(&unsequenced, &["j"]).0;
    let order = |state: &ModelSimState| match flags(state) {
        (1, 1, 1) => 0,
        (1, 1, 0) => 1,
        (1, 0, 0) => 2,
        (0, 1, 0) => 3,
        (0, 0, 0) => 4,
        other => panic!("unreachable {other:?}"),
    };
    let all_one = StateGraphInstanceOutcome {
        over: Some(unsequenced.reference("j")),
        outcome: StateGraphOutcome::Holds {
            certificate: StateGraphCertificate {
                closure: states.keys().copied().collect(),
                ranks: Vec::new(),
                counts: states
                    .values()
                    .enumerate()
                    .map(|(index, state)| (u64::try_from(index).expect("fits"), 1, order(state)))
                    .collect(),
            },
        },
        sampled: Vec::new(),
        walks_stopped: 0,
    };
    assert_rejected(
        &settle(
            &unsequenced,
            &["j"],
            "InOneWay",
            &all_one,
            no_samples(),
            &mut replays,
        ),
        CertificateRule::CountBroken,
        state_where(&unsequenced, &["j"], |state| flags(state) == (0, 0, 0)),
    );

    let sequenced = job_world(true, false);
    let result = run(sequenced.request(&["j"], vec![sequenced.item("InOneWay")], no_samples()));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Holds { certificate } = &instance.outcome else {
        panic!("holds: {:?}", instance.outcome);
    };
    let last = certificate
        .counts
        .iter()
        .map(|(_, _, order)| *order)
        .max()
        .expect("counts");
    let mut reversed = certificate.clone();
    for entry in &mut reversed.counts {
        entry.2 = last - entry.2;
    }
    let first_not_to = reachable(&sequenced, &["j"])
        .0
        .into_iter()
        .find(|(_, state)| sequenced.value(state, "j", "done") == 0)
        .map(|(digest, _)| digest)
        .expect("a node other than `to`");
    assert_rejected(
        &settle(
            &sequenced,
            &["j"],
            "InOneWay",
            &with_certificate(instance, reversed),
            no_samples(),
            &mut replays,
        ),
        CertificateRule::OrderBroken,
        first_not_to,
    );
    assert_eq!(replays, 0);
}

/// FR-169-AC-11 (settlement half): §7.1's subject with universe `{a}`,
/// initial `va = 0` and TC-592's guard, with a hand-built `Witnessed`
/// certificate of its 3 states ranked 2, 1 and 0 for `ReachesTwo`, settles
/// `inconclusive`, `CertificateRejected{BadState, at}` at `va = 2`, never
/// `proved`.
#[trace("TC-644", "FR-169-AC-11")]
#[test]
fn a_bad_state_certificate_settles_certificate_rejected() {
    let world = config_world_over(
        "pre Guard using v on Config::ConfigVersion::attemptUpdate { \
         2 div (2 - self.versionNumber) >= 1 }\n",
        REACHES_TWO,
        &["a"],
        &[vec![0]],
    );
    let states = reachable(&world, &["a"]).0;
    let va = |value| {
        state_where(&world, &["a"], |state| {
            world.value(state, "a", "versionNumber") == value
        })
    };
    let ranks = BTreeMap::from([(va(0), 2), (va(1), 1), (va(2), 0)]);
    let instance = StateGraphInstanceOutcome {
        over: Some(world.reference("a")),
        outcome: StateGraphOutcome::Witnessed {
            paths: stand_ins(1),
            certificate: with_ranks(states.keys().copied().collect(), &ranks),
        },
        sampled: Vec::new(),
        walks_stopped: 0,
    };
    let mut replays = 0;
    assert_rejected(
        &settle(
            &world,
            &["a"],
            "ReachesTwo",
            &instance,
            no_samples(),
            &mut replays,
        ),
        CertificateRule::BadState,
        va(2),
    );
    assert_eq!(replays, 0);
}

/// FR-282's package binding of a state-graph certificate and its evidence:
/// a `Witnessed` or `Holds` outcome of a run that checked another package
/// settles `inconclusive`, `ReplayRefused(stale_dependency)` with nothing
/// checked or replayed; evidence of a `Witnessed`, `WitnessedUnchecked`,
/// `Trapped` or `PathPair` outcome replayed against another package settles
/// the same, never `proved` or `refuted`; and a check whose package
/// emission a cancelled handle stops settles `Incomplete(Cancelled)`.
#[trace("TC-644")]
#[test]
fn a_certificate_or_evidence_of_another_package_never_settles() {
    let stale =
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::StaleDependency));
    let elsewhere = DigestRecord::mint(DigestDomain::PackageSemanticV2, [9; 32]);
    let other = job_world(true, false).package_id();

    let world = config_world(&format!("{REACHES_TWO}{REACHES_THREE}"), &[(0, 0)]);
    let witnessed = run(world.request(AB, vec![world.item("ReachesTwo")], no_samples()));
    let witnessed = &witnessed.items[0][0];
    let trapped = run(world.request(AB, vec![world.item("ReachesThree")], no_samples()));
    let trapped = &trapped.items[0][0];
    assert!(matches!(trapped.outcome, StateGraphOutcome::Trapped { .. }));
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 2;
    let unchecked = run(world.request(AB, vec![world.item("ReachesTwo")], limits));
    let unchecked = &unchecked.items[0][0];
    assert!(matches!(
        unchecked.outcome,
        StateGraphOutcome::WitnessedUnchecked { .. }
    ));
    let game = game_world(false, false, GUARDED);
    let holds = run(game.request(&["g"], vec![game.item("Guarded")], no_samples()));
    let holds = &holds.items[0][0];
    let job = job_world(false, false);
    let pair = run(job.request(&["j"], vec![job.item("InOneWay")], no_samples()));
    let pair = &pair.items[0][0];
    assert!(matches!(pair.outcome, StateGraphOutcome::PathPair { .. }));

    let mut replays = 0;
    let run_elsewhere = |world: &World, limits| Settling {
        package_id: elsewhere,
        ..Settling::of(world, limits)
    };
    for (world, keys, claim, instance) in [
        (&world, AB, "ReachesTwo", witnessed),
        (&game, &["g"][..], "Guarded", holds),
    ] {
        let record = settle_with(
            world,
            keys,
            claim,
            instance,
            &run_elsewhere(world, no_samples()),
            &mut replays,
        );
        assert_eq!(record.value, stale, "{claim}");
    }
    assert_eq!(replays, 0, "a stale certificate is not checked or replayed");

    let replayed_elsewhere = |world: &World, limits| Settling {
        replayed: other,
        ..Settling::of(world, limits)
    };
    for (world, keys, claim, instance, limits) in [
        (&world, AB, "ReachesTwo", witnessed, no_samples()),
        (&world, AB, "ReachesTwo", unchecked, limits),
        (&world, AB, "ReachesThree", trapped, no_samples()),
        (&job, &["j"][..], "InOneWay", pair, no_samples()),
    ] {
        let record = settle_with(
            world,
            keys,
            claim,
            instance,
            &replayed_elsewhere(world, limits),
            &mut replays,
        );
        assert_eq!(record.value, stale, "{claim}: {:?}", instance.outcome);
        assert_eq!((record.end, record.limit_reached), (None, None));
    }
    assert_eq!(replays, 4);

    let cancelled = Settling::of(&game, no_samples());
    cancelled.cancel.cancel(CancelCause::Requested);
    let record = settle_with(&game, &["g"], "Guarded", holds, &cancelled, &mut replays);
    assert_eq!(
        record.value,
        TerminalValue::Incomplete(IncompleteCause::Cancelled {
            source: CancelCause::Requested,
        })
    );
    assert_eq!(record.execution, Execution::ResourceIncomplete);
}

/// FR-169-AC-8: FR-168-AC-8's `WitnessedUnchecked` outcome (`ReachesTwo`,
/// default `witness_samples`, `max_states` 2) settles `inconclusive`,
/// `unsettled`, `Inconclusive(WellDefinednessUnchecked{sources})` with its
/// `Sampled` sources once they replay, naming `max_states` and 2 and the
/// run's end, never `proved`; its `NoDecision` variant with
/// `witness_samples` 0 settles `failed`, `resource-incomplete`,
/// `Incomplete(LimitReached{MaxStates, 2})`, replaying nothing.
#[trace("TC-614", "FR-169-AC-8")]
#[test]
fn a_stopped_run_with_witnesses_settles_well_definedness_unchecked() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let reached = ReachedLimit {
        limit: ModelCheckLimit::MaxStates,
        value: 2,
    };
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 2;
    let result = run(world.request(AB, vec![world.item("ReachesTwo")], limits));
    let instance = &result.items[0][0];
    let StateGraphOutcome::WitnessedUnchecked { paths, end, .. } = &instance.outcome else {
        panic!("unchecked: {:?}", instance.outcome);
    };
    let sources: Vec<WitnessSource> = paths.iter().map(|path| path.source.clone()).collect();
    assert!(matches!(sources[..], [WitnessSource::Sampled(_)]));
    let mut replays = 0;
    let record = settle(&world, AB, "ReachesTwo", instance, limits, &mut replays);
    assert_eq!(replays, 1);
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::WellDefinednessUnchecked { sources })
    );
    assert_eq!(
        (record.label, record.basis, record.execution, record.truth),
        (
            ResultLabel::Inconclusive,
            SettlementBasis::Unsettled,
            Execution::Completed,
            Truth::Pending
        )
    );
    assert_eq!(record.limit_reached, Some(reached));
    assert_eq!(record.end.as_ref(), Some(end));

    limits.witness_samples = 0;
    let result = run(world.request(AB, vec![world.item("ReachesTwo")], limits));
    let instance = &result.items[0][0];
    assert!(
        matches!(instance.outcome, StateGraphOutcome::NoDecision { .. }),
        "{:?}",
        instance.outcome
    );
    let record = settle(&world, AB, "ReachesTwo", instance, limits, &mut replays);
    assert_eq!(replays, 1, "`NoDecision` carries no evidence");
    assert_eq!(
        record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 2,
        })
    );
    assert_eq!(
        (record.label, record.execution),
        (ResultLabel::Failed, Execution::ResourceIncomplete)
    );
    assert_eq!(record.limit_reached, Some(reached));
}

/// Assert `record` settles `refuted` on `basis`, category violation.
fn assert_refuted(record: &StateGraphRecord, basis: SettlementBasis) {
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(
        (record.label, record.basis, record.execution, record.truth),
        (
            ResultLabel::Refuted,
            basis,
            Execution::Completed,
            Truth::Violated
        )
    );
    assert_eq!(record.value.category(), Category::Violation);
}

/// FR-169-AC-1, AC-2: `ReachesTwo` with `witness_samples` 0 settles
/// `proved`, `decisive-witness`, `Proved{Witness{[Explored]}, Certified}`,
/// success, its record naming 0 walks; `ReachesThree` returns `Trapped` and
/// settles `refuted`, `closed-scope`, violation (V-10), only after its trap
/// replays.
#[trace("TC-594", "FR-169-AC-1", "FR-169-AC-2")]
#[test]
fn an_explored_witness_proves_and_a_replayed_trap_refutes() {
    let world = config_world(&format!("{REACHES_TWO}{REACHES_THREE}"), &[(0, 0)]);
    let result = run(world.request(AB, vec![world.item("ReachesTwo")], no_samples()));
    let mut replays = 0;
    let record = settle(
        &world,
        AB,
        "ReachesTwo",
        &result.items[0][0],
        no_samples(),
        &mut replays,
    );
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::Witness {
                sources: vec![WitnessSource::Explored],
            },
            certification: Certification::Certified,
        }
    );
    assert_eq!(record.basis, SettlementBasis::DecisiveWitness);
    assert_eq!(record.value.category(), Category::Success);
    assert_eq!((record.witness_samples, record.walks_stopped), (0, 0));

    let result = run(world.request(AB, vec![world.item("ReachesThree")], no_samples()));
    let trapped = &result.items[0][0];
    assert!(matches!(trapped.outcome, StateGraphOutcome::Trapped { .. }));
    let record = settle(
        &world,
        AB,
        "ReachesThree",
        trapped,
        no_samples(),
        &mut replays,
    );
    assert_refuted(&record, SettlementBasis::ClosedScope);
    assert_eq!(replays, 2, "the witness and the trap each replay once");
}

/// FR-169-AC-1, AC-3: §7.2's `CanStillWin` returns `Trapped` at `Lost` and
/// settles `refuted`, `closed-scope` (V-10); §7.3's `InOneWay` over the
/// unsequenced job returns `PathPair` and settles `refuted`,
/// `decisive-counterexample` (V-4), each after its evidence replays.
#[trace("TC-594", "FR-169-AC-1", "FR-169-AC-3")]
#[test]
fn a_replayed_trap_and_path_pair_refute() {
    let game = game_world(false, false, CAN_STILL_WIN);
    let result = run(game.request(&["g"], vec![game.item("CanStillWin")], no_samples()));
    let trapped = &result.items[0][0];
    assert!(matches!(trapped.outcome, StateGraphOutcome::Trapped { .. }));
    let mut replays = 0;
    assert_refuted(
        &settle(
            &game,
            &["g"],
            "CanStillWin",
            trapped,
            no_samples(),
            &mut replays,
        ),
        SettlementBasis::ClosedScope,
    );

    let job = job_world(false, false);
    let result = run(job.request(&["j"], vec![job.item("InOneWay")], no_samples()));
    let pair = &result.items[0][0];
    assert!(matches!(pair.outcome, StateGraphOutcome::PathPair { .. }));
    assert_refuted(
        &settle(&job, &["j"], "InOneWay", pair, no_samples(), &mut replays),
        SettlementBasis::DecisiveCounterexample,
    );
    assert_eq!(replays, 2);
}

/// FR-169-AC-1, AC-4: FR-168-AC-4's game under horizon 3: the trap at
/// `Lost` settles `refuted` (V-10), and `Fifty`'s `NoDecision` settles
/// `inconclusive`, `BoundReached{depth: 3}`, execution `completed`, truth
/// `pending`, the record stating horizon 3 as a method parameter and no
/// limit reached. A run `max_states` stopped settles `incomplete`,
/// `LimitReached`, naming `max_states`; a node whose guard has no value,
/// with no decisive evidence, settles `inconclusive`, `UndecidedSuccessor`;
/// a subject with no initial state settles `inconclusive`,
/// `NoInitialState`.
#[trace("TC-594", "FR-169-AC-1", "FR-169-AC-4")]
#[test]
fn horizon_limit_undecided_and_empty_runs_settle_their_rows() {
    let body = format!(
        "{CAN_STILL_WIN}possible Fifty using v over (x: G::Game) {{ x.phase = 3 and x.n = 50 }}\n"
    );
    let game = game_world(true, false, &body);
    let mut request = game.request(
        &["g"],
        vec![game.item("CanStillWin"), game.item("Fifty")],
        no_samples(),
    );
    request.max_depth = Some(3);
    let result = run(request);
    let horizon = Settling {
        method: StateGraphMethod {
            max_depth: Some(3),
            ..method(no_samples())
        },
        ..Settling::of(&game, no_samples())
    };
    let mut replays = 0;
    let trap = settle_with(
        &game,
        &["g"],
        "CanStillWin",
        &result.items[0][0],
        &horizon,
        &mut replays,
    );
    assert_refuted(&trap, SettlementBasis::ClosedScope);
    let bound = settle_with(
        &game,
        &["g"],
        "Fifty",
        &result.items[1][0],
        &horizon,
        &mut replays,
    );
    assert_eq!(
        bound.value,
        TerminalValue::Inconclusive(InconclusiveCause::BoundReached { depth: 3 })
    );
    assert_eq!(
        (bound.label, bound.basis, bound.execution, bound.truth),
        (
            ResultLabel::Inconclusive,
            SettlementBasis::Unsettled,
            Execution::Completed,
            Truth::Pending
        )
    );
    assert_eq!((bound.max_depth, bound.limit_reached), (Some(3), None));
    assert_eq!(bound.limits, no_samples().used());

    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let mut limits = no_samples();
    limits.limits.max_states = 2;
    let result = run(world.request(AB, vec![world.item("ReachesTwo")], limits));
    let stopped = settle(
        &world,
        AB,
        "ReachesTwo",
        &result.items[0][0],
        limits,
        &mut replays,
    );
    assert_eq!(
        stopped.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 2,
        })
    );
    assert_eq!(
        stopped.limit_reached.map(|reached| reached.limit.setting()),
        Some("max_states")
    );

    let guarded = config_world_over(
        "pre Guard using v on Config::ConfigVersion::attemptUpdate { \
         2 div (2 - self.versionNumber) >= 1 }\n",
        REACHES_THREE,
        &["a"],
        &[vec![0]],
    );
    let result = run(guarded.request(&["a"], vec![guarded.item("ReachesThree")], no_samples()));
    let undecided = &result.items[0][0];
    assert!(
        matches!(&undecided.outcome, StateGraphOutcome::NoDecision { end: RunEnd::Completed, open }
            if open.contains(&OpenCause::UndecidedSuccessor)),
        "{:?}",
        undecided.outcome
    );
    assert_eq!(
        settle(
            &guarded,
            &["a"],
            "ReachesThree",
            undecided,
            no_samples(),
            &mut replays
        )
        .value,
        TerminalValue::Inconclusive(InconclusiveCause::UndecidedSuccessor)
    );

    let empty = World::new(
        config_version(),
        &format!("{CONFIG_CONTRACT}{REACHES_TWO}"),
        &[],
    );
    let result = run(empty.request(AB, vec![empty.item("ReachesTwo")], no_samples()));
    let none = &result.items[0][0];
    assert_eq!(none.outcome, StateGraphOutcome::NoInitialState);
    assert_eq!(
        settle(&empty, AB, "ReachesTwo", none, no_samples(), &mut replays).value,
        TerminalValue::Inconclusive(InconclusiveCause::NoInitialState)
    );
    assert_eq!(replays, 1, "only the trap carries evidence");
}

/// FR-169-AC-1, AC-5: a `Witnessed` outcome whose replay settles
/// `inconclusive` settles `inconclusive`, `ReplayParity`, never `proved`; a
/// `Trapped` outcome whose replay a limit stops settles `incomplete`,
/// `LimitReached`, naming the limit and its value.
#[trace("TC-594", "FR-169-AC-1", "FR-169-AC-5")]
#[test]
fn a_disagreeing_or_stopped_replay_never_settles_decisively() {
    let world = config_world(&format!("{REACHES_TWO}{REACHES_THREE}"), &[(0, 0)]);
    let result = run(world.request(AB, vec![world.item("ReachesTwo")], no_samples()));
    let disagrees = DisagreementCause::Verdicts {
        proved: Verdict::from_category(Category::Success),
        replayed: Verdict::from_category(Category::Violation),
    };
    let mut replays = 0;
    let record = settle_with(
        &world,
        AB,
        "ReachesTwo",
        &result.items[0][0],
        &Settling {
            replay: GraphReplaySettlement::Inconclusive(disagrees.clone()),
            ..Settling::of(&world, no_samples())
        },
        &mut replays,
    );
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(disagrees))
    );
    assert_eq!(record.label, ResultLabel::Inconclusive);

    let reached = ReachedLimit {
        limit: ModelCheckLimit::MaxStates,
        value: 4,
    };
    let result = run(world.request(AB, vec![world.item("ReachesThree")], no_samples()));
    let record = settle_with(
        &world,
        AB,
        "ReachesThree",
        &result.items[0][0],
        &Settling {
            replay: GraphReplaySettlement::Stopped(reached),
            ..Settling::of(&world, no_samples())
        },
        &mut replays,
    );
    assert_eq!(
        record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 4,
        })
    );
    assert_eq!(
        (record.label, record.execution, record.limit_reached),
        (
            ResultLabel::Failed,
            Execution::ResourceIncomplete,
            Some(reached)
        )
    );
    assert_eq!(replays, 2);
}

/// FR-169-AC-6: FR-168-AC-5's `Undefined` outcome settles `refuted`,
/// `decisive-counterexample`, violation, after its replay reproduces it,
/// with cause `UndefinedEvaluation` naming the node `(2, 0)` and division
/// by zero.
#[trace("TC-612", "FR-169-AC-6")]
#[test]
fn an_undefined_target_settles_refuted_with_its_site() {
    let world = config_world(DIVIDES, &[(0, 0)]);
    let result = run(world.request(AB, vec![world.item("Divides")], no_samples()));
    let mut replays = 0;
    let record = settle(
        &world,
        AB,
        "Divides",
        &result.items[0][0],
        no_samples(),
        &mut replays,
    );
    assert_eq!(replays, 1);
    assert_refuted(&record, SettlementBasis::DecisiveCounterexample);
    let undefined = record.undefined.expect("the record names the site");
    assert_eq!(undefined.state_key, config(&world, 2, 0));
    assert_eq!(
        undefined.cause,
        UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
}

/// FR-169-AC-7: FR-168-AC-7's `Undefined` outcome, found although a sampled
/// witness existed, settles `refuted`, `decisive-counterexample`, with its
/// `UndefinedEvaluation`; `ReachesTwo` under the same default limits
/// settles `proved`, `decisive-witness`, with `Sampled` sources, and its
/// record names the explicit-state method of a completed exploration.
#[trace("TC-613", "FR-169-AC-7")]
#[test]
fn a_sampled_witness_never_hides_an_undefined_refutation() {
    let world = config_world(&format!("{REACHES_TWO}{DIVIDES}"), &[(0, 0)]);
    let limits = ModelCheckLimits::default();
    let result = run(world.request(AB, vec![world.item("Divides")], limits));
    let undefined = &result.items[0][0];
    assert!(matches!(undefined.sampled.as_slice(), [Some(_)]));
    let mut replays = 0;
    let record = settle(&world, AB, "Divides", undefined, limits, &mut replays);
    assert_refuted(&record, SettlementBasis::DecisiveCounterexample);
    assert!(record.undefined.is_some());

    let result = run(world.request(AB, vec![world.item("ReachesTwo")], limits));
    let record = settle(
        &world,
        AB,
        "ReachesTwo",
        &result.items[0][0],
        limits,
        &mut replays,
    );
    let TerminalValue::Proved {
        basis: ProofBasis::Witness { sources },
        certification: Certification::Certified,
    } = &record.value
    else {
        panic!("proved: {:?}", record.value);
    };
    assert!(matches!(sources[..], [WitnessSource::Sampled(_)]));
    assert_eq!(record.basis, SettlementBasis::DecisiveWitness);
    assert_eq!((record.method, record.end), (EXPLICIT_STATE, None));
}

/// FR-168's one witness per initial state, guarded by the map: a
/// `Witnessed` outcome with no witness for its initial state never settles
/// `proved`, even with an accepted certificate; it settles `failed`,
/// replaying nothing.
#[trace("TC-643")]
#[test]
fn a_witnessed_outcome_missing_a_witness_never_settles_proved() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let result = run(world.request(AB, vec![world.item("ReachesTwo")], no_samples()));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Witnessed { certificate, .. } = &instance.outcome else {
        panic!("witnessed: {:?}", instance.outcome);
    };
    let empty = StateGraphInstanceOutcome {
        outcome: StateGraphOutcome::Witnessed {
            paths: Vec::new(),
            certificate: certificate.clone(),
        },
        ..instance.clone()
    };
    let mut replays = 0;
    let record = settle(&world, AB, "ReachesTwo", &empty, no_samples(), &mut replays);
    assert_eq!(record.value, TerminalValue::Failed);
    assert_eq!(replays, 0);
}
