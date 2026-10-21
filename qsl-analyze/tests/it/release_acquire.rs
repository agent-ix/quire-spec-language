// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-222, FR-223, FR-227 and FR-229 (TC-667, TC-668, TC-672, TC-674):
//! `ProtocolSystem<Ra>` over ADR-025 §10's store buffering and §3.3's
//! litmus tests (`crate::litmus`).
//!
//! State counts follow `crate::weak_memory`'s convention: states inside the
//! `parallel` (some branch thread live) and joined states (the root at
//! `finish`).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ix_trace_rs::trace;
use qsl_eval::model_subject::ModelLetters;
use qsl_eval::simulation::model::Cell;
use qsl_eval::simulation::protocol::{
    visible, ClaimReads, MemoryChoice, MemorySelection, ProtocolState, ProtocolSystem,
    ProtocolTransition, Ra, RestKind, StepKind,
};
use qsl_eval::simulation::{independent, SubjectSystem, TransitionSystem};
use qsl_foundation::MemoryModelName;
use qsl_replay::{
    MemoryBound, MemoryBoundMember, ModelCheckOutcome, ModelTraceRefusal, ModelTraceSettlement,
    ReplayRefusal, ResultLabel, StatedMemoryBound, TemporalCounterexample,
};
use quire_exact::Value;
use serde_json::Value as Json;

use crate::fill::reachable;
use crate::litmus::{
    attempt, contracts, field, protocol, protocol_with, sb_run, sb_unit, NO_BOTH_ZERO,
};
use crate::weak_memory::{
    branch, counts, fenced_sb, load_ordered, memory_member, register, select, steps, Case, Forbid,
    CORR, IRIW, LB, MP, TWO_PLUS_TWO_W,
};

type RaState = ProtocolState<Ra>;

/// `SB::Both` resolved to `ra`.
fn ra_both() -> [MemorySelection; 1] {
    [select("SB::Both", MemoryModelName::Ra)]
}

/// A litmus test's `parallel` inside `sequence Main`, resolved to `ra`.
fn ra_main() -> [MemorySelection; 1] {
    [select("SB::Main::Both", MemoryModelName::Ra)]
}

/// The cell of the shared location the object `key` holds.
fn cell_of(case: &Case, selection: &[MemorySelection], key: &str) -> Cell {
    let accesses = case.accesses(selection);
    accesses
        .shared()
        .iter()
        .filter_map(|location| accesses.cell(location))
        .find(|cell| cell.key == key)
        .cloned()
        .expect("a shared cell")
}

fn integer(value: Option<&Value>) -> i64 {
    match value {
        Some(Value::Integer(value)) => value.to_string().parse().expect("small"),
        other => panic!("{other:?}"),
    }
}

/// The values of `cell`'s messages in mo at `state`.
fn values(state: &RaState, cell: &Cell) -> Vec<i64> {
    state
        .memory()
        .values(cell)
        .into_iter()
        .map(integer)
        .collect()
}

/// The successor of `state` by the step at `node`, naming `read` as the
/// depth it reads.
fn step(
    system: &ProtocolSystem<'_, Ra>,
    state: &RaState,
    node: &str,
    read: Option<u64>,
) -> (ProtocolTransition, RaState) {
    system
        .successors(state)
        .expect("expands")
        .successors
        .into_iter()
        .find(|(transition, _)| {
            transition.node() == Some(node)
                && transition
                    .memory_choice()
                    .and_then(|choice| choice.read_depth)
                    == read
        })
        .unwrap_or_else(|| panic!("{node} reading {read:?} is enabled"))
}

/// `state` after the fork.
fn forked(system: &ProtocolSystem<'_, Ra>) -> RaState {
    let initial = system.initial().remove(0);
    system
        .successors(&initial)
        .expect("expands")
        .successors
        .into_iter()
        .find(|(transition, _)| transition.kind() == StepKind::Fork)
        .map(|(_, next)| next)
        .expect("a fork")
}

/// Whether `state`'s root rests at `Done`: the `parallel` is joined.
fn joined(system: &ProtocolSystem<'_, Ra>, state: &RaState) -> bool {
    system
        .threads(state)
        .expect("threads")
        .iter()
        .any(|thread| {
            thread.rest_point.kind == RestKind::Node
                && thread.rest_point.node.as_deref() == Some("Done")
        })
}

/// The store `node` of 1 to `cell` with `ordering`.
fn put(node: &str, cell: &str, ordering: &str) -> String {
    let select = if cell == "x" { "IsX" } else { "IsY" };
    attempt(
        node,
        "w",
        "Cell::set",
        ordering,
        &format!("{select}, StoreOne"),
    )
}

/// A `parallel Both` inside `sequence Main` over the branches `branches`
/// (label, attempts).
fn run(branches: &[(&str, Vec<String>)], after: &str) -> String {
    let bodies: Vec<(&str, String)> = branches
        .iter()
        .map(|(label, attempts)| (*label, steps(&label.to_uppercase(), attempts)))
        .collect();
    crate::weak_memory::litmus_run(&bodies, after)
}

/// A unit over `run` with the claim `NoBothZero`, whose atom is read by
/// `letters`.
fn unit(run: &str, letters: &'static dyn ModelLetters) -> Case {
    Case::with_letters(
        &(contracts() + &protocol("SB", run) + NO_BOTH_ZERO),
        letters,
    )
}

/// MP: `a` stores the data `x` then the flag `y`; `b` loads the flag then
/// the data. The weak outcome is flag 1 with data 0.
fn message_passing(data: &str, flag_store: &str, flag_load: &str) -> String {
    run(
        &[
            ("a", vec![put("Sx", "x", data), put("Sy", "y", flag_store)]),
            (
                "b",
                vec![
                    load_ordered("Ly", "y", 1, flag_load),
                    load_ordered("Lx", "x", 2, data),
                ],
            ),
        ],
        "",
    )
}

/// IRIW: two writers and two readers that read the two cells in opposite
/// orders.
fn iriw(store: &str, load: &str) -> String {
    run(
        &[
            ("a", vec![put("Sx", "x", store)]),
            ("b", vec![put("Sy", "y", store)]),
            (
                "c",
                vec![
                    load_ordered("L1", "x", 1, load),
                    load_ordered("L2", "y", 2, load),
                ],
            ),
            (
                "d",
                vec![
                    load_ordered("L3", "y", 3, load),
                    load_ordered("L4", "x", 4, load),
                ],
            ),
        ],
        "",
    )
}

/// The claim over `case` under `selection` holds: the weak outcome is
/// unreachable.
fn forbids(case: &Case, selection: &[MemorySelection]) -> bool {
    match case.claim(selection, None) {
        ModelCheckOutcome::Holds { .. } => true,
        ModelCheckOutcome::Violated { .. } => false,
        other => panic!("{other:?}"),
    }
}

/// ADR-025 §10's `SB` under `ra` with `release` stores and `acquire` loads:
/// 14 states in the `parallel` and 4 joined, 18 in all; the claim settles
/// `refuted`. Every load of the counterexample reads the old message, depth
/// 1, and every store inserts at the end, slot depth 0. §10's own order, `Sx`,
/// `Ly` at depth 0 (`y0`), `Sy`, `Lx` at depth 1 (`x0`), is a path of the
/// system with the messages and views its table gives at each position.
#[trace("TC-667", "FR-222-AC-1")]
#[trace("TC-674", "FR-229-AC-1")]
#[test]
fn store_buffering_under_ra_reaches_18_states_and_is_refuted() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let ra = ra_both();
    let system = case.system(&ra, Ra::default()).expect("builds");
    let reached = reachable(&system);
    assert_eq!(counts(&system, &reached), (14, 4));

    let outcome = case.claim(&ra, None);
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("{outcome:?}");
    };
    let choices: Vec<(&str, MemoryChoice)> = counterexample
        .prefix
        .iter()
        .filter_map(|step| Some((step.transition.node()?, *step.transition.memory_choice()?)))
        .collect();
    let named = |node: &str| {
        choices
            .iter()
            .find(|(candidate, _)| candidate.ends_with(node))
            .map(|(_, choice)| *choice)
            .unwrap_or_else(|| panic!("{node} is a step"))
    };
    for load in ["Both/left/L/Ly", "Both/right/R/Lx"] {
        assert_eq!(named(load).read_depth, Some(1), "{load}");
    }
    for store in ["Both/left/L/Sx", "Both/right/R/Sy"] {
        assert_eq!(named(store).slot_depth, Some(0), "{store}");
    }

    // §10's counterexample table, position by position.
    let (x, y) = (cell_of(&case, &ra, "x"), cell_of(&case, &ra, "y"));
    let (left, right) = (branch("left"), branch("right"));
    let at_fork = forked(&system);
    assert_eq!(values(&at_fork, &x), [0]);
    assert_eq!(values(&at_fork, &y), [0]);
    let (_, stored_x) = step(&system, &at_fork, "Both/left/L/Sx", None);
    assert_eq!(values(&stored_x, &x), [0, 1]);
    assert_eq!(values(&stored_x, &y), [0]);
    let memory = stored_x.memory();
    assert_eq!(memory.message_view(&x, 1, &x), Some(1));
    assert_eq!(memory.message_view(&x, 1, &y), Some(0));
    assert_eq!(memory.current(&left, &x), Some(1));
    assert_eq!(memory.current(&right, &x), Some(0));
    let (_, loaded_y) = step(&system, &stored_x, "Both/left/L/Ly", Some(0));
    assert_eq!(register(loaded_y.model(), "core", "r1"), 0);
    assert_eq!(values(&loaded_y, &y), [0]);
    let (_, stored_y) = step(&system, &loaded_y, "Both/right/R/Sy", None);
    assert_eq!(values(&stored_y, &y), [0, 1]);
    let memory = stored_y.memory();
    assert_eq!(memory.message_view(&y, 1, &x), Some(0));
    assert_eq!(memory.message_view(&y, 1, &y), Some(1));
    assert_eq!(memory.current(&right, &y), Some(1));
    assert_eq!(memory.current(&right, &x), Some(0));
    assert_eq!(memory.current(&left, &y), Some(0));
    // At position 4 the load could also read `x1`, depth 0; depth 1 is the
    // old message and violates the claim.
    let (_, new) = step(&system, &stored_y, "Both/right/R/Lx", Some(0));
    assert_eq!(register(new.model(), "core", "r2"), 1);
    let (_, old) = step(&system, &stored_y, "Both/right/R/Lx", Some(1));
    assert_eq!(register(old.model(), "core", "r2"), 0);
    assert_eq!(register(old.model(), "core", "r1"), 0);
    assert_eq!(values(&old, &x), [0, 1]);
    assert_eq!(values(&old, &y), [0, 1]);
}

/// FR-229-AC-1 (TC-674): the memory member under `sc` is empty at every
/// state, and under `ra` empty at every joined state; while the `parallel`
/// is active it holds a `messages` entry per location, a `views` entry per
/// live thread, `S` and the SC event graph.
#[trace("TC-674", "FR-229-AC-1")]
#[test]
fn the_memory_member_is_empty_outside_a_weak_parallel() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let sc = case
        .system(&[], qsl_eval::simulation::protocol::Sc)
        .expect("builds");
    let empty = serde_json::json!({"entries": [], "type": "map"});
    for state in reachable(&sc) {
        assert_eq!(memory_member(&sc, &state.state), empty);
    }
    let system = case.system(&ra_both(), Ra::default()).expect("builds");
    let (mut inside, mut outside) = (0, 0);
    for state in reachable(&system) {
        let member = memory_member(&system, &state.state);
        if state.state.memory().is_empty() {
            assert_eq!(member, empty);
            outside += 1;
            continue;
        }
        inside += 1;
        let kinds: BTreeMap<&str, usize> = member["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .fold(BTreeMap::new(), |mut kinds, entry| {
                let kind = entry["key"]["type"].as_str().expect("a typed key");
                *kinds.entry(kind).or_default() += 1;
                kinds
            });
        // Two locations, the root and two branch threads, `S` and `P`.
        assert_eq!(
            kinds,
            BTreeMap::from([
                ("messages", 2),
                ("views", 3),
                ("sc-view", 1),
                ("sc-graph", 1)
            ])
        );
    }
    assert!(inside > 0 && outside > 0);
    // Every joined state has an empty component.
    for state in reachable(&system) {
        if joined(&system, &state.state) {
            assert!(state.state.memory().is_empty());
        }
    }
}

/// FR-222-AC-2 (TC-667): each litmus test of ADR-025 §3.3's table, run
/// under `ra`, reaches its weak outcome exactly when its `ra` column says
/// allowed: SB allowed; SB with `seq_cst` fences forbidden; MP with
/// `release`/`acquire` flag forbidden; MP with `relaxed` flag allowed; LB
/// forbidden; IRIW with `release`/`acquire` allowed; 2+2W with `release`
/// stores allowed; CoRR forbidden.
#[trace("TC-667", "FR-222-AC-2")]
#[test]
fn the_ra_column_of_the_litmus_table_decides_each_weak_outcome() {
    let sb = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    assert!(!forbids(&sb, &ra_both()), "SB is allowed");
    let fenced = Case::new(&sb_unit(&fenced_sb("seq_cst", "seq_cst")));
    assert!(forbids(&fenced, &ra_both()), "SB with seq_cst fences");

    let mp = |data: &str, store: &str, load: &str| unit(&message_passing(data, store, load), &MP);
    assert!(
        forbids(&mp("relaxed", "release", "acquire"), &ra_main()),
        "MP with a release/acquire flag"
    );
    assert!(
        !forbids(&mp("relaxed", "relaxed", "relaxed"), &ra_main()),
        "MP with a relaxed flag"
    );

    // LB: each load is followed by the store the other thread's load reads.
    let lb = unit(
        &run(
            &[
                (
                    "a",
                    vec![
                        load_ordered("Lx", "x", 1, "relaxed"),
                        put("Sy", "y", "relaxed"),
                    ],
                ),
                (
                    "b",
                    vec![
                        load_ordered("Ly", "y", 2, "relaxed"),
                        put("Sx", "x", "relaxed"),
                    ],
                ),
            ],
            "",
        ),
        &LB,
    );
    assert!(forbids(&lb, &ra_main()), "LB");

    assert!(
        !forbids(&unit(&iriw("release", "acquire"), &IRIW), &ra_main()),
        "IRIW with release/acquire"
    );

    // 2+2W: after the join each location holds its first-written value.
    let two_plus_two = unit(
        &run(
            &[
                (
                    "a",
                    vec![put("Sx1", "x", "release"), {
                        let mut second = put("Sy2", "y", "release");
                        second = second.replace("StoreOne", "StoreTwo");
                        second
                    }],
                ),
                (
                    "b",
                    vec![
                        put("Sy1", "y", "release"),
                        put("Sx2", "x", "release").replace("StoreOne", "StoreTwo"),
                    ],
                ),
            ],
            &(load_ordered("Fx", "x", 1, "") + &load_ordered("Fy", "y", 2, "")),
        ),
        &TWO_PLUS_TWO_W,
    );
    assert!(!forbids(&two_plus_two, &ra_main()), "2+2W");

    // CoRR: the second read of `x` never returns an mo-earlier value.
    let corr = unit(
        &run(
            &[
                ("a", vec![put("Sx", "x", "relaxed")]),
                (
                    "b",
                    vec![
                        load_ordered("L1", "x", 1, "relaxed"),
                        load_ordered("L2", "x", 2, "relaxed"),
                    ],
                ),
            ],
            "",
        ),
        &CORR,
    );
    assert!(forbids(&corr, &ra_main()), "CoRR");
}

/// The weak outcome the SB-shaped tests settle: both loads read 0.
fn no_both_zero_unit(run: &str) -> Case {
    Case::new(&(contracts() + &protocol("SB", run) + NO_BOTH_ZERO))
}

/// FR-222-AC-3 (TC-667): two threads each `fetch_add(1)` with `ordering
/// relaxed` on a location at 0: every joined state holds 2, and no state
/// has two RMWs reading one message, so the messages of `x` always count up
/// by 1. After `SB`'s `join all` the component is empty, the model state
/// holding each location's value. In every state of every program below,
/// the oldest message of a location is the one some thread's current view
/// points at: nothing earlier is kept.
#[trace("TC-667", "FR-222-AC-3")]
#[test]
fn read_modify_writes_chain_and_unreachable_messages_are_removed() {
    let add = |node: &str| attempt(node, "w", "Cell::add", "relaxed", "AddX, AddOne");
    let both = run(&[("a", vec![add("A1")]), ("b", vec![add("A2")])], "");
    let case = no_both_zero_unit(&both);
    let system = case.system(&ra_main(), Ra::default()).expect("builds");
    let x = cell_of(&case, &ra_main(), "x");
    let reached = reachable(&system);
    let mut chained = 0;
    for state in &reached {
        let held = values(&state.state, &x);
        assert!(
            held.windows(2).all(|pair| pair[1] == pair[0] + 1),
            "two RMWs read one message: {held:?}"
        );
        chained += usize::from(held.len() == 2 && held == [1, 2]);
        if joined(&system, &state.state) {
            assert_eq!(field(state.state.model(), "cells", "x", "v"), 2);
            assert!(state.state.memory().is_empty());
        }
        // RA-7: the first message is the one a current view points at.
        if let Some(first) = state.state.memory().currents(&x).into_iter().min() {
            assert_eq!(first, 0);
        }
    }
    // Once both threads have added, the message they started from is gone.
    assert!(chained > 0);
    assert!(reached.iter().any(|state| joined(&system, &state.state)));

    // A message every thread has moved past is removed in the same step.
    let sb = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let write_read = no_both_zero_unit(&run(
        &[
            ("a", vec![put("Sx", "x", "release")]),
            ("b", vec![load_ordered("Lx", "x", 2, "acquire")]),
        ],
        "",
    ));
    let system = write_read
        .system(&ra_main(), Ra::default())
        .expect("builds");
    let x = cell_of(&write_read, &ra_main(), "x");
    let mut collected = false;
    for state in reachable(&system) {
        if let Some(first) = state.state.memory().currents(&x).into_iter().min() {
            assert_eq!(first, 0);
        }
        collected |= values(&state.state, &x) == [1];
    }
    assert!(collected, "`x0` leaves once `b` has read `x1`");
    // After `SB`'s join all, the model state holds each location's value.
    let system = sb.system(&ra_both(), Ra::default()).expect("builds");
    for state in reachable(&system) {
        if joined(&system, &state.state) {
            assert!(state.state.memory().is_empty());
            let model = state.state.model();
            assert_eq!(field(model, "cells", "x", "v"), 1);
            assert_eq!(field(model, "cells", "y", "v"), 1);
        }
    }
}

/// FR-222-AC-4 (TC-667): MP with `relaxed` flag accesses, a `release` fence
/// before the flag store and an `acquire` fence after the flag load, settles
/// `proved` under `ra`; so does MP whose flag is a channel `send` and
/// `receive`. Without the fences the same MP is refuted.
#[trace("TC-667", "FR-222-AC-4")]
#[test]
fn fences_and_channels_pass_a_message_under_ra() {
    let fenced = run(
        &[
            (
                "a",
                vec![
                    put("Sx", "x", "relaxed"),
                    "fence Fr ordering release;".to_owned(),
                    put("Sy", "y", "relaxed"),
                ],
            ),
            (
                "b",
                vec![
                    load_ordered("Ly", "y", 1, "relaxed"),
                    "fence Fa ordering acquire;".to_owned(),
                    load_ordered("Lx", "x", 2, "relaxed"),
                ],
            ),
        ],
        "",
    );
    let case = unit(&fenced, &MP);
    assert!(forbids(&case, &ra_main()));
    let outcome = case.claim(&ra_main(), None);
    let record = case.settle(&ra_main(), &outcome, false, None);
    assert_eq!(record.label, ResultLabel::Proved, "{record:?}");
    assert!(!forbids(
        &unit(&message_passing("relaxed", "relaxed", "relaxed"), &MP),
        &ra_main()
    ));

    // The flag is a channel: the data load after the `receive` must see the
    // data stored before the `send`.
    let channel = "channel ch from w to w carries Boolean \
                   ordering unordered delivery unknown capacity 2 overflow block;";
    let sent = format!(
        "parallel Both {{ branch s sequence Sender {{ {} send Tx via ch as (tx: Boolean) {{ tx }}; }} \
         branch r sequence Recv {{ receive Rx via ch of s::Sender::Tx as (rx: Boolean) {{ true }}; {} }} }} \
         join all [s, r];",
        put("Sx", "x", "relaxed"),
        load_ordered("Lx", "x", 2, "relaxed"),
    );
    static DATA_SEEN: Forbid = Forbid(|state, key| register(state, key, "r2") == 0);
    let over_channel = Case::with_letters(
        &(contracts() + &protocol_with("SB", channel, &sent) + NO_BOTH_ZERO),
        &DATA_SEEN,
    );
    assert!(forbids(&over_channel, &ra_both()));
    let outcome = over_channel.claim(&ra_both(), None);
    let record = over_channel.settle(&ra_both(), &outcome, false, None);
    assert_eq!(record.label, ResultLabel::Proved, "{record:?}");
    assert_eq!(
        record.memory_bound,
        Some(StatedMemoryBound {
            bound: MemoryBound {
                member: MemoryBoundMember::MaxMessages,
                value: 4,
            },
            reached: false,
        })
    );
    // Without the `send`'s release the data may be missed: the load of
    // `x` after a `receive` of a send by a thread that stored nothing
    // before it reads either message.
    let unsynchronised = format!(
        "parallel Both {{ branch s sequence Sender {{ send Tx via ch as (tx: Boolean) {{ tx }}; {} }} \
         branch r sequence Recv {{ receive Rx via ch of s::Sender::Tx as (rx: Boolean) {{ true }}; {} }} }} \
         join all [s, r];",
        put("Sx", "x", "relaxed"),
        load_ordered("Lx", "x", 2, "relaxed"),
    );
    let racy = Case::with_letters(
        &(contracts() + &protocol_with("SB", channel, &unsynchronised) + NO_BOTH_ZERO),
        &DATA_SEEN,
    );
    assert!(!forbids(&racy, &ra_both()));
}

/// FR-223-AC-1 (TC-668): `SB` with every store and load `seq_cst` and no
/// fence settles `proved` under `ra`. At the state after `Sx`, `Ly`
/// (reading `y0`) and `Sy`, the load `Lx` is enabled reading `x1` and not
/// reading `x0`, because the edges `Sx → Ly → Sy → Lx → Sx` would close a
/// cycle. With `release` and `acquire` the same state enables both.
#[trace("TC-668", "FR-223-AC-1")]
#[test]
fn seq_cst_store_buffering_prunes_the_old_message_choice() {
    let case = Case::new(&sb_unit(&sb_run("", "seq_cst", "seq_cst")));
    let system = case.system(&ra_both(), Ra::default()).expect("builds");
    let outcome = case.claim(&ra_both(), None);
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "{outcome:?}"
    );
    let record = case.settle(&ra_both(), &outcome, false, None);
    assert_eq!(record.label, ResultLabel::Proved, "{record:?}");

    let reads_at = |system: &ProtocolSystem<'_, Ra>| {
        let at_fork = forked(system);
        let (_, stored_x) = step(system, &at_fork, "Both/left/L/Sx", None);
        let (_, loaded_y) = step(system, &stored_x, "Both/left/L/Ly", Some(0));
        let (_, stored_y) = step(system, &loaded_y, "Both/right/R/Sy", None);
        system
            .successors(&stored_y)
            .expect("expands")
            .successors
            .into_iter()
            .filter(|(transition, _)| transition.node() == Some("Both/right/R/Lx"))
            .filter_map(|(transition, _)| transition.memory_choice()?.read_depth)
            .collect::<BTreeSet<u64>>()
    };
    assert_eq!(reads_at(&system), BTreeSet::from([0]));
    let weak = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let weak_system = weak.system(&ra_both(), Ra::default()).expect("builds");
    assert_eq!(reads_at(&weak_system), BTreeSet::from([0, 1]));
}

/// FR-223-AC-2 (TC-668): IRIW with every access `seq_cst` settles with its
/// weak outcome unreachable under `ra`, and IRIW with `release` stores and
/// `acquire` loads with it reachable. `SB` with `seq_cst` fences and
/// `relaxed` accesses settles `proved` under `ra`.
#[trace("TC-668", "FR-223-AC-2")]
#[test]
fn seq_cst_iriw_is_forbidden_and_release_acquire_iriw_is_not() {
    let sc = unit(&iriw("seq_cst", "seq_cst"), &IRIW);
    assert!(forbids(&sc, &ra_main()));
    let outcome = sc.claim(&ra_main(), None);
    let record = sc.settle(&ra_main(), &outcome, false, None);
    assert_eq!(record.label, ResultLabel::Proved, "{record:?}");
    let weak = unit(&iriw("release", "acquire"), &IRIW);
    assert!(!forbids(&weak, &ra_main()));

    let relaxed_fenced = format!(
        "parallel Both {{ branch left sequence L {{ {} fence Fl ordering seq_cst; {} }} \
         branch right sequence R {{ {} fence Fr ordering seq_cst; {} }} }} join all [left, right];",
        put("Sx", "x", "relaxed"),
        load_ordered("Ly", "y", 1, "relaxed"),
        put("Sy", "y", "relaxed"),
        load_ordered("Lx", "x", 2, "relaxed"),
    );
    let fenced = Case::new(&sb_unit(&relaxed_fenced));
    let outcome = fenced.claim(&ra_both(), None);
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "{outcome:?}"
    );
    let record = fenced.settle(&ra_both(), &outcome, false, None);
    assert_eq!(record.label, ResultLabel::Proved, "{record:?}");
}

/// The looping protocol of FR-223-AC-3: two threads each running a
/// `repeat` of maximum 3 over a `seq_cst` store to `x` and a `seq_cst` load
/// of `y`.
fn sc_loops() -> String {
    let body = |label: &str, store: &str, load: &str, register: u8| {
        format!(
            "branch {label} repeat L{label} by w visible () max 3 while {{ true }} \
             sequence B{label} {{ {} {} }} exhausted sequence O{label} {{ }}",
            put(store, "x", "seq_cst"),
            load_ordered(load, "y", register, "seq_cst"),
        )
    };
    format!(
        "sequence Main {{ parallel Both {{ {} {} }} join all [a, b]; }}",
        body("a", "Sxa", "Lya", 1),
        body("b", "Sxb", "Lyb", 2),
    )
}

/// FR-223-AC-3 (TC-668): at every reachable state of the looping
/// protocol, the live SC events in `P` number at most the live messages
/// times the live threads, and every event is on a live message.
#[trace("TC-668", "FR-223-AC-3")]
#[test]
fn the_sc_event_graph_stays_within_the_live_messages() {
    let case = no_both_zero_unit(&sc_loops());
    let system = case.system(&ra_main(), Ra::default()).expect("builds");
    let reached = reachable(&system);
    let mut most = 0;
    for state in &reached {
        let memory = state.state.memory();
        assert!(
            memory.live_events() <= memory.live_messages() * memory.live_threads(),
            "{} events over {} messages and {} threads",
            memory.live_events(),
            memory.live_messages(),
            memory.live_threads()
        );
        assert!(memory.events_are_live());
        most = most.max(memory.live_events());
    }
    assert!(most > 0, "the loop executes SC events");
}

/// The state keys of `system`, in exploration order.
fn keys(system: &ProtocolSystem<'_, Ra>) -> Vec<Vec<u8>> {
    reachable(system)
        .iter()
        .map(|state| system.state_key(&state.state).expect("a key").0)
        .collect()
}

/// FR-223-AC-4 (TC-668): running AC-1 and AC-3 twice gives equal state
/// counts and byte-equal keys; two behaviours that differ only in an SC
/// event since removed reach one state.
#[trace("TC-668", "FR-223-AC-4")]
#[test]
fn the_sc_event_graph_is_deterministic_and_coalesces_removed_events() {
    for body in [
        sb_unit(&sb_run("", "seq_cst", "seq_cst")),
        contracts() + &protocol("SB", &sc_loops()) + NO_BOTH_ZERO,
    ] {
        let case = Case::new(&body);
        let selection = if body.contains("Main") {
            ra_main()
        } else {
            ra_both()
        };
        let first = keys(&case.system(&selection, Ra::default()).expect("builds"));
        let second = keys(&case.system(&selection, Ra::default()).expect("builds"));
        assert!(!first.is_empty());
        assert_eq!(first, second);
    }

    // `b` loads `x` with `seq_cst` or with `acquire`, by the `choice` on an
    // event's record, then loads it again. When it has read `x0` and then
    // `x1`, the first load's SC event leaves with `x0`: both behaviours
    // reach the same states.
    let picked = format!(
        "event F by w as (f: Boolean) {{ true }}; \
         choice Pick by w visible (f) {{ case yes when {{ f }} {} case no when {{ not f }} {} }} {}",
        load_ordered("Lsc", "x", 1, "seq_cst"),
        load_ordered("Lacq", "x", 1, "acquire"),
        load_ordered("Lagain", "x", 2, "relaxed"),
    );
    let body = format!(
        "sequence Main {{ parallel Both {{ branch a {} branch b sequence B {{ {picked} }} }} \
         join all [a, b]; }}",
        put("Sx", "x", "release"),
    );
    let case = no_both_zero_unit(&body);
    let system = case.system(&ra_main(), Ra::default()).expect("builds");
    let reached = reachable(&system);
    let descendants = |from: usize| {
        let mut seen = BTreeSet::from([from]);
        let mut queue = VecDeque::from([from]);
        while let Some(at) = queue.pop_front() {
            for (_, next) in &reached[at].steps {
                if seen.insert(*next) {
                    queue.push_back(*next);
                }
            }
        }
        seen
    };
    let entered = |case_node: &str| -> BTreeSet<usize> {
        reached
            .iter()
            .flat_map(|state| state.steps.iter())
            .filter(|(transition, _)| {
                transition
                    .node()
                    .is_some_and(|node| node.ends_with(case_node))
            })
            .flat_map(|(_, next)| descendants(*next))
            .collect()
    };
    let (via_sc, via_acquire) = (entered("Lsc"), entered("Lacq"));
    let shared: BTreeSet<usize> = via_sc.intersection(&via_acquire).copied().collect();
    assert!(
        shared.iter().any(|at| {
            !reached[*at].state.memory().is_empty() && !joined(&system, &reached[*at].state)
        }),
        "the two behaviours meet inside the parallel"
    );
}

/// FR-229-AC-3 (TC-674): a `compare_exchange` on `x` at 0 that leaves it
/// unchanged applies as a load under `ra`: it adds no message and moves
/// only the acting thread's view.
#[trace("TC-674", "FR-229-AC-3")]
#[test]
fn a_read_modify_write_that_changes_nothing_applies_as_a_load() {
    let cas = attempt("Cas", "w", "Cell::add", "acq_rel", "CasX, CasKeep");
    let body = format!(
        "{}pre CasX using v on M::Cell::add {{ self.id = 0 }}\n\
         post CasKeep using v on M::Cell::add {{ self.v = pre(self.v) }}\n{}{NO_BOTH_ZERO}",
        contracts(),
        protocol(
            "SB",
            &format!(
                "parallel Both {{ branch left sequence L {{ {cas} }} \
                 branch right sequence R {{ {} }} }} join all [left, right];",
                put("Sy", "y", "release"),
            ),
        ),
    );
    let case = Case::new(&body);
    let system = case.system(&ra_both(), Ra::default()).expect("builds");
    let x = cell_of(&case, &ra_both(), "x");
    let at_fork = forked(&system);
    let (cas_step, after) = step(&system, &at_fork, "Both/left/L/Cas", Some(0));
    let choice = cas_step.memory_choice().expect("a chosen message");
    assert_eq!((choice.read_depth, choice.slot_depth), (Some(0), None));
    assert_eq!(values(&after, &x), [0]);
    assert_eq!(after.memory().current(&branch("left"), &x), Some(0));
}

/// FR-229-AC-4 (TC-674), `ra` half: `Sx` is visible to a claim that reads
/// `x`; a load is not. A load of `y` by `left` and a store to `x` by
/// `right` are independent (distinct locations, distinct views), a load and
/// a store of one location are dependent through `msgs`, and two
/// `seq_cst` accesses are dependent through the SC event graph.
#[trace("TC-674", "FR-229-AC-4")]
#[test]
fn ra_steps_carry_message_and_view_footprints() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let system = case.system(&ra_both(), Ra::default()).expect("builds");
    let accesses = system.weak_accesses();
    let shared = |key: &str| {
        accesses
            .shared()
            .iter()
            .find(|location| accesses.cell(location).is_some_and(|cell| cell.key == key))
            .cloned()
            .expect("a shared location")
    };
    let x = shared("x");
    let at_fork = forked(&system);
    let footprint = |state: &RaState, node: &str, read: Option<u64>| {
        let (transition, _) = step(&system, state, node, read);
        (
            system.footprint(state, &transition).expect("a footprint"),
            transition,
        )
    };
    let (store, sx) = footprint(&at_fork, "Both/left/L/Sx", None);
    let claim = ClaimReads {
        locations: BTreeSet::from([x.clone()]),
        anchors: BTreeSet::new(),
    };
    assert!(visible(&sx, &store, &claim), "a store to x is visible");
    // `Sx` by left and `Lx` by right meet at `msgs(x)`; the loads `Ly` and
    // `Lx` of two locations by two threads do not.
    let (_, stored_y) = step(&system, &at_fork, "Both/right/R/Sy", None);
    let (stored_x_first, _) = footprint(&stored_y, "Both/left/L/Sx", None);
    let (loaded_x_first, lx) = footprint(&stored_y, "Both/right/R/Lx", Some(0));
    assert!(
        !visible(&lx, &loaded_x_first, &claim),
        "a load of x is invisible"
    );
    assert!(
        !independent(&stored_x_first, &loaded_x_first),
        "Sx and Lx meet at x"
    );
    let (_, both) = step(&system, &stored_y, "Both/left/L/Sx", None);
    let (loaded_y, _) = footprint(&both, "Both/left/L/Ly", Some(0));
    let (loaded_x, _) = footprint(&both, "Both/right/R/Lx", Some(0));
    assert!(
        independent(&loaded_y, &loaded_x),
        "two loads of two locations by two threads"
    );

    // `seq_cst` accesses meet at the SC event graph.
    let sc = Case::new(&sb_unit(&sb_run("", "seq_cst", "seq_cst")));
    let system = sc.system(&ra_both(), Ra::default()).expect("builds");
    let at_fork = forked(&system);
    let (_, stored_x) = step(&system, &at_fork, "Both/left/L/Sx", None);
    let (_, both) = step(&system, &stored_x, "Both/right/R/Sy", None);
    let graph = |node: &str, read: Option<u64>| {
        let (transition, _) = step(&system, &both, node, read);
        system.footprint(&both, &transition).expect("a footprint")
    };
    assert!(!independent(
        &graph("Both/left/L/Ly", Some(0)),
        &graph("Both/right/R/Lx", Some(0))
    ));
}

/// ADR-027 FT-4 over `ProtocolSystem<Ra>` (ADR-025 MR-1): over `SB`, MP,
/// the fenced `SB`, the `seq_cst` `SB`, the `send` fixture and the RMW
/// pair under `ra`, every pair of steps the footprints report independent
/// commutes.
#[trace("TC-661", "TC-674")]
#[test]
fn every_independent_pair_of_ra_steps_commutes() {
    let channel = "channel ch from w to w carries Boolean \
                   ordering unordered delivery unknown capacity 2 overflow block;";
    let sent = format!(
        "parallel Both {{ branch s sequence Sender {{ {} send Tx via ch as (tx: Boolean) {{ tx }}; }} \
         branch r sequence Recv {{ receive Rx via ch of s::Sender::Tx as (rx: Boolean) {{ true }}; {} }} }} \
         join all [s, r];",
        put("Sx", "x", "relaxed"),
        load_ordered("Lx", "x", 2, "relaxed"),
    );
    let add = |node: &str| attempt(node, "w", "Cell::add", "relaxed", "AddX, AddOne");
    let units = [
        (sb_unit(&sb_run("", "release", "acquire")), "SB::Both"),
        (
            contracts()
                + &protocol("SB", &message_passing("relaxed", "release", "acquire"))
                + NO_BOTH_ZERO,
            "SB::Main::Both",
        ),
        (sb_unit(&fenced_sb("seq_cst", "acquire")), "SB::Both"),
        (sb_unit(&sb_run("", "seq_cst", "seq_cst")), "SB::Both"),
        (
            contracts() + &protocol_with("SB", channel, &sent) + NO_BOTH_ZERO,
            "SB::Both",
        ),
        (
            contracts()
                + &protocol(
                    "SB",
                    &run(&[("a", vec![add("A1")]), ("b", vec![add("A2")])], ""),
                )
                + NO_BOTH_ZERO,
            "SB::Main::Both",
        ),
    ];
    let mut independent_pairs = 0;
    for (body, parallel) in &units {
        let case = Case::new(body);
        let system = case
            .system(&[select(parallel, MemoryModelName::Ra)], Ra::default())
            .expect("builds");
        let reached = reachable(&system);
        independent_pairs +=
            crate::protocol_footprints::assert_independent_steps_commute(&system, &reached);
    }
    assert!(independent_pairs > 0);
}

/// FR-227-AC-2 (TC-672): `SB`'s `ra` counterexample names `Lx`'s message by
/// depth 1 and replays to `reproduced-with-evaluated-witness`. Changing
/// that depth to 2 refuses `invalid_runtime_input`/`invalid-value`;
/// changing a carried message view refuses `stale_dependency`/
/// `content-mismatch`.
#[trace("TC-672", "FR-227-AC-2")]
#[test]
fn an_ra_counterexample_replays_by_its_message_depths() {
    let case = Case::new(&sb_unit(&sb_run("", "release", "acquire")));
    let ra = ra_both();
    assert_eq!(
        case.trace_subject(&ra).memory.model_of("SB::Both"),
        Some(MemoryModelName::Ra)
    );
    let outcome = case.claim(&ra, None);
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("{outcome:?}");
    };
    // Each carried component is the memory member of the state its step
    // reaches.
    let system = case.system(&ra, Ra::default()).expect("builds");
    let mut state = system.initial().remove(0);
    for step in &counterexample.prefix {
        state = system
            .successors(&state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| *transition == step.transition)
            .map(|(_, next)| next)
            .expect("the step is enabled");
        let component = step
            .transition
            .memory_component()
            .expect("a step over a weak parallel carries its memory component");
        let text: Json = serde_json::from_str(component.as_str()).expect("JSON");
        assert_eq!(text, memory_member(&system, &state));
    }
    let record = case.settle(&ra, &outcome_of(&counterexample), false, None);
    assert_eq!(record.label, ResultLabel::Refuted, "{record:?}");
    let replayed = case
        .replay(&ra, counterexample.as_ref().clone())
        .expect("replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );

    // A depth the pre-state has no message at is refused.
    let last_load = counterexample
        .prefix
        .iter()
        .rposition(|step| {
            step.transition
                .memory_choice()
                .is_some_and(|choice| choice.read_depth == Some(1))
        })
        .expect("a load of the old message");
    let mut deeper = counterexample.as_ref().clone();
    deeper.prefix[last_load].transition =
        deeper.prefix[last_load]
            .transition
            .clone()
            .naming(Some(MemoryChoice {
                read_depth: Some(2),
                slot_depth: None,
            }));
    let Err(ReplayRefusal::ModelTrace(refusal)) = case.replay(&ra, deeper) else {
        panic!("a depth without a message refuses");
    };
    assert_eq!(
        (refusal.code().as_str(), refusal.cause()),
        ("invalid_runtime_input", "invalid-value"),
        "{refusal:?}"
    );

    // A carried component that is not the recomputed one is refused.
    let mut forged = counterexample.as_ref().clone();
    let first = forged.prefix[0].transition.memory_component().cloned();
    forged.prefix[2].transition = forged.prefix[2].transition.clone().carrying_memory(first);
    let Err(ReplayRefusal::ModelTrace(refusal)) = case.replay(&ra, forged) else {
        panic!("a forged memory component refuses");
    };
    assert!(
        matches!(*refusal, ModelTraceRefusal::MemoryComponent { step: 2, .. }),
        "{refusal:?}"
    );
    assert_eq!(
        (refusal.code().as_str(), refusal.cause()),
        ("stale_dependency", "content-mismatch")
    );
}

/// The outcome that carries `counterexample`.
fn outcome_of(
    counterexample: &TemporalCounterexample<ProtocolTransition>,
) -> ModelCheckOutcome<ProtocolTransition> {
    ModelCheckOutcome::Violated {
        counterexample: Box::new(counterexample.clone()),
        stats: Default::default(),
    }
}

/// A subject that resolves a `parallel` to `ra` is stepped by `Ra` alone,
/// and its record states `max_messages` 4, not reached.
#[trace("TC-667", "FR-222-AC-1")]
#[test]
fn a_proved_ra_run_states_its_message_bound() {
    let case = Case::new(&sb_unit(&fenced_sb("seq_cst", "seq_cst")));
    let outcome = case.claim(&ra_both(), None);
    let record = case.settle(&ra_both(), &outcome, false, None);
    assert_eq!(record.label, ResultLabel::Proved, "{record:?}");
    assert_eq!(
        record.memory_bound,
        Some(StatedMemoryBound {
            bound: MemoryBound {
                member: MemoryBoundMember::MaxMessages,
                value: 4,
            },
            reached: false,
        })
    );
}
