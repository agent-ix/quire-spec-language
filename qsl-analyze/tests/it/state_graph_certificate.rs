// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-169's state-graph certificate (ADR-022 GC-1 to GC-4): the certificate
//! EN-1 returns with `Witnessed` and `Holds`, and the core checker
//! `qsl_replay::check_state_graph`, which accepts the engine's certificates
//! of ADR-022 §7's true claims and rejects altered or false ones.
//!
//! Every expected closure, rank, count and locus is computed here from a
//! breadth-first enumeration of the subject's `ModelSystem`
//! (`crate::state_graph::reachable`) and each world's arithmetic, not read
//! from the engine.
//!
//! Not covered here: the terminal record each check settles to (FR-169's
//! map) and the reduced-run `Uncertified` proof.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_eval::simulation::model::ModelSimState;
use qsl_eval::simulation::TransitionSystem;
use qsl_foundation::digest::DigestRecord;
use qsl_replay::{
    check_state_graph, CertificateLocus, CertificateRejection, CertificateRule, IncompleteCause,
    ModelCheckLimit, ModelCheckLimits, StateGraphCertificate, StateGraphCertificateRequest,
    StateGraphOutcome, TemporalCertificateCheck,
};
use quire_exact::{Cancel, ObjectReference};

use crate::counter::UNLIMITED;
use crate::graph_world::{config_world, game_world, job_world, World, DIVIDES, REACHES_TWO};
use crate::state_graph::{no_samples, reachable, run, system, AB};

/// Check `certificate` for `claim`'s instance bound to `over` over `world`
/// with universe `keys`, under `limits`.
fn check(
    world: &World,
    keys: &[&str],
    claim: &str,
    over: Option<ObjectReference>,
    certificate: &StateGraphCertificate,
    limits: &ModelCheckLimits,
) -> TemporalCertificateCheck {
    let request = StateGraphCertificateRequest {
        subject: world.subject(keys, UNLIMITED),
        claim: world.claim(claim),
        over,
    };
    check_state_graph(&request, certificate, limits, &Cancel::new()).expect("the request admits")
}

fn rejected(rule: CertificateRule, at: DigestRecord) -> TemporalCertificateCheck {
    TemporalCertificateCheck::Rejected(CertificateRejection {
        rule,
        at: CertificateLocus::ModelState(at),
    })
}

/// The digest of the one reachable state where `select` holds.
fn state_where(
    world: &World,
    keys: &[&str],
    select: impl Fn(&ModelSimState) -> bool,
) -> DigestRecord {
    let found: Vec<DigestRecord> = reachable(world, keys)
        .0
        .into_iter()
        .filter(|(_, state)| select(state))
        .map(|(digest, _)| digest)
        .collect();
    assert_eq!(found.len(), 1, "one state selected");
    found[0]
}

/// §7.1's state `(va, vb)`.
fn config(world: &World, va: i64, vb: i64) -> DigestRecord {
    state_where(world, AB, |state| {
        world.value(state, "a", "versionNumber") == va
            && world.value(state, "b", "versionNumber") == vb
    })
}

/// A certificate's ranks by state.
fn ranks_of(certificate: &StateGraphCertificate) -> BTreeMap<DigestRecord, u32> {
    certificate
        .ranks
        .iter()
        .map(|(index, d)| {
            let at = usize::try_from(*index).expect("fits");
            (certificate.closure[at], *d)
        })
        .collect()
}

/// The certificate over `closure` with `ranks`, re-indexed; a rank of a
/// state outside `closure` is dropped.
fn with_ranks(
    closure: Vec<DigestRecord>,
    ranks: &BTreeMap<DigestRecord, u32>,
) -> StateGraphCertificate {
    let ranks = closure
        .iter()
        .enumerate()
        .filter_map(|(index, state)| {
            ranks
                .get(state)
                .map(|d| (u64::try_from(index).expect("fits"), *d))
        })
        .collect();
    StateGraphCertificate {
        closure,
        ranks,
        counts: Vec::new(),
    }
}

/// `ReachesTwo`'s instance for `c = a` with phase 0 off: its binding and
/// the certificate of its `Witnessed`.
fn reaches_two(world: &World) -> (Option<ObjectReference>, StateGraphCertificate) {
    let result = run(world.request(AB, vec![world.item("ReachesTwo")], no_samples()));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Witnessed { certificate, .. } = &instance.outcome else {
        panic!("witnessed: {:?}", instance.outcome);
    };
    (instance.over.clone(), certificate.clone())
}

const GUARDED: &str =
    "always possible Guarded using v over (x: G::Game) from (x.phase != 2) { x.phase = 3 }\n";

/// FR-169-AC-9 (check half): `ReachesTwo` for `c = a` returns a certificate
/// whose closure is §7.1's 9 states and whose ranks are `d = 0` at `va = 2`,
/// 1 at `va = 1` and 2 at `va = 0`, and `check_state_graph` accepts it;
/// §7.2's `CanStillWin` `from (x.phase != Lost)` returns ranks `Won` 0,
/// `Mid` 1, `Start` 2 and none for `Lost`, accepted.
#[trace("TC-643", "FR-169-AC-9")]
#[test]
fn the_checker_accepts_the_engines_rank_certificates() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let (over, certificate) = reaches_two(&world);
    let states = reachable(&world, AB).0;
    assert_eq!(states.len(), 9);
    assert_eq!(
        certificate.closure,
        states.keys().copied().collect::<Vec<_>>()
    );
    let expected: BTreeMap<DigestRecord, u32> = states
        .iter()
        .map(|(digest, state)| {
            let d = match world.value(state, "a", "versionNumber") {
                2 => 0,
                1 => 1,
                0 => 2,
                other => panic!("va = {other}"),
            };
            (*digest, d)
        })
        .collect();
    assert_eq!(ranks_of(&certificate), expected);
    assert!(certificate.counts.is_empty());
    assert_eq!(
        check(
            &world,
            AB,
            "ReachesTwo",
            over,
            &certificate,
            &ModelCheckLimits::default()
        ),
        TemporalCertificateCheck::Accepted
    );

    let game = game_world(false, false, GUARDED);
    let result = run(game.request(&["g"], vec![game.item("Guarded")], no_samples()));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Holds { certificate } = &instance.outcome else {
        panic!("holds: {:?}", instance.outcome);
    };
    let phase = |value| {
        state_where(&game, &["g"], |state| {
            game.value(state, "g", "phase") == value
        })
    };
    // Start 0, Mid 1, Lost 2, Won 3.
    assert_eq!(
        ranks_of(certificate),
        BTreeMap::from([(phase(3), 0), (phase(1), 1), (phase(0), 2)])
    );
    assert_eq!(certificate.closure.len(), 4);
    assert_eq!(
        check(
            &game,
            &["g"],
            "Guarded",
            instance.over.clone(),
            certificate,
            &ModelCheckLimits::default()
        ),
        TemporalCertificateCheck::Accepted
    );
}

/// The successor digests of each reachable state of `world`, one per edge.
fn successors(world: &World, keys: &[&str]) -> BTreeMap<DigestRecord, Vec<DigestRecord>> {
    let system = system(world, keys);
    reachable(world, keys)
        .0
        .into_iter()
        .map(|(digest, state)| {
            let next = system
                .successors(&state)
                .expect("expands")
                .successors
                .into_iter()
                .map(|(_, next)| system.state_key(&next).expect("encodes").1)
                .collect();
            (digest, next)
        })
        .collect()
}

/// FR-169-AC-9 (check half): §7.3's sequenced `InOneWay` returns count 1 at
/// every node of `H` with a reverse topological order, accepted.
#[trace("TC-643", "FR-169-AC-9")]
#[test]
fn the_checker_accepts_the_engines_count_certificate() {
    let world = job_world(true, false);
    let result = run(world.request(&["j"], vec![world.item("InOneWay")], no_samples()));
    let instance = &result.items[0][0];
    let StateGraphOutcome::Holds { certificate } = &instance.outcome else {
        panic!("holds: {:?}", instance.outcome);
    };
    let graph = successors(&world, &["j"]);
    // Sequenced: 000 -> 100 -> 110 -> 111, every node reaching `done`.
    assert_eq!(graph.len(), 4);
    assert_eq!(
        certificate.closure,
        graph.keys().copied().collect::<Vec<_>>()
    );
    assert_eq!(certificate.counts.len(), 4);
    let order: BTreeMap<DigestRecord, u64> = certificate
        .counts
        .iter()
        .map(|(index, count, order)| {
            assert_eq!(*count, 1);
            (
                certificate.closure[usize::try_from(*index).expect("fits")],
                *order,
            )
        })
        .collect();
    let done = state_where(&world, &["j"], |state| world.value(state, "j", "done") == 1);
    for (source, targets) in &graph {
        if *source == done {
            continue;
        }
        for target in targets {
            assert!(order[target] < order[source], "reverse topological order");
        }
    }
    assert_eq!(
        check(
            &world,
            &["j"],
            "InOneWay",
            instance.over.clone(),
            certificate,
            &ModelCheckLimits::default()
        ),
        TemporalCertificateCheck::Accepted
    );
}

/// FR-169-AC-10 (check half): AC-9's `ReachesTwo` certificate with `(2, 2)`
/// removed is rejected `SuccessorMissing` at the first, in certificate
/// order, of the states that reach it; with the rank of `(1, 0)` changed to
/// 2 it is rejected `RankBroken` at `(0, 0)`, the one state left with no
/// successor ranked one less (`(1, 0)` itself still has `(1, 1)` at rank
/// 1); with `(0, 0)` unranked it is rejected `RankMissing` at `(0, 0)`; and
/// checked with `max_states` 2 it stops naming `max_states` and 2.
#[trace("TC-644", "FR-169-AC-10")]
#[test]
fn the_checker_rejects_altered_rank_certificates() {
    let world = config_world(REACHES_TWO, &[(0, 0)]);
    let (over, certificate) = reaches_two(&world);
    let ranks = ranks_of(&certificate);
    let checked = |certificate: &StateGraphCertificate, limits: &ModelCheckLimits| {
        check(&world, AB, "ReachesTwo", over.clone(), certificate, limits)
    };
    let defaults = ModelCheckLimits::default();

    let removed = config(&world, 2, 2);
    let closure: Vec<_> = certificate
        .closure
        .iter()
        .copied()
        .filter(|state| *state != removed)
        .collect();
    let first_reaching = config(&world, 1, 2).min(config(&world, 2, 1));
    assert_eq!(
        checked(&with_ranks(closure, &ranks), &defaults),
        rejected(CertificateRule::SuccessorMissing, first_reaching)
    );

    let mut changed = ranks.clone();
    changed.insert(config(&world, 1, 0), 2);
    assert_eq!(
        checked(
            &with_ranks(certificate.closure.clone(), &changed),
            &defaults
        ),
        rejected(CertificateRule::RankBroken, config(&world, 0, 0))
    );

    let mut unranked = ranks.clone();
    unranked.remove(&config(&world, 0, 0));
    assert_eq!(
        checked(
            &with_ranks(certificate.closure.clone(), &unranked),
            &defaults
        ),
        rejected(CertificateRule::RankMissing, config(&world, 0, 0))
    );

    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 2;
    let stopped = TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
        limit: ModelCheckLimit::MaxStates,
        value: 2,
    });
    assert_eq!(checked(&certificate, &limits), stopped);
    assert_eq!(checked(&certificate, &limits), stopped);
}

/// FR-169-AC-10 (check half): a hand-built certificate for FR-168-AC-5's
/// `Divides` over §7.1's 9 states, ranked as if the target held at `va = 1`,
/// is rejected `PredicateUndefined` at the first, in certificate order, of
/// the states where `va = 2`, where the target divides by zero; `(2, 0)` is
/// one of them.
#[trace("TC-644", "FR-169-AC-10")]
#[test]
fn the_checker_rejects_a_certificate_over_an_undefined_target() {
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
    let undefined: BTreeSet<DigestRecord> = states
        .iter()
        .filter(|(_, state)| va(state) == 2)
        .map(|(digest, _)| *digest)
        .collect();
    assert!(undefined.contains(&config(&world, 2, 0)));
    let certificate = with_ranks(states.keys().copied().collect(), &ranks);
    assert_eq!(
        check(
            &world,
            AB,
            "Divides",
            Some(world.reference("a")),
            &certificate,
            &ModelCheckLimits::default()
        ),
        rejected(
            CertificateRule::PredicateUndefined,
            *undefined.first().expect("a state with va = 2")
        )
    );
}

/// FR-169-AC-10 (check half): the `InOneWay` certificate over §7.3's
/// unsequenced job with every count 1 is rejected `CountBroken` at `000`,
/// the node with two paths; the sequenced certificate with its order
/// reversed is rejected `OrderBroken` at the first, in certificate order, of
/// its nodes other than `to`.
#[trace("TC-644", "FR-169-AC-10")]
#[test]
fn the_checker_rejects_altered_count_certificates() {
    let world = job_world(false, false);
    let flags = |state: &ModelSimState| {
        (
            world.value(state, "j", "a"),
            world.value(state, "j", "b"),
            world.value(state, "j", "done"),
        )
    };
    let states = reachable(&world, &["j"]).0;
    assert_eq!(states.len(), 5);
    let closure: Vec<DigestRecord> = states.keys().copied().collect();
    // A reverse topological order of `H`: `done` first, `000` last.
    let order = |state: &ModelSimState| match flags(state) {
        (1, 1, 1) => 0,
        (1, 1, 0) => 1,
        (1, 0, 0) => 2,
        (0, 1, 0) => 3,
        (0, 0, 0) => 4,
        other => panic!("unreachable {other:?}"),
    };
    let counts = states
        .values()
        .enumerate()
        .map(|(index, state)| (u64::try_from(index).expect("fits"), 1, order(state)))
        .collect();
    let all_one = StateGraphCertificate {
        closure,
        ranks: Vec::new(),
        counts,
    };
    let start = state_where(&world, &["j"], |state| flags(state) == (0, 0, 0));
    assert_eq!(
        check(
            &world,
            &["j"],
            "InOneWay",
            Some(world.reference("j")),
            &all_one,
            &ModelCheckLimits::default()
        ),
        rejected(CertificateRule::CountBroken, start)
    );

    let world = job_world(true, false);
    let result = run(world.request(&["j"], vec![world.item("InOneWay")], no_samples()));
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
    let first_not_to = reachable(&world, &["j"])
        .0
        .into_iter()
        .find(|(_, state)| world.value(state, "j", "done") == 0)
        .map(|(digest, _)| digest)
        .expect("a node other than `to`");
    assert_eq!(
        check(
            &world,
            &["j"],
            "InOneWay",
            instance.over.clone(),
            &reversed,
            &ModelCheckLimits::default()
        ),
        rejected(CertificateRule::OrderBroken, first_not_to)
    );
}
