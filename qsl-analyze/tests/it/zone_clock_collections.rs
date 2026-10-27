// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120-AC-26 (TC-1270) end to end through EN-6: a granted clock
//! collection with a declared maximum is a finite domain, so the zone search
//! takes a model that grants `ts: Sequence<Timer>[0, N]` and reaches its
//! values, each slot's clock a place.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{ZoneCheckLimits, ZoneCheckOutcome};
use quire_exact::Cancel;

use crate::timed_world::{timers_document, world_holding, World, DENSE};
use crate::zone_search::{claim, deadlock, operations, violated, INFINITE};

/// `ts: Sequence<Timer>[0, max]` (`Timer { x: Clock; n: Phase }`) granted by
/// `grow`, with the precondition `guard` and the further text `claim`.
fn timers(max: u64, guard: &str, claim: &str) -> World {
    world_holding(
        &timers_document(max, &[("grow", &["ts"])]),
        INFINITE,
        &format!("{DENSE}pre Grow using v on M::Call::grow {{ {guard} }}\n{claim}"),
        &serde_json::json!({"ts": {"sequence": []}}),
    )
}

const SHORT: &str = "temporal Short using inf over (c: M::Call) clock \"model-steps\" on origin \
                     { always holds(size(c.ts) < 3) }\n";

/// Over `ts: Sequence<Timer>[0, 3]` and `grow`, which grants `ts` and
/// constrains nothing, EN-6 takes the model (no requires-bound refusal):
/// the deadlock-freedom item holds, and `always holds(size(c.ts) < 3)` is
/// violated by one `grow` to a three-element `ts`.
#[trace("TC-1270")]
#[test]
fn the_zone_search_reaches_a_granted_clock_collection() {
    assert!(
        matches!(
            deadlock(&timers(3, "true", "")),
            ZoneCheckOutcome::Holds { .. }
        ),
        "the deadlock-freedom item holds"
    );
    let counterexample = violated(claim(
        &timers(3, "true", SHORT),
        ZoneCheckLimits::default(),
        &Cancel::new(),
    ));
    assert_eq!(operations(&counterexample), ["grow"]);
}

/// Over `ts: Sequence<Timer>[0, 1]`, `grow` enabled only while every
/// element's clock reads at most 2 ms: each slot's clock is a place the
/// quantified guard reads, so once the element's clock passes 2 ms nothing
/// is enabled, and the deadlock-freedom item does not hold.
#[trace("TC-1270")]
#[test]
fn the_zone_search_reads_the_clocks_of_a_granted_collection() {
    let outcome = deadlock(&timers(1, "forall(e in self.ts: e.x <= 2 ms)", ""));
    assert!(
        !matches!(outcome, ZoneCheckOutcome::Holds { .. }),
        "a deadlock is reached: {outcome:?}"
    );
}
