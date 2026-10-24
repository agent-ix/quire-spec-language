// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-223-AC-5 (TC-668): ADR-025 §15's LT-12 and LT-13 under `sc`, `tso` and
//! `ra`, against an RC11 execution-graph checker (`crate::rc11_support`)
//! that shares no code with `qsl-eval`'s `Ra`.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_eval::simulation::protocol::{MemoryModel, Ra, Sc, Tso};
use qsl_foundation::MemoryModelName;

use crate::litmus::sb_unit;
use crate::rc11_support::{self as rc11, Op, Order};
use crate::weak_memory::{litmus_run, outcomes, select, steps, Case};

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

fn under<M: MemoryModel>(
    case: &Case,
    model: MemoryModelName,
    memory: M,
    select_it: bool,
) -> BTreeSet<Vec<i64>> {
    let selection = [select("SB::Main::Both", model)];
    let selection: &[_] = if select_it { &selection } else { &[] };
    outcomes(&case.system(selection, memory).expect("builds"))
}

/// LT-12 or LT-13: the weak outcome is reachable under `ra` and under neither
/// `sc` nor `tso`; RC11 forbids it; every outcome RC11 allows is reachable
/// under `ra`, and the weak outcome is the only one `ra` adds.
fn check(name: &str, threads: &[Vec<Op>], weak: &[i64]) {
    let case = case_of(threads);
    let ra = under(&case, MemoryModelName::Ra, Ra::default(), true);
    let sc = under(&case, MemoryModelName::Sc, Sc, false);
    let tso = under(&case, MemoryModelName::Tso, Tso::default(), true);
    let rc11 = rc11::outcomes(threads, 3);
    let weak = weak.to_vec();

    assert!(ra.contains(&weak), "{name}: ra reaches {weak:?}: {ra:?}");
    assert!(!sc.contains(&weak), "{name}: sc: {sc:?}");
    assert!(!tso.contains(&weak), "{name}: tso: {tso:?}");
    assert!(!rc11.contains(&weak), "{name}: RC11 forbids {weak:?}");
    assert!(
        rc11.is_subset(&ra),
        "{name}: RC11 outcomes missing under ra: {:?}",
        rc11.difference(&ra).collect::<Vec<_>>()
    );
    assert!(sc.is_subset(&rc11), "{name}: sc outcomes are RC11 outcomes");
    assert_eq!(
        ra.difference(&rc11).cloned().collect::<Vec<_>>(),
        vec![weak],
        "{name}: ra adds only the PSC-7 execution"
    );
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

/// LT-12: a release flag between an SC write and an SC read through a third
/// thread.
#[trace("TC-668", "FR-223-AC-5")]
#[test]
fn lt12_reaches_its_weak_outcome_under_ra_only() {
    check("LT-12", &lt12(), &[1, 0, 0, 3]);
}

/// LT-13: an SC access pair against a `relaxed` store, an SC fence and a
/// `relaxed` load.
#[trace("TC-668", "FR-223-AC-5")]
#[test]
fn lt13_reaches_its_weak_outcome_under_ra_only() {
    check("LT-13", &lt13(), &[0, 0, 3, 3]);
}

/// The checker allows and forbids the classical weak outcomes of RC11, so
/// its sets are an oracle: store buffering is weak unless `seq_cst`, message
/// passing is ordered by release and acquire, IRIW needs `seq_cst`, and a
/// `seq_cst` fence orders `relaxed` store buffering.
#[trace("TC-668", "FR-223-AC-5")]
#[test]
fn the_rc11_checker_matches_the_known_litmus_outcomes() {
    let sb = |store: Order, load: Order, fence: Option<Order>| {
        let thread = |own: usize, other: usize, register: usize| {
            let mut ops = vec![Op::store(own, 1, store)];
            ops.extend(fence.map(Op::fence));
            ops.push(Op::load(other, register, load));
            ops
        };
        rc11::outcomes(&[thread(X, Y, 1), thread(Y, X, 2)], 2)
    };
    let both_zero = vec![0, 0, 3, 3];
    assert!(sb(Order::Relaxed, Order::Relaxed, None).contains(&both_zero));
    assert!(sb(Order::Release, Order::Acquire, None).contains(&both_zero));
    assert!(!sb(Order::SeqCst, Order::SeqCst, None).contains(&both_zero));
    assert!(!sb(Order::Relaxed, Order::Relaxed, Some(Order::SeqCst)).contains(&both_zero));
    assert_eq!(sb(Order::SeqCst, Order::SeqCst, None).len(), 3);

    let mp = |flag_store: Order, flag_load: Order| {
        rc11::outcomes(
            &[
                vec![Op::store(X, 1, Order::Relaxed), Op::store(Y, 1, flag_store)],
                vec![Op::load(Y, 1, flag_load), Op::load(X, 2, Order::Relaxed)],
            ],
            2,
        )
    };
    let stale_data = vec![1, 0, 3, 3];
    assert!(mp(Order::Relaxed, Order::Relaxed).contains(&stale_data));
    assert!(!mp(Order::Release, Order::Acquire).contains(&stale_data));

    let iriw = |order: Order| {
        rc11::outcomes(
            &[
                vec![Op::store(X, 1, order)],
                vec![Op::store(Y, 1, order)],
                vec![Op::load(X, 1, order), Op::load(Y, 2, order)],
                vec![Op::load(Y, 3, order), Op::load(X, 4, order)],
            ],
            2,
        )
    };
    let opposite_orders = vec![1, 0, 1, 0];
    assert!(iriw(Order::Acquire).contains(&opposite_orders));
    assert!(!iriw(Order::SeqCst).contains(&opposite_orders));
}
