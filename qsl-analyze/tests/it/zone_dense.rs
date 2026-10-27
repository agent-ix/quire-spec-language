// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-430's dense-time interactions on EN-6 (TC-1170): symmetry with
//! clocks, the reductions EN-6 does not apply, and strong fairness read
//! over time.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{admit, Reductions, SymmetryDeclaration};
use qsl_analyze::zone_check::{
    canonical_symbolic_state, settle_zone_check, Bound, Dbm, ZoneCheckLimits, ZoneCheckOutcome,
    ZoneCheckRefusal, ZoneItem, ZoneReduction,
};
use qsl_eval::simulation::model::TimedSystem;
use qsl_eval::simulation::Permutation;
use qsl_foundation::ReductionKind;
use qsl_replay::{
    AppliedReduction, CertificateCheckLimits, ComponentReason, CounterexampleKind,
    InconclusiveCause, ProofBasis, SymmetryRefusal, TerminalValue, UnavailabilityCause,
    ZoneCertificateVerdict,
};
use quire_checked::key::DeclarationKey;
use quire_exact::{Cancel, Integer, Rational};

use crate::timed_world::{
    document, ident, symmetric_document, world, world_armed, World, DENSE, PACKAGE,
};
use crate::zone_liveness::decide;
use crate::zone_replay::{package_id, replay, reproduced};
use crate::zone_search::{
    check, claim_with, deadlock, deadlock_result, deadlock_with, delays, operations, proved,
    time_lock_item_with, violated, INFINITE,
};

/// The population of [`timers`].
fn calls() -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident("calls"),
    }
}

/// FR-430-AC-3's `Timer`: objects `t1` and `t2` of a `symmetric`
/// population, each with `armed` (here `inflight`) and clock `x`; `arm`,
/// guarded `not self.armed` and `x >= 1`, and, unless `latching`,
/// `disarm`, guarded `self.armed` and `x >= 1`, each resetting `x`. The
/// claim `always holds(not t.armed)` over `t`.
fn timers(latching: bool) -> World {
    timers_claiming(latching, "always holds(not t.inflight)")
}

/// [`timers`] with the claim `body` over `t`.
fn timers_claiming(latching: bool, body: &str) -> World {
    timers_over(latching, body, &["t1", "t2"], &[])
}

/// [`timers_claiming`] over the timers `keys`, those `armed` armed at
/// the start.
fn timers_over(latching: bool, body: &str, keys: &[&str], armed: &[&str]) -> World {
    let arm: (&str, &[&str]) = ("arm", &["inflight", "x"]);
    let both = [arm, ("disarm", &["inflight", "x"])];
    let operations: &[(&str, &[&str])] = if latching { &both[..1] } else { &both };
    let disarm = if latching {
        ""
    } else {
        "pre Disarm using v on M::Call::disarm { self.inflight and self.x >= 1 ms }\n\
         post Disarmed using v on M::Call::disarm { not self.inflight and self.x = 0 ms }\n"
    };
    world_armed(
        &symmetric_document(&["x"], operations),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Arm using v on M::Call::arm {{ not self.inflight and self.x >= 1 ms }}\n\
             post Armed using v on M::Call::arm {{ self.inflight and self.x = 0 ms }}\n\
             {disarm}\
             temporal Unarmed using inf over (t: M::Call) clock \"model-steps\" on origin \
             {{ {body} }}\n"
        ),
        keys,
        armed,
    )
}

/// `outcome` settles V-8 naming symmetry, with no state stored.
fn refused_symmetry(outcome: ZoneCheckOutcome<&'static str>) {
    assert!(
        matches!(
            outcome,
            ZoneCheckOutcome::Reduction {
                cause: ZoneReduction::Unsupported {
                    reduction: ReductionKind::Symmetry
                },
                stats,
            } if stats.symbolic_states == 0
        ),
        "{outcome:?}"
    );
}

/// The symmetry `[[t1, t2]]` over [`calls`].
fn swap() -> Reductions {
    Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: calls(),
            classes: vec![vec!["t1".to_owned(), "t2".to_owned()]],
        }],
        ..Reductions::default()
    }
}

fn stored(outcome: &ZoneCheckOutcome<&'static str>) -> u64 {
    match outcome {
        ZoneCheckOutcome::Holds { stats, .. }
        | ZoneCheckOutcome::HoldsReduced { stats, .. }
        | ZoneCheckOutcome::Violated { stats, .. } => stats.symbolic_states,
        other => panic!("expected a completed run, got {other:?}"),
    }
}

/// The receiver key of each step of `counterexample`, read from the
/// receiver reference's hex-encoded identity.
fn receivers(counterexample: &qsl_replay::TemporalCounterexample) -> Vec<&'static str> {
    counterexample
        .prefix
        .iter()
        .map(|step| {
            let receiver = step.transition.receiver().expect("a receiver");
            [("t1", "7431"), ("t2", "7432"), ("t3", "7433")]
                .into_iter()
                .find(|(_, hex)| receiver.contains(&format!("\"identity\":\"{hex}\"")))
                .map_or("?", |(key, _)| key)
        })
        .collect()
}

/// FR-430-AC-3 (TC-1170): under `[[t1, t2]]`, the deadlock-freedom item
/// holds as it does with no reduction, from fewer stored symbolic states,
/// and settles `Proved{Reduced{symmetry}, Uncertified}` (FR-160); with no
/// `disarm`, the state with both timers armed deadlocks, and the reduced
/// run's counterexample, whose first target canonicalises by the swap, is
/// lifted to concrete steps, one `arm` of each timer, that replay.
#[trace("TC-1170", "FR-430-AC-3")]
#[test]
fn symmetry_permutes_each_timer_with_its_clock() {
    let world = timers(false);
    let plain = deadlock(&world);
    assert!(matches!(plain, ZoneCheckOutcome::Holds { .. }), "{plain:?}");
    let reduced = deadlock_with(&world, &swap());
    let ZoneCheckOutcome::HoldsReduced { reductions, .. } = &reduced else {
        panic!("expected a reduced proof, got {reduced:?}");
    };
    assert!(stored(&reduced) < stored(&plain), "{reduced:?} {plain:?}");
    let [AppliedReduction::Symmetry {
        groups, stabiliser, ..
    }] = reductions.as_slice()
    else {
        panic!("one applied symmetry: {reductions:?}");
    };
    assert_eq!(groups, stabiliser);
    let record = settle_zone_check(
        ZoneItem::Outcome(&reduced),
        &package_id(),
        &Cancel::new(),
        |_| panic!("a reduced proof has no certificate"),
        |_| panic!("a proof has no counterexample"),
    );
    assert!(
        matches!(
            &record.value,
            TerminalValue::Proved {
                basis: ProofBasis::Reduced { .. },
                certification: qsl_replay::Certification::Uncertified,
            }
        ),
        "{record:?}"
    );

    let latching = timers(true);
    // The first `arm`'s target canonicalises by the swap, so the reduced
    // path's second step is lifted to the other timer's `arm`.
    let system = latching.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let group = admit(&system, &swap().symmetry).expect("the declaration admits");
    let initial = timed.initial().remove(0);
    let first = timed
        .guarded_steps(initial.discrete())
        .expect("steps")
        .remove(0);
    let (_, _, permutation) =
        canonical_symbolic_state(&timed, &group, first.post(), &Dbm::zero(3)).expect("canonical");
    assert_ne!(permutation, Permutation::identity());
    let plain = violated(deadlock(&latching));
    let lifted = violated(deadlock_with(&latching, &swap()));
    assert_eq!(lifted.kind, CounterexampleKind::Deadlock);
    assert_eq!(operations(&lifted), ["arm", "arm"]);
    let mut keys = receivers(&lifted);
    keys.sort();
    assert_eq!(keys, ["t1", "t2"]);
    assert_eq!(delays(&lifted), delays(&plain));
    let settlement = replay(&latching, &lifted).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");
}

/// FR-430-AC-3 (TC-1170) for a claim over `t`: FR-151 checks one instance
/// for the orbit `[t1, t2]`, bound to `t1` under its trivial stabiliser;
/// `always holds(not t.armed)` is violated as with no reduction, and its
/// counterexample replays.
#[trace("TC-1170", "FR-430-AC-3")]
#[test]
fn a_claim_over_a_timer_is_checked_once_per_orbit() {
    let world = timers(false);
    let limits = ZoneCheckLimits::default();
    let plain = violated(claim_with(
        &world,
        limits,
        &Cancel::new(),
        &Reductions::default(),
    ));
    let reduced = violated(claim_with(&world, limits, &Cancel::new(), &swap()));
    assert_eq!(reduced.over_binding, plain.over_binding);
    assert_eq!(
        reduced
            .over_binding
            .as_ref()
            .map(|binding| binding.key.as_str()),
        Some("t1")
    );
    let settlement = replay(&world, &reduced).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");
}

/// FR-430-AC-3 (TC-1170): the symbolic state with `t1` armed, `t2`
/// unarmed and zone `Z`, and its image with `t1` unarmed, `t2` armed and
/// `Z` with the two clocks swapped, canonicalise to equal states.
#[trace("TC-1170", "FR-430-AC-3")]
#[test]
fn a_state_and_its_swapped_image_canonicalise_equal() {
    let world = timers(false);
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let group = admit(&system, &swap().symmetry).expect("the declaration admits");
    let initial = timed.initial().remove(0);
    let armed: Vec<_> = timed
        .guarded_steps(initial.discrete())
        .expect("steps")
        .into_iter()
        .filter(|step| step.transition().operation().contains("Call/arm\""))
        .collect();
    let [first, second] = armed.as_slice() else {
        panic!("one arm per timer: {armed:?}");
    };
    // `Z`: x of the armed timer at 0, the other's clock at least 2.
    let mut zone = Dbm::zero(3);
    zone.up();
    let other = timed
        .places()
        .iter()
        .position(|place| place.object() != first.receiver())
        .expect("the other timer's clock")
        + 1;
    zone.constrain(0, other, Bound::le((-2).into()))
        .expect("in range");
    let armed_place = 3 - other;
    zone.reset(armed_place, &0.into()).expect("in range");
    let swapped = Permutation::of(
        &calls(),
        [
            ("t1".to_owned(), "t2".to_owned()),
            ("t2".to_owned(), "t1".to_owned()),
        ],
    );
    let image = zone
        .renamed(
            &timed
                .place_images(&swapped)
                .expect("each clock has an image"),
        )
        .expect("in range");
    assert_ne!(image, zone);
    let (left, left_zone, _) =
        canonical_symbolic_state(&timed, &group, first.post(), &zone).expect("canonical");
    let (right, right_zone, _) =
        canonical_symbolic_state(&timed, &group, second.post(), &image).expect("canonical");
    let key = |state| system.state_key(state).expect("a key").0;
    assert_eq!(key(&left), key(&right));
    assert_eq!(left_zone, right_zone);
}

/// FR-430-AC-3 (TC-1170): EN-6 applies no partial-order reduction and no
/// state constraint, so the deadlock-freedom item selecting either, or
/// both, settles V-8 `unsupported-requested-capability`, naming
/// partial-order reduction first, with no state stored.
#[trace("TC-1170", "FR-430-AC-3")]
#[test]
fn partial_order_reduction_and_a_constraint_are_not_applied() {
    let world = timers(false);
    let unsupported = |reductions: Reductions| match deadlock_with(&world, &reductions) {
        ZoneCheckOutcome::Reduction {
            cause: ZoneReduction::Unsupported { reduction },
            stats,
        } => {
            assert_eq!(stats.symbolic_states, 0);
            reduction
        }
        other => panic!("expected V-8, got {other:?}"),
    };
    let por = Reductions {
        partial_order: true,
        ..swap()
    };
    let constraint = Reductions {
        constraint: Some("Low".to_owned()),
        ..Reductions::default()
    };
    let both = Reductions {
        partial_order: true,
        constraint: Some("Low".to_owned()),
        ..Reductions::default()
    };
    assert_eq!(unsupported(por.clone()), ReductionKind::PartialOrder);
    assert_eq!(unsupported(constraint), ReductionKind::StateConstraint);
    assert_eq!(unsupported(both), ReductionKind::PartialOrder);
    let outcome = deadlock_with(&world, &por);
    let record = settle_zone_check(
        ZoneItem::Outcome(&outcome),
        &package_id(),
        &Cancel::new(),
        |_| panic!("no certificate"),
        |_| panic!("no counterexample"),
    );
    let TerminalValue::Unsupported(cause) = &record.value else {
        panic!("expected V-8, got {record:?}");
    };
    assert_eq!(
        *cause,
        UnavailabilityCause::UnsupportedReduction {
            reduction: ReductionKind::PartialOrder
        }
    );
    assert_eq!(cause.as_str(), "unsupported-requested-capability");
}

/// FR-430-AC-4's `Blink`: `tick`, guarded `x >= 1 ms`, resets `x` under
/// `x <= 1 ms`, so it fires exactly at `x = 1`; `fire`, guarded `x > 0 ms
/// and x < 1 ms`, sets `fired` (here `retried`). The claim `eventually
/// holds(b.fired)` under infinite-trace and `fairness`.
fn blink(fairness: &str) -> World {
    world(
        &document(&["x"], &[("tick", &["x"]), ("fire", &["retried"])]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Tick using v on M::Call::tick {{ self.x >= 1 ms }}\n\
             post Ticked using v on M::Call::tick {{ self.x = 0 ms }}\n\
             pre Fire using v on M::Call::fire {{ self.x > 0 ms and self.x < 1 ms }}\n\
             post Fired using v on M::Call::fire {{ self.retried }}\n\
             time invariant Blinking when {{ true }} {{ self.x <= 1 ms }}\n\
             temporal Fires using inf over (b: M::Call) clock \"model-steps\" on origin \
             {{ {fairness} eventually holds(b.retried) }}\n"
        ),
    )
}

/// FR-430-AC-4 (TC-1170): over `Blink`, `eventually holds(b.fired)` under
/// `fair weak fire` is violated by a lasso of `tick` steps whose loop delay
/// is 1, since `fire` is never continuously enabled; under `fair strong
/// fire` it holds, with a certificate the core accepts, since `fire` is
/// enabled at instants inside every delay of that lasso though at no
/// position of it; the certificate states the strong reason with its
/// nested numbering (ADR-026 CF-2).
#[trace("TC-1170", "FR-430-AC-4")]
#[test]
fn strong_fairness_reads_enabledness_between_positions() {
    let weak = blink("fair weak M::Call::fire;");
    let counterexample = violated(decide(&weak));
    let lasso = counterexample.lasso.expect("a lasso");
    let looped = &operations(&counterexample)[lasso.entry..];
    assert!(
        !looped.is_empty() && looped.iter().all(|step| step == "tick"),
        "{looped:?}"
    );
    let total = delays(&counterexample)[lasso.entry..]
        .iter()
        .fold(Rational::from_integer(Integer::zero()), |sum, delay| {
            sum.add(delay)
        });
    assert_eq!(total, Rational::from_integer(Integer::one()));
    let settlement = replay(&weak, &counterexample).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");

    let strong = blink("fair strong M::Call::fire;");
    let outcome = decide(&strong);
    assert!(
        matches!(outcome, ZoneCheckOutcome::Holds { .. }),
        "{outcome:?}"
    );
    let certificate = proved(outcome);
    // ADR-026 CF-2: the strong reason with its nested numbering.
    let components = certificate.components.as_ref().expect("a component part");
    assert!(
        components
            .reasons
            .iter()
            .any(|reason| matches!(reason, ComponentReason::Strong { .. })),
        "{components:?}"
    );
    assert_eq!(
        check(&strong, &certificate, CertificateCheckLimits::default()),
        Ok(ZoneCertificateVerdict::Accepted)
    );
}

/// QSpec FR-430 applies symmetry to EN-6's search. The time-lock-freedom
/// item, decided on the completed graph's edges, and a clause decided on
/// the liveness product read no reduced graph, so selecting symmetry for
/// either settles V-8 naming symmetry, with no state stored.
#[trace("TC-1170", "FR-430-AC-3")]
#[test]
fn symmetry_is_refused_where_no_reduced_graph_is_read() {
    let world = timers(false);
    let refused = refused_symmetry;
    refused(time_lock_item_with(
        &world,
        ZoneCheckLimits::default(),
        &swap(),
    ));
    let live = timers_claiming(false, "always eventually holds(t.inflight)");
    refused(claim_with(
        &live,
        ZoneCheckLimits::default(),
        &Cancel::new(),
        &swap(),
    ));
}

/// FR-151 over EN-6 (FR-430-AC-3): initial states that `[[t1, t2]]` does
/// not close (`t1` armed, `t2` not) settle `SymmetryBroken`, inconclusive
/// by FR-160's row, with no state stored; a declaration naming a key
/// outside the universe is refused; and an item selecting symmetry over
/// the time-lock item settles V-8 before either is judged.
#[trace("TC-1170", "FR-430-AC-3")]
#[test]
fn symmetry_admission_is_judged_before_the_search() {
    let broken = timers_over(
        false,
        "always holds(not t.inflight)",
        &["t1", "t2"],
        &["t1"],
    );
    let outcome = deadlock_with(&broken, &swap());
    let ZoneCheckOutcome::Reduction {
        cause: ZoneReduction::SymmetryBroken { population, .. },
        stats,
    } = &outcome
    else {
        panic!("expected SymmetryBroken, got {outcome:?}");
    };
    assert_eq!(*population, calls());
    assert_eq!(stats.symbolic_states, 0);
    let record = settle_zone_check(
        ZoneItem::Outcome(&outcome),
        &package_id(),
        &Cancel::new(),
        |_| panic!("no certificate"),
        |_| panic!("no counterexample"),
    );
    assert!(
        matches!(
            record.value,
            TerminalValue::Inconclusive(InconclusiveCause::SymmetryBroken { .. })
        ),
        "{record:?}"
    );
    refused_symmetry(time_lock_item_with(
        &broken,
        ZoneCheckLimits::default(),
        &swap(),
    ));

    let world = timers(false);
    let unknown = Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: calls(),
            classes: vec![vec!["t1".to_owned(), "t3".to_owned()]],
        }],
        ..Reductions::default()
    };
    assert!(
        matches!(
            deadlock_result(&world, &unknown),
            Err(ZoneCheckRefusal::Symmetry(
                SymmetryRefusal::UnknownKey { .. }
            ))
        ),
        "{:?}",
        deadlock_result(&world, &unknown)
    );
}

/// FR-151 over EN-6: with three timers and the class `[[t1, t2]]`, `t3`
/// is fixed. The deadlock item holds reduced as unreduced; in the
/// latching variant both runs find the deadlock with equal delays, and the
/// lifted counterexample replays.
#[trace("TC-1170", "FR-430-AC-3")]
#[test]
fn a_partial_class_fixes_the_timers_outside_it() {
    let three = ["t1", "t2", "t3"];
    let world = timers_over(false, "always holds(not t.inflight)", &three, &[]);
    assert!(matches!(deadlock(&world), ZoneCheckOutcome::Holds { .. }));
    assert!(matches!(
        deadlock_with(&world, &swap()),
        ZoneCheckOutcome::HoldsReduced { .. }
    ));
    let latching = timers_over(true, "always holds(not t.inflight)", &three, &[]);
    let plain = violated(deadlock(&latching));
    let lifted = violated(deadlock_with(&latching, &swap()));
    assert_eq!(delays(&lifted), delays(&plain));
    let mut keys = receivers(&lifted);
    keys.sort_unstable();
    assert_eq!(keys, ["t1", "t2", "t3"]);
    let settlement = replay(&latching, &lifted).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");
}

/// FR-160 over EN-6: a claim over an empty universe has no orbit to
/// reduce, so under a selected symmetry it runs unreduced and its proof is
/// certified, never `Reduced` with no reduction applied.
#[trace("TC-1170", "FR-430-AC-3")]
#[test]
fn symmetry_over_no_instance_runs_unreduced() {
    let empty = timers_over(false, "always holds(not t.inflight)", &[], &[]);
    let declared = Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: calls(),
            classes: Vec::new(),
        }],
        ..Reductions::default()
    };
    let outcome = claim_with(
        &empty,
        ZoneCheckLimits::default(),
        &Cancel::new(),
        &declared,
    );
    assert!(
        matches!(outcome, ZoneCheckOutcome::Holds { .. }),
        "{outcome:?}"
    );
}
