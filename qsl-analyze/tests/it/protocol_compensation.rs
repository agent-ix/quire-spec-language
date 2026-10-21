// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-207 (TC-652), FR-205-AC-5, FR-211-AC-5 and FR-216-AC-4: compensation
//! templates run as protocol threads (ADR-027 CP-1 to CP-5, ST-11, ST-12).
//!
//! ADR-027 §7.1's `Pay` over the `test/fill` fixture: `Acct.bal` is
//! `Cell.v`, `charge` is `inc` and `refund` is `dec` (precondition `v >= 1`,
//! postcondition `v` one lower). The effect binds the account (`M::Cell`,
//! one record over the universe `{c}`), and `Failure.failed` is a Boolean
//! trigger. The recovery predicate is a constant: no rule derives a
//! recovery view from a model state, so the protocol system refuses a
//! predicate that reads its recovery binder.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_eval::simulation::protocol::{
    visible, AnchorName, ClaimReads, PathStep, ProtocolLocation, ProtocolRefusal, ProtocolSystem,
    ProtocolTransition, RegistrationKey, RestKind, Sc, StepKind, ThreadPath, Unsupported,
    WaitCause,
};
use qsl_eval::simulation::{independent, Footprint, Location, SubjectSystem};
use qsl_replay::{ModelCheckOutcome, ResultLabel};
use serde_json::{json, Value as Json};

use crate::fill::{contracts, json_of, protocol, reachable, sc_system, Reached};
use crate::protocol_system::{violated, Case};

/// `dec`'s contracts: the refund.
const REFUND: &str = "pre DecPre using v on M::Cell::dec { self.v >= 1 }\n\
                      post DecPost using v on M::Cell::dec { pre(self.v) - self.v = 1 }\n";

/// The template's qualified name.
const TEMPLATE: &str = "Pay::Refund";

/// A `Refund` template for the effect `effect`, of operation `operation`,
/// with `attempts`, retry relation `retry`, commit `commit` and recovery
/// predicate `recover`.
fn refund(
    effect: &str,
    operation: &str,
    attempts: u64,
    retry: &str,
    commit: &str,
    recover: &str,
) -> String {
    format!(
        "compensate Refund for {effect} as (fwd: M::Cell)\n\
         by w on M::Cell::{operation} using v clock \"c\" {{\n\
         activate first (f: Boolean) when {{ f }} {{ }}\n\
         within [0, 10];\n\
         attempts {attempts} of M::Attempt;\n\
         retry (e: M::Attempt, l: M::Attempt) {{ {retry} }};\n\
         commit {commit};\n\
         recover (r: M::Attempt) {{ {recover} }};\n\
         }}"
    )
}

/// §7.1's charge, `inc` on the account.
const CHARGE: &str =
    "attempt C by w on M::Cell::inc contracts [IncPre, IncPost] as (c: Boolean) { true };";

/// §7.1's effect `Paid` of `of`.
fn paid(of: &str) -> String {
    format!("effect Paid of {of} as (pd: M::Cell) {{ true }};")
}

/// §7.1's failure event.
const FAIL: &str = "event Fail by w as (failed: Boolean) { true };";

/// `Pay`'s `run` with `between` between `Paid` and `Fail`.
fn pay_run(between: &str) -> String {
    format!(
        "sequence Main {{ {CHARGE} {} {between} {FAIL} }}",
        paid("Main::C")
    )
}

/// A unit of `Pay` with `template` and `run`.
fn unit(template: &str, run: &str) -> String {
    contracts("self.v = 0") + REFUND + &protocol("Pay", template, run, "d")
}

/// ADR-027 §7.1's `Pay` with the recovery predicate `recover`.
pub(crate) fn pay(recover: &str) -> String {
    unit(
        &refund("Main::Paid", "dec", 1, "false", "never", recover),
        &pay_run(""),
    )
}

/// A `Pay` whose effect and failure run twice, in a `repeat` of maximum 2,
/// after one charge, with a refund of `touch`, which is always enabled.
pub(crate) fn twice() -> String {
    let run = format!(
        "sequence Main {{ {} repeat Loop by w visible () max 2 while {{ true }} \
         sequence Body {{ {} {FAIL} }} exhausted sequence Out {{ }} }}",
        CHARGE.replace(
            "M::Cell::inc contracts [IncPre, IncPost]",
            "M::Cell::touch contracts [TouchPre]"
        ),
        paid("Main::C"),
    );
    unit(
        &refund(
            "Main::Loop::Body::Paid",
            "touch",
            1,
            "false",
            "never",
            "true",
        ),
        &run,
    )
}

/// The compensation thread of `Refund` with ordinal `ordinal`.
fn compensation(ordinal: u64) -> ThreadPath {
    ThreadPath::compensation(0, TEMPLATE, ordinal)
}

/// One explored case: its reachable states, explored under `Sc`.
struct Explored {
    reached: Vec<Reached>,
}

/// The reachable states of `Pay` in `body`, and the system, handed to `check`.
fn explore(body: &str, check: impl FnOnce(&ProtocolSystem<'_, Sc>, &Explored)) {
    let case = Case::new(body, &[0]);
    let subject = case.subject("Pay");
    let system = sc_system(&subject);
    let explored = Explored {
        reached: reachable(&system),
    };
    check(&system, &explored);
}

/// The JSON of `state`'s key.
fn key_json(system: &ProtocolSystem<'_, Sc>, state: &Reached) -> Json {
    let (bytes, _) = SubjectSystem::state_key(system, &state.state).expect("a key");
    json_of(&bytes)
}

/// The JSON of `transition`'s identity.
fn identity(system: &ProtocolSystem<'_, Sc>, transition: &ProtocolTransition) -> Json {
    json_of(&system.transition_key(transition).expect("an identity"))
}

/// Each entry of the map member `member` of `key`, as (key, value).
fn entries(key: &Json, member: &str) -> Vec<(Json, Json)> {
    key[member]["entries"]
        .as_array()
        .expect("a map")
        .iter()
        .map(|entry| (entry["key"].clone(), entry["value"].clone()))
        .collect()
}

/// The status of each registration of `key`.
fn statuses(key: &Json) -> Vec<String> {
    entries(key, "control")
        .into_iter()
        .filter(|(key, _)| key["type"] == "registration")
        .map(|(_, value)| value["status"].as_str().expect("a status").to_owned())
        .collect()
}

/// The attempt count of the compensation thread `thread` in `key`.
fn count(key: &Json, thread: &ThreadPath) -> Option<String> {
    entries(key, "bounds")
        .into_iter()
        .find(|(key, _)| key["type"] == "attempt-count" && key["thread"] == path_json(thread))
        .map(|(_, value)| value["value"].as_str().expect("an integer").to_owned())
}

/// `thread` as the state key encodes it.
fn path_json(thread: &ThreadPath) -> Json {
    let steps: Vec<Json> = thread
        .steps
        .iter()
        .map(|step| match step {
            PathStep::Branch {
                node,
                branch,
                ordinal,
            } => json!({"node": node, "branch": branch, "ordinal": ordinal.to_string()}),
            PathStep::Compensation { template, ordinal } => {
                json!({"template": template, "ordinal": ordinal.to_string()})
            }
        })
        .collect();
    json!({"type": "thread-path", "instance": thread.instance.to_string(), "steps": steps})
}

/// The thread a key's thread-path JSON names.
fn path_of(json: &Json) -> ThreadPath {
    let ordinal = |step: &Json| -> u64 {
        step["ordinal"]
            .as_str()
            .and_then(|text| text.parse().ok())
            .expect("an ordinal")
    };
    let text = |value: &Json| value.as_str().expect("a name").to_owned();
    ThreadPath {
        instance: json["instance"]
            .as_str()
            .and_then(|text| text.parse().ok())
            .expect("an instance"),
        steps: json["steps"]
            .as_array()
            .expect("steps")
            .iter()
            .map(|step| match step.get("template") {
                Some(template) => PathStep::Compensation {
                    template: text(template),
                    ordinal: ordinal(step),
                },
                None => PathStep::Branch {
                    node: text(&step["node"]),
                    branch: text(&step["branch"]),
                    ordinal: ordinal(step),
                },
            })
            .collect(),
    }
}

/// The kinds of the steps of `reached[at]`.
fn kinds(reached: &[Reached], at: usize) -> Vec<StepKind> {
    reached[at]
        .steps
        .iter()
        .map(|(step, _)| step.kind())
        .collect()
}

/// The state the one step of `kind` from `reached[at]` reaches.
fn after(reached: &[Reached], at: usize, kind: StepKind) -> usize {
    let found: Vec<usize> = reached[at]
        .steps
        .iter()
        .filter(|(step, _)| step.kind() == kind)
        .map(|(_, next)| *next)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "one {kind:?} from {at}: {:?}",
        kinds(reached, at)
    );
    found[0]
}

/// The `event(Fail)` from `reached[at]` binding `failed`, and its target.
fn fail<'r>(
    system: &ProtocolSystem<'_, Sc>,
    reached: &'r [Reached],
    at: usize,
    failed: bool,
) -> (&'r ProtocolTransition, usize) {
    reached[at]
        .steps
        .iter()
        .find(|(step, _)| {
            step.kind() == StepKind::Event
                && step.node().is_some_and(|node| node.ends_with("Fail"))
                && identity(system, step)["record"]["value"] == json!(failed)
        })
        .map(|(step, next)| (step, *next))
        .unwrap_or_else(|| panic!("event(Fail) with {failed} from {at}"))
}

/// The compensation threads live at `reached[at]`.
fn compensations(
    system: &ProtocolSystem<'_, Sc>,
    reached: &[Reached],
    at: usize,
) -> Vec<ThreadPath> {
    system
        .threads(&reached[at].state)
        .expect("the threads name")
        .into_iter()
        .map(|view| view.path)
        .filter(|path| {
            path.steps
                .iter()
                .any(|step| matches!(step, PathStep::Compensation { .. }))
        })
        .collect()
}

/// FR-207-AC-1 (TC-652): ADR-027 §7.1's `Pay` reaches exactly t0 to t8 with
/// the enabled steps of its table, nine states and eight transitions.
/// `event(Paid)` registers `Refund` in its own step; `event(Fail)` with
/// `failed` true activates it in its own step and creates the compensation
/// thread `(0, Refund, 0)` at its compensation attempt with count 0; with
/// `failed` false the registration stays `registered` until `finish`, which
/// removes the instance with its registration.
#[trace("TC-652", "FR-207-AC-1")]
#[test]
fn pay_reaches_the_nine_states_of_its_table() {
    explore(&pay("true"), |system, explored| {
        let reached = &explored.reached;
        assert_eq!(reached.len(), 9);
        let transitions: usize = reached.iter().map(|state| state.steps.len()).sum();
        assert_eq!(transitions, 8);
        let status = |at: usize| statuses(&key_json(system, &reached[at]));

        let t0 = 0;
        assert_eq!(kinds(reached, t0), [StepKind::Attempt]);
        let t1 = after(reached, t0, StepKind::Attempt);
        assert_eq!(kinds(reached, t1), [StepKind::Event]);
        assert!(status(t1).is_empty());
        let t2 = after(reached, t1, StepKind::Event);
        assert_eq!(status(t2), ["registered"]);
        assert_eq!(kinds(reached, t2), [StepKind::Event, StepKind::Event]);
        let (_, t3) = fail(system, reached, t2, false);
        let (_, t4) = fail(system, reached, t2, true);
        assert_eq!(status(t3), ["registered"]);
        assert!(compensations(system, reached, t3).is_empty());
        assert_eq!(kinds(reached, t3), [StepKind::Finish]);
        let t5 = after(reached, t3, StepKind::Finish);
        assert!(!reached[t5].state.has_live_instance());
        assert!(status(t5).is_empty());

        assert_eq!(status(t4), ["activated"]);
        assert_eq!(compensations(system, reached, t4), [compensation(0)]);
        let key = key_json(system, &reached[t4]);
        assert_eq!(count(&key, &compensation(0)).as_deref(), Some("0"));
        let view = system
            .threads(&reached[t4].state)
            .expect("the threads name")
            .into_iter()
            .find(|view| view.path == compensation(0))
            .expect("the compensation thread");
        assert_eq!(view.rest_point.kind, RestKind::Node);
        assert_eq!(view.rest_point.node.as_deref(), Some(TEMPLATE));
        assert_eq!(kinds(reached, t4), [StepKind::CAttempt]);
        let t6 = after(reached, t4, StepKind::CAttempt);
        assert_eq!(crate::fill::v(reached[t6].state.model(), "c"), 0);
        assert_eq!(kinds(reached, t6), [StepKind::CEnd]);
        let t7 = after(reached, t6, StepKind::CEnd);
        assert_eq!(status(t7), ["recovered"]);
        assert!(compensations(system, reached, t7).is_empty());
        assert_eq!(kinds(reached, t7), [StepKind::Finish]);
        let t8 = after(reached, t7, StepKind::Finish);
        assert!(!reached[t8].state.has_live_instance());
        assert!(kinds(reached, t8).is_empty() && kinds(reached, t5).is_empty());
        assert_ne!(t5, t8);

        // `cattempt` and `cend` name the template and the registration.
        let (cattempt, _) = &reached[t4].steps[0];
        let json = identity(system, cattempt);
        assert_eq!(json["step"], "cattempt");
        assert_eq!(json["template"], TEMPLATE);
        assert!(json["operation"]
            .as_str()
            .is_some_and(|operation| operation.contains("Cell/dec")));
        assert_eq!(
            json["registration"],
            json!({"type": "registration", "template": TEMPLATE,
                "thread": path_json(&ThreadPath::root(0)), "ordinal": "0"})
        );
        assert!(json.get("node").is_none(), "{json}");
        let (cend, _) = &reached[t6].steps[0];
        assert_eq!(identity(system, cend)["step"], "cend");
        assert_eq!(cend.thread(), Some(&compensation(0)));
    });
}

/// Part of FR-207-AC-2: in §7.1 `finish` is not enabled at t4 or t6 while
/// the compensation thread is `running`; at t6 the count has reached
/// `attempts 1`, so `cend` is the only step; a recovery predicate that is
/// false leaves the registration `unrecovered`. The criterion's `bal = 0`
/// reads the recovery view, which no rule derives from a model state: a
/// predicate that reads the recovery binder is refused.
#[test]
fn finish_waits_for_the_compensation_thread_and_cend_records_its_status() {
    explore(&pay("false"), |system, explored| {
        let reached = &explored.reached;
        let t2 = after(
            reached,
            after(reached, 0, StepKind::Attempt),
            StepKind::Event,
        );
        let (_, t4) = fail(system, reached, t2, true);
        let t6 = after(reached, t4, StepKind::CAttempt);
        assert!(!kinds(reached, t4).contains(&StepKind::Finish));
        assert_eq!(kinds(reached, t6), [StepKind::CEnd]);
        assert_eq!(
            count(&key_json(system, &reached[t6]), &compensation(0)).as_deref(),
            Some("1")
        );
        let t7 = after(reached, t6, StepKind::CEnd);
        assert_eq!(statuses(&key_json(system, &reached[t7])), ["unrecovered"]);
    });

    let case = Case::new(&pay("r.n = 0"), &[0]);
    let subject = case.subject("Pay");
    let model = qsl_eval::simulation::model::ModelSystem::new(subject.model.simulation.clone())
        .expect("the subject admits");
    let refused = ProtocolSystem::new(model, &subject, Sc).err();
    assert_eq!(
        refused,
        Some(ProtocolRefusal::Unsupported(Unsupported::RecoveryView))
    );
}

/// FR-207-AC-3 (TC-652): with `attempts 2` and the retry relation `true`,
/// `cattempt(Refund)` and `cend(Refund)` are both enabled at the retry point
/// after the first attempt, and only `cend` after the second; with the
/// relation `false`, only `cend` after the first. The refund is `touch`,
/// which is always enabled, so only the count and the relation decide.
#[trace("TC-652", "FR-207-AC-3")]
#[test]
fn the_retry_relation_and_the_count_decide_a_retry() {
    for (retry, retries) in [("true", true), ("false", false)] {
        let body = unit(
            &refund("Main::Paid", "touch", 2, retry, "never", "true"),
            &pay_run(""),
        );
        explore(&body, |system, explored| {
            let reached = &explored.reached;
            let t2 = after(
                reached,
                after(reached, 0, StepKind::Attempt),
                StepKind::Event,
            );
            let (_, t4) = fail(system, reached, t2, true);
            let first = after(reached, t4, StepKind::CAttempt);
            let view = system
                .threads(&reached[first].state)
                .expect("the threads name")
                .into_iter()
                .find(|view| view.path == compensation(0))
                .expect("the compensation thread");
            assert_eq!(view.rest_point.kind, RestKind::RetryPoint);
            if retries {
                assert_eq!(kinds(reached, first), [StepKind::CAttempt, StepKind::CEnd]);
                let second = after(reached, first, StepKind::CAttempt);
                assert_eq!(
                    count(&key_json(system, &reached[second]), &compensation(0)).as_deref(),
                    Some("2")
                );
                assert_eq!(kinds(reached, second), [StepKind::CEnd]);
            } else {
                assert_eq!(kinds(reached, first), [StepKind::CEnd]);
            }
        });
    }
}

/// FR-207-AC-4 (TC-652): with a `commit Ok` node between `Paid` and `Fail`
/// and the template's `commit Main::Ok`, the registration closes at `Ok`'s
/// step, `event(Fail)` with `failed` true activates nothing, and no state
/// holds a compensation thread. With the effect and the commit in a
/// `repeat` of maximum 2, the second step of `Paid`, after `Ok`'s, registers
/// nothing.
#[trace("TC-652", "FR-207-AC-4")]
#[test]
fn the_commit_closes_the_registration_and_stops_new_ones() {
    let ok = "commit Ok by w as (ok: M::Cell) { true };";
    let body = unit(
        &refund("Main::Paid", "dec", 1, "false", "Main::Ok", "true"),
        &pay_run(ok),
    );
    explore(&body, |system, explored| {
        let reached = &explored.reached;
        let t2 = after(
            reached,
            after(reached, 0, StepKind::Attempt),
            StepKind::Event,
        );
        assert_eq!(statuses(&key_json(system, &reached[t2])), ["registered"]);
        let committed = after(reached, t2, StepKind::Event);
        assert_eq!(statuses(&key_json(system, &reached[committed])), ["closed"]);
        let (_, failed) = fail(system, reached, committed, true);
        assert_eq!(statuses(&key_json(system, &reached[failed])), ["closed"]);
        for at in 0..reached.len() {
            assert!(compensations(system, reached, at).is_empty(), "{at}");
            assert!(!kinds(reached, at).contains(&StepKind::CAttempt));
        }
    });

    let run = format!(
        "sequence Main {{ {CHARGE} repeat Loop by w visible () max 2 while {{ true }} \
         sequence Body {{ {} {ok} }} exhausted sequence Out {{ }} {FAIL} }}",
        paid("Main::C"),
    );
    let body = unit(
        &refund(
            "Main::Loop::Body::Paid",
            "dec",
            1,
            "false",
            "Main::Loop::Body::Ok",
            "true",
        ),
        &run,
    );
    explore(&body, |system, explored| {
        let reached = &explored.reached;
        let mut paid_twice = false;
        for state in reached {
            let key = key_json(system, state);
            assert!(statuses(&key).len() <= 1, "{key}");
            for (step, next) in &state.steps {
                let second = step.node().is_some_and(|node| node.ends_with("Paid"))
                    && statuses(&key) == ["closed"];
                if second {
                    paid_twice = true;
                    assert_eq!(statuses(&key_json(system, &reached[*next])), ["closed"]);
                }
            }
        }
        assert!(paid_twice, "the second iteration takes `Paid`");
    });
}

/// FR-205-AC-5 (TC-650): a template `Refund` with two registrations, whose
/// first activates and whose compensation thread `(0, Refund, 0)` is still
/// live when the second activates: the second thread takes ordinal 1. When
/// `cend` has removed `(0, Refund, 0)` before the second activates, the
/// second thread takes ordinal 0.
#[trace("TC-650", "FR-205-AC-5")]
#[test]
fn a_compensation_thread_takes_the_smallest_free_ordinal() {
    explore(&twice(), |system, explored| {
        let reached = &explored.reached;
        let mut beside = false;
        let mut reused = false;
        for (at, state) in reached.iter().enumerate() {
            let status = statuses(&key_json(system, state));
            let waiting = status
                .iter()
                .filter(|status| *status == "registered")
                .count();
            for (step, next) in &state.steps {
                let activates = step.node().is_some_and(|node| node.ends_with("Fail"))
                    && identity(system, step)["record"]["value"] == json!(true);
                // The second registration activates, the first having
                // activated before.
                if !activates || status.len() < 2 || waiting != 1 {
                    continue;
                }
                let before = compensations(system, reached, at);
                let created: Vec<ThreadPath> = compensations(system, reached, *next)
                    .into_iter()
                    .filter(|thread| !before.contains(thread))
                    .collect();
                match before[..] {
                    [ref live] => {
                        assert_eq!(*live, compensation(0));
                        assert_eq!(created, [compensation(1)]);
                        beside = true;
                    }
                    [] => {
                        assert_eq!(created, [compensation(0)]);
                        reused = true;
                    }
                    _ => panic!("{before:?}"),
                }
            }
        }
        assert!(beside && reused, "both orders are reached");
    });
}

/// The location a changed state-key entry of `member` names, for an entry
/// that stands for one.
fn changed_location(member: &str, key: &Json) -> Option<Location> {
    let protocol = Location::Protocol;
    match (member, key["type"].as_str()) {
        ("control", Some("thread-path")) | ("bounds", Some("attempt-count")) => Some(protocol(
            ProtocolLocation::Control(path_of(if member == "control" {
                key
            } else {
                &key["thread"]
            })),
        )),
        ("control", Some("registration")) => Some(protocol(ProtocolLocation::Registration {
            instance: 0,
            key: RegistrationKey {
                template: key["template"].as_str().expect("a template").to_owned(),
                thread: path_of(&key["thread"]),
                ordinal: key["ordinal"]
                    .as_str()
                    .and_then(|text| text.parse().ok())
                    .expect("an ordinal"),
            },
        })),
        ("observations", Some("binder")) => Some(protocol(ProtocolLocation::Binder {
            node: key["node"].as_str().expect("a node").to_owned(),
            thread: path_of(&key["thread"]),
        })),
        _ => None,
    }
}

/// The locations whose entries differ between the keys `pre` and `post`,
/// and whether the model state differs.
fn changes(pre: &Json, post: &Json) -> (Vec<Location>, bool) {
    let mut changed = Vec::new();
    for member in ["control", "observations", "bounds"] {
        let before = entries(pre, member);
        let later = entries(post, member);
        for (key, value) in before.iter().chain(&later) {
            let same = before.contains(&(key.clone(), value.clone()))
                && later.contains(&(key.clone(), value.clone()));
            if !same {
                changed.extend(changed_location(member, key));
            }
        }
    }
    (changed, pre["semantic"] != post["semantic"])
}

/// Every change of every step of `reached` is confined to its write
/// footprint (ADR-027 FT-1): each changed thread, binder and registration
/// entry meets a written location, and a changed model state has a written
/// model location. The number of steps checked.
fn assert_changes_are_written(system: &ProtocolSystem<'_, Sc>, reached: &[Reached]) -> usize {
    let mut checked = 0;
    for state in reached {
        let pre = key_json(system, state);
        for (step, next) in &state.steps {
            let footprint = system.footprint(&state.state, step).expect("a footprint");
            let (locations, model) = changes(&pre, &key_json(system, &reached[*next]));
            for location in &locations {
                assert!(
                    footprint
                        .writes
                        .iter()
                        .any(|written| written.meets(location)),
                    "{step:?} changes {location:?} outside {:?}",
                    footprint.writes
                );
            }
            if model {
                assert!(
                    footprint
                        .writes
                        .iter()
                        .any(|written| !matches!(written, Location::Protocol(_))),
                    "{step:?} changes the model outside {:?}",
                    footprint.writes
                );
            }
            checked += 1;
        }
    }
    checked
}

/// `reg(Refund, ·)` of instance 0.
fn registrations() -> Location {
    Location::Protocol(ProtocolLocation::Registrations {
        instance: 0,
        template: TEMPLATE.to_owned(),
    })
}

fn ctl(thread: &ThreadPath) -> Location {
    Location::Protocol(ProtocolLocation::Control(thread.clone()))
}

/// The footprint of `step` from `reached[at]`.
fn footprint_at(
    system: &ProtocolSystem<'_, Sc>,
    reached: &[Reached],
    at: usize,
    step: &ProtocolTransition,
) -> Footprint {
    system
        .footprint(&reached[at].state, step)
        .expect("a footprint")
}

/// FR-216-AC-4 (TC-661): in ADR-027 §7.1, `event(Paid)` writes
/// `reg(Refund, ·)`; `event(Fail)` reads and writes it and writes the
/// compensation thread's `ctl`. Every step's change, from every reachable
/// state, is confined to its write footprint. Where `Refund` has two
/// registrations, `cend(Refund, r1)` of `(0, Refund, 0)` writes `reg(Refund,
/// r1)` and that thread's `ctl`, and the step that activates the second
/// registration reads that `ctl`; the two are dependent, and their two
/// orders give the new thread ordinals 1 and 0. A `cattempt` is visible to
/// an atom that reads its anchor and names its template, and invisible to
/// one naming another template.
#[trace("TC-661", "FR-216-AC-4")]
#[test]
fn compensation_steps_read_and_write_their_registrations() {
    explore(&pay("true"), |system, explored| {
        let reached = &explored.reached;
        let t1 = after(reached, 0, StepKind::Attempt);
        let (paid, _) = &reached[t1].steps[0];
        let footprint = footprint_at(system, reached, t1, paid);
        assert!(footprint.writes.contains(&registrations()), "{footprint:?}");
        let t2 = after(reached, t1, StepKind::Event);
        let (failing, t4) = fail(system, reached, t2, true);
        let footprint = footprint_at(system, reached, t2, failing);
        assert!(footprint.reads.contains(&registrations()));
        assert!(footprint.writes.contains(&registrations()));
        assert!(footprint.writes.contains(&ctl(&compensation(0))));
        assert!(assert_changes_are_written(system, reached) == 8);

        let (cattempt, _) = &reached[t4].steps[0];
        let footprint = footprint_at(system, reached, t4, cattempt);
        let naming = |template: &str| ClaimReads {
            locations: BTreeSet::new(),
            anchors: BTreeSet::from([AnchorName::Template(template.to_owned())]),
        };
        assert!(visible(cattempt, &footprint, &naming(TEMPLATE)));
        assert!(!visible(cattempt, &footprint, &naming("Pay::Other")));
    });

    explore(&twice(), |system, explored| {
        let reached = &explored.reached;
        assert!(assert_changes_are_written(system, reached) > 0);
        let first = RegistrationKey {
            template: TEMPLATE.to_owned(),
            thread: ThreadPath::root(0),
            ordinal: 0,
        };
        let mut raced = false;
        for (at, state) in reached.iter().enumerate() {
            let cend = state.steps.iter().find(|(step, _)| {
                step.kind() == StepKind::CEnd && step.thread() == Some(&compensation(0))
            });
            let activating = state.steps.iter().find(|(step, _)| {
                step.node().is_some_and(|node| node.ends_with("Fail"))
                    && identity(system, step)["record"]["value"] == json!(true)
            });
            let (Some((cend, ended)), Some((activating, activated))) = (cend, activating) else {
                continue;
            };
            if statuses(&key_json(system, state)).len() < 2 {
                continue;
            }
            let ending = footprint_at(system, reached, at, cend);
            assert!(ending.writes.contains(&ctl(&compensation(0))));
            assert!(ending
                .writes
                .contains(&Location::Protocol(ProtocolLocation::Registration {
                    instance: 0,
                    key: first.clone(),
                })));
            let activation = footprint_at(system, reached, at, activating);
            assert!(activation.reads.contains(&ctl(&compensation(0))));
            assert!(!independent(&ending, &activation));
            assert!(compensations(system, reached, *activated).contains(&compensation(1)));
            let later = reached[*ended]
                .steps
                .iter()
                .find(|(step, _)| step == activating)
                .map(|(_, next)| *next)
                .expect("the activation stays enabled");
            assert_eq!(compensations(system, reached, later), [compensation(0)]);
            raced = true;
        }
        assert!(raced, "a state with both steps enabled");
    });
}

/// §7.1's `Pay` as FR-211-AC-5 runs it: a `parallel Both` joined `all` of
/// branch `pay` (the charge, `Paid` and `Fail`) and branch `settle`, which
/// takes `before` and then `Committed`, the template's commit node.
pub(crate) fn settled(before: &str) -> String {
    let run = format!(
        "parallel Both {{ branch pay sequence Main {{ {CHARGE} {} {FAIL} }} \
         branch settle sequence Settle {{ {before} commit Committed by w as (ok: M::Cell) {{ true }}; }} }} \
         join all [pay, settle];",
        paid("pay::Main::C"),
    );
    unit(
        &refund(
            "Both::pay::Main::Paid",
            "dec",
            1,
            "false",
            "Both::settle::Settle::Committed",
            "true",
        ),
        &run,
    )
}

/// Part of FR-211-AC-5 (TC-656, ADR-027 PD-4): §7.1's `Pay` with its
/// template committing at `Committed` and its `run` a `parallel Both`
/// joined `all` of branch `pay` (the charge, `Paid` and `Fail`) and branch
/// `settle`, which takes `Committed`, deadlocks once `Fail` with `failed`
/// true has created `(0, Refund, 0)` and `settle` takes `Committed` while
/// that thread is still at its first compensation attempt: the thread waits
/// with `CommitClosed{Refund}` and the root at `finish` with
/// `FinishWaiting{[(0, Refund, 0)]}`. Replay recomputes the list.
#[test]
fn a_closed_compensation_attempt_waits_with_commit_closed() {
    let body = settled("");
    let case = Case::new(&body, &[0]);
    let outcome = case.deadlock("Pay");
    let counterexample = violated(&outcome);
    let causes: Vec<(ThreadPath, WaitCause)> = counterexample
        .blocked
        .iter()
        .map(|thread| (thread.path.clone(), thread.cause.clone()))
        .collect();
    assert_eq!(
        causes,
        [
            (
                ThreadPath::root(0),
                WaitCause::FinishWaiting {
                    threads: vec![compensation(0)]
                }
            ),
            (
                compensation(0),
                WaitCause::CommitClosed {
                    template: TEMPLATE.to_owned()
                }
            ),
        ]
    );
    assert!(matches!(outcome, ModelCheckOutcome::Violated { .. }));
    let record = case.settle("Pay", &outcome);
    assert_eq!(record.label, ResultLabel::Refuted, "{record:?}");
}

/// ADR-027 PX-2: a deadlock counterexample whose prefix takes `cattempt`
/// and `cend` replays through the protocol system. Every `Fail` activates
/// the refund, and the root then waits at a false `check` after the
/// compensation thread has ended.
#[test]
fn a_prefix_through_a_compensation_replays() {
    let body = unit(
        &refund("Main::Paid", "dec", 1, "false", "never", "true")
            .replace("when { f }", "when { true }"),
        &pay_run("").replace(FAIL, &format!("{FAIL} check Never using v {{ false }};")),
    );
    let case = Case::new(&body, &[0]);
    let outcome = case.deadlock("Pay");
    let counterexample = violated(&outcome);
    let kinds: Vec<StepKind> = counterexample
        .prefix
        .iter()
        .map(|step| step.transition.kind())
        .collect();
    assert_eq!(
        kinds,
        [
            StepKind::Attempt,
            StepKind::Event,
            StepKind::Event,
            StepKind::CAttempt,
            StepKind::CEnd
        ]
    );
    let [blocked] = &counterexample.blocked[..] else {
        panic!("{:?}", counterexample.blocked);
    };
    assert!(matches!(blocked.cause, WaitCause::CheckFalse { .. }));
    let record = case.settle("Pay", &outcome);
    assert_eq!(record.label, ResultLabel::Refuted, "{record:?}");
}

/// ADR-027 CP-1 and CP-2: a registration capture is evaluated over the
/// forward record when `Paid` registers, and an activation capture over the
/// trigger when `Fail` activates; both are in the registration's key entry,
/// and the recovery predicate reads them at `cend`. With the guard `true`,
/// `Fail` with `failed` false activates too, and the recovery predicate
/// `failedThen` leaves that registration `unrecovered`.
#[test]
fn captures_are_evaluated_at_registration_and_activation() {
    let template = refund("Main::Paid", "touch", 1, "false", "never", "failedThen").replace(
        "activate first (f: Boolean) when { f } { }",
        "capture account: M::Cell = fwd;\n\
             activate first (f: Boolean) when { true } { capture failedThen: Boolean = f; }",
    );
    explore(&unit(&template, &pay_run("")), |system, explored| {
        let reached = &explored.reached;
        let t2 = after(
            reached,
            after(reached, 0, StepKind::Attempt),
            StepKind::Event,
        );
        let captures = |at: usize| -> Vec<Json> {
            entries(&key_json(system, &reached[at]), "control")
                .into_iter()
                .filter(|(key, _)| key["type"] == "registration")
                .flat_map(|(_, value)| value["captures"]["entries"].as_array().cloned())
                .flatten()
                .map(|entry| entry["key"]["value"].clone())
                .collect()
        };
        assert_eq!(captures(t2), [json!("account")]);
        for (failed, status) in [(true, "recovered"), (false, "unrecovered")] {
            let (_, activated) = fail(system, reached, t2, failed);
            assert_eq!(
                statuses(&key_json(system, &reached[activated])),
                ["activated"]
            );
            let mut named = captures(activated);
            named.sort_by_key(ToString::to_string);
            assert_eq!(named, [json!("account"), json!("failedThen")]);
            let ended = after(
                reached,
                after(reached, activated, StepKind::CAttempt),
                StepKind::CEnd,
            );
            assert_eq!(statuses(&key_json(system, &reached[ended])), [status]);
        }
    });
}
