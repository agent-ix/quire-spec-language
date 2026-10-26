// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-224 and FR-227 (TC-669, TC-672): the `ra` race summary, the derived
//! race-freedom item it decides and the replay of its counterexample, over
//! ADR-025 §10's message-passing shape: the writer stores the plain field
//! `data` then the `flag`, and the reader loads `flag` and loads `data` only
//! in the case of a `choice` where the flag load read 1.

use ix_trace_rs::trace;
use qsl_eval::model_subject::ModelLetters;
use qsl_eval::simulation::protocol::{MemorySelection, ProtocolTransition, Ra, StepKind};
use qsl_eval::simulation::{Location, SubjectSystem, TransitionSystem};
use qsl_foundation::MemoryModelName;
use qsl_replay::{
    CounterexampleKind, ModelCheckItem, ModelCheckOutcome, ModelStep, ModelTraceSettlement, Race,
    ResultLabel, TemporalCounterexample,
};
use qsl_route::deadlock::{race_freedom_items, ProtocolMemoryItem};

use crate::litmus::{attempt, contracts, protocol, NO_BOTH_ZERO};
use crate::weak_memory::{
    litmus_run, load_ordered, select, steps, Case, MP,
};

/// `SB::Main::Both` resolved to `ra`.
fn ra() -> [MemorySelection; 1] {
    [select("SB::Main::Both", MemoryModelName::Ra)]
}

/// The store `node` of 1 to the cell `cell` with `ordering` (empty for a
/// non-atomic store).
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

/// FR-224's shape: `x` is the field `data` accessed with `data`'s ordering,
/// `y` the `flag` stored with `flag_store` and loaded with `flag_load`; the
/// reader loads `data` only after a flag load that read 1.
fn message_passing(data: &str, flag_store: &str, flag_load: &str) -> String {
    let reader = format!(
        "{} choice Pick by c visible (bLy) {{ case yes when {{ bLy }} {} \
         case no when {{ not bLy }} sequence Skip {{ }} }}",
        attempt("Ly", "c", "Core::look", flag_load, "LooksY, Looked"),
        load_ordered("Lx", "x", 2, data),
    );
    litmus_run(
        &[
            (
                "a",
                steps("A", &[put("Sx", "x", data), put("Sy", "y", flag_store)]),
            ),
            ("b", format!("sequence B {{ {reader} }}")),
        ],
        "",
    )
}

/// The unit over `run` with the claim `MP`'s letters.
fn unit(run: &str, letters: &'static dyn ModelLetters) -> Case {
    Case::with_letters(
        &(contracts() + &protocol("SB", run) + NO_BOTH_ZERO),
        letters,
    )
}

/// The location of the cell the object `key` holds.
fn location_of(case: &Case, key: &str) -> Location {
    let accesses = case.accesses(&ra());
    accesses
        .shared()
        .iter()
        .find(|location| accesses.cell(location).is_some_and(|cell| cell.key == key))
        .cloned()
        .expect("a shared location")
}

/// FR-224's request writer over `case`'s `ra` request: the number of
/// race-freedom items its two items over `SB` give.
fn race_freedom_item_count(case: &Case, selection: &[MemorySelection]) -> usize {
    let accesses = case.accesses(selection);
    let subject = case.subject(selection);
    let models: Vec<(String, MemoryModelName)> = subject
        .memory
        .parallels()
        .iter()
        .map(|resolved| (resolved.parallel.clone(), resolved.model))
        .collect();
    let item = ProtocolMemoryItem {
        subject: &subject.protocol,
        models: &models,
        non_atomic_access: !accesses.non_atomic().is_empty(),
    };
    race_freedom_items(&[item, item]).len()
}

/// Whether `transition` is the step at the node named `node`, the last
/// segment of its static declaration path.
fn is_at(transition: &ProtocolTransition, node: &str) -> bool {
    transition
        .node()
        .is_some_and(|path| path.rsplit('/').next() == Some(node))
}

/// The counterexample `outcome` carries.
fn violated(
    outcome: ModelCheckOutcome<ProtocolTransition>,
) -> TemporalCounterexample<ProtocolTransition> {
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("{outcome:?}");
    };
    *counterexample
}

/// The position the trace reaches by the step at `node`.
fn position_of(counterexample: &TemporalCounterexample<ProtocolTransition>, node: &str) -> u64 {
    let at = counterexample
        .prefix
        .iter()
        .position(|step| is_at(&step.transition, node))
        .unwrap_or_else(|| panic!("a step at {node}"));
    u64::try_from(at + 1).expect("small")
}

/// FR-224-AC-1 (TC-669), the `ra` half: ADR-025 §10's message-passing shape
/// with `data` non-atomic and a `release` flag store and `acquire` flag load
/// carries one `RaceFreedom` item, which settles `proved`.
#[trace("TC-669", "FR-224-AC-1")]
#[test]
fn a_release_acquire_flag_proves_race_freedom_under_ra() {
    let case = unit(&message_passing("", "release", "acquire"), &MP);
    assert_eq!(race_freedom_item_count(&case, &ra()), 1);
    let outcome = case.check(&ra(), ModelCheckItem::RaceFreedom, None);
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "{outcome:?}"
    );
    let record = case.settle_race(&ra(), &outcome);
    assert_eq!(record.label, ResultLabel::Proved, "{record:?}");
}

/// FR-224-AC-2 (TC-669), the `ra` half, and FR-227-AC-3 (TC-672), its first
/// sentence: with a `relaxed` flag the item settles `refuted` under `ra`
/// with kind `Race{location: data, earlier, later}`, `earlier` the position
/// of `data := 1` and `later` that of `r2 := data`, and the counterexample
/// replays to `reproduced-with-evaluated-witness`.
#[trace("TC-669", "FR-224-AC-2")]
#[trace("TC-672", "FR-227-AC-3")]
#[test]
fn a_relaxed_flag_refutes_race_freedom_under_ra_and_the_race_replays() {
    let case = unit(&message_passing("", "relaxed", "relaxed"), &MP);
    assert_eq!(race_freedom_item_count(&case, &ra()), 1);
    let outcome = case.check(&ra(), ModelCheckItem::RaceFreedom, None);
    let counterexample = violated(outcome.clone());
    assert_eq!(
        counterexample.kind,
        CounterexampleKind::Race(Race {
            location: location_of(&case, "x"),
            earlier: position_of(&counterexample, "Sx"),
            later: position_of(&counterexample, "Lx"),
        })
    );
    assert_eq!(
        counterexample
            .prefix
            .last()
            .map(|step| is_at(&step.transition, "Lx")),
        Some(true),
        "the prefix ends at the step that completes the race"
    );
    let record = case.settle_race(&ra(), &outcome);
    assert_eq!(record.label, ResultLabel::Refuted, "{record:?}");
    let replayed = case
        .replay_race(&ra(), counterexample)
        .expect("the race replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
}

/// The settlement of `payload`'s replay, which must be a disagreement.
fn disagreement(case: &Case, payload: TemporalCounterexample<ProtocolTransition>) {
    let replayed = case.replay_race(&ra(), payload).expect("replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Inconclusive(_)),
        "{replayed:?}"
    );
}

/// FR-227-AC-3 (TC-672), the forged races: an envelope whose `earlier` names
/// the flag store, which is not an access to `data`, settles `inconclusive`,
/// `ReplayParity`; so does the `release`/`acquire` shape, whose `data` load
/// runs only after its flag load read 1, with a forged race on `data`.
#[trace("TC-672", "FR-227-AC-3")]
#[test]
fn a_forged_race_settles_inconclusive_under_ra() {
    let relaxed = unit(&message_passing("", "relaxed", "relaxed"), &MP);
    let counterexample = violated(relaxed.check(&ra(), ModelCheckItem::RaceFreedom, None));
    let CounterexampleKind::Race(race) = counterexample.kind.clone() else {
        panic!("a race counterexample");
    };
    let mut forged = counterexample.clone();
    forged.kind = CounterexampleKind::Race(Race {
        earlier: position_of(&counterexample, "Sy"),
        ..race.clone()
    });
    disagreement(&relaxed, forged);

    // The release/acquire shape has no race: the path to its data load, after
    // a flag load that read 1, with a race claimed on `data`.
    let ordered = unit(&message_passing("", "release", "acquire"), &MP);
    let system = ordered.system(&ra(), Ra::default()).expect("builds");
    let mut state = system.initial().remove(0);
    let mut prefix = Vec::new();
    let path: [(&str, Option<u64>); 5] = [
        ("", None),
        ("Sx", None),
        ("Sy", None),
        ("Ly", Some(0)),
        ("Lx", Some(0)),
    ];
    for (node, read) in path {
        let (transition, next) = system
            .successors(&state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| {
                let at = if node.is_empty() {
                    transition.kind() == StepKind::Fork
                } else {
                    is_at(transition, node)
                };
                at && transition.memory_choice().and_then(|choice| choice.read_depth) == read
            })
            .unwrap_or_else(|| panic!("{node} is enabled"));
        let (_, digest) = SubjectSystem::state_key(&system, &next).expect("a key");
        prefix.push(ModelStep {
            transition: system.reported(transition, &next).expect("reports"),
            post_state: digest,
        });
        state = next;
    }
    let forged = TemporalCounterexample {
        initial_state: 0,
        prefix,
        over_binding: None,
        fairness: Vec::new(),
        kind: CounterexampleKind::Race(Race {
            location: race.location,
            earlier: 2,
            later: 5,
        }),
        lasso: None,
        blocked: Vec::new(),
        refinement: None,
    };
    disagreement(&ordered, forged);
}

/// FR-224-AC-4 (TC-669), the `ra` half: in the `relaxed`-flag shape the
/// authored claim `always holds(not (r1 = 1 and r2 = 0))` settles `refuted`
/// with a counterexample of kind `Formula`, the same verdict as with `data`
/// atomic and `relaxed`.
#[trace("TC-669", "FR-224-AC-4")]
#[test]
fn a_race_changes_no_authored_claims_verdict_under_ra() {
    let verdict = |data: &str| {
        let case = unit(&message_passing(data, "relaxed", "relaxed"), &MP);
        let counterexample = violated(case.claim(&ra(), None));
        counterexample.kind
    };
    assert_eq!(verdict(""), CounterexampleKind::Formula);
    assert_eq!(verdict("relaxed"), CounterexampleKind::Formula);
}
