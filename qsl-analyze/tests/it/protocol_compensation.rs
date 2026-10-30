// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-207 (TC-652), FR-205-AC-5, FR-211-AC-5 and FR-216-AC-4: compensation
//! templates run as protocol threads (ADR-027 CP-1 to CP-5, ST-11, ST-12).
//!
//! ADR-027 §7.1's `Pay` over the `test/fill` fixture: `Acct.bal` is
//! `Cell.v`, `charge` is `inc` and `refund` is `dec` (precondition `v >= 1`,
//! postcondition `v` one lower). The effect binds the account (`M::Cell`,
//! one record over the universe `{c}`), and `Failure.failed` is a Boolean
//! trigger. The recovery predicate is a constant: no rule derives a
//! recovery view from a model state, so a predicate that reads its recovery
//! binder settles V-8 `unsupported`.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_protocol, ModelCheckRefusal, ProtocolCheckRequest};
use qsl_eval::simulation::protocol::{
    visible, AnchorName, ClaimReads, PathStep, ProtocolLocation, ProtocolSystem,
    ProtocolTransition, RegistrationKey, RestKind, Sc, StepKind, ThreadPath, WaitCause,
};
use qsl_eval::simulation::{independent, Footprint, Location, SubjectSystem};
use qsl_foundation::bound::{DomainKey, DomainKind};
use qsl_foundation::BackendId;
use qsl_replay::{
    ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, OutcomeItem, RequestIndex, ResultLabel,
    TerminalRecord, TerminalValue, UnavailabilityCause,
};
use quire_exact::Cancel;
use serde_json::{json, Value as Json};

use crate::fill::{contracts, json_of, key, protocol, reachable, sc_system, Reached};
use crate::protocol_system::{violated, Case};

/// `dec`'s contracts: the refund.
pub(crate) const REFUND: &str = "pre DecPre using v on M::Cell::dec { self.v >= 1 }\n\
                      post DecPost using v on M::Cell::dec { pre(self.v) - self.v = 1 }\n";

/// The template's qualified name.
const TEMPLATE: &str = "Pay::Refund";

/// A `Refund` template for the effect `effect`, of operation `operation`,
/// with `attempts`, retry relation `retry`, commit `commit` and recovery
/// predicate `recover`.
pub(crate) fn refund(
    effect: &str,
    operation: &str,
    attempts: u64,
    retry: &str,
    commit: &str,
    recover: &str,
) -> String {
    refund_of(
        effect,
        operation,
        &format!("{attempts} of M::Attempt"),
        retry,
        commit,
        recover,
    )
}

/// [`refund`] with `attempts` written as the template writes it, `n of T`,
/// the retry binders and the recovery binder all of the attempt type `T`.
fn refund_of(
    effect: &str,
    operation: &str,
    attempts: &str,
    retry: &str,
    commit: &str,
    recover: &str,
) -> String {
    let attempt_type = attempts.rsplit(' ').next().expect("an attempt type");
    format!(
        "compensate Refund for {effect} as (fwd: M::Cell)\n\
         by w on M::Cell::{operation} using v clock \"c\" {{\n\
         activate first (f: Boolean) when {{ f }} {{ }}\n\
         within [0, 10];\n\
         attempts {attempts};\n\
         retry (e: {attempt_type}, l: {attempt_type}) {{ {retry} }};\n\
         commit {commit};\n\
         recover (r: {attempt_type}) {{ {recover} }};\n\
         }}"
    )
}

/// §7.1's charge, `inc` on the account.
pub(crate) const CHARGE: &str =
    "attempt C by w on M::Cell::inc contracts [IncPre, IncPost] as (c: Boolean) { true };";

/// §7.1's effect `Paid` of `of`.
pub(crate) fn paid(of: &str) -> String {
    format!("effect Paid of {of} as (pd: M::Cell) {{ true }};")
}

/// §7.1's failure event.
pub(crate) const FAIL: &str = "event Fail by w as (failed: Boolean) { true };";

/// `Pay`'s `run` with `between` between `Paid` and `Fail`.
pub(crate) fn pay_run(between: &str) -> String {
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
/// false leaves the registration `unrecovered`.
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
        ("observations", Some("attempt-record")) => {
            Some(protocol(ProtocolLocation::Control(path_of(&key["thread"]))))
        }
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

/// SR-12241's fixture: two registrations of a Boolean-forward template,
/// each capturing its forward record, activated by a trigger equal to it,
/// in either order, so that two compensation threads compensate the two
/// registrations crosswise depending on the order of activation.
pub(crate) fn crosswise() -> String {
    let template = "compensate Refund for Main::Loop::Body::Paid as (fwd: Boolean)\n\
         by w on M::Cell::touch using v clock \"c\" {\n\
         capture b: Boolean = fwd;\n\
         activate first (f: Boolean) when { f = b } { }\n\
         within [0, 10];\n\
         attempts 1 of M::Attempt;\n\
         retry (e: M::Attempt, l: M::Attempt) { false };\n\
         commit never;\n\
         recover (r: M::Attempt) { b };\n\
         }";
    let run = format!(
        "sequence Main {{ {} repeat Loop by w visible () max 2 while {{ true }} \
         sequence Body {{ effect Paid of Main::C as (pd: Boolean) {{ true }}; {FAIL} }} \
         exhausted sequence Out {{ }} event Fail2 by w as (failed2: Boolean) {{ true }}; }}",
        CHARGE.replace(
            "M::Cell::inc contracts [IncPre, IncPost]",
            "M::Cell::touch contracts [TouchPre]"
        ),
    );
    unit(template, &run)
}

/// A state's successors, each as its identity's and post-state key's bytes.
type Successors = BTreeSet<(Vec<u8>, Vec<u8>)>;

/// One state's step identities and each step's change (see [`changes`]).
type StateSteps = (Vec<Json>, Vec<(Vec<Location>, bool)>);

/// ADR-027 PS-8: every state `system` reaches, explored without merging
/// states by key, has the successors (identity and post-state key) that
/// the first state of its key has: equal keys are one state. The number of
/// states met again under a known key.
fn assert_keys_determine_successors(system: &ProtocolSystem<'_, Sc>) -> usize {
    use qsl_eval::simulation::TransitionSystem;
    let key = |state: &qsl_eval::simulation::protocol::ProtocolState<Sc>| {
        SubjectSystem::state_key(system, state).expect("a key").0
    };
    let mut known: std::collections::BTreeMap<Vec<u8>, Successors> =
        std::collections::BTreeMap::new();
    let mut pending = system.initial();
    let mut again = 0;
    while let Some(state) = pending.pop() {
        let successors = system
            .successors(&state)
            .expect("the state expands")
            .successors;
        let set: Successors = successors
            .iter()
            .map(|(step, next)| (system.transition_key(step).expect("an identity"), key(next)))
            .collect();
        match known.get(&key(&state)) {
            Some(earlier) => {
                assert!(*earlier == set, "two states of one key step differently");
                again += 1;
            }
            None => {
                known.insert(key(&state), set);
                pending.extend(successors.into_iter().map(|(_, next)| next));
            }
        }
    }
    again
}

/// ADR-027 PS-8 and QSpec FR-425-AC-1 (SR-12241 FND-001): which
/// registration a compensation thread compensates is part of its
/// `thread-state`, so two orders of activation that map the threads to the
/// registrations crosswise reach different keys, and every state of a key
/// steps alike. Without the member, twelve states share a key with a state
/// whose successors differ.
#[test]
fn a_compensation_thread_keys_its_registration() {
    explore(&crosswise(), |system, explored| {
        assert!(assert_keys_determine_successors(system) > 0);
        let reached = &explored.reached;
        let named = reached.iter().any(|state| {
            entries(&key_json(system, state), "control")
                .into_iter()
                .any(|(key, value)| {
                    key["steps"][0].get("template").is_some()
                        && value["registration"]["type"] == "registration"
                })
        });
        assert!(named, "a compensation thread names its registration");
    });
}

/// ADR-027 ST-2: an `effect` is enabled while its attempt's binder is bound
/// in its thread's environment. An attempt in a branch of an `all`-joined
/// `parallel` binds its record, which the join copies to the root, so an
/// effect after the join registers; an attempt in one case of a `choice`
/// enables the effect after the `choice` on that case only, and on the
/// other case the root waits at the effect.
#[trace("TC-651", "FR-206-AC-10")]
#[test]
fn an_effect_waits_for_its_attempts_binder() {
    let joined = format!(
        "sequence Main {{ parallel P {{ branch a {CHARGE} branch b {} }} join all [a, b]; {} {FAIL} }}",
        crate::protocol_system::touch("TB"),
        paid("Main::P::a::C"),
    );
    let body = unit(
        &refund("Main::Paid", "dec", 1, "false", "never", "true"),
        &joined,
    );
    explore(&body, |system, explored| {
        let reached = &explored.reached;
        let registered = reached
            .iter()
            .any(|state| statuses(&key_json(system, state)) == ["registered"]);
        assert!(registered, "the effect after the join registers");
        // At each state right after `join(P)` the effect is enabled and the
        // root's environment holds `C`'s binder, added by the join.
        let mut joins = 0;
        for state in reached {
            for (step, next) in &state.steps {
                if step.kind() != StepKind::Join {
                    continue;
                }
                joins += 1;
                assert!(kinds(reached, *next).contains(&StepKind::Event));
                let held = system.binders(&reached[*next].state).expect("binders");
                assert!(
                    held.iter().any(|(node, holder)| {
                        node.rsplit('/').next() == Some("C") && *holder == ThreadPath::root(0)
                    }),
                    "{held:?}"
                );
            }
        }
        assert!(joins > 0);
    });

    let chosen = format!(
        "sequence Main {{ event X by w as (x: Boolean) {{ true }}; \
         choice Pick by w visible (x and true) {{ case yes when {{ x and true }} {CHARGE} case no when {{ not (x and true) }} {} }} \
         {} {FAIL} }}",
        crate::protocol_system::touch("TN"),
        paid("Main::Pick::yes::C"),
    );
    let body = unit(
        &refund("Main::Paid", "dec", 1, "false", "never", "true"),
        &chosen,
    );
    explore(&body, |system, explored| {
        let reached = &explored.reached;
        let mut stuck = false;
        let mut registered = false;
        for (at, state) in reached.iter().enumerate() {
            registered |= statuses(&key_json(system, state)) == ["registered"];
            let at_paid = system
                .threads(&state.state)
                .expect("the threads name")
                .iter()
                .any(|view| {
                    view.rest_point
                        .node
                        .as_deref()
                        .is_some_and(|node| node.ends_with("Paid"))
                });
            if at_paid && kinds(reached, at).is_empty() {
                stuck = true;
            }
        }
        assert!(
            registered && stuck,
            "the effect steps on `yes` and waits on `no`"
        );
    });
}

/// FR-216-AC-4 (TC-661), the perturbation clause: §7.1 explored from
/// `c.w = 0` and from `c.w = 1`, a location no step of `Pay` reads, takes
/// the same steps from corresponding states, and each step's change is the
/// same: perturbing a location outside every step's `R` changes no step's
/// enabledness or effect.
#[trace("TC-661", "FR-216-AC-4")]
#[test]
fn a_location_outside_the_reads_changes_no_step() {
    let checked = crate::fill::package(&pay("true"));
    let explored: Vec<Vec<StateSteps>> = [0, 1]
        .into_iter()
        .map(|weight| {
            let mut documents = crate::fill::Documents::new();
            let initial = vec![documents.start_cells("s0", &[("c", 0, weight)])];
            let subject = crate::fill::subject(&checked, &documents, initial, "Pay", "c");
            let system = sc_system(&subject);
            let reached = reachable(&system);
            reached
                .iter()
                .map(|state| {
                    let pre = key_json(&system, state);
                    let mut identities = Vec::new();
                    let mut changed = Vec::new();
                    for (step, next) in &state.steps {
                        let footprint = system.footprint(&state.state, step).expect("a footprint");
                        assert!(
                            !footprint.reads.iter().any(|read| matches!(
                                read,
                                Location::Field { field, .. } | Location::AnyField { field }
                                    if *field == crate::fill::key("Cell/w")
                            )),
                            "{step:?} reads c.w"
                        );
                        identities.push(identity(&system, step));
                        changed.push(changes(&pre, &key_json(&system, &reached[*next])));
                    }
                    (identities, changed)
                })
                .collect()
        })
        .collect();
    assert_eq!(explored[0].len(), 9);
    assert_eq!(explored[0], explored[1]);
}

/// §7.1's commit node, after the failure event.
const COMMITTED: &str = "commit Committed by w as (ok: M::Cell) { true };";

/// §7.1's `Pay` with the template attempting `attempts` (`n of T`), its retry
/// relation `retry`, and a `commit Committed` node after `Fail` that the
/// template names when `commit`. The refund is `touch`, which is always
/// enabled.
fn attempt_unit(attempts: &str, retry: &str, commit: bool) -> String {
    let run = format!(
        "sequence Main {{ {CHARGE} {} {FAIL} {COMMITTED} }}",
        paid("Main::C")
    );
    let commit = if commit { "Main::Committed" } else { "never" };
    unit(
        &refund_of("Main::Paid", "touch", attempts, retry, commit, "true"),
        &run,
    )
}

/// The first `bool` in `record`, the wire value of a record of one Boolean
/// field.
fn first_bool(record: &Json) -> Option<bool> {
    match record {
        Json::Bool(value) => Some(*value),
        Json::Array(items) => items.iter().find_map(first_bool),
        Json::Object(members) => members.values().find_map(first_bool),
        _ => None,
    }
}

/// The `cattempt` steps of `reached[at]`: each one's `ok`, its target and
/// its identity.
fn cattempts(
    system: &ProtocolSystem<'_, Sc>,
    reached: &[Reached],
    at: usize,
) -> Vec<(bool, usize, Json)> {
    reached[at]
        .steps
        .iter()
        .filter(|(step, _)| step.kind() == StepKind::CAttempt)
        .map(|(step, next)| {
            let identity = identity(system, step);
            let ok = first_bool(&identity["record"]).expect("a Boolean record");
            (ok, *next, identity)
        })
        .collect()
}

/// The attempt records `key` holds, as (key, value).
fn attempt_records(key: &Json) -> Vec<(Json, Json)> {
    entries(key, "observations")
        .into_iter()
        .filter(|(key, _)| key["type"] == "attempt-record")
        .collect()
}

/// The commit entries `key` holds.
fn commit_entries(key: &Json) -> Vec<(Json, Json)> {
    entries(key, "control")
        .into_iter()
        .filter(|(key, _)| key["type"] == "commit")
        .collect()
}

/// The state `reached[at]` reaches by the step of `Committed`.
fn committed(reached: &[Reached], at: usize) -> usize {
    let found: Vec<usize> = reached[at]
        .steps
        .iter()
        .filter(|(step, _)| step.node().is_some_and(|node| node.ends_with("Committed")))
        .map(|(_, next)| *next)
        .collect();
    assert_eq!(found.len(), 1, "one step of Committed from {at}");
    found[0]
}

/// FR-207-AC-5 (TC-652): with the recovery predicate `r.n = 0`, which reads
/// its recovery binder `r`, the item settles V-8 `unsupported` with cause
/// `RecoveryView{predicate}` naming the template, before any state is
/// explored, and the cause travels as `recovery-view` with its template.
#[trace("TC-652", "FR-207-AC-5")]
#[test]
fn a_recovery_predicate_over_the_recovery_view_settles_unsupported_before_exploring() {
    let body = pay("r.n = 0");
    let case = Case::new(&body, &[0]);
    let outcome = case.deadlock("Pay");
    let ModelCheckOutcome::RecoveryView { predicate, stats } = &outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(predicate, TEMPLATE);
    assert_eq!(stats.product_states, 0);
    let record = case.settle("Pay", &outcome);
    let cause = UnavailabilityCause::RecoveryView {
        predicate: TEMPLATE.to_owned(),
    };
    assert_eq!(record.label, ResultLabel::Unsupported);
    assert_eq!(record.value, TerminalValue::Unsupported(cause.clone()));
    assert_eq!(cause.as_str(), "recovery-view");

    let item = OutcomeItem::from_terminal(&TerminalRecord::unsupported(
        RequestIndex::new(0),
        BackendId::new("backend"),
        cause,
    ));
    let written = serde_json::to_value(&item).expect("an item");
    assert_eq!(written["result"], "unsupported");
    assert_eq!(
        written["cause"],
        json!({"kind": "recovery-view", "predicate": TEMPLATE})
    );
}

/// FR-207-AC-6 (TC-652): with `attempts 2 of M::RefundAttempt` and the retry
/// relation `not e.ok`, two `cattempt` steps are enabled at t4 whose
/// identities differ only in `record`, one with `ok` true and one with
/// false. After the false attempt two retry `cattempt` steps, one per
/// record, and `cend` are enabled; after the true attempt only `cend`. With
/// the relation `l.ok`, either first attempt enables one retry `cattempt`,
/// whose record has `ok` true, and `cend`.
#[trace("TC-652", "FR-207-AC-6")]
#[test]
fn a_cattempt_takes_one_step_per_attempt_record_the_retry_relation_admits() {
    for (retry, admitted) in [
        ("not e.ok", [vec![true, false], vec![]]),
        ("l.ok", [vec![true], vec![true]]),
    ] {
        explore(
            &attempt_unit("2 of M::RefundAttempt", retry, false),
            |system, explored| {
                let reached = &explored.reached;
                let t2 = after(
                    reached,
                    after(reached, 0, StepKind::Attempt),
                    StepKind::Event,
                );
                let (_, t4) = fail(system, reached, t2, true);
                let mut first = cattempts(system, reached, t4);
                first.sort_by_key(|(ok, ..)| *ok);
                let oks: Vec<bool> = first.iter().map(|(ok, ..)| *ok).collect();
                assert_eq!(oks, [false, true]);
                let strip = |identity: &Json| {
                    let mut identity = identity.clone();
                    identity
                        .as_object_mut()
                        .expect("an identity")
                        .remove("record");
                    identity
                };
                assert_eq!(strip(&first[0].2), strip(&first[1].2));
                assert_ne!(first[0].2, first[1].2);

                for (ok, at, _) in &first {
                    let mut retries: Vec<bool> = cattempts(system, reached, *at)
                        .iter()
                        .map(|(ok, ..)| *ok)
                        .collect();
                    retries.sort_by_key(|ok| !ok);
                    let expected = &admitted[usize::from(*ok)];
                    assert_eq!(&retries, expected, "after ok = {ok} under `{retry}`");
                    assert!(kinds(reached, *at).contains(&StepKind::CEnd));
                }
            },
        );
    }
}

/// FR-205-AC-8's second scenario: with `attempts 2 of M::RefundAttempt`, a
/// precondition-free operation and the retry relation `not e.ok`,
/// `observations` holds the thread's attempt record with `ok` after its first
/// `cattempt`, a retry `cattempt` is enabled after `ok` false and only `cend`
/// after `ok` true, and the two post-states have different keys. The record
/// is absent after the second `cattempt`, after `cend` and in the post-state
/// of the step of `Committed` taken while the thread rests at its retry point
/// holding one. With the relation `l.ok`, which does not read `e`, no state
/// holds a record and the two first `cattempt` steps reach one state. The
/// commit entry is in the key after the step of `Committed` and absent once
/// the instance has finished.
fn the_attempt_record_is_keyed_while_a_retry_can_read_it() {
    explore(
        &attempt_unit("2 of M::RefundAttempt", "not e.ok", true),
        |system, explored| {
            let reached = &explored.reached;
            let t2 = after(
                reached,
                after(reached, 0, StepKind::Attempt),
                StepKind::Event,
            );
            let (_, t4) = fail(system, reached, t2, true);
            assert!(attempt_records(&key_json(system, &reached[t4])).is_empty());
            let first = cattempts(system, reached, t4);
            assert_eq!(first.len(), 2);
            let keys: Vec<Json> = first
                .iter()
                .map(|(_, at, _)| key_json(system, &reached[*at]))
                .collect();
            assert_ne!(keys[0], keys[1]);
            // The commit entry is absent once the instance has finished.
            for state in reached {
                let key = key_json(system, state);
                let live = entries(&key, "control")
                    .iter()
                    .any(|(key, _)| key["type"] == "protocol-instance");
                assert!(live || commit_entries(&key).is_empty());
            }
            for (ok, at, _) in &first {
                let held = attempt_records(&key_json(system, &reached[*at]));
                assert_eq!(held.len(), 1, "after ok = {ok}");
                assert_eq!(held[0].0["thread"], path_json(&compensation(0)));
                assert_eq!(first_bool(&held[0].1), Some(*ok));
                let kinds = kinds(reached, *at);
                assert_eq!(kinds.contains(&StepKind::CAttempt), !ok);
                assert!(kinds.contains(&StepKind::CEnd));
                if *ok {
                    continue;
                }
                for (_, second, _) in cattempts(system, reached, *at) {
                    assert!(attempt_records(&key_json(system, &reached[second])).is_empty());
                }
                let ended = after(reached, *at, StepKind::CEnd);
                assert!(attempt_records(&key_json(system, &reached[ended])).is_empty());
                let closed = committed(reached, *at);
                let closed_key = key_json(system, &reached[closed]);
                assert!(attempt_records(&closed_key).is_empty());
                assert_eq!(commit_entries(&closed_key).len(), 1);
            }
        },
    );

    explore(
        &attempt_unit("2 of M::RefundAttempt", "l.ok", true),
        |system, explored| {
            let reached = &explored.reached;
            for state in reached {
                assert!(attempt_records(&key_json(system, state)).is_empty());
            }
            let t2 = after(
                reached,
                after(reached, 0, StepKind::Attempt),
                StepKind::Event,
            );
            let (_, t4) = fail(system, reached, t2, true);
            let first = cattempts(system, reached, t4);
            assert_eq!(first.len(), 2);
            assert_eq!(first[0].1, first[1].1);
        },
    );
}

/// FR-216-AC-7 (TC-661): with `attempts 2` and the retry relation `not
/// e.ok`, `cattempt(Refund, r)` reads and writes the `ctl` of its thread and
/// reads `reg(Refund, ·)`. The step of `Committed` writes `reg(Refund, ·)`
/// and the `ctl` of thread `(0, Refund, 0)` in every state where it exists,
/// including one where it holds an attempt record, and with `attempts 1`,
/// where no thread ever holds one. A `cattempt` whose attempt type has a
/// reference field reads the membership of the referenced population.
#[trace("TC-661", "FR-216-AC-7")]
#[test]
fn cattempt_and_the_commit_step_cover_the_attempt_record() {
    for (attempts, retry, holds) in [
        ("2 of M::RefundAttempt", "not e.ok", true),
        ("1 of M::RefundAttempt", "false", false),
    ] {
        explore(&attempt_unit(attempts, retry, true), |system, explored| {
            let reached = &explored.reached;
            let t2 = after(
                reached,
                after(reached, 0, StepKind::Attempt),
                StepKind::Event,
            );
            let (_, t4) = fail(system, reached, t2, true);
            let (cattempt, _) = reached[t4]
                .steps
                .iter()
                .find(|(step, _)| step.kind() == StepKind::CAttempt)
                .expect("a cattempt");
            let footprint = footprint_at(system, reached, t4, cattempt);
            assert!(footprint.reads.contains(&ctl(&compensation(0))));
            assert!(footprint.writes.contains(&ctl(&compensation(0))));
            assert!(footprint.reads.contains(&registrations()));
            // `RefundAttempt` holds no reference: no population is read.
            let cells = Location::AnyMembership {
                population: key("cells"),
            };
            assert!(!footprint.reads.contains(&cells), "{footprint:?}");
            // Every change, the attempt record's included, is confined to a write.
            assert!(assert_changes_are_written(system, reached) > 0);

            let mut with_record = false;
            let mut checked = 0;
            for (at, state) in reached.iter().enumerate() {
                if !compensations(system, reached, at).contains(&compensation(0)) {
                    continue;
                }
                let Some((step, _)) = state
                    .steps
                    .iter()
                    .find(|(step, _)| step.node().is_some_and(|node| node.ends_with("Committed")))
                else {
                    continue;
                };
                let footprint = footprint_at(system, reached, at, step);
                assert!(footprint.writes.contains(&registrations()));
                assert!(footprint.writes.contains(&ctl(&compensation(0))));
                with_record |= !attempt_records(&key_json(system, state)).is_empty();
                checked += 1;
            }
            assert!(checked > 0);
            assert_eq!(with_record, holds);
        });
    }

    explore(
        &attempt_unit("1 of M::PeerAttempt", "false", false),
        |system, explored| {
            let reached = &explored.reached;
            let t2 = after(
                reached,
                after(reached, 0, StepKind::Attempt),
                StepKind::Event,
            );
            let (_, t4) = fail(system, reached, t2, true);
            let (cattempt, _) = reached[t4]
                .steps
                .iter()
                .find(|(step, _)| step.kind() == StepKind::CAttempt)
                .expect("a cattempt");
            let footprint = footprint_at(system, reached, t4, cattempt);
            let cells = Location::AnyMembership {
                population: key("cells"),
            };
            assert!(footprint.reads.contains(&cells), "{footprint:?}");
            assert!(footprint.enabling.contains(&cells), "{footprint:?}");
        },
    );
}

/// ADR-027 ST-11, TS-5: the records of the attempt type are its whole
/// finite domain. An attempt type with an optional field has three records,
/// the absent one first, each its own `cattempt` step; one with an unbounded
/// field has no finite domain, so the item refuses `requires-bound` before
/// any state is explored.
#[test]
fn an_optional_field_adds_a_record_and_an_unbounded_one_requires_a_bound() {
    explore(
        &attempt_unit("1 of M::OptAttempt", "false", false),
        |system, explored| {
            let reached = &explored.reached;
            let t2 = after(
                reached,
                after(reached, 0, StepKind::Attempt),
                StepKind::Event,
            );
            let (_, t4) = fail(system, reached, t2, true);
            let records: Vec<Json> = reached[t4]
                .steps
                .iter()
                .filter(|(step, _)| step.kind() == StepKind::CAttempt)
                .map(|(step, _)| identity(system, step)["record"].clone())
                .collect();
            assert_eq!(records.len(), 3);
            let absent = records
                .iter()
                .filter(|record| record["fields"][0]["value"].is_null())
                .count();
            assert_eq!(absent, 1, "{records:?}");
        },
    );

    assert_eq!(
        requires_bound("1 of M::WideAttempt"),
        [(vec![0], DomainKind::Integer)]
    );
}

/// The domains of the `requires-bound` refusal of the deadlock-freedom item
/// over `Pay` with the attempt type `attempts`: each one's child-index path
/// and kind; none when the item does not refuse.
fn requires_bound(attempts: &str) -> Vec<(Vec<u32>, DomainKind)> {
    let body = attempt_unit(attempts, "false", false);
    let case = Case::new(&body, &[0]);
    let refused = check_protocol(
        ProtocolCheckRequest {
            subject: case.subject("Pay"),
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
            limits: ModelCheckLimits::default(),
            ignore_memory_constraints: false,
        },
        &Cancel::new(),
    );
    match refused {
        Err(ModelCheckRefusal::RequiresBound(bound)) => bound
            .domains
            .iter()
            .map(|(key, kind)| match key {
                DomainKey::Node { path, .. } => (path.clone(), kind),
                other => panic!("a node domain, got {other:?}"),
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// ADR-027 ST-11, TS-5: an attempt type with a field this system cannot
/// enumerate, though bounded, settles `requires-bound` like an unbounded one,
/// before any state is explored; an enum field enumerates, and each record
/// writes the enum as FR-120's `{"type":"enum","name","member"}`.
#[test]
fn a_string_attempt_field_requires_a_bound_and_an_enum_one_is_named() {
    // The field's path is named; the kinds a domain can have include none
    // for `String`, so it is listed as the unbounded `Integer` it stands in for.
    assert_eq!(
        requires_bound("1 of M::TextAttempt"),
        [(vec![0], DomainKind::Integer)]
    );
    assert!(requires_bound("1 of M::ModeAttempt").is_empty());
    explore(
        &attempt_unit("1 of M::ModeAttempt", "false", false),
        |system, explored| {
            let reached = &explored.reached;
            let t2 = after(
                reached,
                after(reached, 0, StepKind::Attempt),
                StepKind::Event,
            );
            let (_, t4) = fail(system, reached, t2, true);
            let mut members: Vec<(String, Json)> = reached[t4]
                .steps
                .iter()
                .filter(|(step, _)| step.kind() == StepKind::CAttempt)
                .map(|(step, _)| {
                    let value = identity(system, step)["record"]["fields"][0]["value"].clone();
                    (value["member"].as_str().expect("a case").to_owned(), value)
                })
                .collect();
            members.sort_by(|left, right| left.0.cmp(&right.0));
            let cases: Vec<&str> = members.iter().map(|(case, _)| case.as_str()).collect();
            assert_eq!(cases, ["fast", "slow"]);
            let enum_value = &members[0].1;
            assert_eq!(enum_value["type"], "enum");
            assert_eq!(enum_value["name"].as_str().map(str::len), Some(64));
            assert!(enum_value.get("rank").is_none(), "{enum_value}");
        },
    );
}

/// FR-205-AC-8's first scenario: `Refund` with `attempts 1` and `commit
/// Committed`, whose forward `effect` is reached twice in a `repeat` of
/// maximum 2, and then a `parallel Triggers` of two sibling branches that
/// each bind a trigger record, followed by `commit Committed`. The two
/// registrations differ in the capture `id`, and the activation guard
/// compares `id` with the trigger record.
fn registrations_by_trigger() -> String {
    let template = "compensate Refund for Main::Loop::Body::Paid as (fwd: M::Level)\n\
         by w on M::Cell::touch using v clock \"c\" {\n\
         capture id: Boolean = fwd = 1;\n\
         activate first (f: Boolean) when { f = id } { }\n\
         within [0, 10];\n\
         attempts 1 of M::Attempt;\n\
         retry (e: M::Attempt, l: M::Attempt) { false };\n\
         commit Main::Committed;\n\
         recover (r: M::Attempt) { true };\n\
         }";
    let run = format!(
        "sequence Main {{ {} repeat Loop by w visible () max 2 while {{ true }} \
         sequence Body {{ effect Paid of Main::C as (pd: M::Level) {{ pd <= 1 }}; }} \
         exhausted sequence Out {{ }} \
         parallel Triggers {{ branch t1 event T1 by w as (a: Boolean) {{ a }}; \
         branch t2 event T2 by w as (b: Boolean) {{ not b }}; }} join all [t1, t2]; \
         {COMMITTED} }}",
        CHARGE.replace(
            "M::Cell::inc contracts [IncPre, IncPost]",
            "M::Cell::touch contracts [TouchPre]"
        ),
    );
    unit(template, &run)
}

/// FR-205-AC-8 (TC-650): the registrations `r1` and `r2` of the twice-reached
/// effect differ in `id`; after `r1`'s trigger and then `r2`'s, thread
/// `(0, Refund, 0)` names `r1`, and after `r2`'s and then `r1`'s it names
/// `r2`, so the two states, whose registrations are alike, have different
/// keys. The commit entry is in the key after the step of `Committed` and
/// absent once the instance finishes, and in the `attempts 2` variant with
/// the retry relation `not e.ok` the thread's attempt record is keyed while
/// a retry can read it, as `the_attempt_record_is_keyed_while_a_retry_can_read_it`
/// states.
#[trace("TC-650", "FR-205-AC-8")]
#[test]
fn a_thread_names_its_registration_and_the_commit_and_attempt_records_are_keyed() {
    explore(&registrations_by_trigger(), |system, explored| {
        let reached = &explored.reached;
        let registrations = |at: usize| -> Vec<(Json, Json)> {
            entries(&key_json(system, &reached[at]), "control")
                .into_iter()
                .filter(|(key, _)| key["type"] == "registration")
                .collect()
        };
        // For each state with both threads live, the ordinal of the
        // registration thread 0 names, with the registrations beside it.
        let mut named: std::collections::BTreeMap<String, Vec<usize>> =
            std::collections::BTreeMap::new();
        for at in 0..reached.len() {
            if compensations(system, reached, at).len() != 2 {
                continue;
            }
            let key = key_json(system, &reached[at]);
            let thread = entries(&key, "control")
                .into_iter()
                .find(|(key, _)| key["type"] == "thread-path" && path_of(key) == compensation(0))
                .expect("thread 0");
            let ordinal = thread.1["registration"]["ordinal"]
                .as_str()
                .expect("an ordinal")
                .to_owned();
            // Registrations alike but for their status are grouped by their
            // captures.
            let captures: Vec<Json> = registrations(at)
                .into_iter()
                .map(|(key, value)| json!([key["ordinal"], value["captures"]]))
                .collect();
            named
                .entry(format!("{ordinal}/{captures:?}"))
                .or_default()
                .push(at);
        }
        let by_ordinal = |wanted: &str| -> Vec<usize> {
            named
                .iter()
                .filter(|(label, _)| label.starts_with(wanted))
                .flat_map(|(_, states)| states.clone())
                .collect()
        };
        let (first, second) = (by_ordinal("0/"), by_ordinal("1/"));
        assert!(!first.is_empty() && !second.is_empty(), "{named:?}");
        let alike = first.iter().find_map(|a| {
            second
                .iter()
                .find(|b| registrations(*a) == registrations(**b))
                .map(|b| (*a, *b))
        });
        let (a, b) = alike.expect("two orders over alike registrations");
        assert_ne!(key_json(system, &reached[a]), key_json(system, &reached[b]));

        // The commit entry: absent before the step of `Committed`, present
        // after it, absent once the instance has finished.
        let mut committed_seen = false;
        for state in reached {
            let key = key_json(system, state);
            let live = entries(&key, "control")
                .iter()
                .any(|(key, _)| key["type"] == "protocol-instance");
            let entries = commit_entries(&key);
            assert!(live || entries.is_empty());
            committed_seen |= entries.len() == 1;
        }
        assert!(committed_seen);
        assert!(commit_entries(&key_json(system, &reached[0])).is_empty());
    });
    the_attempt_record_is_keyed_while_a_retry_can_read_it();
}
