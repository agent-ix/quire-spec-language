// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-249-AC-7 to AC-9 (TC-3008): a task automaton lowers to a timed subject
//! that carries the declared periods and deadlines, EN-6's verdict on it
//! equals EN-7's response-time verdict on the three-task set `B`, and an
//! unschedulable set refutes with a replayed counterexample whose first
//! deadline miss is exact. The source is ADR-026 section 11's `Rpc` with one
//! always-enabled `fire_*` operation per task, each releasing its task.

use fsl_eval::simulation::model::{
    TaskProductState, TaskProductSystem, TaskTransitionAction, TimedSystem,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, request_over, Run, Workspace};
use crate::analyze_family_cli::{rpc_snapshot, rpc_unit};

/// One always-enabled operation per task, each releasing that task of `set`.
fn sources(set: &str, tasks: &[&str]) -> String {
    tasks
        .iter()
        .map(|task| {
            format!(
                "post Fire_{task} using v on M::Call::fire_{task} {{ self.phase = pre(self.phase) }}\n\
                 release {set}.{task} on M::Call::fire_{task};\n"
            )
        })
        .collect()
}

fn run(name: &str, tasks: &[&str], declarations: &str, items: Value) -> Run {
    let ws = Workspace::new(name);
    let operations: Vec<(String, &[&str])> = tasks
        .iter()
        .map(|task| (format!("fire_{task}"), &["phase"][..]))
        .collect();
    let extra: Vec<(&str, &[&str])> = operations
        .iter()
        .map(|(name, fields)| (name.as_str(), *fields))
        .collect();
    analyze(&request_over(
        &ws,
        &rpc_unit(4, declarations),
        &crate::zone_search::rpc_document_with(&extra),
        &[rpc_snapshot()],
        json!({"items": items}),
    ))
}

/// `Pair`: `a` periodic 4 ms, deadline 3 ms; `b` sporadic 6 ms, deadline
/// 5 ms; both fixed-work.
const PAIR: &str = "taskset Pair {\n\
     task a periodic 4 ms deadline 3 ms bcet 1 ms wcet 1 ms priority 1 lo;\n\
     task b sporadic 6 ms deadline 5 ms bcet 2 ms wcet 2 ms priority 2 lo;\n\
     }\n\
     schedulable PairFp on Pair under fixed_priority;\n";

/// Task set `B`, `(C, T = D)` of `(1, 4)`, `(2, 6)` and
/// `(third, 10)` ms, priorities by period, every task fixed-work.
fn task_set_b(third: u32) -> String {
    format!(
        "taskset B {{\n\
         task t1 periodic 4 ms deadline 4 ms bcet 1 ms wcet 1 ms priority 1 lo;\n\
         task t2 periodic 6 ms deadline 6 ms bcet 2 ms wcet 2 ms priority 2 lo;\n\
         task t3 periodic 10 ms deadline 10 ms bcet {third} ms wcet {third} ms priority 3 lo;\n\
         }}\n\
         schedulable BFp on B under fixed_priority;\n\
         schedulable BEdf on B under edf;\n{}",
        sources("B", &["t1", "t2", "t3"])
    )
}

/// FR-249-AC-3: concrete scheduler progress stops at active task events.
/// t1 completes at 1, t2 completes before the next t1 release at 4, and the
/// first deadline miss is t3#0 at 10 with exactly three units of service.
#[trace("FR-249-AC-3")]
#[test]
fn task_scheduler_stops_at_completion_and_release_boundaries() {
    let model_document = crate::zone_search::rpc_document_with(&[
        ("fire_t1", &["phase"]),
        ("fire_t2", &["phase"]),
        ("fire_t3", &["phase"]),
    ]);
    let body = format!("{}{}", crate::timed_world::DENSE, task_set_b(4));
    let world = crate::timed_world::world(&model_document, crate::zone_search::TIMED, &body);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "BFp").expect("checked task product");
    let mut state = product.initial().into_iter().next().expect("initial state");

    for task in ["t1", "t2", "t3"] {
        state = product
            .source_steps(&state)
            .expect("source transitions")
            .into_iter()
            .find(|step| {
                step.activations
                    .iter()
                    .any(|activation| activation.task == task)
            })
            .expect("the task release is enabled")
            .state;
    }
    let take_action = |state: &mut TaskProductState, action: TaskTransitionAction, task: &str| {
        let step = product
            .scheduler_steps(state)
            .into_iter()
            .find(|step| step.identity.action == action && step.identity.task == task)
            .expect("the expected scheduler action is enabled");
        *state = step.state;
    };

    take_action(&mut state, TaskTransitionAction::Dispatch, "t1");
    state = product
        .delay(&state, &crate::zone_search::exact(1, 1))
        .expect("source admits time")
        .expect("t1 reaches its completion boundary at 1");
    take_action(&mut state, TaskTransitionAction::Complete, "t1");
    take_action(&mut state, TaskTransitionAction::Dispatch, "t2");
    state = product
        .delay(&state, &crate::zone_search::exact(2, 1))
        .expect("source admits time")
        .expect("t2 reaches its completion boundary at 3");
    take_action(&mut state, TaskTransitionAction::Complete, "t2");
    take_action(&mut state, TaskTransitionAction::Dispatch, "t3");
    assert_eq!(state.now(), &crate::zone_search::exact(3, 1));

    state = product
        .delay(&state, &crate::zone_search::exact(1, 1))
        .expect("source admits time")
        .expect("stop at t1's next periodic release at 4");
    assert_eq!(state.now(), &crate::zone_search::exact(4, 1));
    state = product
        .source_steps(&state)
        .expect("source transitions")
        .into_iter()
        .find(|step| {
            step.activations
                .iter()
                .any(|activation| activation.task == "t1")
        })
        .expect("t1 releases its next job at 4")
        .state;
    take_action(&mut state, TaskTransitionAction::Preempt, "t3");
    take_action(&mut state, TaskTransitionAction::Dispatch, "t1");
    state = product
        .delay(&state, &crate::zone_search::exact(1, 1))
        .expect("source admits time")
        .expect("t1's second job completes at 5");
    take_action(&mut state, TaskTransitionAction::Complete, "t1");
    take_action(&mut state, TaskTransitionAction::Resume, "t3");
    state = product
        .delay(&state, &crate::zone_search::exact(1, 1))
        .expect("source admits time")
        .expect("reach t2's next release at 6");
    state = product
        .source_steps(&state)
        .expect("source transitions")
        .into_iter()
        .find(|step| {
            step.activations
                .iter()
                .any(|activation| activation.task == "t2")
        })
        .expect("t2 releases its next job at 6")
        .state;
    take_action(&mut state, TaskTransitionAction::Preempt, "t3");
    take_action(&mut state, TaskTransitionAction::Dispatch, "t2");
    state = product
        .delay(&state, &crate::zone_search::exact(2, 1))
        .expect("source admits time")
        .expect("t2's second job completes at 8");
    take_action(&mut state, TaskTransitionAction::Complete, "t2");
    state = product
        .source_steps(&state)
        .expect("source transitions")
        .into_iter()
        .find(|step| {
            step.activations
                .iter()
                .any(|activation| activation.task == "t1")
        })
        .expect("t1 releases its third job at 8")
        .state;
    take_action(&mut state, TaskTransitionAction::Dispatch, "t1");
    state = product
        .delay(&state, &crate::zone_search::exact(1, 1))
        .expect("source admits time")
        .expect("t1's third job completes at 9");
    take_action(&mut state, TaskTransitionAction::Complete, "t1");
    take_action(&mut state, TaskTransitionAction::Resume, "t3");
    state = product
        .delay(&state, &crate::zone_search::exact(1, 1))
        .expect("source admits time")
        .expect("reach t3's first deadline at 10");
    state = product
        .source_steps(&state)
        .expect("source transitions")
        .into_iter()
        .find(|step| {
            step.activations
                .iter()
                .any(|activation| activation.task == "t3")
        })
        .expect("t3 releases its next job at 10")
        .state;
    let miss = product
        .scheduler_steps(&state)
        .into_iter()
        .find(|step| step.identity.action == TaskTransitionAction::DeadlineMiss)
        .expect("t3's first missed deadline is explicit");
    assert_eq!((miss.identity.task.as_str(), miss.identity.job), ("t3", 0));
    assert_eq!(
        miss.state.jobs()[0].service,
        crate::zone_search::exact(3, 1)
    );
}

fn rational(value: &Value) -> (String, String) {
    (
        value["numerator"].as_str().unwrap_or("-").to_owned(),
        value["denominator"].as_str().unwrap_or("-").to_owned(),
    )
}

fn whole(value: i64) -> (String, String) {
    (value.to_string(), "1".to_owned())
}

#[trace("TC-3008", "FR-249-AC-7")]
#[test]
fn a_two_task_set_lowers_with_its_declared_periods_and_deadlines() {
    let declarations = format!("{PAIR}{}", sources("Pair", &["a", "b"]));
    let run = run(
        "task-product-timing",
        &["a", "b"],
        &declarations,
        json!([{"claim": "PairFp", "backend": "en-6"}, {"claim": "PairFp"}]),
    );
    let item = run.item(0);
    assert_eq!(item["request"]["candidates"]["backends"], json!(["en-6"]));
    // Unnamed, FR-962 narrows the release-backed `[en-6, en-7]` to EN-7.
    assert_eq!(
        run.item(1)["request"]["candidates"]["backends"],
        json!(["en-7"])
    );
    assert_eq!(label(&item), "proved", "{item}");
    let rows = item["certificate"]["task_product"]["queue_bounds"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let read: Vec<_> = rows
        .iter()
        .map(|row| {
            (
                row["taskset"].as_str().unwrap_or("-").to_owned(),
                row["task"].as_str().unwrap_or("-").to_owned(),
                row["arrival"].as_str().unwrap_or("-").to_owned(),
                rational(&row["interarrival"]),
                rational(&row["relative_deadline"]),
                rational(&row["max_jitter"]),
                rational(&row["work_lower"]),
                rational(&row["work_upper"]),
                row["max_jobs"].clone(),
            )
        })
        .collect();
    assert_eq!(
        read,
        vec![
            (
                "Pair".to_owned(),
                "a".to_owned(),
                "periodic".to_owned(),
                whole(4),
                whole(3),
                whole(0),
                whole(1),
                whole(1),
                json!(2)
            ),
            (
                "Pair".to_owned(),
                "b".to_owned(),
                "sporadic".to_owned(),
                whole(6),
                whole(5),
                whole(0),
                whole(2),
                whole(2),
                json!(2)
            ),
        ],
        "{item}"
    );
    // The lowered subject's task-timing rows are the same rows, in order.
    let model_document =
        crate::zone_search::rpc_document_with(&[("fire_a", &["phase"]), ("fire_b", &["phase"])]);
    let body = format!(
        "{}{PAIR}{}",
        crate::timed_world::DENSE,
        sources("Pair", &["a", "b"])
    );
    let world = crate::timed_world::world(&model_document, crate::zone_search::TIMED, &body);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "PairFp").expect("checked task product");
    let plan = product.plan();
    let lowered: Vec<_> = plan
        .task_set
        .tasks
        .iter()
        .zip(&plan.tasks)
        .map(|(task, slots)| {
            let (kind, interarrival) = match &task.arrival {
                fsl_semantics::check::Arrival::Periodic { period } => ("periodic", period.clone()),
                fsl_semantics::check::Arrival::Sporadic { min_interarrival } => {
                    ("sporadic", min_interarrival.clone())
                }
            };
            (
                plan.task_set.name.clone(),
                task.name.clone(),
                kind,
                interarrival,
                task.deadline.clone(),
                task.jitter.clone(),
                task.fixed_work().cloned(),
                slots.max_jobs,
            )
        })
        .collect();
    let exact = crate::zone_search::exact;
    assert_eq!(
        lowered,
        vec![
            (
                "Pair".to_owned(),
                "a".to_owned(),
                "periodic",
                exact(4, 1),
                exact(3, 1),
                exact(0, 1),
                Some(exact(1, 1)),
                2
            ),
            (
                "Pair".to_owned(),
                "b".to_owned(),
                "sporadic",
                exact(6, 1),
                exact(5, 1),
                exact(0, 1),
                Some(exact(2, 1)),
                2
            ),
        ]
    );
    // Without a release the claim is not release-backed: naming EN-6 leaves
    // no candidate.
    let bare = run_bare();
    assert_eq!(bare["request"]["candidates"]["backends"], json!([]));
    assert_eq!(label(&bare), "unsupported", "{bare}");
}

fn run_bare() -> Value {
    run(
        "task-product-bare",
        &[],
        PAIR,
        json!([{"claim": "PairFp", "backend": "en-6"}]),
    )
    .item(0)
}

#[trace("TC-3008", "FR-249-AC-8")]
#[test]
#[ignore = "FR-249-AC-8: set B region quotient exceeds max_check_work (100M) at 791 s release; pending arch ruling on abstraction/AC wording, QSL-2240"]
fn the_zone_verdict_equals_the_response_time_verdict_on_task_set_b() {
    let items = json!([
        {"claim": "BFp"}, {"claim": "BFp", "backend": "en-6"},
        {"claim": "BEdf"}, {"claim": "BEdf", "backend": "en-6"},
    ]);
    for (third, expected) in [(3, ["proved", "proved"]), (4, ["refuted", "proved"])] {
        let run = run(
            &format!("task-set-b-{third}"),
            &["t1", "t2", "t3"],
            &task_set_b(third),
            items.clone(),
        );
        for (claim, want) in expected.iter().enumerate() {
            let closed = run.item(2 * claim);
            let zone = run.item(2 * claim + 1);
            assert_eq!(label(&closed), *want, "EN-7, t3 work {third}: {closed}");
            assert_eq!(
                label(&zone),
                label(&closed),
                "EN-6 = EN-7, t3 work {third}: {zone}"
            );
        }
    }
}

#[trace("TC-3008", "FR-249-AC-9")]
#[test]
fn an_unschedulable_set_refutes_with_its_first_missed_deadline() {
    let run = run(
        "task-set-b-miss",
        &["t1", "t2", "t3"],
        &task_set_b(4),
        json!([{"claim": "BFp", "backend": "en-6"}]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true);
    let counterexample = &item["counterexample"]["counterexample"];
    let loop_steps = counterexample["loop"]
        .as_array()
        .expect("a time-divergent continuation is carried");
    assert!(!loop_steps.is_empty(), "the miss alone is not a refutation");
    let prefix = counterexample["prefix"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let misses: Vec<&Value> = prefix
        .iter()
        .map(|step| &step["transition"])
        .filter(|identity| {
            identity["type"] == "task-transition" && identity["action"] == "deadline-miss"
        })
        .collect();
    let first = misses.first().expect("a deadline-miss step");
    let miss_position = prefix
        .iter()
        .position(|step| {
            let identity = &step["transition"];
            identity["type"] == "task-transition" && identity["action"] == "deadline-miss"
        })
        .expect("the prefix carries the first missed deadline");
    assert_eq!(counterexample["trace_position"], json!(miss_position));
    assert_eq!(misses.len(), 1, "the first miss is latched across the loop");
    let timeline: Vec<_> = prefix
        .iter()
        .map(|step| {
            format!(
                "{}:{}@{}",
                step["transition"]["action"].as_str().unwrap_or("source"),
                step["transition"]["task"].as_str().unwrap_or("-"),
                step["delay"]["numerator"].as_str().unwrap_or("0")
            )
        })
        .collect();
    assert_eq!(first["task"], "t3", "timeline={timeline:?}");
    let completed: Vec<_> = prefix
        .iter()
        .map(|step| &step["transition"])
        .filter(|identity| {
            identity["type"] == "task-transition" && identity["action"] == "complete"
        })
        .map(|identity| identity["task"].as_str().unwrap_or("-"))
        .collect();
    assert_eq!(
        completed.iter().take(2).copied().collect::<Vec<_>>(),
        ["t1", "t2"],
        "the scheduler must stop at t1's completion before source@4, then finish t2 before t3's first miss; timeline={timeline:?}"
    );
    let t1_second_release = prefix
        .iter()
        .position(|step| {
            let identity = &step["transition"];
            identity["type"] == "task-transition"
                && identity["action"] == "activate"
                && identity["task"] == "t1"
                && identity["job"] == 1
        })
        .expect("source@4 releases t1's second job");
    let t2_first_completion = prefix
        .iter()
        .position(|step| {
            let identity = &step["transition"];
            identity["type"] == "task-transition"
                && identity["action"] == "complete"
                && identity["task"] == "t2"
                && identity["job"] == 0
        })
        .expect("t2 completes its first job");
    assert!(
        t2_first_completion < t1_second_release,
        "t2's first job must complete before source@4: {timeline:?}"
    );
    assert_eq!(first["job"], 0);
    assert_eq!(rational(&first["demand"]), whole(4));
    let last_transition = &prefix.last().expect("non-empty counterexample")["transition"];
    assert_eq!(
        last_transition["action"], "activate",
        "timeline={timeline:?}, trace_position={}",
        counterexample["trace_position"]
    );
    assert_eq!(last_transition["task"], "t3");
    assert_eq!(last_transition["job"], 1);
    // Least-delay concretisation: every task activates at 0, so t3's
    // absolute deadline is 10.
    assert_eq!(
        rational(&first["deadline"]),
        whole(10),
        "{}",
        item["counterexample"]
    );
    let loop_delay_sum: i64 = loop_steps
        .iter()
        .map(|step| {
            assert_eq!(rational(&step["delay"]).1, "1");
            rational(&step["delay"])
                .0
                .parse::<i64>()
                .expect("fixture delays are exact integers")
        })
        .sum();
    assert_eq!(loop_delay_sum, 60, "the queue repeats after 60 ms");
    assert!(loop_steps.iter().all(|step| {
        let identity = &step["transition"];
        identity["type"] != "task-transition" || identity["action"] != "deadline-miss"
    }));
    // The earliest miss of the product, whatever order the search explores:
    // declaring the sources in reverse reports the same miss.
    let reversed = format!(
        "{}{}",
        task_set_b(4).split("post Fire_").next().unwrap_or_default(),
        sources("B", &["t3", "t2", "t1"])
    );
    let again = self::run(
        "task-set-b-miss-reversed",
        &["t3", "t2", "t1"],
        &reversed,
        json!([{"claim": "BFp", "backend": "en-6"}]),
    )
    .item(0);
    let again_counterexample = &again["counterexample"]["counterexample"];
    let miss = again_counterexample["prefix"]
        .as_array()
        .and_then(|prefix| {
            prefix
                .iter()
                .map(|step| step["transition"].clone())
                .find(|identity| identity["action"] == "deadline-miss")
        })
        .unwrap_or(Value::Null);
    assert_eq!((&miss["task"], &miss["job"]), (&json!("t3"), &json!(0)));
    assert_eq!(rational(&miss["deadline"]), whole(10), "{again}");
    let reversed_loop: i64 = again_counterexample["loop"]
        .as_array()
        .expect("reversed declarations retain the loop")
        .iter()
        .map(|step| {
            assert_eq!(rational(&step["delay"]).1, "1");
            rational(&step["delay"])
                .0
                .parse::<i64>()
                .expect("fixture delays are exact integers")
        })
        .sum();
    assert_eq!(reversed_loop, loop_delay_sum);
}

/// FR-249-AC-11/FR-237: a one-shot release settles through a replayed
/// positive-delay terminal-stutter loop after the missed job completes.
#[trace("TC-704", "FR-249-AC-11")]
#[test]
fn one_shot_miss_refutes_with_terminal_stutter_replay() {
    let run = run_once(
        "task-product-terminal-stutter",
        json!([{"claim": "OnceFp", "backend": "en-6"}]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
    let counterexample = &item["counterexample"]["counterexample"];
    let prefix = counterexample["prefix"].as_array().expect("prefix");
    assert_eq!(
        prefix.last().expect("completion step")["transition"]["action"],
        "complete"
    );
    assert_eq!(counterexample["trace_position"], 2);
    let looped = counterexample["loop"].as_array().expect("terminal loop");
    assert_eq!(looped.len(), 1);
    assert_eq!(looped[0]["transition"]["type"], "terminal-stutter");
    assert!(
        rational(&looped[0]["delay"])
            .0
            .parse::<i64>()
            .unwrap_or_default()
            > 0
    );
}

const BAD_SET: &str =
    "taskset Bad { task command periodic 4 ms deadline 1 ms bcet 2 ms wcet 2 ms priority 1 lo; }\n\
     schedulable BadFp on Bad under fixed_priority;\n";

/// FR-249-AC-2: a periodic overrun (period 4, deadline 1, wcet 2) misses in
/// every period, so the product closes a positive-delay lasso and the miss
/// settles `refuted` with that replayed loop, never a bare prefix.
#[trace("FR-249-AC-2")]
#[test]
fn a_periodic_overrun_settles_refuted_with_its_replayed_lasso() {
    let declarations = format!("{BAD_SET}{}", sources("Bad", &["command"]));
    let run = run(
        "task-product-periodic-overrun",
        &["command"],
        &declarations,
        json!([{"claim": "BadFp", "backend": "en-6"}]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    let counterexample = &item["counterexample"]["counterexample"];
    assert!(
        counterexample["loop"]
            .as_array()
            .is_some_and(|steps| !steps.is_empty()),
        "the refutation carries its lasso: {item}"
    );
    assert!(
        counterexample["prefix"]
            .as_array()
            .is_some_and(|steps| steps
                .iter()
                .any(|step| step["transition"]["action"] == "deadline-miss")),
        "{item}"
    );
}

/// FR-249-AC-2: a miss prefix whose search stops before any loop closes is not
/// a settled refutation; it stays `incomplete` with the work limit as cause.
#[trace("FR-249-AC-2")]
#[test]
fn a_miss_prefix_with_no_closed_loop_stays_incomplete() {
    let declarations = format!("{BAD_SET}{}", sources("Bad", &["command"]));
    let run = run(
        "task-product-open-miss",
        &["command"],
        &declarations,
        json!([{
            "claim": "BadFp",
            "backend": "en-6",
            "temporal": {"limits": {"max_task_product_work": 1}}
        }]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "incomplete", "{item}");
    assert_eq!(item["counterexample"], Value::Null, "{item}");
    assert_eq!(
        item["record"]["cause"]["limit"], "max_task_product_work",
        "{item}"
    );
}

/// FR-246-AC-5, FR-075-AC-23, FR-962-AC-6: a release makes the `schedulable`
/// claim release-backed, so EN-6 matches it; unnamed, the `[en-6, en-7]`
/// tie narrows to EN-7 and never settles `ambiguous-backend`.
#[trace("TC-3009", "FR-246-AC-5")]
#[trace("TC-3009", "FR-075-AC-23")]
#[trace("TC-3009", "FR-962-AC-6")]
#[test]
fn a_release_backed_claim_routes_to_en_6_only_when_named() {
    let backed = format!("{PAIR}{}", sources("Pair", &["a", "b"]));
    let items = json!([{"claim": "PairFp"}, {"claim": "PairFp", "backend": "en-6"}]);
    let with = run("release-backed", &["a", "b"], &backed, items.clone());
    let unnamed = with.item(0);
    assert_eq!(
        unnamed["request"]["candidates"]["backends"],
        json!(["en-7"])
    );
    assert_eq!(
        unnamed["record"]["basis"]["type"], "closed-form",
        "{unnamed}"
    );
    assert_eq!(
        with.item(1)["request"]["candidates"]["backends"],
        json!(["en-6"])
    );
    let without = run("not-release-backed", &[], PAIR, items);
    assert_eq!(
        without.item(0)["request"]["candidates"]["backends"],
        json!(["en-7"])
    );
    assert_eq!(label(&without.item(1)), "unsupported");
}

/// FR-249-AC-12: a work budget one short of what the refutation needs stops
/// the search; the item settles `incomplete` with `limit-reached` for
/// `max_task_product_work` and FR-331 `task_search` evidence that holds the
/// missed deadline, and never refutes.
#[trace("TC-704", "FR-249-AC-12")]
#[test]
fn a_work_limit_stop_settles_incomplete_with_task_search_evidence() {
    let at = |work: u64| {
        run_once(
            &format!("task-product-work-{work}"),
            json!([{
                "claim": "OnceFp",
                "backend": "en-6",
                "temporal": {"limits": {"max_task_product_work": work}}
            }]),
        )
        .item(0)
    };
    // The smallest budget that still refutes is read from the runs themselves.
    let (mut low, mut high) = (1_u64, 4096_u64);
    assert_eq!(label(&at(high)), "refuted", "{}", at(high));
    while low < high {
        let middle = low + (high - low) / 2;
        if label(&at(middle)) == "refuted" {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    assert!(low > 1, "the refutation needs more than one expansion");
    let stopped = at(low - 1);
    assert_eq!(label(&stopped), "incomplete", "{stopped}");
    assert_eq!(stopped["counterexample"], Value::Null, "{stopped}");
    let cause = &stopped["record"]["cause"];
    assert_eq!(cause["kind"], "limit-reached", "{stopped}");
    assert_eq!(cause["limit"], "max_task_product_work", "{stopped}");
    assert_eq!(cause["value"], json!(low - 1), "{stopped}");
    let evidence = &stopped["record"]["task_search"];
    assert_eq!(evidence["type"], "task-search-frontier", "{stopped}");
    assert_eq!(evidence["limit"]["value"], json!(low - 1), "{stopped}");
    assert!(
        evidence["frontier"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["reason"] == "work-limit")),
        "{stopped}"
    );
    assert!(
        evidence["edges"].as_array().is_some_and(|edges| edges
            .iter()
            .any(|edge| edge["transition"]["action"] == "deadline-miss")),
        "the evidence holds the missed deadline: {stopped}"
    );
}

/// FR-249-AC-4: a mixed-criticality product with unequal budgets is outside
/// the exact gate, so a bounded run never settles `proved`, and when its work
/// limit is reached it settles `incomplete` with `task_search` evidence.
#[trace("TC-704", "FR-249-AC-4")]
#[test]
fn an_unequal_budget_product_never_proves_and_stops_with_task_search_evidence() {
    let declarations = format!(
        "taskset Mixed {{ task h periodic 10 ms deadline 10 ms bcet 3 ms wcet 2 ms priority 1 hi wcet_hi 3 ms; }}\n\
         schedulable MixedFp on Mixed under fixed_priority;\n{}",
        sources("Mixed", &["h"])
    );
    let run = run(
        "task-product-unequal-budgets",
        &["h"],
        &declarations,
        json!([
            {"claim": "MixedFp", "backend": "en-6"},
            {
                "claim": "MixedFp",
                "backend": "en-6",
                "temporal": {"limits": {"max_task_product_work": 2}}
            }
        ]),
    );
    let unlimited = run.item(0);
    assert_ne!(label(&unlimited), "proved", "{unlimited}");
    let limited = run.item(1);
    assert_eq!(label(&limited), "incomplete", "{limited}");
    assert_eq!(
        limited["record"]["cause"]["kind"], "limit-reached",
        "{limited}"
    );
    assert_eq!(
        limited["record"]["task_search"]["type"], "task-search-frontier",
        "{limited}"
    );
}

/// FR-249-AC-11's `Once` model: a `time dense` model whose one operation `go`
/// is enabled until it fires, releasing the one-shot task `s` whose demand
/// exceeds its deadline.
fn run_once(name: &str, items: Value) -> Run {
    use crate::timed_world::{document, unit_text, DENSE, PACKAGE};
    let body = format!(
        "{DENSE}\
         pre GoReady using v on M::Call::go {{ self.phase = 0 }}\n\
         post GoFired using v on M::Call::go {{ self.phase = 1 }}\n\
         taskset Once {{ task s sporadic 10 ms deadline 2 ms bcet 3 ms wcet 3 ms priority 1 lo; }}\n\
         release Once.s on M::Call::go;\n\
         schedulable OnceFp on Once under fixed_priority;\n"
    );
    let ws = Workspace::new(name);
    analyze(&request_over(
        &ws,
        &unit_text(PACKAGE, crate::zone_search::TIMED, &body),
        &document(&[], &[("go", &["phase"])]),
        &[rpc_snapshot()],
        json!({"items": items}),
    ))
}

/// FSpec FR-331: the `task_search` evidence EN-6 writes is read, unchanged, by
/// the planner's result-entry reader (state keys are unpadded base64url).
#[trace("TC-704", "FR-249-AC-12")]
#[test]
fn the_written_task_search_evidence_round_trips_through_the_planner_reader() {
    let declarations = format!("{BAD_SET}{}", sources("Bad", &["command"]));
    let run = run(
        "task-product-evidence-round-trip",
        &["command"],
        &declarations,
        json!([{
            "claim": "BadFp",
            "backend": "en-6",
            "temporal": {"limits": {"max_task_product_work": 1}}
        }]),
    );
    let item = run.item(0);
    let record = &item["record"];
    assert!(record["task_search"].is_object(), "{item}");
    let read = fsl_replay::planner::read_result_entry(
        &planner_entry(record),
        0,
        fsl_replay::planner::EntryOrigin::FirstParty,
    )
    .unwrap_or_else(|refusal| panic!("the planner reader refused {refusal:?}: {item}"));
    let evidence = read.task_search.expect("the evidence is retained");
    assert_eq!(
        evidence.nodes.len(),
        record["task_search"]["nodes"]
            .as_array()
            .map_or(0, Vec::len)
    );
    assert!(evidence
        .frontier
        .iter()
        .any(|row| row.reason == "work-limit"));
}

/// FSpec FR-331: a stop at a full job queue settles `incomplete` with
/// `bounded-search-incomplete{reason: unresolved-dynamics}` and `task_search`
/// evidence that carries no limit and an `unresolved-choice` frontier row.
#[trace("TC-704", "FR-249-AC-5")]
#[test]
fn a_queue_overflow_stop_settles_unresolved_dynamics_with_task_search_evidence() {
    let declarations = format!(
        "taskset Over {{ task t periodic 1 ms deadline 1 ms bcet 2 ms wcet 2 ms priority 1 lo; }}\n\
         schedulable OverFp on Over under fixed_priority;\n{}",
        sources("Over", &["t"])
    );
    let run = run(
        "task-product-queue-overflow",
        &["t"],
        &declarations,
        json!([{"claim": "OverFp", "backend": "en-6"}]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "incomplete", "{item}");
    assert_eq!(item["counterexample"], Value::Null, "{item}");
    let cause = &item["record"]["cause"];
    assert_eq!(cause["kind"], "bounded-search-incomplete", "{item}");
    assert_eq!(cause["reason"], "unresolved-dynamics", "{item}");
    let evidence = &item["record"]["task_search"];
    assert_eq!(evidence["type"], "task-search-frontier", "{item}");
    assert!(evidence.get("limit").is_none(), "{item}");
    assert!(
        evidence["frontier"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["reason"] == "unresolved-choice")),
        "{item}"
    );
    let read = fsl_replay::planner::read_result_entry(
        &planner_entry(&item["record"]),
        0,
        fsl_replay::planner::EntryOrigin::FirstParty,
    )
    .unwrap_or_else(|refusal| panic!("the planner reader refused {refusal:?}: {item}"));
    assert!(read.task_search.is_some_and(|found| found.limit.is_none()));
}

/// The FSpec FR-331 result entry of an analyze item `record`: its cause under
/// the entry's `type` spelling and its `task_search` member as written.
fn planner_entry(record: &Value) -> Value {
    let mut cause = record["cause"].clone();
    let kind = cause
        .as_object_mut()
        .and_then(|members| members.remove("kind"))
        .expect("the cause names its kind");
    cause["type"] = kind;
    json!({
        "request_index": 0,
        "backend": record["backend"],
        "kind": "schedulability",
        "result": "incomplete",
        "cause": cause,
        "task_search": record["task_search"],
    })
}

/// FR-249: a closed task-product search with no accepted FR-418 certificate
/// settles `incomplete` with `bounded-search-incomplete{reason:
/// certificate-unchecked}` and no `task_search` evidence, never a limit.
#[trace("TC-704", "FR-249-AC-8")]
#[test]
fn a_closed_search_without_a_certificate_settles_certificate_unchecked() {
    let declarations = format!(
        "taskset Calm {{ task t periodic 10 ms deadline 10 ms bcet 1 ms wcet 1 ms priority 1 hi wcet_hi 1 ms; }}\n\
         mode Calm hi on M::Call::fire_t;\n\
         schedulable CalmFp on Calm under fixed_priority;\n{}",
        sources("Calm", &["t"])
    );
    let run = run(
        "task-product-certificate-unchecked",
        &["t"],
        &declarations,
        json!([{"claim": "CalmFp", "backend": "en-6"}]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "incomplete", "{item}");
    let cause = &item["record"]["cause"];
    assert_eq!(cause["kind"], "bounded-search-incomplete", "{item}");
    assert_eq!(cause["reason"], "certificate-unchecked", "{item}");
    assert!(item["record"].get("task_search").is_none(), "{item}");
    let mut entry = planner_entry(&item["record"]);
    entry.as_object_mut().expect("entry").remove("task_search");
    let read = fsl_replay::planner::read_result_entry(
        &entry,
        0,
        fsl_replay::planner::EntryOrigin::FirstParty,
    )
    .unwrap_or_else(|refusal| panic!("the planner reader refused {refusal:?}: {item}"));
    assert!(read.task_search.is_none());
}

/// FR-418-AC-21: a dual-WCET set with a mode switch (equal budgets, so the
/// product closes) has no certificate the checker can check: it settles
/// `incomplete` / `certificate-unchecked`, never `proved`, with no
/// certificate member.
#[trace("TC-371", "FR-418-AC-21")]
#[test]
fn a_dual_wcet_mode_switch_set_settles_certificate_unchecked() {
    let declarations = format!(
        "taskset Calm {{ task t periodic 10 ms deadline 10 ms bcet 1 ms wcet 1 ms priority 1 hi wcet_hi 1 ms; }}\n\
         mode Calm hi on M::Call::fire_t;\n\
         schedulable CalmFp on Calm under fixed_priority;\n{}",
        sources("Calm", &["t"])
    );
    let run = run(
        "task-product-dual-wcet-mode",
        &["t"],
        &declarations,
        json!([{"claim": "CalmFp", "backend": "en-6"}]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "incomplete", "{item}");
    assert_eq!(
        item["record"]["cause"],
        json!({"kind": "bounded-search-incomplete", "reason": "certificate-unchecked"}),
        "{item}"
    );
    assert!(item.get("certificate").is_none(), "{item}");
}

/// FR-418: the fixed-work single task that the dual-WCET set above shares its
/// shape with proves through an accepted certificate, and the item carries
/// FR-621's `certificate` members with the `task_product` part beside them.
#[trace("TC-371", "FR-418-AC-20")]
#[test]
fn a_fixed_work_single_task_set_proves_at_the_default_check_work() {
    let declarations = format!(
        "taskset Calm {{ task t periodic 10 ms deadline 10 ms bcet 1 ms wcet 1 ms priority 1 lo; }}\n\
         schedulable CalmFp on Calm under fixed_priority;\n{}",
        sources("Calm", &["t"])
    );
    let run = run(
        "task-product-calm-proved",
        &["t"],
        &declarations,
        json!([{"claim": "CalmFp", "backend": "en-6"}]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    let certificate = item["certificate"].as_object().expect("a certificate");
    let mut members: Vec<_> = certificate.keys().map(String::as_str).collect();
    members.sort_unstable();
    assert_eq!(
        members,
        ["accepted", "checker", "identity", "task_product"],
        "{item}"
    );
    assert_eq!(certificate["accepted"], true);
}

/// FR-418-AC-20: with `zone_check.max_check_work` below the work the check
/// needs the item settles `incomplete` / `limit-reached` naming `max_check_work`
/// and its setting, never `proved`.
#[trace("TC-371", "FR-418-AC-20")]
#[test]
fn a_small_max_check_work_settles_limit_reached() {
    let declarations = format!(
        "taskset Calm {{ task t periodic 10 ms deadline 10 ms bcet 1 ms wcet 1 ms priority 1 lo; }}\n\
         schedulable CalmFp on Calm under fixed_priority;\n{}",
        sources("Calm", &["t"])
    );
    let run = run(
        "task-product-check-work",
        &["t"],
        &declarations,
        json!([{
            "claim": "CalmFp",
            "backend": "en-6",
            "temporal": {"limits": {"max_check_work": 3}}
        }]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "incomplete", "{item}");
    let cause = &item["record"]["cause"];
    assert_eq!(cause["kind"], "limit-reached", "{item}");
    assert_eq!(cause["limit"], "max_check_work", "{item}");
    assert_eq!(cause["setting"], "zone_check.max_check_work", "{item}");
    assert!(item.get("certificate").is_none(), "{item}");
}

/// FR-418: a concrete refutation bypasses the symbolic pass and carries its
/// replayed counterexample.
#[trace("TC-371")]
#[test]
fn a_refutable_set_refutes_before_the_symbolic_pass() {
    let run = run(
        "task-product-refutable-through-symbolic-pass",
        &["command"],
        &format!("{BAD_SET}{}", sources("Bad", &["command"])),
        json!([{"claim": "BadFp", "backend": "en-6"}]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}
