// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-223-AC-5 (TC-668): ADR-025 §15's LT-12 and LT-13 under `sc`, `tso` and
//! `ra`, against an RC11 execution-graph checker (`crate::rc11_support`)
//! that shares no code with `qsl-eval`'s `Ra`.

use std::collections::{BTreeSet, HashMap, VecDeque};

use ix_trace_rs::trace;
use qsl_eval::simulation::protocol::{MemoryModel, Ra, Sc, Tso};
use qsl_eval::simulation::{StateKey, SubjectSystem, TransitionSystem};
use qsl_foundation::MemoryModelName;

use crate::fill::reachable;
use crate::litmus::sb_unit;
use crate::rc11_support::{self as rc11, Op, Order};
use crate::weak_memory::{litmus_run, outcomes, register, select, steps, Case};
use quire_exact::Value;

const X: usize = 0;
const Y: usize = 1;
const Z: usize = 2;

/// One thread's QSL attempts: the store, load and fence nodes of `label`.
fn thread_text(label: &str, ops: &[Op]) -> String {
    let attempts: Vec<String> = ops
        .iter()
        .enumerate()
        .map(|(index, op)| op.render(&format!("{label}{index}")))
        .collect();
    steps(&label.to_uppercase(), &attempts)
}

/// The `SB` unit running `threads` as the branches of one `parallel`.
fn case_of(threads: &[Vec<Op>]) -> Case {
    let labels = ["a", "b", "c", "d"];
    let branches: Vec<(&str, String)> = threads
        .iter()
        .zip(labels)
        .map(|(ops, label)| (label, thread_text(label, ops)))
        .collect();
    Case::with_three_cells(&sb_unit(&litmus_run(&branches, "")))
}

fn under<M>(case: &Case, model: MemoryModelName, memory: M, select_it: bool) -> BTreeSet<Vec<i64>>
where
    M: MemoryModel + Sync,
    M::Component: Send + Sync,
{
    let selection = [select("SB::Main::Both", model)];
    let selection: &[_] = if select_it { &selection } else { &[] };
    outcomes(&case.system(selection, memory).expect("builds"))
}

/// Compare `threads` under `sc`, `tso` and `ra` with the RC11 checker: every
/// outcome RC11 allows is reachable under `ra`, and `ra` reaches exactly the
/// RC11 outcomes plus `extra` (the executions ADR-025 PSC-7 admits). With
/// `extra` empty, `ra` allowing any RC11-forbidden outcome fails.
fn compare(name: &str, threads: &[Vec<Op>], extra: &[Vec<i64>]) -> BTreeSet<Vec<i64>> {
    let case = case_of(threads);
    let ra = under(&case, MemoryModelName::Ra, Ra::default(), true);
    let sc = under(&case, MemoryModelName::Sc, Sc, false);
    let tso = under(&case, MemoryModelName::Tso, Tso::default(), true);
    let rc11 = rc11::outcomes(threads, 3);
    assert!(
        rc11.is_subset(&ra),
        "{name}: RC11 outcomes missing under ra: {:?}",
        rc11.difference(&ra).collect::<Vec<_>>()
    );
    assert_eq!(
        ra.difference(&rc11).cloned().collect::<BTreeSet<_>>(),
        extra.iter().cloned().collect::<BTreeSet<_>>(),
        "{name}: ra reaches the RC11 outcomes and exactly the PSC-7 extras"
    );
    assert!(sc.is_subset(&rc11), "{name}: sc outcomes are RC11 outcomes");
    assert!(
        tso.is_subset(&rc11),
        "{name}: tso outcomes are RC11 outcomes"
    );
    for weak in extra {
        assert!(!sc.contains(weak), "{name}: sc reaches {weak:?}");
        assert!(!tso.contains(weak), "{name}: tso reaches {weak:?}");
        assert!(!rc11.contains(weak), "{name}: RC11 forbids {weak:?}");
    }
    rc11
}

fn lt12() -> Vec<Vec<Op>> {
    vec![
        vec![
            Op::store(X, 1, Order::SeqCst),
            Op::store(Y, 1, Order::Release),
        ],
        vec![
            Op::load(Y, 1, Order::Acquire),
            Op::load(Z, 2, Order::SeqCst),
        ],
        vec![
            Op::store(Z, 1, Order::SeqCst),
            Op::load(X, 3, Order::SeqCst),
        ],
    ]
}

fn lt13() -> Vec<Vec<Op>> {
    vec![
        vec![
            Op::store(X, 1, Order::SeqCst),
            Op::load(Y, 1, Order::SeqCst),
        ],
        vec![
            Op::store(Y, 1, Order::Relaxed),
            Op::fence(Order::SeqCst),
            Op::load(X, 2, Order::Relaxed),
        ],
    ]
}

/// LT-14 and LT-18 exercise both PSC-3 edge directions involving a fence:
/// a release-message view carries a fence to an SC access, and an SC access
/// happens before a fence through an acquire read.
#[trace("TC-668", "FR-223-AC-6")]
#[test]
fn sc_fence_edges_match_rc11_for_lt14_and_lt18() {
    let lt14 = vec![
        vec![
            Op::store(X, 1, Order::SeqCst),
            Op::fence(Order::SeqCst),
            Op::store(Y, 1, Order::Relaxed),
        ],
        vec![
            Op::load(Y, 1, Order::Acquire),
            Op::store(Z, 1, Order::SeqCst),
        ],
        vec![
            Op::load(Z, 2, Order::SeqCst),
            Op::store(Z, 2, Order::SeqCst),
            Op::load(X, 3, Order::SeqCst),
            Op::load(Z, 4, Order::SeqCst),
        ],
    ];
    let rc11 = compare("LT-14", &lt14, &[]);
    assert!(!rc11.contains(&vec![1, 1, 0, 2]));

    let lt18 = vec![
        vec![
            Op::store(X, 1, Order::SeqCst),
            Op::store(Y, 1, Order::Release),
        ],
        vec![
            Op::load(Y, 1, Order::Acquire),
            Op::fence(Order::SeqCst),
            Op::load(Z, 2, Order::SeqCst),
        ],
        vec![
            Op::store(Z, 1, Order::SeqCst),
            Op::load(X, 3, Order::SeqCst),
        ],
    ];
    let rc11 = compare("LT-18", &lt18, &[]);
    assert!(!rc11.contains(&vec![1, 0, 0, 3]));
}

/// LT-16 carries a predecessor of a collected SC event through an acquire
/// fence and an SC load, so the later SC store cannot move before X := 1.
#[trace("TC-668", "FR-223-AC-8")]
#[test]
fn lt16_collected_sc_predecessors_still_constrain_the_sc_load() {
    let lt16 = vec![
        vec![
            Op::store(X, 1, Order::SeqCst),
            Op::store(Y, 1, Order::SeqCst),
            Op::load(Y, 1, Order::Relaxed),
            Op::load(Y, 2, Order::Relaxed),
        ],
        vec![
            Op::store(Y, 2, Order::SeqCst),
            Op::store(Y, 3, Order::Relaxed),
        ],
        // The shared fixture's Int[0, 3] domain encodes LT-16's distinct
        // final relaxed message as value 0; its message position stays new.
        vec![Op::store(Y, 0, Order::Relaxed)],
        vec![
            Op::load(Y, 3, Order::Relaxed),
            Op::fence(Order::Acquire),
            Op::load(Y, 4, Order::SeqCst),
            Op::store(X, 2, Order::SeqCst),
        ],
    ];
    let case = case_of(&lt16);
    let selection = [select("SB::Main::Both", MemoryModelName::Ra)];
    let system = case.system(&selection, Ra::default()).expect("builds");
    let reached = reachable(&system);
    let transitions = reached.iter().map(|state| state.steps.len()).sum::<usize>();
    assert_eq!(reached.len(), 7_607, "LT-16 reachable state count");
    assert_eq!(transitions, 19_574, "LT-16 successor edge count");
    let accesses = case.accesses(&selection);
    let x = accesses
        .shared()
        .iter()
        .filter_map(|location| accesses.cell(location))
        .find(|cell| cell.key == "x")
        .expect("the shared X cell");
    assert!(!reached
        .iter()
        .filter(|state| state.steps.is_empty())
        .any(|state| {
            let model = state.state.model();
            ["r1", "r2", "r3", "r4"]
                .iter()
                .map(|name| register(model, "core", name))
                .eq([2, 3, 3, 0])
                && matches!(model.cell(x), Some(Value::Integer(value)) if value.to_string() == "1")
        }));
}

/// LT-12: a release flag between an SC write and an SC read through a third
/// thread. The weak outcome is the one execution of PSC-7 (a) that `ra`
/// admits beyond RC11; the other seven outcomes are RC11's.
#[trace("TC-668", "FR-223-AC-5")]
#[test]
fn lt12_reaches_its_weak_outcome_under_ra_only() {
    let rc11 = compare("LT-12", &lt12(), &[vec![1, 0, 0, 3]]);
    assert_eq!(rc11.len(), 7);
}

/// The parallel `reachable` explorer admits states in the order a sequential
/// FIFO search would: the same indices, steps and edges on LT-12 under `ra`.
#[test]
fn the_parallel_explorer_matches_a_sequential_fifo_search() {
    let case = case_of(&lt12());
    let selection = [select("SB::Main::Both", MemoryModelName::Ra)];
    let system = case.system(&selection, Ra::default()).expect("builds");
    let parallel = reachable(&system);

    let key_of = |state: &_| SubjectSystem::canonical_state_key(&system, state).expect("a key");
    let mut known: HashMap<StateKey, usize> = HashMap::new();
    let mut queue = VecDeque::new();
    let mut keys: Vec<Vec<u8>> = Vec::new();
    let mut edges: Vec<Vec<usize>> = Vec::new();
    for state in system.initial() {
        let key = key_of(&state);
        if !known.contains_key(&key) {
            known.insert(key.clone(), keys.len());
            keys.push(key.as_bytes().to_vec());
            edges.push(Vec::new());
            queue.push_back((keys.len() - 1, state));
        }
    }
    while let Some((at, state)) = queue.pop_front() {
        let mut successors: Vec<_> = system
            .successors(&state)
            .expect("the state expands")
            .successors
            .into_iter()
            .map(|(transition, next)| {
                (
                    system.transition_key(&transition).expect("an identity"),
                    next,
                )
            })
            .collect();
        successors.sort_by(|left, right| left.0.cmp(&right.0));
        for (_, next) in successors {
            let key = key_of(&next);
            let index = if let Some(&index) = known.get(&key) {
                index
            } else {
                let index = keys.len();
                known.insert(key.clone(), index);
                keys.push(key.as_bytes().to_vec());
                edges.push(Vec::new());
                queue.push_back((index, next));
                index
            };
            edges[at].push(index);
        }
    }

    assert_eq!(parallel.len(), keys.len(), "state count");
    for (index, reached) in parallel.iter().enumerate() {
        assert_eq!(reached.key, keys[index], "state {index} key");
        let steps: Vec<usize> = reached.steps.iter().map(|(_, next)| *next).collect();
        assert_eq!(steps, edges[index], "state {index} successor indices");
    }
}

/// LT-13: an SC access pair against a `relaxed` store, an SC fence and a
/// `relaxed` load. The weak outcome is the one execution of PSC-7 (b) that
/// `ra` admits beyond RC11.
#[trace("TC-668", "FR-223-AC-5")]
#[test]
fn lt13_reaches_its_weak_outcome_under_ra_only() {
    let rc11 = compare("LT-13", &lt13(), &[vec![0, 0, 3, 3]]);
    assert_eq!(rc11.len(), 3);
}

/// Programs outside PSC-7 on which `ra` reaches exactly RC11's outcomes, so
/// an `ra` that allows an RC11-forbidden outcome (for instance by not pruning
/// a cyclic SC choice) fails here. Each program's RC11 set excludes the
/// listed outcomes.
#[trace("TC-668")]
#[test]
fn ra_reaches_exactly_the_rc11_outcomes_outside_psc7() {
    let sb = vec![
        vec![
            Op::store(X, 1, Order::SeqCst),
            Op::load(Y, 1, Order::SeqCst),
        ],
        vec![
            Op::store(Y, 1, Order::SeqCst),
            Op::load(X, 2, Order::SeqCst),
        ],
    ];
    let rc11 = compare("SB seq_cst", &sb, &[]);
    assert!(!rc11.contains(&vec![0, 0, 3, 3]));

    // SB with an observer: every outcome with both SB loads 0 is excluded,
    // whatever the observer read.
    let mut observed = sb.clone();
    observed.push(vec![
        Op::load(X, 3, Order::SeqCst),
        Op::load(Y, 4, Order::SeqCst),
    ]);
    let rc11 = compare("SB seq_cst with an observer", &observed, &[]);
    let excluded = [[0, 0, 0, 0], [0, 0, 0, 1], [0, 0, 1, 0], [0, 0, 1, 1]];
    assert!(excluded.iter().all(|o| !rc11.contains(o.as_slice())));

    let iriw = vec![
        vec![Op::store(X, 1, Order::SeqCst)],
        vec![Op::store(Y, 1, Order::SeqCst)],
        vec![Op::load(X, 1, Order::SeqCst), Op::load(Y, 2, Order::SeqCst)],
        vec![Op::load(Y, 3, Order::SeqCst), Op::load(X, 4, Order::SeqCst)],
    ];
    let rc11 = compare("IRIW seq_cst", &iriw, &[]);
    assert!(!rc11.contains(&vec![1, 0, 1, 0]));
}

/// Hand-checked RC11 verdicts that the checker must reproduce: each row names
/// an outcome and whether RC11 allows it. The rows cover the clauses the LT
/// vectors depend on (release sequences, fence synchronisation, coherence, no
/// thin air, and the `psc` fence clause). This test runs the checker alone,
/// not `sc`, `tso` or `ra`.
#[trace("TC-668")]
#[test]
fn the_rc11_checker_matches_known_litmus_verdicts() {
    use Order::{Acquire, Relaxed, Release, SeqCst};
    let rows: Vec<Row> = vec![
        (
            "SB relaxed",
            sb(Relaxed, Relaxed, None),
            vec![0, 0, 3, 3],
            true,
        ),
        (
            "SB release/acquire",
            sb(Release, Acquire, None),
            vec![0, 0, 3, 3],
            true,
        ),
        (
            "SB seq_cst",
            sb(SeqCst, SeqCst, None),
            vec![0, 0, 3, 3],
            false,
        ),
        (
            "SB relaxed with seq_cst fences",
            sb(Relaxed, Relaxed, Some(SeqCst)),
            vec![0, 0, 3, 3],
            false,
        ),
        (
            "MP relaxed",
            mp(Relaxed, Relaxed, None, None),
            vec![1, 0, 3, 3],
            true,
        ),
        (
            "MP release/acquire",
            mp(Release, Acquire, None, None),
            vec![1, 0, 3, 3],
            false,
        ),
        (
            "MP release fence only",
            mp(Relaxed, Relaxed, Some(Release), None),
            vec![1, 0, 3, 3],
            true,
        ),
        (
            "MP acquire fence only",
            mp(Relaxed, Relaxed, None, Some(Acquire)),
            vec![1, 0, 3, 3],
            true,
        ),
        (
            "MP release and acquire fences",
            mp(Relaxed, Relaxed, Some(Release), Some(Acquire)),
            vec![1, 0, 3, 3],
            false,
        ),
        (
            "MP release sequence",
            vec![
                vec![
                    Op::store(X, 1, Relaxed),
                    Op::store(Y, 1, Release),
                    Op::store(Y, 2, Relaxed),
                ],
                vec![Op::load(Y, 1, Acquire), Op::load(X, 2, Relaxed)],
            ],
            vec![2, 0, 3, 3],
            false,
        ),
        (
            "MP stale data past a later store by another thread",
            vec![
                vec![Op::store(X, 1, Relaxed), Op::store(Y, 1, Release)],
                vec![Op::store(Y, 2, Relaxed)],
                vec![Op::load(Y, 1, Acquire), Op::load(X, 2, Relaxed)],
            ],
            vec![2, 0, 3, 3],
            true,
        ),
        (
            "CoRR stale second read",
            vec![
                vec![Op::store(X, 1, Relaxed)],
                vec![Op::load(X, 1, Relaxed), Op::load(X, 2, Relaxed)],
            ],
            vec![1, 0, 3, 3],
            false,
        ),
        (
            "CoRR fresh second read",
            vec![
                vec![Op::store(X, 1, Relaxed)],
                vec![Op::load(X, 1, Relaxed), Op::load(X, 2, Relaxed)],
            ],
            vec![0, 1, 3, 3],
            true,
        ),
        (
            "LB relaxed (no thin air)",
            vec![
                vec![Op::load(X, 1, Relaxed), Op::store(Y, 1, Relaxed)],
                vec![Op::load(Y, 2, Relaxed), Op::store(X, 1, Relaxed)],
            ],
            vec![1, 1, 3, 3],
            false,
        ),
        (
            "seq_cst fences ordered through a relaxed write and an acquire read",
            vec![
                vec![
                    Op::store(Y, 1, SeqCst),
                    Op::fence(SeqCst),
                    Op::load(X, 1, Relaxed),
                ],
                vec![Op::store(X, 1, Relaxed)],
                vec![
                    Op::load(X, 2, Acquire),
                    Op::fence(SeqCst),
                    Op::load(Y, 3, Relaxed),
                ],
            ],
            vec![0, 1, 0, 3],
            false,
        ),
    ];
    for (name, threads, outcome, allowed) in rows {
        let reached = rc11::outcomes(&threads, 3);
        assert_eq!(reached.contains(&outcome), allowed, "{name}: {outcome:?}");
    }
}

/// A program, an outcome, and whether RC11 allows it.
type Row = (&'static str, Vec<Vec<Op>>, Vec<i64>, bool);

fn sb(store: Order, load: Order, fence: Option<Order>) -> Vec<Vec<Op>> {
    let thread = |own: usize, other: usize, register: usize| {
        let mut ops = vec![Op::store(own, 1, store)];
        ops.extend(fence.map(Op::fence));
        ops.push(Op::load(other, register, load));
        ops
    };
    vec![thread(X, Y, 1), thread(Y, X, 2)]
}

/// MP: data `x` then flag `y`, with the optional fences between the data and
/// flag stores (`release_fence`) and between the loads (`acquire_fence`).
fn mp(
    flag_store: Order,
    flag_load: Order,
    release_fence: Option<Order>,
    acquire_fence: Option<Order>,
) -> Vec<Vec<Op>> {
    let mut writer = vec![Op::store(X, 1, Order::Relaxed)];
    writer.extend(release_fence.map(Op::fence));
    writer.push(Op::store(Y, 1, flag_store));
    let mut reader = vec![Op::load(Y, 1, flag_load)];
    reader.extend(acquire_fence.map(Op::fence));
    reader.push(Op::load(X, 2, Order::Relaxed));
    vec![writer, reader]
}
