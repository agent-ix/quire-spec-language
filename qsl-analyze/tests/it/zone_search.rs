// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-239 (TC-694, TC-710) and FR-241's finite paths (TC-696): EN-6's
//! symbolic zone search over the timed fixtures of `crate::timed_world`.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{
    check_timed_model, DivergentTarget, ZoneCertificate, ZoneCheckItem, ZoneCheckLimit,
    ZoneCheckLimits, ZoneCheckOutcome, ZoneCheckRequest, ZoneStop, ZoneUndecided,
};
use qsl_eval::simulation::model::{ModelSystem, TimedSystem};
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_replay::{CounterexampleKind, TemporalCounterexample, TimedPoint, UndefinedLocus};
use qsl_route::deadlock::{ItemObligation, ObligationKind};
use quire_exact::{Cancel, Integer, Origin, Rational, Role, Undefined};

use crate::timed_world::{document, world, World, DENSE};

const INFINITE: &str = "profile inf = \"quire.temporal.infinite-trace/v1\";\n";
const TIMED: &str = "profile tt = \"quire.temporal.timed/v1\";\n";

fn rpc_document() -> Vec<u8> {
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
fn rpc(t: u32) -> World {
    world(
        &rpc_document(),
        INFINITE,
        &format!(
            "{DENSE}\
             pre SendIdle using v on M::Call::send {{ self.phase = 0 }}\n\
             post Send using v on M::Call::send {{ \
                 self.phase = 1 and self.inflight and not self.late and self.x = 0 ms }}\n\
             pre Reply using v on M::Call::reply {{ self.inflight and self.x >= 1 ms }}\n\
             post Replied using v on M::Call::reply {{ not self.inflight and \
                 self.late = (pre(self.phase) = 3) and \
                 (pre(self.phase) = 1 implies self.phase = 2) and \
                 (pre(self.phase) != 1 implies self.phase = pre(self.phase)) }}\n\
             pre Timeout using v on M::Call::timeout {{ self.phase = 1 and self.x >= {t} ms }}\n\
             post TimedOut using v on M::Call::timeout {{ self.phase = 3 }}\n\
             pre Reset using v on M::Call::reset {{ \
                 not self.inflight and (self.phase = 2 or self.phase = 3) }}\n\
             post ResetIdle using v on M::Call::reset {{ self.phase = 0 }}\n\
             time invariant ServerDeadline when {{ self.inflight }} {{ self.x <= 3 ms }}\n\
             time invariant ClientDeadline when {{ self.phase = 1 }} {{ self.x <= {t} ms }}\n\
             temporal NoLateReply using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(not c.late) }}\n"
        ),
    )
}

fn exact(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("a rational")
}

/// The subject every item here is over.
const SUBJECT: &str = "c";

/// An outcome over [`SUBJECT`].
type Outcome = ZoneCheckOutcome<&'static str>;

/// The occurrence of the claim each world's first clause is.
fn claim_occurrence() -> OccurrenceKey {
    OccurrenceKey::new(
        WireNodeId::from_digest([1; 32]),
        Origin::new(Role::new("temporal"), 0),
    )
}

/// The first clause of `world`, decided by EN-6 under `limits`.
fn claim(world: &World, limits: ZoneCheckLimits, cancel: &Cancel) -> Outcome {
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("letters");
    check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::Invariant {
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

fn deadlock(world: &World) -> Outcome {
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

fn violated(outcome: Outcome) -> TemporalCounterexample {
    match outcome {
        ZoneCheckOutcome::Violated { counterexample, .. } => *counterexample,
        other => panic!("expected a violation, got {other:?}"),
    }
}

fn delays(counterexample: &TemporalCounterexample) -> Vec<Rational> {
    counterexample
        .prefix
        .iter()
        .map(|step| step.delay.clone().expect("a timed step carries its delay"))
        .collect()
}

fn operations(counterexample: &TemporalCounterexample) -> Vec<String> {
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
fn ticker(locked: bool) -> World {
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
             {{ always holds(6 mod (2 - c.phase) >= 0) }}\n"
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
fn lock() -> World {
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
fn proved(outcome: Outcome) -> ZoneCertificate<&'static str> {
    match outcome {
        ZoneCheckOutcome::Holds { certificate, .. } => *certificate,
        other => panic!("expected a proof, got {other:?}"),
    }
}

/// The operation an edge's transition names.
fn operation(certificate: &ZoneCertificate<&'static str>, edge: usize) -> &'static str {
    let operation = certificate.edges[edge].transition.operation();
    ["send", "reply", "timeout", "reset", "go", "ping"]
        .into_iter()
        .find(|name| operation.contains(&format!("Call/{name}\"")))
        .unwrap_or("?")
}

/// Part of FR-244-AC-1 (TC-699): `NoLateReply` over `Rpc` with `T = 4 ms`
/// is proved with a certificate naming the claim's obligation, with a
/// reachability part and no component or divergence part, every edge
/// joining listed nodes, and an `admitted` path from the initial node to
/// the quiescent `Idle` node; two runs give equal certificates, and the
/// deadlock-freedom item's names `deadlock-freedom`. Not covered: FR-245
/// accepting it, which checks each edge's coverage (QSL-582).
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
    assert!(certificate.components.is_none());
    assert!(certificate.divergence.is_none());
    assert_eq!(certificate.initial, [0]);
    assert_eq!(certificate.lu_bounds.len(), 1);
    let size = certificate.nodes.len();
    assert!(size > 1);
    assert!(certificate
        .edges
        .iter()
        .all(|edge| edge.from < size && edge.to < size));
    // Edges by source in discovery order.
    assert!(certificate
        .edges
        .windows(2)
        .all(|pair| pair[0].from <= pair[1].from));
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
}

/// FR-244-AC-4: over `Lock`, `always holds(c.phase = A)` is proved, and its
/// certificate lists the node `(B, x <= 1)` marked `locked`, whose `ping`
/// edges stay in locked `B` nodes, so they reach no quiescent node; the
/// `admitted` witness is the quiescent initial node `A`.
#[trace("TC-699", "FR-244-AC-4")]
#[test]
fn a_locked_node_is_listed_with_its_edges() {
    let certificate = proved(claim(&lock(), ZoneCheckLimits::default(), &Cancel::new()));
    let locked: Vec<usize> = (0..certificate.nodes.len())
        .filter(|&node| certificate.nodes[node].locked)
        .collect();
    let [b] = locked.as_slice() else {
        panic!("one locked node: {locked:?}");
    };
    // `x <= 1` in scaled units: the upper bound of `x` is the scale.
    let zone = &certificate.nodes[*b].zone;
    assert_eq!(
        zone.bound(1, 0).expect("x"),
        &qsl_analyze::zone_check::Bound::le(certificate.scale.clone())
    );
    let leaving: Vec<usize> = (0..certificate.edges.len())
        .filter(|&edge| certificate.edges[edge].from == *b)
        .collect();
    assert!(!leaving.is_empty());
    for edge in leaving {
        assert_eq!(operation(&certificate, edge), "ping");
        assert!(certificate.nodes[certificate.edges[edge].to].locked);
    }
    assert_eq!(certificate.admitted.target, DivergentTarget::Quiescent(0));
    assert!(!certificate.nodes[0].locked);
}

/// FR-244 "Admitted part" with no quiescent state: `x <= 1 ms` always holds
/// as an invariant and `ping`, which needs `x >= 1 ms`, resets `x`, so time
/// diverges only around the `ping` loop. The certificate's `admitted` part
/// is a cycle of its own edges, from and back to the initial node.
#[trace("TC-699")]
#[test]
fn a_proof_with_no_quiescent_state_is_admitted_by_a_cycle() {
    let heartbeat = world(
        &document(&["x"], &[("ping", &["x"])]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Ping using v on M::Call::ping {{ self.x >= 1 ms }}\n\
             post Pinged using v on M::Call::ping {{ self.x = 0 ms }}\n\
             time invariant Beat when {{ true }} {{ self.x <= 1 ms }}\n\
             temporal Quiet using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(not c.retried) }}\n"
        ),
    );
    let certificate = proved(claim(
        &heartbeat,
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ));
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
    for &edge in cycle {
        assert_eq!(operation(&certificate, edge), "ping");
    }
}
