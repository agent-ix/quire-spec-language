// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120-AC-26 (TC-1270) end to end through EN-6: a granted clock
//! collection with a declared maximum is a finite domain, so the zone search
//! takes a model that grants `ts: Sequence<Timer>[0, N]` and reaches its
//! values, each slot's clock a place.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{ZoneCheckLimits, ZoneCheckOutcome};
use quire_exact::Cancel;

use crate::timed_world::{
    ident, timers_document, world_built, world_holding, World, DENSE, PACKAGE,
};
use crate::zone_search::{claim, deadlock, deadlock_result, operations, violated, INFINITE};

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

/// SR-1688001 FND-003 and SR-1671001 FND-005: a step over a granted
/// `Set<Timer>[0, 2]` or `Bag<Timer>[0, 2]` matches elements by equal data
/// and may carry a clock to another slot, which a zone over places does not
/// map yet (QSL-1162). EN-6 refuses the model `CarriedClocks` before any
/// state is explored, whatever the item, and before a selected symmetry
/// over `c1` and `c2` applies (FR-430): no orbit is formed over a carried
/// clock.
#[trace("TC-1270")]
#[test]
fn the_zone_search_refuses_a_set_or_bag_of_clocked_elements() {
    use qsl_analyze::zone_check::ZoneCheckRefusal;
    use qsl_eval::simulation::model::{ModelSystem, TimedSystem};
    use qsl_route::reductions::{Reductions, SymmetryDeclaration};
    use quire_checked::key::DeclarationKey;

    let swap = Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: DeclarationKey {
                package: PACKAGE.to_owned(),
                node: ident("calls"),
            },
            classes: vec![vec!["c1".to_owned(), "c2".to_owned()]],
        }],
        ..Reductions::default()
    };
    for (unique, empty) in [(true, "set"), (false, "bag")] {
        let mut document: serde_json::Value =
            serde_json::from_slice(&timers_document(2, &[("grow", &["ts"])])).expect("JSON");
        let fields = document["types"][1]["fields"]
            .as_array_mut()
            .expect("Call fields");
        let ts = fields
            .iter_mut()
            .find(|field| field["name"] == "ts")
            .expect("ts");
        ts["multiplicity"]["ordered"] = serde_json::json!(false);
        ts["multiplicity"]["unique"] = serde_json::json!(unique);
        let world = world_built(
            document.to_string().as_bytes(),
            INFINITE,
            &format!("{DENSE}pre Grow using v on M::Call::grow {{ true }}\n"),
            &["c1", "c2"],
            &[],
            &serde_json::json!({"ts": {empty: []}}),
        );
        let system: ModelSystem<'_> = world.system();
        let timed = TimedSystem::new(&system, "M").expect("a timed subject");
        assert!(timed.carries_clocks());
        for reductions in [Reductions::default(), swap.clone()] {
            let refused = deadlock_result(&world, &reductions);
            assert!(
                matches!(refused, Err(ZoneCheckRefusal::CarriedClocks)),
                "{empty} under {reductions:?}: {refused:?}"
            );
        }
    }
}
