// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-240's liveness phase on the symbolic graph (TC-695) and FR-241's
//! concretisation of its lassos (TC-696, TC-691), each counterexample
//! replayed by FR-237.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{ZoneCheckLimits, ZoneCheckOutcome, ZoneUndecided};
use qsl_replay::{CounterexampleKind, TemporalCounterexample};
use quire_exact::{Cancel, Integer, Rational};

use crate::timed_world::{document, world, World, DENSE};
use crate::zone_replay::{replay, reproduced};
use crate::zone_search::{claim, delays, rpc_claim, violated, windowed, Outcome, INFINITE};

fn decide(world: &World) -> Outcome {
    claim(world, ZoneCheckLimits::default(), &Cancel::new())
}

fn holds(outcome: &Outcome) -> bool {
    matches!(outcome, ZoneCheckOutcome::Holds { .. })
}

/// `outcome`'s lasso counterexample, replayed to `Reproduced`.
fn reproduced_lasso(world: &World, outcome: Outcome) -> TemporalCounterexample {
    let counterexample = violated(outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert!(counterexample.lasso.is_some(), "{counterexample:?}");
    let settlement = replay(world, &counterexample).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");
    counterexample
}

/// The operation names of the steps of `counterexample` from `from`.
fn names(counterexample: &TemporalCounterexample, from: usize) -> Vec<String> {
    crate::zone_search::operations(counterexample).split_off(from)
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

/// The total delay of the loop of `counterexample`, from its entry.
fn loop_delay(counterexample: &TemporalCounterexample) -> Rational {
    let entry = counterexample.lasso.expect("a lasso").entry;
    delays(counterexample)[entry..]
        .iter()
        .fold(zero(), |sum, delay| sum.add(delay))
}

/// FR-240-AC-1's response claim over `Rpc` with deadline `t`, with
/// `extra` added to the model.
fn response(t: u32, extra: &str) -> World {
    rpc_claim(
        t,
        INFINITE,
        &format!(
            "{extra}temporal Answered using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always (holds(c.phase = 1) implies eventually holds(c.phase = 2)) }}\n"
        ),
    )
}

/// FR-240-AC-1: over `Rpc` with `T = 4 ms` every request is answered:
/// the server's deadline forces `reply` before the client's. In a variant
/// where `reply` may also loop on itself with no reset, a lasso of `reply`
/// steps never answers, but `x` is never reset under the server's deadline
/// `x <= 3 ms`, so its total delay is bounded: it is Zeno and not reported,
/// and the result is still `Holds`.
#[trace("TC-695", "FR-240-AC-1")]
#[test]
fn a_response_claim_holds_and_a_zeno_loop_is_not_reported() {
    assert!(holds(&decide(&response(4, ""))));
    let looping = rpc_reply_loop();
    let outcome = decide(&looping);
    assert!(holds(&outcome), "{outcome:?}");
    // An `echo` that resets `x` repeats with positive delays forever: the
    // claim is violated by that loop.
    let resetting = rpc_echo();
    let counterexample = reproduced_lasso(&resetting, decide(&resetting));
    let entry = counterexample.lasso.expect("a lasso").entry;
    assert!(names(&counterexample, entry)
        .iter()
        .all(|name| name == "echo"));
}

/// FR-240-AC-1's variant of [`response`] over `Rpc` with `T = 4 ms`, where
/// `reply` may also loop on itself, changing nothing and resetting no
/// clock. An operation has one guard, not one per outcome of its post, so
/// `reply` is guarded by `self.inflight` alone: the loop is enabled at
/// `x < 1 ms`, as the AC names, and at every later `x` the server's
/// deadline allows, which bounds its total delay just the same.
fn rpc_reply_loop() -> World {
    world(
        &crate::zone_search::rpc_document(),
        INFINITE,
        &format!(
            "{DENSE}\
             pre SendIdle using v on M::Call::send {{ self.phase = 0 }}\n\
             post Send using v on M::Call::send {{ \
                 self.phase = 1 and self.inflight and not self.late and self.x = 0 ms }}\n\
             pre Reply using v on M::Call::reply {{ self.inflight }}\n\
             post Replied using v on M::Call::reply {{ \
                 (self.inflight and self.phase = pre(self.phase) and \
                     self.late = pre(self.late)) or \
                 (not self.inflight and self.late = (pre(self.phase) = 3) and \
                     (pre(self.phase) = 1 implies self.phase = 2) and \
                     (pre(self.phase) != 1 implies self.phase = pre(self.phase))) }}\n\
             pre Timeout using v on M::Call::timeout {{ self.phase = 1 and self.x >= 4 ms }}\n\
             post TimedOut using v on M::Call::timeout {{ self.phase = 3 }}\n\
             pre Reset using v on M::Call::reset {{ \
                 not self.inflight and (self.phase = 2 or self.phase = 3) }}\n\
             post ResetIdle using v on M::Call::reset {{ self.phase = 0 }}\n\
             time invariant ServerDeadline when {{ self.inflight }} {{ self.x <= 3 ms }}\n\
             time invariant ClientDeadline when {{ self.phase = 1 }} {{ self.x <= 4 ms }}\n\
             temporal Answered using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always (holds(c.phase = 1) implies eventually holds(c.phase = 2)) }}\n"
        ),
    )
}

/// [`response`] over `Rpc` with an `echo` step at a waiting call, guarded
/// `x < 1 ms`, that only resets `x`.
fn rpc_echo() -> World {
    let document = crate::zone_search::rpc_document_with(&[("echo", &["x"])]);
    crate::zone_search::rpc_world(
        &document,
        4,
        INFINITE,
        "pre Echo using v on M::Call::echo { self.phase = 1 and self.x < 1 ms }\n\
         post Echoed using v on M::Call::echo { self.x = 0 ms }\n\
         temporal Answered using inf over (c: M::Call) clock \"model-steps\" on origin \
         { always (holds(c.phase = 1) implies eventually holds(c.phase = 2)) }\n",
    )
}

/// FR-232-AC-4's `Serve`: clock `x` never reset, `idle` with no guard and
/// no data change, `serve` guarded `x {guard}` setting `retried` (the spec's
/// `done`), and the claim
/// `eventually holds(c.retried)` under the fairness set `fairness`.
fn serve(guard: &str, fairness: &str) -> World {
    world(
        &document(&["x"], &[("idle", &[]), ("serve", &["retried"])]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Serve using v on M::Call::serve {{ not self.retried and self.x {guard} }}\n\
             post Served using v on M::Call::serve {{ self.retried }}\n\
             temporal Done using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ {fairness} eventually holds(c.retried) }}\n"
        ),
    )
}

/// FR-240-AC-2 and FR-241-AC-3: in `Serve` with guard `x >= 1 ms`,
/// `eventually holds(done)` holds under `fair weak serve`; with no
/// fairness it is violated by a lasso of `idle` steps whose loop has
/// positive total delay, with `x` above 1 at its entry. Under `fair weak
/// serve` with guard `x <= 1 ms`, the same loop, where `serve` is disabled
/// at every instant, is fair, so the claim is violated (FR-232-AC-4).
#[trace("TC-695", "FR-240-AC-2")]
#[trace("TC-696", "FR-241-AC-3")]
#[test]
fn fairness_is_read_over_time_on_the_split_graph() {
    let fair = serve(">= 1 ms", "fair weak M::Call::serve;");
    let outcome = decide(&fair);
    assert!(holds(&outcome), "{outcome:?}");

    let free = serve(">= 1 ms", "");
    let counterexample = reproduced_lasso(&free, decide(&free));
    let lasso = counterexample.lasso.expect("a lasso");
    assert!(!lasso.terminal_stutter);
    assert_eq!(names(&counterexample, 0), ["idle", "idle"]);
    assert_eq!(lasso.entry, 1);
    assert!(loop_delay(&counterexample) > zero());
    let to_entry = delays(&counterexample)[..lasso.entry]
        .iter()
        .fold(zero(), |sum, delay| sum.add(delay));
    assert!(
        to_entry > Rational::from_integer(Integer::one()),
        "{counterexample:?}"
    );

    let closed = serve("<= 1 ms", "fair weak M::Call::serve;");
    reproduced_lasso(&closed, decide(&closed));
}

/// Part of FR-240-AC-4 (TC-695): the TT-4 claim `always eventually
/// holds(true)` under the timed profile over `Stall` returns
/// `Undecided(NoAdmittedBehaviour)`. Not covered: the TT-1 half, FR-239's
/// vacuity check, tested with FR-239.
#[trace("TC-695")]
#[test]
fn a_liveness_claim_over_no_admitted_behaviour_is_undecided() {
    let stalled = windowed(
        &["x"],
        &[("ping", &[])],
        "time invariant Stall when { true } { self.x <= 1 ms }\n",
        "always eventually holds(true)",
    );
    assert!(matches!(
        decide(&stalled),
        ZoneCheckOutcome::Undecided {
            cause: ZoneUndecided::NoAdmittedBehaviour,
            ..
        }
    ));
}

/// FR-236-AC-3: `always eventually holds(c.phase = 2)` over `Rpc` with
/// `T = 3 ms` is refuted by a lasso through the timeout whose loop has
/// positive total delay. It replays, and FR-237's replay checks that the
/// loop ends in its entry's discrete state and that every clock the loop
/// does not reset is above every constant at its entry.
#[trace("TC-691", "FR-236-AC-3")]
#[test]
fn a_liveness_refutation_is_a_timed_lasso() {
    let world = rpc_claim(
        3,
        INFINITE,
        "temporal Replies using inf over (c: M::Call) clock \"model-steps\" on origin \
         { always eventually holds(c.phase = 2) }\n",
    );
    let counterexample = reproduced_lasso(&world, decide(&world));
    let lasso = counterexample.lasso.expect("a lasso");
    assert!(!lasso.terminal_stutter);
    assert!(loop_delay(&counterexample) > zero());
}

/// FR-240 under strong fairness: in `Serve` with guard `x >= 1 ms`, every
/// divergent behaviour has `serve` enabled infinitely often, so under
/// `fair strong serve` the claim holds.
#[trace("TC-695")]
#[test]
fn strong_fairness_is_read_over_time() {
    let fair = serve(">= 1 ms", "fair strong M::Call::serve;");
    let outcome = decide(&fair);
    assert!(holds(&outcome), "{outcome:?}");
}

/// A model under a tick source of period 1 ms: `tick` at phase 0, `work`
/// from phase 0 to 1, and `spin` at phase 1, which never ticks; the claim
/// `body` under infinite-trace.
fn ticking(body: &str) -> World {
    world(
        &document(&[], &[("work", &["phase"]), ("tick", &[]), ("spin", &[])]),
        INFINITE,
        &format!(
            "dimension Time;\nunit s : Time = rational(1, 1);\n\
             unit ms : Time = rational(1, 1000) * s;\n\
             time M tick M::Call::tick period 1 ms unit ms;\n\
             pre Tick using v on M::Call::tick {{ self.phase = 0 }}\n\
             pre Work using v on M::Call::work {{ self.phase = 0 }}\n\
             post Worked using v on M::Call::work {{ self.phase = 1 }}\n\
             pre Spin using v on M::Call::spin {{ self.phase = 1 }}\n\
             temporal Ticks using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ {body} }}\n"
        ),
    )
}

/// FR-240 under a tick source, where time advances by ticks: ticking
/// forever at phase 0 refutes `eventually holds(c.phase = 1)` by a lasso
/// of ticks, while the `spin` loop at phase 1 never ticks, so it admits no
/// behaviour and `always eventually holds(c.phase = 0)` holds.
#[trace("TC-695")]
#[test]
fn a_tick_source_diverges_by_its_ticks() {
    let idle = ticking("eventually holds(c.phase = 1)");
    let counterexample = reproduced_lasso(&idle, decide(&idle));
    let entry = counterexample.lasso.expect("a lasso").entry;
    assert!(names(&counterexample, entry)
        .iter()
        .all(|name| name == "tick"));
    let spinning = ticking("always eventually holds(c.phase = 0)");
    let outcome = decide(&spinning);
    assert!(holds(&outcome), "{outcome:?}");
}

/// FR-240 with an undefined atom: over `Ticker`, `6 mod (2 - n)` is
/// undefined at `n = 2`, reached by `step` after delay 1 twice at a
/// quiescent state, so `always eventually holds(6 mod (2 - c.phase) >= 0)`
/// is refuted there by an undefined evaluation of cause
/// `division-by-zero`, which replays.
#[trace("TC-695")]
#[test]
fn an_undefined_position_refutes_a_liveness_claim() {
    let world =
        crate::zone_search::ticker_with(false, "always eventually holds(6 mod (2 - c.phase) >= 0)");
    let counterexample = violated(decide(&world));
    assert_eq!(names(&counterexample, 0), ["step", "step"]);
    assert_eq!(
        delays(&counterexample),
        [
            Rational::from_integer(Integer::one()),
            Rational::from_integer(Integer::one())
        ]
    );
    let CounterexampleKind::UndefinedEvaluation(undefined) = &counterexample.kind else {
        panic!("an undefined evaluation: {:?}", counterexample.kind);
    };
    assert_eq!(undefined.site.position, 2);
    assert_eq!(
        undefined.cause,
        qsl_eval::temporal::UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
    let settlement = replay(&world, &counterexample).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");
}

/// FR-240 reads a clause over positions only where that is its meaning: a
/// timed-profile liveness clause with a timed interval counts time, which
/// FR-242's timed automata read, so EN-6 refuses it as a form it does not
/// decide yet.
#[trace("TC-695")]
#[test]
fn a_timed_interval_outside_a_window_is_refused() {
    let world = windowed(
        &["x"],
        &[("ping", &[])],
        "",
        "always eventually[0 ms, 3 ms] holds(true)",
    );
    assert_eq!(
        crate::zone_search::request(&world).err(),
        Some(qsl_analyze::zone_check::ZoneCheckRefusal::UnsupportedForm)
    );
}

/// FR-240 "Vacuity" under fairness: `ping` resets `x` under `x <= 1 ms`
/// at phase 0, and `serve`, always enabled there, resets `x` and leads to
/// phase 1, a time-lock at `x = 0`. Every time-divergent behaviour pings forever with
/// `serve` enabled at every instant, so under `fair weak serve` none is
/// fair and no state is quiescent: `always eventually holds(true)` is
/// `Undecided(NoAdmittedBehaviour)`. With no fairness it holds.
#[trace("TC-695")]
#[test]
fn a_subject_with_no_fair_divergent_behaviour_admits_none() {
    let model = |fairness: &str| {
        world(
            &document(&["x"], &[("ping", &["x"]), ("serve", &["phase", "x"])]),
            INFINITE,
            &format!(
                "{DENSE}\
                 pre Ping using v on M::Call::ping {{ self.phase = 0 }}\n\
                 post Pinged using v on M::Call::ping {{ self.x = 0 ms }}\n\
                 pre Serve using v on M::Call::serve {{ self.phase = 0 }}\n\
                 post Served using v on M::Call::serve {{ self.phase = 1 and self.x = 0 ms }}\n\
                 time invariant Busy when {{ self.phase = 0 }} {{ self.x <= 1 ms }}\n\
                 time invariant Done when {{ self.phase = 1 }} {{ self.x <= 0 ms }}\n\
                 temporal Live using inf over (c: M::Call) clock \"model-steps\" on origin \
                 {{ {fairness} always eventually holds(true) }}\n"
            ),
        )
    };
    assert!(holds(&decide(&model(""))));
    assert!(matches!(
        decide(&model("fair weak M::Call::serve;")),
        ZoneCheckOutcome::Undecided {
            cause: ZoneUndecided::NoAdmittedBehaviour,
            ..
        }
    ));
}

/// A model where `serve` moves phase 0 to phase 1 and sets `retried`, and
/// phase 1 holds `x <= 1 ms` with `x` never reset, so `serve` is enabled
/// only while `x <= 1` (FR-232: its target's time invariants must hold).
/// With `looping`, phase 0 holds `y <= 1 ms` and `ping`, guarded `y >=
/// 1 ms`, resets `y`; otherwise phase 0 is quiescent. The claim is
/// `eventually holds(c.retried)` under `fairness`.
fn late_serve(looping: bool, fairness: &str) -> World {
    let serve: (&str, &[&str]) = ("serve", &["phase", "retried"]);
    let ping: (&str, &[&str]) = ("ping", &["y"]);
    let document = if looping {
        document(&["x", "y"], &[serve, ping])
    } else {
        document(&["x"], &[serve])
    };
    let model = if looping {
        "pre Ping using v on M::Call::ping { self.phase = 0 and self.y >= 1 ms }\n\
         post Pinged using v on M::Call::ping { self.y = 0 ms }\n\
         time invariant Pinging when { self.phase = 0 } { self.y <= 1 ms }\n"
    } else {
        ""
    };
    world(
        &document,
        INFINITE,
        &format!(
            "{DENSE}{model}\
             pre Serve using v on M::Call::serve {{ self.phase = 0 }}\n\
             post Served using v on M::Call::serve {{ self.phase = 1 and self.retried }}\n\
             time invariant Served when {{ self.phase = 1 }} {{ self.x <= 1 ms }}\n\
             temporal Retries using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ {fairness} eventually holds(c.retried) }}\n"
        ),
    )
}

/// SR-1643001 FND-001: FR-232 reads a step as enabled only where its
/// target's time invariants hold, so once `x > 1` `serve` is disabled and a
/// behaviour that never serves is fair under `fair weak serve` and `fair
/// strong serve`. At a quiescent phase 0 its idle tail refutes `eventually
/// holds(c.retried)`; where phase 0 must `ping`, the `ping` loop does. Each
/// counterexample replays under the clause's fairness.
#[trace("TC-695")]
#[test]
fn enabledness_reads_the_target_time_invariants() {
    for fairness in ["fair weak M::Call::serve;", "fair strong M::Call::serve;"] {
        let idle = late_serve(false, fairness);
        let counterexample = reproduced_lasso(&idle, decide(&idle));
        assert!(counterexample.lasso.expect("a lasso").terminal_stutter);
        let looping = late_serve(true, fairness);
        let counterexample = reproduced_lasso(&looping, decide(&looping));
        let entry = counterexample.lasso.expect("a lasso").entry;
        assert!(
            names(&counterexample, entry)
                .iter()
                .all(|name| name == "ping"),
            "{fairness}"
        );
    }
}
