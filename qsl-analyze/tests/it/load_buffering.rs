// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-226-AC-1 (TC-671 step 1): EN-1's pre-check warns of the
//! load-buffering shapes of a `parallel` resolved to `ra` in the item's
//! terminal record, and of nothing else.

use ix_trace_rs::trace;
use qsl_eval::simulation::protocol::MemorySelection;
use qsl_foundation::MemoryModelName::{self, Ra, Sc, Tso};
use qsl_replay::{
    Execution, InconclusiveCause, MemoryCause, ModelCheckItem, ModelCheckOutcome, ModelCheckRecord,
    RecordWarning, ResultLabel, SettlementBasis, TerminalValue, Truth,
};

use crate::litmus::{attempt, contracts, protocol, NO_BOTH_ZERO};
use crate::weak_memory::{select, Case};

/// The unit whose branch `t` is `t_body` and whose branch `s` stores `y`
/// then `x`.
fn unit(t_body: &str) -> String {
    let run = format!(
        "parallel Both {{ branch t sequence T {{ {t_body} }} \
         branch s sequence S {{ {} {} }} }} join all [t, s];",
        attempt("Sy2", "w", "Cell::set", "relaxed", "IsY, StoreTwo"),
        attempt("Sx2", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
    );
    contracts() + &protocol("SB", &run) + NO_BOTH_ZERO
}

fn load(node: &str, cell: &str, ordering: &str) -> String {
    attempt(
        node,
        "c",
        "Core::read1",
        ordering,
        &format!("Reads{cell}1, Read1"),
    )
}

fn store(node: &str, cell: &str) -> String {
    attempt(
        node,
        "w",
        "Cell::set",
        "relaxed",
        &format!("Is{cell}, StoreOne"),
    )
}

fn selection(model: MemoryModelName) -> [MemorySelection; 1] {
    [select("SB::Both", model)]
}

fn record(
    case: &Case,
    model: MemoryModelName,
) -> ModelCheckRecord<qsl_eval::simulation::protocol::ProtocolTransition> {
    let selection = selection(model);
    let outcome = case.claim(&selection, None);
    case.settle(&selection, &outcome, false, None)
}

fn warnings(body: &str, model: MemoryModelName) -> Vec<RecordWarning> {
    record(&Case::with_three_cells(&unit(body)), model).warnings
}

/// FR-226-AC-1: under `ra`, `t`'s relaxed load of `x` followed by its
/// relaxed store to `y`, both touched by `s`, puts one
/// `memory.load-buffering-shape` warning naming the two `t` attempts in the
/// record, whose other members equal those of the request settled without
/// it; an `acquire` load, an `acquire` fence between, a same-location
/// store, a load of a location only `t` accesses and a store to one report
/// none; neither does the same package under `sc` or `tso`.
#[trace("TC-671", "FR-226-AC-1")]
#[test]
fn a_relaxed_load_then_a_store_to_another_shared_location_warns_under_ra() {
    let shape = format!("{} {}", load("Lx", "X", "relaxed"), store("Sy", "Y"));
    let case = Case::with_three_cells(&unit(&shape));
    let accesses = case.accesses(&selection(Ra));
    let transition = |node: &str| {
        let attempt = accesses.attempt(node).expect("a weak attempt");
        assert_eq!(attempt.instances.len(), 1, "{node}");
        attempt.instances[0].transition.clone()
    };
    let warned = record(&case, Ra);
    assert_eq!(
        warned.warnings,
        [RecordWarning::LoadBufferingShape {
            load: transition("Both/t/T/Lx"),
            store: transition("Both/t/T/Sy"),
        }]
    );
    assert_eq!(warned.warnings[0].code(), "memory.load-buffering-shape");

    // The warning is no member of the record's value: the item settles as
    // the engine's outcome says, a closed-scope proof, and
    // as the same shape with the load made `acquire` (no warning) does.
    let selection = selection(Ra);
    let outcome = case.claim(&selection, None);
    let settled = case.settle(&selection, &outcome, false, None);
    assert_eq!(settled.warnings, warned.warnings);
    assert!(
        matches!(outcome, qsl_replay::ModelCheckOutcome::Holds { .. }),
        "{outcome:?}"
    );
    assert_eq!(settled.label, ResultLabel::Proved);
    assert_eq!(settled.basis, SettlementBasis::ClosedScope);
    assert!(
        matches!(settled.value, TerminalValue::Proved { .. }),
        "{:?}",
        settled.value
    );
    assert_eq!(settled.counterexample, None);
    assert!(settled.hints.is_empty());
    let obligation = case.obligation(&selection);

    let quiet_case = Case::with_three_cells(&unit(&format!(
        "{} {}",
        load("Lx", "X", "acquire"),
        store("Sy", "Y")
    )));
    let quiet = record(&quiet_case, Ra);
    assert!(quiet.warnings.is_empty(), "{:?}", quiet.warnings);
    assert_eq!(warned.value, quiet.value);
    assert_eq!(warned.label, quiet.label);
    assert_eq!(warned.basis, quiet.basis);
    assert_eq!(warned.execution, quiet.execution);
    assert_eq!(warned.truth, quiet.truth);
    assert_eq!(warned.depth, quiet.depth);
    assert_eq!(warned.limits, quiet.limits);
    assert_eq!(warned.counterexample, quiet.counterexample);
    assert_eq!(warned.hints, quiet.hints);
    assert_eq!(obligation, quiet_case.obligation(&selection));

    let fence = |ordering: &str| {
        format!(
            "{} fence F ordering {ordering}; {}",
            load("Lx", "X", "relaxed"),
            store("Sy", "Y")
        )
    };
    let none = [
        ("acquire fence", fence("acquire")),
        (
            "same location",
            format!("{} {}", load("Lx", "X", "relaxed"), store("Sx", "X")),
        ),
        (
            "load only t accesses",
            format!("{} {}", load("Lz", "Z", "relaxed"), store("Sy", "Y")),
        ),
        (
            "store only t accesses",
            format!("{} {}", load("Lx", "X", "relaxed"), store("Sz", "Z")),
        ),
    ];
    for (name, body) in none {
        assert!(warnings(&body, Ra).is_empty(), "{name}");
    }
    // A release fence is no acquire: the shape stays.
    assert_eq!(warnings(&fence("release"), Ra).len(), 1);

    for model in [Sc, Tso] {
        assert!(warnings(&shape, model).is_empty(), "{model:?}");
    }
}

/// The shapes along every path through a branch, not only a straight line
/// (FR-226 Behavior): a store or local access between does not stop a
/// pending load, an acquire fence in every case of a `choice` does, a
/// `repeat` that may run zero times does not, and a non-atomic load pends
/// like a relaxed one.
#[trace("TC-671")]
#[test]
fn the_shapes_follow_every_path_through_a_branch() {
    let lx = load("Lx", "X", "relaxed");
    let sy = store("Sy", "Y");
    let acquire_fences = "choice Pick by w visible () { \
        case yes when { true } fence F1 ordering acquire; \
        case no when { false } fence F2 ordering acquire; }";
    let one_fence = "choice Pick by w visible () { \
        case yes when { true } fence F1 ordering acquire; \
        case no when { false } sequence N { } }";
    let seq_cst_store = attempt("Sz", "w", "Cell::set", "seq_cst", "IsZ, StoreOne");
    let acquire_in_repeat = format!(
        "repeat L by w visible () max 2 while {{ false }} {} exhausted sequence O {{ }}",
        load("Lq", "Y", "acquire")
    );
    let awaiting = |after: &str, then: &str, timeout: &str| {
        format!(
            "await Wait after {after} using v clock \"model-steps\" within [0,1] \
             match event Seen by w as (seen: Boolean) {{ true }}; \
             then sequence T2 {{ {then} }} timeout sequence T3 {{ {timeout} }}"
        )
    };
    let acquire_y = load("Lq2", "Y", "acquire");
    let cases = [
        (
            "await then and timeout are alternatives",
            format!(
                "{} {}",
                load("Lw", "Z", "relaxed"),
                awaiting("Lw", &lx, &sy)
            ),
            0,
        ),
        (
            "await timeout keeps the load its then path clears",
            format!("{lx} {}", awaiting("Lx", &acquire_y, &sy)),
            1,
        ),
        (
            "every choice case fences",
            format!("{lx} {acquire_fences} {sy}"),
            0,
        ),
        (
            "one choice case fences",
            format!("{lx} {one_fence} {sy}"),
            1,
        ),
        (
            "a seq_cst store is no acquire",
            format!("{lx} {seq_cst_store} {sy}"),
            1,
        ),
        (
            "a repeat may run zero times",
            format!("{lx} {acquire_in_repeat} {sy}"),
            1,
        ),
        (
            "a non-atomic load",
            format!("{} {sy}", load("Lx", "X", "")),
            1,
        ),
    ];
    for (name, body, expected) in cases {
        assert_eq!(warnings(&body, Ra).len(), expected, "{name}");
    }
}

/// A pre-check that cannot classify a subject the run explored is an
/// internal fault: the item settles `failed`, with no value of the run
/// kept, instead of settling without the warnings it owes.
#[trace("TC-671")]
#[test]
fn a_pre_check_that_cannot_classify_the_subject_fails_the_item() {
    let shape = format!("{} {}", load("Lx", "X", "relaxed"), store("Sy", "Y"));
    let case = Case::with_three_cells(&unit(&shape));
    let tso = selection(Tso);
    let outcome = case.check(&tso, ModelCheckItem::DeadlockFreedom, Some(1));
    assert!(
        matches!(
            outcome,
            ModelCheckOutcome::Memory {
                cause: MemoryCause::BoundReached { .. },
                ..
            }
        ),
        "{outcome:?}"
    );
    let settled = case.settle(&tso, &outcome, true, Some(1));
    assert!(matches!(
        settled.value,
        TerminalValue::Inconclusive(InconclusiveCause::MemoryBoundReached { .. })
    ));
    let failed = case.settle_naming(&tso, &outcome, Some(1), "NoSuchProtocol");
    assert_eq!(failed.value, TerminalValue::Failed);
    assert_eq!(failed.label, ResultLabel::Failed);
    assert_eq!(failed.execution, Execution::Failed);
    assert_eq!(failed.truth, Truth::Unavailable);
}

/// A `repeat` body runs at most `max` times: with `max 1` the load of the
/// only iteration cannot meet the store of another.
#[trace("TC-671")]
#[test]
fn a_repeat_body_runs_at_most_its_maximum() {
    let body = format!("{} {}", store("Sy", "Y"), load("Lx", "X", "relaxed"));
    let repeated = |max: u32| {
        format!(
            "repeat L by w visible () max {max} while {{ true }} sequence B {{ {body} }} exhausted sequence O {{ }}"
        )
    };
    assert!(warnings(&repeated(1), Ra).is_empty());
    assert_eq!(warnings(&repeated(2), Ra).len(), 1);
}
