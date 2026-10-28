// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120-AC-26 (TC-1270) end to end through EN-6: a granted clock
//! collection with a declared maximum is a finite domain, so the zone search
//! takes a model that grants `ts: Sequence<Timer>[0, N]` and reaches its
//! values, each slot's clock a place. Also FR-230's bare-clock counts and a
//! union case's clock through EN-6.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{ZoneCheckLimits, ZoneCheckOutcome};
use quire_exact::Cancel;

use crate::timed_world::{
    document, ident, timers_document, wire_field, wire_type, world_built, world_holding, World,
    DENSE, PACKAGE, PLACEHOLDER,
};
use crate::zone_search::{
    claim, deadlock, deadlock_result, operations, time_lock_item, violated, INFINITE,
};
use qsl_semantics::model::intake::meaning;

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

/// `u: Sequence<Clock>`, `Bag<Clock>` or `Set<Clock>` of `[lower, upper]`
/// on `Call`, with `work` always enabled and writing nothing.
fn bare_clocks(lower: u64, upper: Option<u64>, unique: bool) -> World {
    let mut document: serde_json::Value =
        serde_json::from_slice(&document(&[], &[("work", &[])])).expect("JSON");
    let mut u = wire_field("u", "ix://quire/native/Clock");
    u["multiplicity"] = serde_json::json!({"lower": lower, "ordered": !unique, "unique": unique});
    if let Some(upper) = upper {
        u["multiplicity"]["upper"] = serde_json::json!(upper);
    }
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(u);
    world_holding(
        document.to_string().as_bytes(),
        INFINITE,
        &format!("{DENSE}pre Work using v on M::Call::work {{ true }}\n"),
        &serde_json::json!({}),
    )
}

/// FR-230-AC-47 and AC-48 (TC-685 steps 31 and 32) as EN-6 settlements: a
/// `Set<Clock>[2, 5]` holds at most one bare clock, so the subject has no
/// initial state and the deadlock-freedom item settles
/// `Undecided(NoInitialState)` (FR-239); `u: Sequence<Clock>` with no
/// declared maximum ranges its initial count up from 0, and under
/// `max_clocks` 2 the count-3 state past the bound stops the run
/// `limit-reached` on `max_clocks`, an incomplete run, never a verdict.
#[trace("TC-685")]
#[test]
fn en6_settles_the_bare_clock_counts() {
    use qsl_analyze::zone_check::{
        check_timed_model, ZoneCheckItem, ZoneCheckLimit, ZoneCheckRequest, ZoneStop, ZoneUndecided,
    };
    use qsl_eval::simulation::model::{ModelSystem, TimedSystem};
    use qsl_route::deadlock::{ItemObligation, ObligationKind};
    use qsl_route::reductions::{ObligationReductions, Reductions};

    let empty = bare_clocks(2, Some(5), true);
    let outcome = deadlock_result(&empty, &Reductions::default());
    assert!(
        matches!(
            outcome,
            Ok(ZoneCheckOutcome::Undecided {
                cause: ZoneUndecided::NoInitialState,
                ..
            })
        ),
        "{outcome:?}"
    );

    let unbounded = bare_clocks(0, None, false);
    let system: ModelSystem<'_> = unbounded.system();
    let timed =
        TimedSystem::with_max_clocks(&system, "M", std::num::NonZeroU64::new(2).expect("2"))
            .expect("a timed subject");
    let outcome = check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::DeadlockFreedom,
            obligation: ItemObligation {
                subject: "c",
                kind: ObligationKind::DeadlockFreedom,
                reductions: ObligationReductions::default(),
            },
            limits: ZoneCheckLimits::default(),
            reductions: &Reductions::default(),
        },
        &Cancel::new(),
    );
    assert!(
        matches!(
            outcome,
            Ok(ZoneCheckOutcome::Stopped {
                cause: ZoneStop::LimitReached(ZoneCheckLimit::MaxClocks(2)),
                ..
            })
        ),
        "{outcome:?}"
    );
}

/// `state: Mode` on `Call`, with `Mode { Idle, Armed(Clock) }` (FR-230
/// "Records, tuples and unions"), written by `arm` and `disarm`, `c`
/// starting `Idle`, under `body`.
fn armed(body: &str) -> World {
    let mut document: serde_json::Value = serde_json::from_slice(&document(
        &[],
        &[("arm", &["state"]), ("disarm", &["state"])],
    ))
    .expect("JSON");
    let case = |name: &str, line: u64, payload: Option<&str>| {
        let mut case = serde_json::json!({
            "identity": ident(&format!("variant/Mode-{name}")), "name": name,
            "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.qspec",
                "startLine": line, "startColumn": 3}},
        });
        if let Some(payload) = payload {
            case["payloadType"] = serde_json::json!(payload);
        }
        case
    };
    let mut mode = wire_type(
        &ident("Mode"),
        "mode",
        serde_json::json!({"variants": [
            case("Idle", 20, None),
            case("Armed", 21, Some("ix://quire/native/Clock")),
        ]}),
    );
    mode["displayName"] = serde_json::json!("Mode");
    document["types"].as_array_mut().expect("types").push(mode);
    document["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(serde_json::json!({
            "kind": {"module": PACKAGE, "name": "mode"}, "moduleVersion": "1.0.0",
            "manifestDigest": PLACEHOLDER,
            "construct": {"identity": "none", "shape": "enumeration",
                "members": {"variants": "required"}, "meaning": meaning::VARIANT_TYPE}}));
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(wire_field("state", &ident("Mode")));
    world_holding(
        document.to_string().as_bytes(),
        INFINITE,
        &format!("{DENSE}{body}"),
        &serde_json::json!({"state": {"variant": {"type": ident("Mode"), "case": "Idle"}}}),
    )
}

/// FR-230-AC-12 (TC-685) through EN-6: a union case's clock is bounded only
/// while its case holds. With `time invariant Bound when
/// { present(self.state.Armed) } { value(self.state.Armed) <= 3 ms }`,
/// `arm` (enabled while not armed, `self.state = M::Mode::Armed(0 ms)`) and
/// `disarm` (`self.state = M::Mode::Idle`), the time-lock-freedom item holds: once disarmed, the
/// dropped clock is unconstrained and time passes past 3 ms. With `disarm`
/// disabled, an armed state is a time-lock at 3 ms, so the item is
/// violated.
#[trace("TC-685")]
#[test]
fn the_zone_search_drops_a_union_case_clock_with_its_case() {
    let body = |disarm: &str| {
        format!(
            "pre Arming using v on M::Call::arm {{ not present(self.state.Armed) }}\n\
             post Arm using v on M::Call::arm {{ self.state = M::Mode::Armed(0 ms) }}\n\
             pre Disarming using v on M::Call::disarm {{ {disarm} }}\n\
             post Disarm using v on M::Call::disarm {{ self.state = M::Mode::Idle }}\n\
             time invariant Bound when {{ present(self.state.Armed) }} {{ value(self.state.Armed) <= 3 ms }}\n"
        )
    };
    let free = armed(&body("true"));
    assert!(
        matches!(time_lock_item(&free), ZoneCheckOutcome::Holds { .. }),
        "disarming drops the bounded clock"
    );
    let locked = armed(&body("false"));
    let counterexample = violated(time_lock_item(&locked));
    assert_eq!(operations(&counterexample), ["arm"]);
}
