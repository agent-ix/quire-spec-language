// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-239 (TC-694, TC-710) and FR-241's finite paths (TC-696): EN-6's
//! symbolic zone search over the timed fixtures of `crate::timed_world`.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{
    bounded_window_verdict, check_timed_model, ZoneCheckItem, ZoneCheckLimit, ZoneCheckLimits,
    ZoneCheckOutcome, ZoneCheckRefusal, ZoneCheckRequest, ZoneStop, ZoneUndecided,
};
use qsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem, TimedSystem};
use qsl_eval::temporal::{
    evaluate_timed_at, AtomReading, InsufficientData, LetterStop, TemporalLetters, TemporalOutcome,
    TemporalPosition, TimedTrace, UndefinedCause,
};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_replay::{
    check_zone_certificate, CertEdge, CertificateCheckLimits, CounterexampleKind, DivergentTarget,
    ModelCheckItem, TemporalCounterexample, TimedPoint, UndefinedLocus, ZoneCertificate,
    ZoneCertificateRefusal, ZoneCertificateRequest, ZoneCertificateVerdict,
};
use qsl_route::deadlock::{ItemObligation, ObligationKind};
use qsl_semantics::check::footprint::{FootprintRefusal, SymbolicLocation};
use qsl_semantics::check::{FormulaNode, FormulaNodeId};
use quire_exact::{Cancel, Integer, Meter, Origin, Rational, Role, Undefined};

use crate::timed_world::{document, world, World, DENSE};

pub(crate) const INFINITE: &str = "profile inf = \"quire.temporal.infinite-trace/v1\";\n";
pub(crate) const TIMED: &str = "profile tt = \"quire.temporal.timed/v1\";\n";

pub(crate) fn rpc_document() -> Vec<u8> {
    document(
        &["x"],
        &[
            ("send", &["phase", "inflight", "late", "x"]),
            ("reply", &["phase", "inflight", "late"]),
            ("timeout", &["phase"]),
            ("reset", &["phase"]),
        ],
    )
}

/// ADR-026 §11's `Rpc` with client deadline `t` ms and the claim
/// `NoLateReply` under infinite-trace.
pub(crate) fn rpc(t: u32) -> World {
    rpc_claim(
        t,
        INFINITE,
        "temporal NoLateReply using inf over (c: M::Call) clock \"model-steps\" on origin \
         { always holds(not c.late) }\n",
    )
}

/// ADR-026 §11's `Rpc` with client deadline `t` ms, under the profile
/// lines `profiles`, with `claim` as its one temporal clause.
pub(crate) fn rpc_claim(t: u32, profiles: &str, claim: &str) -> World {
    rpc_variant(t, ">= 1", ">=", profiles, claim)
}

/// [`rpc_claim`] with `reply` guarded by `self.x {reply} ms` and `timeout`
/// by `self.x {timeout} {t} ms`: ADR-026 §11's strict-guard variant is
/// `("> 3", ">")`.
pub(crate) fn rpc_variant(
    t: u32,
    reply: &str,
    timeout: &str,
    profiles: &str,
    claim: &str,
) -> World {
    world(
        &rpc_document(),
        profiles,
        &format!(
            "{DENSE}\
             pre SendIdle using v on M::Call::send {{ self.phase = 0 }}\n\
             post Send using v on M::Call::send {{ \
                 self.phase = 1 and self.inflight and not self.late and self.x = 0 ms }}\n\
             pre Reply using v on M::Call::reply {{ self.inflight and self.x {reply} ms }}\n\
             post Replied using v on M::Call::reply {{ not self.inflight and \
                 self.late = (pre(self.phase) = 3) and \
                 (pre(self.phase) = 1 implies self.phase = 2) and \
                 (pre(self.phase) != 1 implies self.phase = pre(self.phase)) }}\n\
             pre Timeout using v on M::Call::timeout {{ self.phase = 1 and self.x {timeout} {t} ms }}\n\
             post TimedOut using v on M::Call::timeout {{ self.phase = 3 }}\n\
             pre Reset using v on M::Call::reset {{ \
                 not self.inflight and (self.phase = 2 or self.phase = 3) }}\n\
             post ResetIdle using v on M::Call::reset {{ self.phase = 0 }}\n\
             time invariant ServerDeadline when {{ self.inflight }} {{ self.x <= 3 ms }}\n\
             time invariant ClientDeadline when {{ self.phase = 1 }} {{ self.x <= {t} ms }}\n\
             {claim}"
        ),
    )
}

pub(crate) fn exact(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("a rational")
}

/// The subject every item here is over.
pub(crate) const SUBJECT: &str = "c";

/// An outcome over [`SUBJECT`].
pub(crate) type Outcome = ZoneCheckOutcome<&'static str>;

/// The occurrence of the claim each world's first clause is.
fn claim_occurrence() -> OccurrenceKey {
    OccurrenceKey::new(
        WireNodeId::from_digest([1; 32]),
        Origin::new(Role::new("temporal"), 0),
    )
}

/// The first clause of `world`, decided by EN-6 under `limits`.
pub(crate) fn claim(world: &World, limits: ZoneCheckLimits, cancel: &Cancel) -> Outcome {
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("letters");
    check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::Clause {
                clause: node.clause(),
                letters: &letters,
            },
            obligation: ItemObligation {
                subject: SUBJECT,
                kind: ObligationKind::Authored(claim_occurrence()),
            },
            limits,
        },
        cancel,
    )
    .expect("EN-6 takes the claim")
}

pub(crate) fn deadlock(world: &World) -> Outcome {
    let system: ModelSystem<'_> = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::DeadlockFreedom,
            obligation: ItemObligation {
                subject: SUBJECT,
                kind: ObligationKind::DeadlockFreedom,
            },
            limits: ZoneCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("EN-6 takes the item")
}

pub(crate) fn violated(outcome: Outcome) -> TemporalCounterexample {
    match outcome {
        ZoneCheckOutcome::Violated { counterexample, .. } => *counterexample,
        other => panic!("expected a violation, got {other:?}"),
    }
}

pub(crate) fn delays(counterexample: &TemporalCounterexample) -> Vec<Rational> {
    counterexample
        .prefix
        .iter()
        .map(|step| step.delay.clone().expect("a timed step carries its delay"))
        .collect()
}

pub(crate) fn operations(counterexample: &TemporalCounterexample) -> Vec<String> {
    counterexample
        .prefix
        .iter()
        .map(|step| {
            let operation = step.transition.operation();
            [
                "send",
                "reply",
                "timeout",
                "reset",
                "heartbeat",
                "tick",
                "retry",
                "step",
                "go",
                "ping",
            ]
            .into_iter()
            .find(|name| operation.contains(&format!("Call/{name}\"")))
            .unwrap_or("?")
            .to_owned()
        })
        .collect()
}

/// FR-239-AC-1 and FR-241-AC-1: `NoLateReply` holds over `Rpc` with
/// `T = 4 ms`, with a certificate, and, with `T = 3 ms`, is violated at
/// `trace_position` 3 by `send`, `timeout`, `reply` with the least delays
/// `0`, `3`, `0`; the deadlock-freedom item holds under both.
#[trace("TC-694", "FR-239-AC-1")]
#[trace("TC-696", "FR-241-AC-1")]
#[test]
fn no_late_reply_holds_at_four_and_is_refuted_at_three() {
    let four = rpc(4);
    let ZoneCheckOutcome::Holds { certificate, stats } =
        claim(&four, ZoneCheckLimits::default(), &Cancel::new())
    else {
        panic!("NoLateReply holds at T = 4");
    };
    assert_eq!(
        stats.symbolic_states,
        u64::try_from(certificate.nodes.len()).unwrap()
    );
    assert!(certificate.nodes.iter().all(|node| !node.locked));

    let three = rpc(3);
    let counterexample = violated(claim(&three, ZoneCheckLimits::default(), &Cancel::new()));
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(counterexample.trace_position(), Some(3));
    assert_eq!(operations(&counterexample), ["send", "timeout", "reply"]);
    assert_eq!(
        delays(&counterexample),
        [exact(0, 1), exact(3, 1), exact(0, 1)]
    );
    assert_eq!(counterexample.final_delay, None);
    assert!(counterexample.over_binding.is_some());

    for world in [&four, &three] {
        assert!(matches!(deadlock(world), ZoneCheckOutcome::Holds { .. }));
    }
}

/// ADR-026 §9's retry model: `heartbeat` while `x < 1` resets `y`, and
/// `retry` when `x > 1 and y < 1` sets `retried`.
fn retry() -> World {
    world(
        &document(
            &["x", "y"],
            &[("heartbeat", &["y"]), ("retry", &["retried"])],
        ),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Beat using v on M::Call::heartbeat {{ self.x < 1 ms }}\n\
             post Beaten using v on M::Call::heartbeat {{ self.y = 0 ms }}\n\
             pre Retry using v on M::Call::retry {{ \
                 not self.retried and self.x > 1 ms and self.y < 1 ms }}\n\
             post Retried using v on M::Call::retry {{ self.retried }}\n\
             temporal NoRetry using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(not c.retried) }}\n"
        ),
    )
}

/// FR-239-AC-2, and part of FR-241-AC-2 (TC-696): the retry is reachable in
/// dense time, by a heartbeat after `d1` in `(0, 1)` and a retry after
/// `d2 < 1` with `d1 + d2 > 1`, each exact; two runs give equal
/// counterexamples. Not covered: its replay to
/// `reproduced-with-evaluated-witness` (FR-237).
#[trace("TC-694", "FR-239-AC-2")]
#[trace("TC-696")]
#[test]
fn the_retry_that_integer_time_misses_is_found() {
    let world = retry();
    let counterexample = violated(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    assert_eq!(operations(&counterexample), ["heartbeat", "retry"]);
    let [d1, d2] = &delays(&counterexample)[..] else {
        panic!("two delays");
    };
    let (zero, one) = (exact(0, 1), exact(1, 1));
    assert!(d1 > &zero && d1 < &one, "{d1:?}");
    assert!(d2 < &one && d1.add(d2) > one, "{d2:?}");
    assert_eq!(
        counterexample,
        violated(claim(&world, ZoneCheckLimits::default(), &Cancel::new()))
    );
}

/// A model whose clock `x` is compared only with `bound` ms: `go` needs
/// `x >= bound` and sets `retried`, `work` changes nothing, and `x` is
/// never reset.
fn unbounded_growth(bound: &str) -> World {
    world(
        &document(&["x"], &[("go", &["retried"]), ("work", &[])]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Go using v on M::Call::go {{ not self.retried and self.x >= {bound} ms }}\n\
             post Gone using v on M::Call::go {{ self.retried }}\n\
             temporal Fine using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(c.phase = 0) }}\n"
        ),
    )
}

/// FR-239-AC-3: with `x` compared only with 5, and growing without bound
/// in a loop, the search stores finitely many symbolic states, and as many
/// with the constant `10^12`.
#[trace("TC-694", "FR-239-AC-3")]
#[test]
fn stored_states_do_not_grow_with_the_constant() {
    let count = |bound: &str| match claim(
        &unbounded_growth(bound),
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ) {
        ZoneCheckOutcome::Holds { stats, .. } => stats.symbolic_states,
        other => panic!("expected a proof, got {other:?}"),
    };
    let small = count("5");
    assert!(small > 0);
    assert_eq!(small, count("1000000000000"));
}

/// Part of FR-239-AC-4 (TC-694): `max_symbolic_states` 2 stops naming the
/// value 2, a cancelled handle stops `Cancelled`, and two runs give equal
/// outcomes, over `NoLateReply`. Not covered: `Settles`, its
/// `RequiresBound` refusal with no state explored, and byte-equal
/// certificates.
#[trace("TC-694")]
#[test]
fn budgets_and_cancellation_stop_the_search() {
    let world = rpc(4);
    let limits = ZoneCheckLimits {
        max_symbolic_states: 2,
        ..ZoneCheckLimits::default()
    };
    let ZoneCheckOutcome::Stopped { cause, stats } = claim(&world, limits, &Cancel::new()) else {
        panic!("a budget of 2 stops");
    };
    assert_eq!(
        cause,
        ZoneStop::LimitReached(ZoneCheckLimit::MaxSymbolicStates(2))
    );
    assert_eq!(stats.symbolic_states, 2);

    let cancel = Cancel::new();
    cancel.cancel(quire_exact::CancelCause::Requested);
    let ZoneCheckOutcome::Stopped { cause, .. } =
        claim(&world, ZoneCheckLimits::default(), &cancel)
    else {
        panic!("a cancelled run stops");
    };
    assert_eq!(cause, ZoneStop::Cancelled);

    let summary = |outcome: Outcome| format!("{outcome:?}");
    assert_eq!(
        summary(claim(&world, ZoneCheckLimits::default(), &Cancel::new())),
        summary(claim(&world, ZoneCheckLimits::default(), &Cancel::new()))
    );
}

/// The `Ticker` model: `step` while `n < 2` (here `phase`) needs `x >= 1`,
/// resets `x` and increments `n`; the time invariant `x <= 1` applies while
/// `n < 2`, or always when `locked`.
pub(crate) fn ticker(locked: bool) -> World {
    ticker_with(locked, "always holds(6 mod (2 - c.phase) >= 0)")
}

/// [`ticker`] with `body` as its clause on origin, under the timed profile.
fn ticker_with(locked: bool, body: &str) -> World {
    let when = if locked { "true" } else { "self.phase < 2" };
    world(
        &document(&["x"], &[("step", &["phase", "x"])]),
        TIMED,
        &format!(
            "{DENSE}\
             pre Step using v on M::Call::step {{ self.phase < 2 and self.x >= 1 ms }}\n\
             post Stepped using v on M::Call::step {{ \
                 (pre(self.phase) = 0 implies self.phase = 1) and \
                 (pre(self.phase) = 1 implies self.phase = 2) and self.x = 0 ms }}\n\
             time invariant Tick when {{ {when} }} {{ self.x <= 1 ms }}\n\
             temporal Defined using tt over (c: M::Call) clock \"model-time\" on origin \
             {{ {body} }}\n"
        ),
    )
}

/// Part of FR-239-AC-5 (TC-710): over `Ticker`, `6 mod (2 - n)` is undefined
/// at `n = 2`, reached by `step` after delay 1 twice, and the state there is
/// quiescent, so the search is violated with an undefined evaluation of
/// cause `division-by-zero` at position 2, time stamp 2 and `x = 0`, keyed
/// by the model state alone. With the unconditional invariant that state is
/// a time-lock, and no behaviour diverges: `NoAdmittedBehaviour`. Not
/// covered: the time-lock-freedom item refuted over the time-locked
/// variant, which FR-240's search decides (QSL-583).
#[trace("TC-710")]
#[test]
fn an_undefined_atom_refutes_only_where_time_diverges() {
    let counterexample = violated(claim(
        &ticker(false),
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ));
    assert_eq!(operations(&counterexample), ["step", "step"]);
    assert_eq!(delays(&counterexample), [exact(1, 1), exact(1, 1)]);
    let CounterexampleKind::UndefinedEvaluation(undefined) = &counterexample.kind else {
        panic!(
            "expected an undefined evaluation, got {:?}",
            counterexample.kind
        );
    };
    assert_eq!(undefined.site.position, 2);
    assert_eq!(
        undefined.cause,
        UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
    assert!(matches!(undefined.site.locus, UndefinedLocus::Atom(_)));
    assert_eq!(
        undefined.site.time,
        Some(Box::new(TimedPoint {
            stamp: exact(2, 1),
            clocks: vec![exact(0, 1)],
        }))
    );
    // The site's key names the model state; the step's post-state digest
    // names it with its clocks.
    assert_ne!(
        undefined.site.state_key,
        counterexample.prefix[1].post_state
    );

    match claim(&ticker(true), ZoneCheckLimits::default(), &Cancel::new()) {
        ZoneCheckOutcome::Undecided { cause, .. } => {
            assert_eq!(cause, ZoneUndecided::NoAdmittedBehaviour);
        }
        other => panic!("expected no admitted behaviour, got {other:?}"),
    }
}

/// The deadlock-freedom item over a quiescent state with no step: `go`
/// sets `retried`, after which nothing is enabled, and none intends the
/// state. With `go` guarded by `x <= 1 ms` and `work` enabled once
/// `retried` holds, the initial valuations past 1 are deadlocked only after
/// a delay, which no position names.
#[trace("TC-694")]
#[test]
fn a_quiescent_terminal_state_is_a_deadlock_over_time() {
    let counterexample = violated(deadlock(&world(
        &document(&["x"], &[("go", &["retried"])]),
        "",
        &format!(
            "{DENSE}\
             pre Go using v on M::Call::go {{ not self.retried }}\n\
             post Gone using v on M::Call::go {{ self.retried }}\n"
        ),
    )));
    assert_eq!(counterexample.kind, CounterexampleKind::Deadlock);
    assert_eq!(operations(&counterexample), ["go"]);
    assert_eq!(delays(&counterexample), [exact(0, 1)]);

    let delayed = deadlock(&world(
        &document(&["x"], &[("go", &["retried"]), ("work", &[])]),
        "",
        &format!(
            "{DENSE}\
             pre Go using v on M::Call::go {{ not self.retried and self.x <= 1 ms }}\n\
             post Gone using v on M::Call::go {{ self.retried }}\n\
             pre Work using v on M::Call::work {{ self.retried }}\n"
        ),
    ));
    match delayed {
        ZoneCheckOutcome::Undecided { cause, .. } => {
            assert_eq!(cause, ZoneUndecided::DeadlockAfterDelay);
        }
        other => panic!("expected a deadlock after a delay, got {other:?}"),
    }
}

/// FR-239 "Admitted behaviour" over the `Lock` model: `go` moves `A` (`0`)
/// to `B` (`1`), `x <= 1 ms` while in `B`, and `ping`, enabled in `B`,
/// changes nothing. `always holds(c.phase = 0)` fails only in `B`, where
/// every behaviour is Zeno: the violation is kept as a `locked` node and
/// the claim holds. When `ping` resets `x`, `B` diverges and the claim is
/// refuted by `go` after delay 0.
#[trace("TC-694")]
#[test]
fn a_violation_on_time_locked_states_holds_vacuously() {
    let lock = |ping_resets: bool| {
        let (modifies, post): (&[&str], &str) = if ping_resets {
            (
                &["x"],
                "post Pinged using v on M::Call::ping { self.x = 0 ms }\n",
            )
        } else {
            (&[], "")
        };
        world(
            &document(&["x"], &[("go", &["phase"]), ("ping", modifies)]),
            INFINITE,
            &format!(
                "{DENSE}\
                 pre Go using v on M::Call::go {{ self.phase = 0 }}\n\
                 post Gone using v on M::Call::go {{ self.phase = 1 }}\n\
                 pre Ping using v on M::Call::ping {{ self.phase = 1 }}\n{post}\
                 time invariant Locked when {{ self.phase = 1 }} {{ self.x <= 1 ms }}\n\
                 temporal StaysA using inf over (c: M::Call) clock \"model-steps\" on origin \
                 {{ always holds(c.phase = 0) }}\n"
            ),
        )
    };
    let ZoneCheckOutcome::Holds { certificate, .. } =
        claim(&lock(false), ZoneCheckLimits::default(), &Cancel::new())
    else {
        panic!("the claim holds vacuously in B");
    };
    assert_eq!(
        certificate.nodes.iter().filter(|node| node.locked).count(),
        1
    );

    let counterexample = violated(claim(
        &lock(true),
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ));
    assert_eq!(operations(&counterexample), ["go"]);
    assert_eq!(delays(&counterexample), [exact(0, 1)]);
}

/// FR-239 "Vacuity" under a tick source (FR-231 "Digital time"): with the
/// tick operation never enabled and `work` enabled forever, no behaviour
/// takes a tick, so none diverges and the claim is `NoAdmittedBehaviour`;
/// with the tick enabled the claim holds.
#[trace("TC-694")]
#[test]
fn a_tick_source_admits_only_behaviours_that_tick() {
    let ticks = |enabled: bool| {
        world(
            &document(&[], &[("work", &[]), ("tick", &[])]),
            INFINITE,
            &format!(
                "dimension Time;\nunit s : Time = rational(1, 1);\n\
                 unit ms : Time = rational(1, 1000) * s;\n\
                 time M tick M::Call::tick period 5 ms unit ms;\n\
                 pre Tick using v on M::Call::tick {{ {enabled} }}\n\
                 temporal Quiet using inf over (c: M::Call) clock \"model-steps\" on origin \
                 {{ always holds(not c.retried) }}\n"
            ),
        )
    };
    match claim(&ticks(false), ZoneCheckLimits::default(), &Cancel::new()) {
        ZoneCheckOutcome::Undecided { cause, .. } => {
            assert_eq!(cause, ZoneUndecided::NoAdmittedBehaviour);
        }
        other => panic!("expected no admitted behaviour, got {other:?}"),
    }
    assert!(matches!(
        claim(&ticks(true), ZoneCheckLimits::default(), &Cancel::new()),
        ZoneCheckOutcome::Holds { .. }
    ));
}

/// FR-241: a counterexample ends at a valuation with a time-divergent
/// continuation. `go` (`x > 2 ms`) moves `A` to `B`, where `x <= 4 ms`, and
/// `ping` (`x < 3 ms`) leaves `B` for the quiescent `C`. `B` violates
/// `always holds(c.phase = 0)`; entered at `x = 3` it is time-locked, so the
/// counterexample takes `go` after `5/2`, from where `ping` escapes.
#[trace("TC-696")]
#[test]
fn a_counterexample_ends_where_time_can_diverge() {
    let counterexample = violated(claim(
        &world(
            &document(&["x"], &[("go", &["phase"]), ("ping", &["phase"])]),
            INFINITE,
            &format!(
                "{DENSE}\
                 pre Go using v on M::Call::go {{ self.phase = 0 and self.x > 2 ms }}\n\
                 post Gone using v on M::Call::go {{ self.phase = 1 }}\n\
                 pre Ping using v on M::Call::ping {{ self.phase = 1 and self.x < 3 ms }}\n\
                 post Pinged using v on M::Call::ping {{ self.phase = 2 }}\n\
                 time invariant InB when {{ self.phase = 1 }} {{ self.x <= 4 ms }}\n\
                 temporal StaysA using inf over (c: M::Call) clock \"model-steps\" on origin \
                 {{ always holds(c.phase = 0) }}\n"
            ),
        ),
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ));
    assert_eq!(operations(&counterexample), ["go"]);
    assert_eq!(delays(&counterexample), [exact(5, 2)]);
}

/// FR-239's budgets bound the divergence checks with the search (SR-1599001
/// FND-009): over the `Lock` model, whose claim holds vacuously after a
/// divergence check of `B`, a `max_symbolic_states` that the search alone
/// fits stops once the checks' states are charged with it.
#[trace("TC-694")]
#[test]
fn a_tight_budget_stops_a_divergence_check() {
    let lock = world(
        &document(&["x"], &[("go", &["phase"]), ("ping", &[])]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Go using v on M::Call::go {{ self.phase = 0 }}\n\
             post Gone using v on M::Call::go {{ self.phase = 1 }}\n\
             pre Ping using v on M::Call::ping {{ self.phase = 1 }}\n\
             time invariant Locked when {{ self.phase = 1 }} {{ self.x <= 1 ms }}\n\
             temporal StaysA using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(c.phase = 0) }}\n"
        ),
    );
    let ZoneCheckOutcome::Holds { certificate, .. } =
        claim(&lock, ZoneCheckLimits::default(), &Cancel::new())
    else {
        panic!("the claim holds vacuously in B");
    };
    let searched = u64::try_from(certificate.nodes.len()).expect("a small graph");
    let limits = ZoneCheckLimits {
        max_symbolic_states: searched,
        ..ZoneCheckLimits::default()
    };
    let ZoneCheckOutcome::Stopped { cause, .. } = claim(&lock, limits, &Cancel::new()) else {
        panic!("the divergence check exceeds the budget");
    };
    assert_eq!(
        cause,
        ZoneStop::LimitReached(ZoneCheckLimit::MaxSymbolicStates(searched))
    );
}

/// The `Lock` model of FR-244-AC-4: `go` moves `A` (`0`) to `B` (`1`) with
/// no guard, `x <= 1 ms` while in `B`, and `ping`, enabled in `B`, changes
/// no data field; the claim is `always holds(c.phase = 0)`.
pub(crate) fn lock() -> World {
    world(
        &document(&["x"], &[("go", &["phase"]), ("ping", &[])]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Go using v on M::Call::go {{ self.phase = 0 }}\n\
             post Gone using v on M::Call::go {{ self.phase = 1 }}\n\
             pre Ping using v on M::Call::ping {{ self.phase = 1 }}\n\
             time invariant Locked when {{ self.phase = 1 }} {{ self.x <= 1 ms }}\n\
             temporal StaysA using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(c.phase = 0) }}\n"
        ),
    )
}

/// The certificate of a `Holds` outcome.
pub(crate) fn proved(outcome: Outcome) -> Certificate {
    match outcome {
        ZoneCheckOutcome::Holds { certificate, .. } => *certificate,
        other => panic!("expected a proof, got {other:?}"),
    }
}

/// The operation an edge's transition names.
fn operation(certificate: &Certificate, edge: usize) -> &'static str {
    let Some(transition) = &certificate.edges[edge].transition else {
        return "stutter";
    };
    let operation = transition.operation();
    [
        "send", "reply", "timeout", "reset", "go", "ping", "tock", "back", "tick", "work", "alpha",
    ]
    .into_iter()
    .find(|name| operation.contains(&format!("Call/{name}\"")))
    .unwrap_or("?")
}

/// FR-244's certificate of a proof of an item over [`SUBJECT`].
pub(crate) type Certificate = ZoneCertificate<ItemObligation<&'static str>>;

/// FR-245's check of `certificate` against `world`'s subject, for the item
/// its obligation names: the first clause, or the deadlock-freedom item.
pub(crate) fn check(
    world: &World,
    certificate: &Certificate,
    limits: CertificateCheckLimits,
) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<ItemObligation<&'static str>>> {
    let system = world.system();
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("letters");
    let item = match certificate.obligation.kind {
        ObligationKind::DeadlockFreedom => ModelCheckItem::DeadlockFreedom,
        _ => ModelCheckItem::Clause {
            clause: node.clause(),
            letters: &letters,
        },
    };
    check_zone_certificate(
        ZoneCertificateRequest {
            simulation: world.request(),
            model: "M",
            item,
        },
        &certificate.obligation,
        certificate,
        limits,
        &Cancel::new(),
    )
}

/// FR-244 "Reachability part", checked by the core (FR-245): every edge's
/// successor is covered by its target, and the certificate is accepted.
fn assert_edges_cover(world: &World, certificate: &Certificate) {
    assert_eq!(
        check(world, certificate, CertificateCheckLimits::default()),
        Ok(ZoneCertificateVerdict::Accepted)
    );
}

/// Part of FR-244-AC-1 (TC-699): `NoLateReply` over `Rpc` with `T = 4 ms`
/// is proved with a certificate naming the claim's obligation, with a
/// reachability part in which every edge's target zone covers its
/// successor, and an `admitted` path from the initial node to the
/// quiescent `Idle` node; two runs give equal certificates, and the
/// deadlock-freedom item's names `deadlock-freedom`. Not covered: FR-245
/// accepting it (QSL-582).
#[trace("TC-699")]
#[test]
fn a_safety_proof_carries_a_reachability_certificate() {
    let four = rpc(4);
    let certificate = proved(claim(&four, ZoneCheckLimits::default(), &Cancel::new()));
    assert_eq!(certificate.obligation.subject, SUBJECT);
    assert_eq!(
        certificate.obligation.kind,
        ObligationKind::Authored(claim_occurrence())
    );
    assert_eq!(certificate.initial, [0]);
    assert_eq!(certificate.lu_bounds.lower.len(), 1);
    assert!(certificate.nodes.len() > 1);
    // Edges by source in discovery order.
    assert!(certificate
        .edges
        .windows(2)
        .all(|pair| pair[0].from <= pair[1].from));
    assert_edges_cover(&four, &certificate);
    // `Idle`, the initial state with no invariant, is quiescent.
    let admitted = &certificate.admitted;
    assert_eq!(admitted.start, 0);
    assert!(admitted.path.is_empty());
    assert_eq!(admitted.target, DivergentTarget::Quiescent(0));
    assert_eq!(
        certificate,
        proved(claim(&four, ZoneCheckLimits::default(), &Cancel::new()))
    );

    let deadlock_free = proved(deadlock(&four));
    assert_eq!(
        deadlock_free.obligation.kind,
        ObligationKind::DeadlockFreedom
    );
    assert_edges_cover(&four, &deadlock_free);
}

/// FR-244-AC-4: over `Lock`, `always holds(c.phase = A)` is proved, and its
/// certificate lists the node `(B, x <= 1)` marked `locked`. Its `ping`
/// edges stay in locked `B` nodes, so they reach no quiescent node; and
/// every node they reach bounds `x` by 1, which no edge among them resets,
/// so no cycle there has positive total delay on repetition. The
/// `admitted` witness is the quiescent initial node `A` and enters no
/// locked node.
#[trace("TC-699", "FR-244-AC-4")]
#[test]
fn a_locked_node_is_listed_with_its_edges() {
    let world = lock();
    let certificate = proved(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    assert_edges_cover(&world, &certificate);
    let locked: Vec<usize> = (0..certificate.nodes.len())
        .filter(|&node| certificate.nodes[node].locked)
        .collect();
    let [b] = locked.as_slice() else {
        panic!("one locked node: {locked:?}");
    };
    // `x <= 1` in scaled units: the upper bound of `x` is the scale.
    let one = Ok(Some((certificate.scale.clone(), false)));
    assert_eq!(certificate.nodes[*b].zone.bound(1, 0), one);
    // Everything `B` reaches.
    let mut reached = vec![*b];
    let mut at = 0;
    while let Some(&node) = reached.get(at) {
        at += 1;
        for edge in certificate.edges.iter().filter(|edge| edge.from == node) {
            assert_eq!(
                operation(&certificate, edge_index(&certificate, edge)),
                "ping"
            );
            if !reached.contains(&edge.to) {
                reached.push(edge.to);
            }
        }
    }
    for &node in &reached {
        let node = &certificate.nodes[node];
        assert!(node.locked);
        assert_eq!(node.zone.bound(1, 0), one);
    }
    assert_eq!(certificate.admitted.target, DivergentTarget::Quiescent(0));
    assert!(certificate
        .admitted
        .path
        .iter()
        .all(|&edge| !certificate.nodes[certificate.edges[edge].to].locked));
}

fn edge_index(certificate: &Certificate, edge: &CertEdge) -> usize {
    certificate
        .edges
        .iter()
        .position(|other| other == edge)
        .expect("a listed edge")
}

/// The witness's cycle, checked to close at the path's end.
fn witness_cycle(certificate: &Certificate) -> Vec<&'static str> {
    let admitted = &certificate.admitted;
    let DivergentTarget::Cycle(cycle) = &admitted.target else {
        panic!("a cycle witness: {admitted:?}");
    };
    assert!(!cycle.is_empty());
    let end = admitted
        .path
        .last()
        .map_or(admitted.start, |&edge| certificate.edges[edge].to);
    assert_eq!(certificate.edges[cycle[0]].from, end);
    assert_eq!(certificate.edges[*cycle.last().expect("an edge")].to, end);
    for pair in cycle.windows(2) {
        assert_eq!(
            certificate.edges[pair[0]].to,
            certificate.edges[pair[1]].from
        );
    }
    cycle
        .iter()
        .map(|&edge| operation(certificate, edge))
        .collect()
}

/// A claim that holds over `body`'s model, with clocks `clocks` and
/// operations `operations`.
fn quiet(
    clocks: &[&str],
    operations: &[(&str, &[&str])],
    profile: &str,
    time: &str,
    body: &str,
) -> World {
    world(
        &document(clocks, operations),
        INFINITE,
        &format!(
            "{time}{body}{profile}\
             temporal Quiet using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(not c.retried) }}\n"
        ),
    )
}

/// FR-244 "Admitted part" with no quiescent state: `x <= 1 ms` always holds
/// as an invariant and `ping`, which needs `x >= 1 ms`, resets `x`, so time
/// diverges only around the `ping` loop. The certificate's `admitted` part
/// is a cycle of its own `ping` edges, closed at the path's end.
#[trace("TC-699")]
#[test]
fn a_proof_with_no_quiescent_state_is_admitted_by_a_cycle() {
    let heartbeat = quiet(
        &["x"],
        &[("ping", &["x"])],
        "",
        DENSE,
        "pre Ping using v on M::Call::ping { self.x >= 1 ms }\n\
         post Pinged using v on M::Call::ping { self.x = 0 ms }\n\
         time invariant Beat when { true } { self.x <= 1 ms }\n",
    );
    let certificate = proved(claim(
        &heartbeat,
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ));
    assert_edges_cover(&heartbeat, &certificate);
    assert!(witness_cycle(&certificate).iter().all(|op| *op == "ping"));
}

/// SR-1627001 FND-001: in phase 0, `x <= 1 ms` with `x` never reset there
/// and a no-op `ping`, a Zeno loop; `go` to phase 1, where `tock` after
/// `x >= 1 ms` resets `x` and returns, a divergent loop. Neither phase is
/// quiescent. The witness is the divergent `go`/`tock` cycle, never the
/// `ping` loop alone.
#[trace("TC-699")]
#[test]
fn a_zeno_loop_is_never_the_admitted_witness() {
    let world = quiet(
        &["x"],
        &[("ping", &[]), ("go", &["phase"]), ("tock", &["phase", "x"])],
        "",
        DENSE,
        "pre Ping using v on M::Call::ping { self.phase = 0 }\n\
         pre Go using v on M::Call::go { self.phase = 0 }\n\
         post Gone using v on M::Call::go { self.phase = 1 }\n\
         pre Tock using v on M::Call::tock { self.phase = 1 and self.x >= 1 ms }\n\
         post Tocked using v on M::Call::tock { self.phase = 0 and self.x = 0 ms }\n\
         time invariant Bounded when { true } { self.x <= 1 ms }\n",
    );
    let certificate = proved(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    assert_edges_cover(&world, &certificate);
    let cycle = witness_cycle(&certificate);
    assert!(
        cycle.contains(&"go") && cycle.contains(&"tock"),
        "{cycle:?}"
    );
}

/// SR-1627001 FND-002: phase 0 admits no delay (`x <= 0 ms`) and has a
/// zero-delay `ping` self-loop; `go` moves to phase 1, which bounds `x` by
/// 2 ms, and `back`, after `x >= 1 ms`, resets `x` and returns. No state is
/// quiescent, and the claim holds, admitted by the divergent `go`/`back`
/// cycle, never by the Zeno `ping` loop.
#[trace("TC-699")]
#[test]
fn an_urgent_zero_delay_loop_does_not_hide_the_divergent_one() {
    let world = quiet(
        &["x"],
        &[("ping", &[]), ("go", &["phase"]), ("back", &["phase", "x"])],
        "",
        DENSE,
        "pre Ping using v on M::Call::ping { self.phase = 0 }\n\
         pre Go using v on M::Call::go { self.phase = 0 }\n\
         post Gone using v on M::Call::go { self.phase = 1 }\n\
         pre Back using v on M::Call::back { self.phase = 1 and self.x >= 1 ms }\n\
         post Returned using v on M::Call::back { self.phase = 0 and self.x = 0 ms }\n\
         time invariant Urgent when { self.phase = 0 } { self.x <= 0 ms }\n\
         time invariant Bounded when { self.phase = 1 } { self.x <= 2 ms }\n",
    );
    let certificate = proved(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    assert_edges_cover(&world, &certificate);
    let cycle = witness_cycle(&certificate);
    assert!(
        cycle.contains(&"go") && cycle.contains(&"back"),
        "{cycle:?}"
    );
    assert!(!cycle.contains(&"ping"), "{cycle:?}");
}

/// SR-1627001 FND-003 and FND-008: under a tick source, phase 0 enables
/// `alpha`, which changes nothing, and `go` to phase 1, and phase 1
/// enables only `tick`, back to phase 0. Staying in phase 0 never ticks,
/// so the witness is the cycle `go`, `tick`, which leaves its start by the
/// tick edge's own state change. The claim and the deadlock-freedom item
/// are both proved, each admitted by that cycle.
#[trace("TC-699")]
#[test]
fn a_tick_source_admits_by_a_tick() {
    let world = quiet(
        &[],
        &[("alpha", &[]), ("go", &["phase"]), ("tick", &["phase"])],
        "",
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         time M tick M::Call::tick period 5 ms unit ms;\n",
        "pre Alpha using v on M::Call::alpha { self.phase = 0 }\n\
         pre Go using v on M::Call::go { self.phase = 0 }\n\
         post Gone using v on M::Call::go { self.phase = 1 }\n\
         pre Tick using v on M::Call::tick { self.phase = 1 }\n\
         post Ticked using v on M::Call::tick { self.phase = 0 }\n",
    );
    for certificate in [
        proved(claim(&world, ZoneCheckLimits::default(), &Cancel::new())),
        proved(deadlock(&world)),
    ] {
        let admitted = &certificate.admitted;
        assert!(
            !matches!(admitted.target, DivergentTarget::Quiescent(_)),
            "a tick source has no quiescent witness: {admitted:?}"
        );
        let cycle = witness_cycle(&certificate);
        assert!(
            cycle.contains(&"tick") && cycle.contains(&"go"),
            "{cycle:?}"
        );
    }
}

/// FR-244: a certificate names its item's obligation, so a request whose
/// obligation is not its item's is refused: a clause with
/// `deadlock-freedom`, the deadlock-freedom item with a claim's.
#[trace("TC-699")]
#[test]
fn a_request_obligation_must_be_its_items() {
    let world = rpc(4);
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("letters");
    let refused = check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::Clause {
                clause: node.clause(),
                letters: &letters,
            },
            obligation: ItemObligation {
                subject: SUBJECT,
                kind: ObligationKind::DeadlockFreedom,
            },
            limits: ZoneCheckLimits::default(),
        },
        &Cancel::new(),
    );
    assert_eq!(refused.unwrap_err(), ZoneCheckRefusal::ObligationMismatch);
    let refused = check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::DeadlockFreedom,
            obligation: ItemObligation {
                subject: SUBJECT,
                kind: ObligationKind::Authored(claim_occurrence()),
            },
            limits: ZoneCheckLimits::default(),
        },
        &Cancel::new(),
    );
    assert_eq!(refused.unwrap_err(), ZoneCheckRefusal::ObligationMismatch);
}

/// `body` as the clause `W` under the timed profile, read over model time
/// on origin, over a model with clocks `clocks` and `operations` whose
/// constraints are `model`.
pub(crate) fn windowed(
    clocks: &[&str],
    operations: &[(&str, &[&str])],
    model: &str,
    body: &str,
) -> World {
    windowed_under(DENSE, clocks, operations, model, body)
}

/// [`windowed`] under the time units and source `time`.
fn windowed_under(
    time: &str,
    clocks: &[&str],
    operations: &[(&str, &[&str])],
    model: &str,
    body: &str,
) -> World {
    world(
        &document(clocks, operations),
        TIMED,
        &format!(
            "{time}{model}\
             temporal W using tt over (c: M::Call) clock \"model-time\" on origin {{ {body} }}\n"
        ),
    )
}

/// The first clause of `world` put to EN-6, refused or decided.
fn request(world: &World) -> Result<Outcome, ZoneCheckRefusal> {
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("letters");
    check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::Clause {
                clause: node.clause(),
                letters: &letters,
            },
            obligation: ItemObligation {
                subject: SUBJECT,
                kind: ObligationKind::Authored(claim_occurrence()),
            },
            limits: ZoneCheckLimits::default(),
        },
        &Cancel::new(),
    )
}

/// `go` sets `retried` at exactly `at` ms, after which nothing happens.
pub(crate) fn retried_at(at: &str) -> World {
    retried_at_with(at, "eventually[2 ms, 2 ms] holds(c.retried)")
}

/// [`retried_at`] with `body` as its clause.
fn retried_at_with(at: &str, body: &str) -> World {
    windowed(
        &["x"],
        &[("go", &["phase", "retried"])],
        &format!(
            "pre Go using v on M::Call::go {{ self.phase = 0 and self.x >= {at} }}\n\
             post Gone using v on M::Call::go {{ self.phase = 1 and self.retried }}\n\
             time invariant Wait when {{ self.phase = 0 }} {{ self.x <= {at} }}\n"
        ),
        body,
    )
}

/// FR-242-AC-2: `eventually[2 ms, 2 ms] holds(q)` on origin translates
/// with a horizon clock. Over a model where `q` first holds at exactly 2 ms
/// EN-6 proves it, with automaton states counted; where `q` first holds at
/// 5/2 ms it is violated, at the step that passes the window.
#[trace("TC-697", "FR-242-AC-2")]
#[test]
fn a_punctual_window_is_decided_by_a_horizon_clock() {
    match claim(
        &retried_at("2 ms"),
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ) {
        ZoneCheckOutcome::Holds { stats, .. } => assert!(stats.automaton_states > 0),
        other => panic!("expected a proof, got {other:?}"),
    }
    let late = violated(claim(
        &retried_at("rational(5, 2) ms"),
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ));
    assert_eq!(late.kind, CounterexampleKind::Formula);
    assert_eq!(operations(&late), ["go"]);
    assert_eq!(delays(&late), [exact(5, 2)]);
}

/// Model letters that read every false `holds` atom as a window aggregate
/// short of data (ADR-024 WA-4), as no TT-2 clause can author today.
struct ShortOfData<'a>(&'a dyn ModelLetters);

impl ModelLetters for ShortOfData<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Ok(match self.0.atom(atom, instance, state, meter)? {
            AtomReading::False => AtomReading::Undefined(short_of_data()),
            reading => reading,
        })
    }

    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        self.0.activation(instance, state, meter)
    }

    fn activation_reads(&self) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        self.0.activation_reads()
    }

    fn reads(&self, atom: FormulaNodeId) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        self.0.reads(atom)
    }
}

fn short_of_data() -> UndefinedCause {
    UndefinedCause::InsufficientData(InsufficientData {
        count: 0,
        min_count: 1,
    })
}

/// Part of FR-242-AC-2 (TC-697): a window that read short of data is not
/// proved. Where every false reading of `retried` is instead a window
/// short of data, the evaluator reports `eventually[2 ms, 2 ms]` over
/// `retried` first holding at 5/2 ms `InsufficientData`, and EN-6 leaves
/// the claim undecided with that cause, never `Holds`. Not covered: an
/// authored aggregate atom, which no TT-2 clause has yet.
#[trace("TC-697")]
#[test]
fn a_window_short_of_data_is_insufficient_data_not_holds() {
    let world = retried_at("rational(5, 2) ms");
    let unrolled = Unrolled {
        stamps: vec![exact(0, 1), exact(5, 2)],
        p: Vec::new(),
        q: vec![false, true],
        odd: vec![Some(short_of_data()), None],
    };
    let clause = world.package.graph().temporal_clauses()[0].clause();
    let trace = TimedTrace::new(unrolled.stamps.clone()).expect("a timed trace");
    let mut meter = Meter::new(world.system().meter_limits());
    let reference = evaluate_timed_at(
        clause,
        &trace,
        TemporalPosition::new(0),
        &mut Lasso {
            unrolled: &unrolled,
            p: None,
        },
        &mut meter,
    )
    .expect("the evaluator reads the trace");
    assert!(
        matches!(reference, TemporalOutcome::InsufficientData(_)),
        "{reference:?}"
    );

    match short_of_data_claim(&world) {
        ZoneCheckOutcome::Undecided {
            cause: ZoneUndecided::InsufficientData(_),
            ..
        } => {}
        other => panic!("expected InsufficientData, got {other:?}"),
    }
}

/// The first clause of `world` put to EN-6 over [`ShortOfData`] letters.
pub(crate) fn short_of_data_claim(world: &World) -> Outcome {
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("letters");
    check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::Clause {
                clause: node.clause(),
                letters: &ShortOfData(&letters),
            },
            obligation: ItemObligation {
                subject: SUBJECT,
                kind: ObligationKind::Authored(claim_occurrence()),
            },
            limits: ZoneCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("EN-6 takes the claim")
}

/// FR-242-AC-3: over ADR-026 §11's `Rpc` (TC-697's fixture), `always
/// holds(not c.late)` under the timed profile is TT-1: with `T = 3 ms` and
/// `T = 4 ms` it builds no automaton and decides on discrete states, with
/// zero automaton states counted.
#[trace("TC-697", "FR-242-AC-3")]
#[test]
fn a_timed_invariant_builds_no_automaton() {
    for t in [3, 4] {
        let timed = rpc_claim(
            t,
            TIMED,
            "temporal NoLateReply using tt over (c: M::Call) clock \"model-time\" on origin \
             { always holds(not c.late) }\n",
        );
        let stats = match claim(&timed, ZoneCheckLimits::default(), &Cancel::new()) {
            ZoneCheckOutcome::Holds { stats, certificate } => {
                assert!(certificate
                    .nodes
                    .iter()
                    .all(|node| node.automaton_state == 0));
                stats
            }
            ZoneCheckOutcome::Violated { stats, .. } => stats,
            other => panic!("expected a decision, got {other:?}"),
        };
        assert_eq!(stats.automaton_states, 0, "T = {t}");
    }
    // As under infinite-trace (FR-239-AC-1): proved at 4, refuted at 3.
    let four = rpc_claim(
        4,
        TIMED,
        "temporal NoLateReply using tt over (c: M::Call) clock \"model-time\" on origin \
         { always holds(not c.late) }\n",
    );
    assert!(matches!(
        claim(&four, ZoneCheckLimits::default(), &Cancel::new()),
        ZoneCheckOutcome::Holds { .. }
    ));
}

/// FR-242: an interval operator under another temporal operator needs a
/// clock per activation, which one horizon clock does not give; EN-6
/// refuses it rather than deciding it wrongly.
#[trace("TC-697")]
#[test]
fn a_nested_window_is_refused() {
    let nested = windowed(
        &[],
        &[("go", &["phase"])],
        "",
        "eventually[0 ms, 3 ms] always[0 ms, 1 ms] holds(c.late)",
    );
    assert_eq!(
        request(&nested).unwrap_err(),
        ZoneCheckRefusal::NestedWindow
    );
}

/// A deterministic stream of pseudo-random numbers (xorshift64).
struct Stream(u64);

impl Stream {
    fn below(&mut self, bound: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % bound
    }
}

/// A timed lasso unrolled past every window: each position's stamp, in
/// quarters of a ms, its `p` and `q`, and what an odd reading of `q` there
/// gives: an undefined value or a window short of data.
struct Unrolled {
    stamps: Vec<Rational>,
    p: Vec<bool>,
    q: Vec<bool>,
    odd: Vec<Option<UndefinedCause>>,
}

/// A random lasso of length at most 8 with stamps in steps of 1/4 ms,
/// starting at 0 and with a loop of positive total delay, unrolled until
/// its stamps pass `beyond` ms. One position in 25 reads `q` oddly.
fn lasso(stream: &mut Stream, beyond: u64) -> Unrolled {
    let length = usize::try_from(stream.below(8) + 1).expect("small");
    let entry = usize::try_from(stream.below(length as u64)).expect("small");
    // `delay[i]`: quarters from position i - 1 to i; `delay[entry]` closes
    // the loop on repetition.
    let mut delay: Vec<u64> = (0..length).map(|_| stream.below(5)).collect();
    let mut wrap = stream.below(5);
    if wrap + delay[entry + 1..].iter().sum::<u64>() == 0 {
        wrap = 1;
    }
    delay[0] = 0;
    let p: Vec<bool> = (0..length).map(|_| stream.below(2) == 1).collect();
    let q: Vec<bool> = (0..length).map(|_| stream.below(2) == 1).collect();
    let odd: Vec<Option<UndefinedCause>> = (0..length)
        .map(|_| match stream.below(50) {
            0 => Some(UndefinedCause::Kernel(Undefined::DivisionByZero)),
            1 => Some(UndefinedCause::InsufficientData(InsufficientData {
                count: 0,
                min_count: 1,
            })),
            _ => None,
        })
        .collect();
    let mut unrolled = Unrolled {
        stamps: Vec::new(),
        p: Vec::new(),
        q: Vec::new(),
        odd: Vec::new(),
    };
    let (mut at, mut quarters) = (0, 0_u64);
    while quarters <= 4 * beyond + 4 {
        unrolled
            .stamps
            .push(exact(i64::try_from(quarters).expect("small"), 4));
        unrolled.p.push(p[at]);
        unrolled.q.push(q[at]);
        unrolled.odd.push(odd[at].clone());
        if at + 1 < length {
            at += 1;
            quarters += delay[at];
        } else {
            at = entry;
            quarters += wrap;
        }
    }
    unrolled
}

/// Reads `p` (the first `holds` atom of a two-atom formula) and `q` (the
/// other, or the only one) at each position of an unrolled lasso.
struct Lasso<'a> {
    unrolled: &'a Unrolled,
    p: Option<FormulaNodeId>,
}

impl Lasso<'_> {
    fn reading(&self, atom: FormulaNodeId, at: usize) -> AtomReading {
        let value = if Some(atom) == self.p {
            self.unrolled.p[at]
        } else if let Some(cause) = &self.unrolled.odd[at] {
            return AtomReading::Undefined(cause.clone());
        } else {
            self.unrolled.q[at]
        };
        if value {
            AtomReading::True
        } else {
            AtomReading::False
        }
    }
}

impl TemporalLetters for Lasso<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let at = usize::try_from(position.index()).expect("a small position");
        Ok(self.reading(atom, at))
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        panic!("these clauses have no `when` condition");
    }
}

/// Part of FR-242-AC-1 (TC-697): for `eventually[0 ms, 3 ms] holds(q)`,
/// `eventually[0 ms, 3 ms) holds(q)` and `holds(p) until(1 ms, 2 ms]
/// holds(q)`, and for `always`, `release`, a punctual window under `and`,
/// past operators on origin and boolean combinations of windows, on 1,000
/// random timed lassos each (stamps in steps of 1/4 ms, length at most 8,
/// unrolled past every window, `q` now and then undefined or short of a
/// window's data), the TT-2 translation's verdict is the reference
/// evaluator's: it rejects the clause exactly where the evaluator returns
/// `false`, and an undefined or short reading settles as the evaluator
/// settles it. Not covered: `always (holds(p) implies eventually[0 ms,
/// 2 ms] holds(q))`, a TT-3 form translated by MightyL (QSL-583).
#[trace("TC-697")]
#[test]
fn the_window_automaton_agrees_with_the_evaluator_on_random_lassos() {
    let formulas = [
        "eventually[0 ms, 3 ms] holds(c.retried)",
        "eventually[0 ms, 3 ms) holds(c.retried)",
        "holds(c.late) until(1 ms, 2 ms] holds(c.retried)",
        "always[1 ms, 2 ms] holds(c.retried)",
        "holds(c.late) release(1 ms, 2 ms] holds(c.retried)",
        "holds(c.late) and eventually[1 ms, 1 ms] holds(c.retried)",
        "once[0 ms, 1 ms] holds(c.late) or eventually[0 ms, 2 ms) holds(c.retried)",
        "holds(c.late) since[0 ms, 1 ms] holds(c.retried)",
        "historically[0 ms, 1 ms] holds(c.late) implies always(0 ms, 1 ms] holds(c.retried)",
    ];
    let mut stream = Stream(0x5eed_1234_abcd_ef01);
    for formula in formulas {
        let world = windowed(&[], &[("go", &["phase"])], "", formula);
        let clause = world.package.graph().temporal_clauses()[0].clause();
        let mut atoms: Vec<(usize, FormulaNodeId)> = clause
            .formula()
            .nodes()
            .iter()
            .enumerate()
            .filter_map(|(index, node)| match node {
                FormulaNode::Holds { span } => Some((
                    span.start,
                    clause
                        .formula()
                        .node_id(index)
                        .expect("a node of the formula"),
                )),
                _ => None,
            })
            .collect();
        atoms.sort_by_key(|(start, _)| *start);
        let p = (atoms.len() == 2).then(|| atoms[0].1);
        let (mut held, mut failed, mut odd) = (0, 0, 0);
        for _ in 0..1_000 {
            let unrolled = lasso(&mut stream, 3);
            let mut letters = Lasso {
                unrolled: &unrolled,
                p,
            };
            let verdict = bounded_window_verdict(clause, &unrolled.stamps, &mut |atom, at| {
                letters.reading(atom, at)
            })
            .expect("the automaton reads the lasso");
            let trace = TimedTrace::new(unrolled.stamps.clone()).expect("a timed trace");
            let mut meter = Meter::new(world.system().meter_limits());
            let reference = evaluate_timed_at(
                clause,
                &trace,
                TemporalPosition::new(0),
                &mut letters,
                &mut meter,
            )
            .expect("the evaluator reads the lasso");
            assert_eq!(
                verdict, reference,
                "{formula} on {:?}, p {:?}, q {:?}, odd {:?}",
                unrolled.stamps, unrolled.p, unrolled.q, unrolled.odd
            );
            match reference {
                TemporalOutcome::Holds => held += 1,
                TemporalOutcome::Violated { .. } => failed += 1,
                _ => odd += 1,
            }
        }
        // Both answers occur, and odd readings too, so the comparison is
        // not vacuous.
        assert!(
            held > 0 && failed > 0 && odd > 0,
            "{formula}: {held} held, {failed} failed, {odd} odd"
        );
    }
}

/// SR-1631001 FND-001: under a tick source of period 1 ms whose `tick` is
/// always enabled, position `i` is stamped `i` ms, so `eventually[0 ms,
/// 3 ms] holds(c.retried)`, with `retried` never set, is violated at the
/// fourth tick, the first position past the window.
#[trace("TC-697")]
#[test]
fn a_window_over_a_tick_source_is_read_at_each_tick() {
    let ticks = windowed_under(
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         time M tick M::Call::tick period 1 ms unit ms;\n",
        &[],
        &[("tick", &[])],
        "",
        "eventually[0 ms, 3 ms] holds(c.retried)",
    );
    let counterexample = violated(claim(&ticks, ZoneCheckLimits::default(), &Cancel::new()));
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(
        operations(&counterexample),
        ["tick", "tick", "tick", "tick"]
    );
    assert_eq!(counterexample.lasso, None);
}

/// FR-242 at the product level: over the model whose `go` sets `retried`
/// at exactly 2 ms, an open end excludes that instant and a closed end
/// includes it, so `always[0 ms, 2 ms) holds(not c.retried)` is proved and
/// `always[0 ms, 2 ms] holds(not c.retried)` is violated by `go` after
/// 2 ms.
#[trace("TC-697")]
#[test]
fn an_open_and_a_closed_end_are_read_on_their_own_pieces() {
    let open = retried_at_with("2 ms", "always[0 ms, 2 ms) holds(not c.retried)");
    assert!(matches!(
        claim(&open, ZoneCheckLimits::default(), &Cancel::new()),
        ZoneCheckOutcome::Holds { .. }
    ));
    let closed = retried_at_with("2 ms", "always[0 ms, 2 ms] holds(not c.retried)");
    let counterexample = violated(claim(&closed, ZoneCheckLimits::default(), &Cancel::new()));
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(operations(&counterexample), ["go"]);
    assert_eq!(delays(&counterexample), [exact(2, 1)]);
}

/// FR-242 with an undefined atom: over `Ticker`, `always[0 ms, 3 ms]
/// holds(6 mod (2 - c.phase) >= 0)` is still open when `phase` reaches 2
/// at 2 ms, where the atom is undefined, and the clause is refuted there
/// with an undefined evaluation of cause `division-by-zero`.
#[trace("TC-697")]
#[test]
fn an_undefined_atom_inside_a_window_refutes_it() {
    let world = ticker_with(false, "always[0 ms, 3 ms] holds(6 mod (2 - c.phase) >= 0)");
    let counterexample = violated(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    let CounterexampleKind::UndefinedEvaluation(undefined) = &counterexample.kind else {
        panic!("an undefined evaluation: {:?}", counterexample.kind);
    };
    assert_eq!(undefined.site.position, 2);
    assert_eq!(
        undefined.cause,
        UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
    assert_eq!(operations(&counterexample), ["step", "step"]);
}

/// The model in which no step is ever enabled: every behaviour stays at
/// its initial state, `retried` false throughout.
pub(crate) fn idle(body: &str) -> World {
    windowed(
        &[],
        &[("go", &["phase"])],
        "pre Go using v on M::Call::go { self.phase = 1 }\n",
        body,
    )
}

/// FR-242 with FR-231's idle tail: where no step is ever enabled,
/// `eventually[0 ms, 3 ms] holds(c.retried)` is violated only by staying:
/// a formula counterexample with no step, whose loop is the terminal
/// stutter.
#[trace("TC-697")]
#[test]
fn an_idle_tail_passes_a_window() {
    let counterexample = violated(claim(
        &idle("eventually[0 ms, 3 ms] holds(c.retried)"),
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ));
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert!(counterexample.prefix.is_empty());
    let lasso = counterexample.lasso.as_ref().expect("a lasso");
    assert!(lasso.terminal_stutter);
    assert_eq!(lasso.entry, 0);
}

/// FR-242 with a lower-bounded window that only the idle tail reaches.
/// The tail's stutter positions come after any positive delays (FR-231),
/// so some idle behaviour has none inside `[2 ms, 3 ms]` and some has one
/// there: `eventually[2 ms, 3 ms] holds(not c.retried)`, true at every
/// position, is violated by a stutter past 3 ms, and `always[2 ms, 3 ms]
/// holds(c.retried)` by one inside the window, each with no step.
#[trace("TC-697")]
#[test]
fn a_lower_bounded_window_is_decided_by_the_idle_tail() {
    for body in [
        "eventually[2 ms, 3 ms] holds(not c.retried)",
        "always[2 ms, 3 ms] holds(c.retried)",
    ] {
        let counterexample = violated(claim(
            &idle(body),
            ZoneCheckLimits::default(),
            &Cancel::new(),
        ));
        assert_eq!(counterexample.kind, CounterexampleKind::Formula, "{body}");
        assert!(counterexample.prefix.is_empty(), "{body}");
        assert!(
            counterexample
                .lasso
                .as_ref()
                .is_some_and(|lasso| lasso.terminal_stutter),
            "{body}"
        );
    }
    // A window that position 0 already decides needs no stutter:
    // `eventually[0 ms, 3 ms] holds(not c.retried)` is proved.
    assert!(matches!(
        claim(
            &idle("eventually[0 ms, 3 ms] holds(not c.retried)"),
            ZoneCheckLimits::default(),
            &Cancel::new()
        ),
        ZoneCheckOutcome::Holds { .. }
    ));
}
