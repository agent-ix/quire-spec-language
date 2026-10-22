// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-205, FR-206 and FR-210 (TC-650, TC-651, TC-655): `ProtocolSystem<Sc>`
//! over channels, `await` with its `timeout`, `choice`, `check` and
//! `repeat` (ADR-027 §9 step 4).

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_eval::simulation::protocol::{
    ProtocolState, ProtocolSystem, ProtocolTransition, Sc, StepKind, ThreadPath, WaitCause,
};
use qsl_eval::simulation::SubjectSystem;
use serde_json::{json, Value as Json};

use qsl_foundation::diagnostic::Code;

use crate::fill::{
    attempt, compiled, contracts, json_of, projections, protocol, reachable, sc_system, Reached,
};
use crate::protocol_system::{touch, Case};

/// The protocol `P` with `members` and `run` over `ADR-027 §7`'s contracts,
/// from `c.v = 0`.
fn case(members: &str, run: &str) -> Case {
    Case::new(
        &(contracts("self.v = 0") + &protocol("P", members, run, "d")),
        &[0],
    )
}

/// A channel `name` of `Boolean` from `w` to `w` with `premises` (ordering,
/// delivery, capacity and overflow).
fn channel(name: &str, premises: &str) -> String {
    format!("channel {name} from w to w carries Boolean {premises};")
}

/// The kind and last node segment of each of `reached[at]`'s steps.
fn steps_at(reached: &[Reached], at: usize) -> Vec<(StepKind, String)> {
    reached[at]
        .steps
        .iter()
        .map(|(transition, _)| (transition.kind(), last(transition)))
        .collect()
}

/// The last segment of `transition`'s node, empty for a channel step.
fn last(transition: &ProtocolTransition) -> String {
    transition
        .node()
        .and_then(|node| node.rsplit('/').next())
        .unwrap_or_default()
        .to_owned()
}

/// The JSON of `state`'s key.
fn key(system: &ProtocolSystem<'_, Sc>, state: &ProtocolState<Sc>) -> Json {
    let (bytes, _) = system.state_key(state).expect("a key");
    json_of(&bytes)
}

/// The JSON of `transition`'s identity.
fn identity(system: &ProtocolSystem<'_, Sc>, transition: &ProtocolTransition) -> Json {
    json_of(&system.transition_key(transition).expect("an identity"))
}

/// The JSON of the send record a `send` step carries (ADR-027 ST-3, PX-1).
fn send_record(transition: &ProtocolTransition) -> Json {
    let record = transition.send_record().expect("a send carries its record");
    json_of(
        &quire_canonical::to_vec(record, quire_canonical::Limits::new(u64::MAX))
            .expect("a record encodes"),
    )
}

/// The payloads of every queue of `state`, by queue, in its stored order.
fn queues(system: &ProtocolSystem<'_, Sc>, state: &ProtocolState<Sc>) -> Vec<(Json, Vec<bool>)> {
    key(system, state)["queues"]["entries"]
        .as_array()
        .expect("queue entries")
        .iter()
        .map(|entry| {
            let payloads = entry["value"]["elements"]
                .as_array()
                .expect("elements")
                .iter()
                .map(|element| element["payload"]["value"].as_bool().expect("a Boolean"))
                .collect();
            (entry["key"].clone(), payloads)
        })
        .collect()
}

/// The index of the state `steps` reach from the initial state, each step
/// named by its kind and last node segment.
fn follow(reached: &[Reached], steps: &[(StepKind, &str)]) -> usize {
    let mut at = 0;
    for (kind, node) in steps {
        at = reached[at]
            .steps
            .iter()
            .find(|(transition, _)| transition.kind() == *kind && last(transition) == *node)
            .map(|(_, next)| *next)
            .unwrap_or_else(|| panic!("no {kind:?} {node} from {at}: {:?}", steps_at(reached, at)));
    }
    at
}

/// FR-206-AC-4 (TC-651): a `fifo` channel of capacity 1 under `block` does
/// not enable a second `send` while its queue holds an entry, and the
/// waiting thread's cause is the full queue; under `reject` the second send
/// is enabled, leaves the queue unchanged and its record says `rejected`.
/// Under `at-least-once` `duplicate` is enabled exactly while the capacity
/// admits one more entry and `lose` never is; under `at-most-once` `lose`
/// is enabled and `duplicate` never is. An `unordered` channel holding two
/// distinct payloads gives two `receive` steps, a `fifo` one gives one, for
/// its head.
#[trace("TC-651", "FR-206-AC-4")]
#[test]
fn channel_premises_admit_exactly_their_queue_steps() {
    let sends = "sequence Main { send S1 via ch as (s1: Boolean) { s1 }; \
                 send S2 via ch as (s2: Boolean) { s2 }; }";
    let premises = |overflow: &str| {
        channel(
            "ch",
            &format!(
                "ordering fifo by (m: Boolean) {{ true }} delivery exactly-once-premise \
                 capacity 1 overflow {overflow}"
            ),
        )
    };

    let blocked = case(&premises("block"), sends);
    let subject = blocked.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    assert_eq!(steps_at(&reached, 0), [(StepKind::Send, "S1".to_owned())]);
    let full = follow(&reached, &[(StepKind::Send, "S1")]);
    assert_eq!(steps_at(&reached, full), []);
    assert_eq!(queues(&system, &reached[full].state)[0].1, [true]);
    let blocked_threads = system.blocked(&reached[full].state, &[]).expect("blocked");
    assert_eq!(
        blocked_threads[0].cause,
        WaitCause::FullQueue {
            channel: "P::ch".to_owned()
        }
    );

    let rejecting = case(&premises("reject"), sends);
    let subject = rejecting.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let full = follow(&reached, &[(StepKind::Send, "S1")]);
    let (transition, after) = &reached[full].steps[0];
    assert_eq!(transition.kind(), StepKind::Send);
    assert_eq!(
        send_record(transition),
        json!({"type": "send-record", "channel": "P::ch", "outcome": "rejected",
            "payload": {"type": "boolean", "value": true}, "displaced": null})
    );
    assert_eq!(queues(&system, &reached[*after].state)[0].1, [true]);
    let (first, _) = &reached[0].steps[0];
    assert_eq!(send_record(first)["outcome"], "accepted");

    // `duplicate` and `lose` over one queued entry of a capacity-2 queue.
    let delivered = |delivery: &str| {
        let unit = case(
            &channel(
                "ch",
                &format!(
                    "ordering fifo by (m: Boolean) {{ true }} delivery {delivery} capacity 2 \
                     overflow block"
                ),
            ),
            "sequence Main { send S1 via ch as (s1: Boolean) { s1 }; \
             receive R via ch of S1 as (r: Boolean) { true }; }",
        );
        let subject = unit.subject("P");
        let system = sc_system(&subject);
        let reached = reachable(&system);
        let one = follow(&reached, &[(StepKind::Send, "S1")]);
        let kinds = |at: usize| -> BTreeSet<StepKind> {
            reached[at]
                .steps
                .iter()
                .map(|(transition, _)| transition.kind())
                .collect()
        };
        let at_one = kinds(one);
        // The copy fills the queue to its capacity: no second copy.
        let two = reached[one]
            .steps
            .iter()
            .find(|(transition, _)| transition.kind() == StepKind::Duplicate)
            .map(|(_, next)| kinds(*next));
        (at_one, two)
    };
    let (at_one, two) = delivered("at-least-once");
    assert!(at_one.contains(&StepKind::Duplicate) && !at_one.contains(&StepKind::Lose));
    let two = two.expect("a duplicate step");
    assert!(!two.contains(&StepKind::Duplicate) && !two.contains(&StepKind::Lose));
    let (at_one, two) = delivered("at-most-once");
    assert!(at_one.contains(&StepKind::Lose) && !at_one.contains(&StepKind::Duplicate));
    assert!(two.is_none());

    // Two distinct payloads from one send node, through a `repeat`.
    let receives = |ordering: &str| {
        let unit = case(
            &channel(
                "ch",
                &format!(
                    "ordering {ordering} delivery exactly-once-premise capacity 2 overflow block"
                ),
            ),
            "sequence Main { repeat Loop by w visible () max 2 while { true } \
             send S via ch as (s: Boolean) { true }; exhausted sequence Out { } \
             receive R via ch of Loop::S as (r: Boolean) { true }; }",
        );
        let subject = unit.subject("P");
        let system = sc_system(&subject);
        let reached = reachable(&system);
        let at = reached
            .iter()
            .position(|state| {
                let queued = queues(&system, &state.state);
                queued.len() == 1
                    && queued[0].1.len() == 2
                    && queued[0].1[0] != queued[0].1[1]
                    && state
                        .steps
                        .iter()
                        .all(|(transition, _)| transition.kind() == StepKind::Receive)
                    && !state.steps.is_empty()
            })
            .expect("a state holding both payloads at R");
        let records: Vec<Json> = reached[at]
            .steps
            .iter()
            .map(|(transition, _)| identity(&system, transition)["record"]["value"].clone())
            .collect();
        (records, queues(&system, &reached[at].state)[0].1.clone())
    };
    let (records, _) = receives("unordered");
    assert_eq!(records.len(), 2);
    let (records, queued) = receives("fifo by (m: Boolean) { true }");
    assert_eq!(records, [json!(queued[0])]);
}

/// FR-206-AC-7 (TC-651, ADR-027 ST-3, PX-1): over a `fifo` channel `ch` of
/// capacity 2, each `send` binds its binder to the payload and its step
/// carries the send record `{channel, payload, outcome, displaced}`, with
/// `channel` in its qualified name, in neither its identity nor the state
/// key. On an empty
/// queue the outcome is `accepted` with nothing displaced. With the queue
/// holding `P1` (`true`) then `P2` (`false`), a third send `N` (`true`) is
/// `rejected` under `overflow reject` and `lost` under `overflow loss new`,
/// each leaving `[P1, P2]`; `overflow loss head` accepts it displacing `P1`,
/// leaving `[P2, N]`, and `overflow loss tail` accepts it displacing `P2`,
/// leaving `[P1, N]`.
#[trace("TC-651", "FR-206-AC-7")]
#[test]
fn a_send_binds_its_send_record() {
    let run = "sequence Main { send P1 via ch as (p1: Boolean) { p1 }; \
               send P2 via ch as (p2: Boolean) { not p2 }; \
               send N via ch as (n: Boolean) { n and p1 }; }";
    let entry = |send: &str, value: bool| {
        json!({"type": "queue-entry", "send": send,
            "thread": {"type": "thread-path", "instance": "0", "steps": []},
            "payload": {"type": "boolean", "value": value}})
    };
    let cases = [
        ("reject", "rejected", Json::Null, vec![true, false]),
        ("loss new", "lost", Json::Null, vec![true, false]),
        (
            "loss head",
            "accepted",
            entry("Main/P1", true),
            vec![false, true],
        ),
        (
            "loss tail",
            "accepted",
            entry("Main/P2", false),
            vec![true, true],
        ),
    ];
    for (overflow, outcome, displaced, queued) in cases {
        let unit = case(
            &channel(
                "ch",
                &format!(
                    "ordering fifo by (m: Boolean) {{ true }} delivery exactly-once-premise \
                     capacity 2 overflow {overflow}"
                ),
            ),
            run,
        );
        let subject = unit.subject("P");
        let system = sc_system(&subject);
        let reached = reachable(&system);
        let (first, sent) = &reached[0].steps[0];
        assert_eq!(
            send_record(first),
            json!({"type": "send-record", "channel": "P::ch", "outcome": "accepted",
                "payload": {"type": "boolean", "value": true}, "displaced": null}),
            "{overflow}"
        );
        // The identity's record and the binder are the payload alone.
        let payload = json!({"type": "boolean", "value": true});
        assert_eq!(identity(&system, first)["record"], payload);
        let observed = key(&system, &reached[*sent].state)["observations"]["entries"].clone();
        let [held] = observed.as_array().expect("observations").as_slice() else {
            panic!("`N` reads `p1`, so it is held: {observed}");
        };
        assert_eq!(held["value"], payload);
        let full = follow(&reached, &[(StepKind::Send, "P1"), (StepKind::Send, "P2")]);
        assert_eq!(queues(&system, &reached[full].state)[0].1, [true, false]);
        let (third, after) = &reached[full].steps[0];
        assert_eq!(last(third), "N");
        assert_eq!(
            send_record(third),
            json!({"type": "send-record", "channel": "P::ch", "outcome": outcome,
                "payload": {"type": "boolean", "value": true}, "displaced": displaced}),
            "{overflow}"
        );
        assert_eq!(identity(&system, third)["record"], payload);
        assert_eq!(
            queues(&system, &reached[*after].state)[0].1,
            queued,
            "{overflow}"
        );
    }
}

/// FR-206-AC-8 (TC-651, ADR-027 SE-5), with `weak_memory`'s `tso` half: a
/// thread resting at `fence F ordering seq_cst;` has one `fence` step,
/// whose identity names its ordering (ADR-027 TS-2) and which changes no
/// model state and moves the thread to its next sibling; under `sc` it is enabled in
/// every state the thread rests there. Two branches that each hold a fence
/// reach no deadlock under `sc` that the same protocol without the fences
/// does not reach, and give the same orders of their counted steps. The
/// `tso` fence rule is `weak_memory`'s
/// `a_seq_cst_fence_waits_for_its_store_buffer_under_tso`.
#[trace("TC-651", "FR-206-AC-8")]
#[test]
fn a_fence_moves_its_thread_and_changes_nothing_under_sc() {
    let run = format!(
        "sequence Main {{ fence F ordering seq_cst; {} }}",
        touch("T")
    );
    let unit = case("", &run);
    let subject = unit.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    assert_eq!(steps_at(&reached, 0), [(StepKind::Fence, "F".to_owned())]);
    let (fence, after) = &reached[0].steps[0];
    assert_eq!(
        identity(&system, fence),
        json!({"type": "protocol-transition", "step": "fence", "node": "Main/F",
            "ordering": "seq_cst",
            "thread": {"type": "thread-path", "instance": "0", "steps": []}})
    );
    assert_eq!(
        key(&system, &reached[0].state)["semantic"],
        key(&system, &reached[*after].state)["semantic"]
    );
    assert_eq!(
        steps_at(&reached, *after),
        [(StepKind::Attempt, "T".to_owned())]
    );

    let branches = |left: &str, right: &str| {
        format!(
            "parallel Both {{ branch l sequence L {{ {left} {} }} \
             branch r sequence R {{ {right} {} }} }} join all [l, r];",
            touch("A"),
            touch("B"),
        )
    };
    let deadlocks = |run: &str| {
        let unit = case("", run);
        let subject = unit.subject("P");
        let system = sc_system(&subject);
        let reached = reachable(&system);
        let fences_enabled = reached.iter().all(|state| {
            system
                .threads(&state.state)
                .expect("threads")
                .iter()
                .filter(|thread| {
                    thread
                        .rest_point
                        .node
                        .as_deref()
                        .is_some_and(|node| node.ends_with("/Fl") || node.ends_with("/Fr"))
                })
                .all(|thread| {
                    state.steps.iter().any(|(transition, _)| {
                        transition.kind() == StepKind::Fence
                            && transition.thread() == Some(&thread.path)
                    })
                })
        });
        assert!(fences_enabled, "{run}");
        // Each fence's identity names its own ordering.
        for state in &reached {
            for (transition, _) in &state.steps {
                if transition.kind() == StepKind::Fence {
                    let expected = if last(transition) == "Fl" {
                        "seq_cst"
                    } else {
                        "acquire"
                    };
                    assert_eq!(identity(&system, transition)["ordering"], expected);
                }
            }
        }
        let deadlocked = reached
            .iter()
            .filter(|state| {
                state.steps.is_empty()
                    && crate::fill::class_of(&system, &state.state)
                        == qsl_eval::simulation::model::TerminalClass::Deadlocked
            })
            .count();
        (deadlocked, projections(&reached))
    };
    let fenced = deadlocks(&branches(
        "fence Fl ordering seq_cst;",
        "fence Fr ordering acquire;",
    ));
    let plain = deadlocks(&branches("", ""));
    assert_eq!(fenced, plain);
}

/// FR-210-AC-3 (TC-655): branch `s` sends `Ping` and then attempts `S`;
/// branch `r` receives it and then attempts `R`. No behaviour takes the
/// `receive` before the `send`, and every other order of the four event
/// steps that keeps each branch's order is reached. A second channel whose
/// sends and receives run in opposite orders adds no order of its own: its
/// projections are exactly the linear extensions of its edge relation.
#[trace("TC-655", "FR-210-AC-3")]
#[test]
fn a_message_orders_its_receive_after_its_send_and_nothing_else() {
    let fifo = "ordering fifo by (m: Boolean) { true } delivery exactly-once-premise \
                capacity 2 overflow block";
    let run = format!(
        "parallel Both {{ branch s sequence Sender {{ send Ping via ch as (ping: Boolean) {{ ping }}; {} }} \
         branch r sequence Receiver {{ receive Rx via ch of s::Sender::Ping as (rx: Boolean) {{ true }}; {} }} }} \
         join all [s, r];",
        touch("S"),
        touch("R"),
    );
    let order = |events: &[&str]| -> Vec<String> {
        events
            .iter()
            .map(|event| (*event).to_owned())
            .chain(std::iter::once("Done".to_owned()))
            .collect()
    };
    let expected: BTreeSet<Vec<String>> = [
        order(&["Ping", "S", "Rx", "R"]),
        order(&["Ping", "Rx", "S", "R"]),
        order(&["Ping", "Rx", "R", "S"]),
    ]
    .into_iter()
    .collect();
    for members in [
        channel("ch", fifo),
        channel("ch", fifo) + &channel("back", fifo),
    ] {
        let unit = case(&members, &run);
        let subject = unit.subject("P");
        let reached = reachable(&sc_system(&subject));
        assert_eq!(projections(&reached), expected, "{members}");
    }

    // `s` sends `A` on `ch` then `B` on `back`; `r` receives `B` then `A`.
    let crossed = "parallel Both { \
         branch s sequence Sender { send A via ch as (a: Boolean) { a }; \
         send B via back as (b: Boolean) { b }; } \
         branch r sequence Receiver { receive RB via back of s::Sender::B as (rb: Boolean) { true }; \
         receive RA via ch of s::Sender::A as (ra: Boolean) { true }; } } join all [s, r];";
    let unit = case(&(channel("ch", fifo) + &channel("back", fifo)), crossed);
    let subject = unit.subject("P");
    let reached = reachable(&sc_system(&subject));
    // A < B, RB < RA, A < RA, B < RB: the one linear extension.
    let expected: BTreeSet<Vec<String>> = [order(&["A", "B", "RB", "RA"])].into_iter().collect();
    assert_eq!(projections(&reached), expected);
}

/// FR-206-AC-5 (TC-651): a `choice` whose guard reads an earlier binder,
/// a `repeat` with maximum 2 whose guard is `true` and a `check` add no
/// step: the `choice` enters the case its binder selects; the `repeat`
/// enters its body twice, counting 1 and 2 in `bounds`, and then
/// `exhausted`; a false `check` leaves its thread resting there in every
/// later state. An `await` whose match is never enabled has `timeout`
/// enabled at every state where it waits, and a matching event's step
/// settles the owner into `then`. A `repeat` with no maximum whose guard is
/// `true` and whose body is one always-enabled attempt enters its body
/// again after each attempt, keeps no counter and never leaves.
#[trace("TC-651", "FR-206-AC-5")]
#[test]
fn structural_moves_add_no_step() {
    // `choice` over the Boolean record `f` of the event `F`.
    let run = format!(
        "sequence Main {{ {} choice Pick by w visible (f) {{ case yes when {{ f }} {} \
         case no when {{ not f }} {} }} }}",
        "event F by w as (f: Boolean) { true };",
        touch("Y"),
        touch("N"),
    );
    let unit = case("", &run);
    let subject = unit.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let expected: BTreeSet<Vec<String>> = [["F", "Y", "Done"], ["F", "N", "Done"]]
        .iter()
        .map(|events| events.iter().map(|event| (*event).to_owned()).collect())
        .collect();
    assert_eq!(projections(&reached), expected);
    assert_eq!(reached[0].steps.len(), 2, "one `F` step per record");
    for (transition, next) in &reached[0].steps {
        let f = identity(&system, transition)["record"]["value"]
            .as_bool()
            .expect("a Boolean record");
        let case = if f { "Y" } else { "N" };
        assert_eq!(
            steps_at(&reached, *next),
            [(StepKind::Attempt, case.to_owned())]
        );
    }
    let kinds: BTreeSet<StepKind> = reached
        .iter()
        .flat_map(|state| state.steps.iter().map(|(transition, _)| transition.kind()))
        .collect();
    assert_eq!(
        kinds,
        [StepKind::Event, StepKind::Attempt, StepKind::Finish]
            .into_iter()
            .collect()
    );

    // `repeat` with maximum 2.
    let run = format!(
        "sequence Main {{ repeat Loop by w visible () max 2 while {{ true }} {} exhausted {} }}",
        touch("X"),
        touch("E"),
    );
    let unit = case("", &run);
    let subject = unit.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let expected: BTreeSet<Vec<String>> = [vec![
        "X".to_owned(),
        "X".to_owned(),
        "E".to_owned(),
        "Done".to_owned(),
    ]]
    .into_iter()
    .collect();
    assert_eq!(projections(&reached), expected);
    let counters: Vec<Json> = [0, 1]
        .iter()
        .map(|at| {
            let at = follow(&reached, &[(StepKind::Attempt, "X")][..*at]);
            key(&system, &reached[at].state)["bounds"]["entries"][0]["value"]["value"].clone()
        })
        .collect();
    assert_eq!(counters, [json!("1"), json!("2")]);

    // A false `check` in one branch.
    let run = format!(
        "parallel Both {{ branch l check Never using v {{ false }}; \
         branch r sequence R {{ {} {} }} }} join all [l, r];",
        touch("A"),
        touch("B"),
    );
    let unit = case("", &run);
    let subject = unit.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    assert_eq!(reached.len(), 4);
    let left = ThreadPath::root(0).child("Both", "l", 0);
    for state in &reached[1..] {
        let thread = system
            .threads(&state.state)
            .expect("threads")
            .into_iter()
            .find(|thread| thread.path == left)
            .expect("l is live");
        assert_eq!(thread.rest_point.node.as_deref(), Some("Both/l/Never"));
    }
    let last_state = follow(
        &reached,
        &[
            (StepKind::Fork, "Both"),
            (StepKind::Attempt, "A"),
            (StepKind::Attempt, "B"),
        ],
    );
    assert!(reached[last_state].steps.is_empty());
    let blocked = system
        .blocked(&reached[last_state].state, &[])
        .expect("blocked");
    assert!(blocked.iter().any(|thread| thread.path == left
        && thread.cause
            == WaitCause::CheckFalse {
                node: "Both/l/Never".to_owned()
            }));

    // `await` with a `timeout`, its match never enabled and then enabled.
    for matched in ["false", "true"] {
        let run = format!(
            "sequence Main {{ {} await Wait after Start using v clock \"c\" within [0,1] \
             match event Seen by w as (seen: Boolean) {{ {matched} }}; then {} timeout {} }}",
            touch("Start"),
            touch("Fine"),
            touch("Late"),
        );
        let unit = case("", &run);
        let subject = unit.subject("P");
        let system = sc_system(&subject);
        let reached = reachable(&system);
        let waiting = follow(&reached, &[(StepKind::Attempt, "Start")]);
        let at_wait = steps_at(&reached, waiting);
        assert!(
            at_wait.contains(&(StepKind::Timeout, "Wait".to_owned())),
            "{matched}"
        );
        let late = follow(
            &reached,
            &[(StepKind::Attempt, "Start"), (StepKind::Timeout, "Wait")],
        );
        assert_eq!(
            steps_at(&reached, late),
            [(StepKind::Attempt, "Late".to_owned())]
        );
        let events: Vec<usize> = reached[waiting]
            .steps
            .iter()
            .filter(|(transition, _)| transition.kind() == StepKind::Event)
            .map(|(_, next)| *next)
            .collect();
        if matched == "false" {
            assert!(events.is_empty());
        } else {
            assert_eq!(events.len(), 2, "one step per Boolean record");
            for next in events {
                assert_eq!(
                    steps_at(&reached, next),
                    [(StepKind::Attempt, "Fine".to_owned())]
                );
            }
        }
    }

    // `repeat` with no maximum. It can never enter `exhausted`: the grammar
    // gives `exhausted` only to a `repeat` with a maximum.
    let exhausted = format!(
        "repeat Loop by w visible () while {{ true }} {} exhausted {}",
        touch("X"),
        touch("E"),
    );
    let Err(refused) = compiled(&(contracts("self.v = 0") + &protocol("P", "", &exhausted, "d")))
    else {
        panic!("`exhausted` needs a maximum");
    };
    assert_eq!(refused.code(), Code::InvalidSyntax, "{refused}");
    let run = format!(
        "repeat Loop by w visible () while {{ true }} {}",
        touch("X")
    );
    let unit = case("", &run);
    let subject = unit.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    assert_eq!(reached.len(), 1);
    assert_eq!(reached[0].steps.len(), 1);
    assert_eq!(reached[0].steps[0].0.kind(), StepKind::Attempt);
    assert_eq!(last(&reached[0].steps[0].0), "X");
    assert_eq!(reached[0].steps[0].1, 0);
    assert_eq!(
        key(&system, &reached[0].state)["bounds"]["entries"],
        json!([])
    );
}

/// FR-205-AC-3 (TC-650): a `repeat` with maximum 2 around `parallel P`
/// joined `any … outstanding continue`: the second fork while the first
/// instance's `b` is running creates threads with ordinal 1; when `b`
/// completes before the second fork, the second fork reuses ordinal 0, so
/// its threads' paths equal the first fork's; and both completion orders of
/// the first instance's branches reach one state.
#[trace("TC-650", "FR-205-AC-3")]
#[test]
fn a_repeated_fork_reuses_the_lowest_free_ordinal() {
    let run = format!(
        "sequence Main {{ repeat Loop by w visible () max 2 while {{ true }} \
         parallel P {{ branch a {} branch b {} }} join any [a, b] outstanding continue; \
         exhausted sequence Out {{ }} }}",
        touch("X"),
        touch("Y"),
    );
    let unit = case("", &run);
    let subject = unit.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let paths = |at: usize| -> BTreeSet<ThreadPath> {
        system
            .threads(&reached[at].state)
            .expect("threads")
            .into_iter()
            .map(|thread| thread.path)
            .collect()
    };
    let root = ThreadPath::root(0);
    let thread = |label: &str, ordinal: u64| root.child("Main/Loop/P", label, ordinal);

    let early = follow(
        &reached,
        &[
            (StepKind::Fork, "P"),
            (StepKind::Attempt, "X"),
            (StepKind::Join, "P"),
            (StepKind::Fork, "P"),
        ],
    );
    let live = paths(early);
    assert!(live.contains(&thread("b", 0)), "{live:?}");
    assert!(
        live.contains(&thread("a", 1)) && live.contains(&thread("b", 1)),
        "{live:?}"
    );

    let one_way = follow(
        &reached,
        &[
            (StepKind::Fork, "P"),
            (StepKind::Attempt, "X"),
            (StepKind::Attempt, "Y"),
            (StepKind::Join, "P"),
        ],
    );
    let other_way = follow(
        &reached,
        &[
            (StepKind::Fork, "P"),
            (StepKind::Attempt, "Y"),
            (StepKind::Attempt, "X"),
            (StepKind::Join, "P"),
        ],
    );
    assert_eq!(one_way, other_way);
    let second = follow(
        &reached,
        &[
            (StepKind::Fork, "P"),
            (StepKind::Attempt, "X"),
            (StepKind::Attempt, "Y"),
            (StepKind::Join, "P"),
            (StepKind::Fork, "P"),
        ],
    );
    let first = follow(&reached, &[(StepKind::Fork, "P")]);
    assert_eq!(paths(second), paths(first));
    assert_eq!(
        paths(second),
        [root.clone(), thread("a", 0), thread("b", 0)]
            .into_iter()
            .collect()
    );
    let counter =
        |at: usize| key(&system, &reached[at].state)["bounds"]["entries"][0]["value"].clone();
    assert_eq!(counter(first)["value"], "1");
    assert_eq!(counter(second)["value"], "2");
}

/// ADR-027 PS-5: a branch reads its parent's binder as of its fork. A
/// `repeat` around the event `F` (binding `f`) and `parallel P`, whose branch `a`
/// reads `f` and whose join is `any … outstanding continue`: once the root
/// binds `f` again while the first instance's `a` still waits, that `a`
/// keeps its own copy and steps exactly when its copy is `true`.
#[trace("TC-650")]
#[test]
fn a_branch_reads_its_parents_binding_as_of_its_fork() {
    let run = format!(
        "sequence Main {{ repeat Loop by w visible () max 2 while {{ true }} sequence Body {{ {} \
         parallel P {{ branch a {} branch b {} }} join any [a, b] outstanding continue; }} \
         exhausted sequence Out {{ }} }}",
        "event F by w as (f: Boolean) { true };",
        attempt("A", "touch", "TouchPre", "aa", "f"),
        touch("B"),
    );
    let unit = case("", &run);
    let subject = unit.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let first_a = ThreadPath::root(0).child("Main/Loop/Body/P", "a", 0);
    let mut checked = 0;
    for state in &reached {
        let observations = key(&system, &state.state)["observations"]["entries"]
            .as_array()
            .expect("observations")
            .clone();
        let value_of = |thread: &Json| {
            observations
                .iter()
                .find(|entry| entry["key"]["thread"] == *thread)
                .map(|entry| entry["value"]["value"].as_bool().expect("a Boolean"))
        };
        let root_json = json!({"type": "thread-path", "instance": "0", "steps": []});
        let a_json = json!({"type": "thread-path", "instance": "0",
            "steps": [{"node": "Main/Loop/Body/P", "branch": "a", "ordinal": "0"}]});
        let (Some(root_value), Some(copy)) = (value_of(&root_json), value_of(&a_json)) else {
            continue;
        };
        if root_value == copy {
            continue;
        }
        let a_steps = state.steps.iter().any(|(transition, _)| {
            transition.kind() == StepKind::Attempt && transition.thread() == Some(&first_a)
        });
        assert_eq!(a_steps, copy, "the first `a` reads its own copy");
        checked += 1;
    }
    assert!(
        checked > 0,
        "some state holds a rebound `f` and a differing copy"
    );
}

/// The value of the Boolean binder whose node ends in `node` in the
/// environment of `thread`, read from `state`'s key; `None` when that
/// environment holds no entry for it.
fn entry(
    system: &ProtocolSystem<'_, Sc>,
    state: &ProtocolState<Sc>,
    node: &str,
    thread: &ThreadPath,
) -> Option<bool> {
    let wanted = path_json(thread);
    key(system, state)["observations"]["entries"]
        .as_array()
        .expect("observations")
        .iter()
        .find(|observed| {
            observed["key"]["thread"] == wanted
                && observed["key"]["node"]
                    .as_str()
                    .is_some_and(|text| text.rsplit('/').next() == Some(node))
        })
        .map(|observed| observed["value"]["value"].as_bool().expect("a Boolean"))
}

/// `thread`'s path as the state key encodes it.
fn path_json(thread: &ThreadPath) -> Json {
    let steps: Vec<Json> = thread
        .steps
        .iter()
        .map(|step| {
            json!({"node": step.node(), "branch": step.branch(),
                "ordinal": step.ordinal().to_string()})
        })
        .collect();
    json!({"type": "thread-path", "instance": thread.instance.to_string(), "steps": steps})
}

/// The last segment of the node `thread` rests at in `state`; `None` when
/// the thread is not live.
fn resting(
    system: &ProtocolSystem<'_, Sc>,
    state: &ProtocolState<Sc>,
    thread: &ThreadPath,
) -> Option<String> {
    system
        .threads(state)
        .expect("threads")
        .into_iter()
        .find(|view| view.path == *thread)
        .map(|view| {
            view.rest_point
                .node
                .and_then(|node| node.rsplit('/').next().map(str::to_owned))
                .unwrap_or_default()
        })
}

/// FR-205-AC-7 (TC-650, ADR-027 PS-5): inside a `repeat` of maximum 2,
/// `parallel P` joined `all` binds `a` in its branch `left`, and `parallel
/// Q` joined `any … outstanding continue` reads it in branch `u`'s `check
/// Ck`. After `join(P)` the root's environment holds `a`, and `fork(Q)`
/// copies it into `u`. With iteration 1's `u` still resting at `U1`, the
/// second `fork(P)` takes ordinal 0 and the second `fork(Q)` ordinal 1;
/// each `u` evaluates `Ck` over its own copy, so a `u` rests at `Ck` exactly
/// when its copy is false, and no state refuses. Every entry is held by a
/// live thread, so a completed `u`'s entry leaves with it.
#[trace("TC-650", "FR-205-AC-7")]
#[test]
fn each_thread_reads_its_own_copy_of_a_rebound_binder() {
    let run = format!(
        "sequence Main {{ repeat Loop by w visible () max 2 while {{ true }} sequence Body {{ \
         parallel P {{ branch left {} branch right {} }} join all [left, right]; \
         parallel Q {{ branch u sequence U {{ {} check Ck using v {{ a }}; }} branch v {} }} \
         join any [u, v] outstanding continue; }} exhausted sequence Out {{ }} }}",
        "event A1 by w as (a: Boolean) { true };",
        touch("B1"),
        touch("U1"),
        touch("V1"),
    );
    let unit = case("", &run);
    let subject = unit.subject("P");
    let system = sc_system(&subject);
    // Exploration completes: no state refuses.
    let reached = reachable(&system);
    let root = ThreadPath::root(0);
    let u = |ordinal: u64| root.child("Main/Loop/Body/Q", "u", ordinal);
    let v = |ordinal: u64| root.child("Main/Loop/Body/Q", "v", ordinal);
    let mut both_iterations = 0;
    let mut checked = 0;
    for state in &reached {
        // Every entry is held by a live thread.
        let live: BTreeSet<ThreadPath> = system
            .threads(&state.state)
            .expect("threads")
            .into_iter()
            .map(|view| view.path)
            .collect();
        for (_, holder) in system.binders(&state.state).expect("binders") {
            assert!(
                live.contains(&holder),
                "{holder:?} holds an entry but is not live"
            );
        }
        for ordinal in [0, 1] {
            let thread = u(ordinal);
            if let Some(copy) = entry(&system, &state.state, "A1", &thread) {
                // `U1` settles `u` through `Ck` over its own copy: it rests at
                // `Ck` exactly when that copy is false.
                for (transition, after) in &state.steps {
                    if transition.thread() == Some(&thread) && last(transition) == "U1" {
                        let rest = resting(&system, &reached[*after].state, &thread);
                        assert_eq!(rest.as_deref() == Some("Ck"), !copy, "u@{ordinal}");
                        checked += 1;
                    }
                }
            }
            // `v` reads no binder, so its copy is dropped once it is forked.
            assert_eq!(entry(&system, &state.state, "A1", &v(ordinal)), None);
        }
        let first = entry(&system, &state.state, "A1", &u(0));
        let second = entry(&system, &state.state, "A1", &u(1));
        if let (Some(first), Some(second)) = (first, second) {
            if first != second {
                both_iterations += 1;
            }
        }
    }
    assert!(
        both_iterations > 0,
        "two `u` threads read different copies of `a` in one state"
    );
    assert!(checked > 0);

    // The forks: after `join(P)` the root holds `a`; `fork(Q)` copies it
    // into `u`; the second `fork(P)`, with iteration 1's `u` still at `U1`,
    // takes ordinal 0 while `Q`'s instance 0 is still live, and the second
    // `fork(Q)` takes ordinal 1.
    let joined_p = follow(
        &reached,
        &[
            (StepKind::Fork, "P"),
            (StepKind::Event, "A1"),
            (StepKind::Attempt, "B1"),
            (StepKind::Join, "P"),
        ],
    );
    let held = entry(&system, &reached[joined_p].state, "A1", &root);
    assert!(held.is_some());
    let next = |at: usize, kind: StepKind, node: &str| {
        reached[at]
            .steps
            .iter()
            .find(|(transition, _)| transition.kind() == kind && last(transition) == node)
            .map(|(_, next)| *next)
            .unwrap_or_else(|| panic!("no {kind:?} {node} from {at}"))
    };
    let forked_q = next(joined_p, StepKind::Fork, "Q");
    assert_eq!(entry(&system, &reached[forked_q].state, "A1", &u(0)), held);
    let joined_q = next(next(forked_q, StepKind::Attempt, "V1"), StepKind::Join, "Q");
    assert_eq!(
        resting(&system, &reached[joined_q].state, &u(0)).as_deref(),
        Some("U1")
    );
    let refork = next(joined_q, StepKind::Fork, "P");
    let p0 = root.child("Main/Loop/Body/P", "left", 0);
    assert!(resting(&system, &reached[refork].state, &p0).is_some());
    let second_q = next(
        next(
            next(next(refork, StepKind::Event, "A1"), StepKind::Attempt, "B1"),
            StepKind::Join,
            "P",
        ),
        StepKind::Fork,
        "Q",
    );
    assert!(resting(&system, &reached[second_q].state, &u(1)).is_some());
    assert!(resting(&system, &reached[second_q].state, &u(0)).is_some());
}

/// ADR-027 PS-2's "no live sibling holds" over a sibling whose branch
/// threads left the state while a thread they forked still runs: a
/// `repeat` with maximum 2 around `parallel Outer`, whose branch `p` binds
/// `f` and forks `Inner` joined `any … outstanding continue`, its branch
/// `c` reading `f`. When the second iteration forks `Outer` while the first
/// `c` still runs, ordinal 0 is not free, so the new threads take ordinal 1
/// and the first `c` keeps reading its own `p`'s binding: the protocol
/// explores. So it does with maximum 1, and with `outstanding cancel`.
#[trace("TC-650")]
#[test]
fn an_ordinal_stays_held_while_a_descendant_of_its_branch_runs() {
    let unit = |max: u64, outstanding: &str| {
        let run = format!(
            "sequence Main {{ repeat Loop by w visible () max {max} while {{ true }} \
             parallel Outer {{ branch p sequence Pb {{ event F by w as (f: Boolean) {{ true }}; \
             parallel Inner {{ branch c {} branch d {} }} join any [c, d] outstanding {outstanding}; }} \
             branch q {} }} join all [p, q]; exhausted sequence Out {{ }} }}",
            attempt("A", "touch", "TouchPre", "aa", "f"),
            touch("Dd"),
            touch("Qq"),
        );
        case("", &run)
    };
    for (max, outstanding) in [(2, "continue"), (1, "continue"), (2, "cancel")] {
        let built = unit(max, outstanding);
        let subject = built.subject("P");
        let system = sc_system(&subject);
        let reached = reachable(&system);
        assert!(!reached.is_empty());
        let late_fork = reached.iter().any(|state| {
            system
                .threads(&state.state)
                .expect("threads")
                .iter()
                .any(|thread| {
                    thread.path.steps.first().is_some_and(|step| {
                        step.node() == Some("Main/Loop/Outer") && step.ordinal() == 1
                    })
                })
        });
        assert_eq!(
            late_fork,
            (max, outstanding) == (2, "continue"),
            "{max} {outstanding}"
        );
    }
}

/// ADR-027 PD-4 for a waiting `receive`: behind another send's head of a
/// `fifo` queue it is `head-blocked`; with its own send's entry deliverable
/// but failing its constraint, `constraint`; with nothing of its send
/// queued, `empty-queue`.
#[trace("TC-651")]
#[test]
fn a_waiting_receive_names_why_it_waits() {
    let premises = "ordering fifo by (m: Boolean) { true } delivery exactly-once-premise \
                    capacity 2 overflow block";
    let cause = |receive: &str| {
        let run = format!(
            "sequence Main {{ send S1 via ch as (s1: Boolean) {{ s1 }}; \
             send S2 via ch as (s2: Boolean) {{ s2 }}; {receive} }}"
        );
        let unit = case(&channel("ch", premises), &run);
        let subject = unit.subject("P");
        let system = sc_system(&subject);
        let reached = reachable(&system);
        let waiting = follow(&reached, &[(StepKind::Send, "S1"), (StepKind::Send, "S2")]);
        assert!(reached[waiting].steps.is_empty());
        system
            .blocked(&reached[waiting].state, &[])
            .expect("blocked")[0]
            .cause
            .clone()
    };
    let channel_name = || "P::ch".to_owned();
    assert_eq!(
        cause("receive R via ch of S2 as (r: Boolean) { true };"),
        WaitCause::HeadBlocked {
            channel: channel_name()
        }
    );
    assert_eq!(
        cause("receive R via ch of S1 as (r: Boolean) { not r };"),
        WaitCause::Constraint
    );
    let run = "sequence Main { send S1 via ch as (s1: Boolean) { s1 }; \
               receive R via ch of S1 as (r: Boolean) { true }; \
               receive Again via ch of S1 as (again: Boolean) { true }; }";
    let unit = case(&channel("ch", premises), run);
    let subject = unit.subject("P");
    let system = sc_system(&subject);
    let reached = reachable(&system);
    let waiting = follow(
        &reached,
        &[(StepKind::Send, "S1"), (StepKind::Receive, "R")],
    );
    assert_eq!(
        system
            .blocked(&reached[waiting].state, &[])
            .expect("blocked")[0]
            .cause,
        WaitCause::EmptyQueue {
            channel: channel_name()
        }
    );
}
