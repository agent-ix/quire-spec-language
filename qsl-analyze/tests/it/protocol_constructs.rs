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
    let blocked_threads = system.blocked(&reached[full].state).expect("blocked");
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
        identity(&system, transition)["record"],
        json!({"type": "send-record", "outcome": "rejected",
            "payload": {"type": "boolean", "value": true}})
    );
    assert_eq!(queues(&system, &reached[*after].state)[0].1, [true]);
    let (first, _) = &reached[0].steps[0];
    assert_eq!(identity(&system, first)["record"]["outcome"], "sent");

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
    let blocked = system.blocked(&reached[last_state].state).expect("blocked");
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
    let refused = compiled(&(contracts("self.v = 0") + &protocol("P", "", &exhausted, "d")))
        .err()
        .expect("`exhausted` needs a maximum");
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
                    thread
                        .path
                        .steps
                        .first()
                        .is_some_and(|step| step.node == "Main/Loop/Outer" && step.ordinal == 1)
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
        system.blocked(&reached[waiting].state).expect("blocked")[0]
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
        system.blocked(&reached[waiting].state).expect("blocked")[0].cause,
        WaitCause::EmptyQueue {
            channel: channel_name()
        }
    );
}
