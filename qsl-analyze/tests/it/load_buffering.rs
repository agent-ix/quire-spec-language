// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-226-AC-1 (TC-671 step 1): EN-1's pre-check warns of the
//! load-buffering shapes of a `parallel` resolved to `ra` in the item's
//! terminal record, and of nothing else.

use ix_trace_rs::trace;
use qsl_eval::simulation::protocol::MemorySelection;
use qsl_foundation::MemoryModelName::{self, Ra, Sc, Tso};
use qsl_replay::{ModelCheckRecord, RecordWarning};

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

    // The same request settled without the warning: the load `acquire`.
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
