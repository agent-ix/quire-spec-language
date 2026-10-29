// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-224 and FR-227 (TC-669, TC-672): the `ra` race summary, the derived
//! race-freedom item it decides and the replay of its counterexample, over
//! ADR-025 §10's message-passing shape: the writer stores the plain field
//! `data` then the `flag`, and the reader loads `flag` and loads `data` only
//! in the case of a `choice` where the flag load read 1.

use std::collections::{BTreeSet, VecDeque};

use ix_trace_rs::trace;
use qsl_analyze::model_check::manifest as en1;
use qsl_analyze::model_check::ModelCheckRefusal;
use qsl_analyze::zone_check::manifest as en6;

use qsl_eval::model_subject::ModelLetters;
use qsl_eval::model_subject::ProtocolSubject;
use qsl_eval::simulation::protocol::{
    MemoryModel, MemorySelection, ProtocolTransition, Ra, StepKind, Tso,
};
use qsl_eval::simulation::{Location, SubjectSystem, TransitionSystem};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::MemoryModelName;
use qsl_replay::node_occurrence;
use qsl_replay::{
    check_protocol_closure, CertificateRule, CounterexampleKind, DisagreementCause, ModelCheckItem,
    ModelCheckLimits, ModelCheckOutcome, ModelStep, ModelTraceSettlement, ProofCertificate,
    ProtocolCertificateRequest, Race, ResultLabel, TemporalCertificateCheck,
    TemporalCounterexample,
};
use qsl_route::deadlock::{ModelTemporalItem, NoAnchor, ObligationKind, ProtocolTemporalItem};
use qsl_route::model_request::{ModelRequest, WrittenRequest};
use qsl_route::request::{DerivedLimits, ItemKind, RequestItem};
use qsl_semantics::check::Capability;
use qsl_semantics::family::{ClaimExtent, Requirements};
use quire_exact::Cancel;
use quire_semantic_value::catalog::{CatalogCode, Category};
use quire_semantic_value::fairness::PropertyForm;
use serde_json::{json, Value as Json};

use crate::dense_route::{ids, registry};
use crate::derived_request::{derived_subjects, items_of_kind, protocol_request};
use crate::litmus::{attempt, contracts, protocol, protocol_with, NO_BOTH_ZERO};
use crate::weak_memory::{litmus_run, load_ordered, select, steps, Case, MP};

/// `SB::Main::Both` resolved to `tso`.
fn tso() -> [MemorySelection; 1] {
    [select("SB::Main::Both", MemoryModelName::Tso)]
}

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
    location_of_under(case, key, &ra())
}

/// [`location_of`] under the resolved models `selection`.
fn location_of_under(case: &Case, key: &str, selection: &[MemorySelection]) -> Location {
    let accesses = case.accesses(selection);
    accesses
        .shared()
        .iter()
        .find(|location| accesses.cell(location).is_some_and(|cell| cell.key == key))
        .cloned()
        .expect("a shared location")
}

/// The resolved model of each outermost `parallel` of `subject`, by scoped
/// path.
fn models_of(subject: &ProtocolSubject<'_>) -> Vec<(String, MemoryModelName)> {
    subject
        .memory
        .parallels()
        .iter()
        .map(|resolved| (resolved.parallel.clone(), resolved.model))
        .collect()
}

/// A second caller item's claim over `SB`, beside the unit's clause.
fn other_claim() -> OccurrenceKey {
    OccurrenceKey::generated(WireNodeId::from_digest([9; 32]))
}

/// The request over `case`'s two temporal items over `SB` under
/// `selection`, its unit's clause and [`other_claim`], when the caller has
/// found a non-atomic shared access or not.
fn race_freedom_request(
    case: &Case,
    selection: &[MemorySelection],
    non_atomic_access: bool,
) -> WrittenRequest<String> {
    let subject = case.subject(selection);
    let models = models_of(&subject);
    let graph = case.checked().graph();
    let protocol = graph.protocol("SB").expect("SB");
    let clause = node_occurrence(&graph.temporal_clauses()[0]);
    let other = other_claim();
    let item = |occurrence| ProtocolTemporalItem {
        occurrence,
        subject: &subject.protocol,
        protocol,
        models: &models,
        non_atomic_access,
    };
    protocol_request(&[item(&clause), item(&other)])
}

/// The obligation identity of the race-freedom item `case`'s request gives
/// under `selection`.
fn race_freedom_obligation(case: &Case, selection: &[MemorySelection]) -> String {
    let accesses = case.accesses(selection);
    let written = race_freedom_request(case, selection, !accesses.non_atomic().is_empty());
    let [(_, obligation)] = &items_of_kind(&written, ItemKind::RaceFreedom)[..] else {
        panic!("one race-freedom item");
    };
    format!("{obligation:?}")
}

/// FR-224's request writer over `case`'s `ra` request: the number of
/// race-freedom items its two items over `SB` give.
fn race_freedom_item_count(case: &Case, selection: &[MemorySelection]) -> usize {
    let accesses = case.accesses(selection);
    race_freedom_item_count_for_access(case, selection, !accesses.non_atomic().is_empty())
}

/// FR-224's request writer when the caller has derived non-atomic access
/// from the protocol's reachable-state classifications.
fn race_freedom_item_count_for_access(
    case: &Case,
    selection: &[MemorySelection],
    non_atomic_access: bool,
) -> usize {
    let written = race_freedom_request(case, selection, non_atomic_access);
    derived_subjects(&written, ItemKind::RaceFreedom).len()
}

/// Two threads of `SB` that each store to `x` with no `ordering`, in a unit
/// selecting the protocol profile, so `SB`'s clause has its node.
fn two_plain_stores() -> Case {
    let run = litmus_run(
        &[
            ("a", steps("A", &[put("Sa", "x", "")])),
            ("b", steps("B", &[put("Sb", "x", "")])),
        ],
        "",
    );
    Case::protocol_profiled(&(contracts() + &protocol("SB", &run) + NO_BOTH_ZERO), &MP)
}

/// Whether EN-1 finds a non-atomic shared access in a reachable state.
fn has_reachable_non_atomic_access<M: MemoryModel>(
    case: &Case,
    selection: &[MemorySelection],
    memory: M,
) -> bool {
    let system = case.system(selection, memory).expect("protocol system");
    let mut queue: VecDeque<_> = system.initial().into();
    let mut seen = BTreeSet::new();
    while let Some(state) = queue.pop_front() {
        let (key, _) = system.state_key(&state).expect("state key");
        if !seen.insert(key) {
            continue;
        }
        let has_access = system
            .has_non_atomic_access_at(&state)
            .expect("state access classification");
        if has_access {
            return true;
        }
        let expansion = system.successors(&state).expect("state successors");
        for (_, successor) in expansion.successors {
            let (key, _) = system.state_key(&successor).expect("successor key");
            if !seen.contains(&key) {
                queue.push_back(successor);
            }
        }
    }
    false
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

/// The settlement of `payload`'s replay, which must be the `ReplayParity`
/// verdict disagreement: the replay completed, and found no race where the
/// payload claimed one. A prefix that failed to replay would settle with the
/// `NoValue` cause instead.
fn disagreement(case: &Case, payload: TemporalCounterexample<ProtocolTransition>) {
    disagreement_under(case, &ra(), payload);
}

/// [`disagreement`] under the resolved models `selection`.
fn disagreement_under(
    case: &Case,
    selection: &[MemorySelection],
    payload: TemporalCounterexample<ProtocolTransition>,
) {
    let replayed = case.replay_race(selection, payload).expect("replays");
    let ModelTraceSettlement::Inconclusive(cause) = &replayed.settlement else {
        panic!("{replayed:?}");
    };
    let DisagreementCause::Verdicts { proved, replayed } = cause else {
        panic!("{cause:?}");
    };
    assert_eq!(proved.category(), Category::Violation);
    assert_eq!(replayed.category(), Category::Success);
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
    let path = [
        Hop::Fork,
        Hop::At("Sx", None),
        Hop::At("Sy", None),
        Hop::At("Ly", Some(0)),
        Hop::At("Lx", Some(0)),
    ];
    let forged = forged_race(&ordered, &path, race.location, 2, 5);
    disagreement(&ordered, forged);
}

/// A step of a hand-walked path: the `fork`, or the step at a node with the
/// read depth it chose.
#[derive(Clone, Copy)]
enum Hop {
    Fork,
    Kind(StepKind),
    /// The step at the node, whatever its read depth.
    Node(&'static str),
    At(&'static str, Option<u64>),
}

/// A counterexample of kind `Race{location, earlier, later}` over the prefix
/// `path` walks through `case`'s `ra` system from its initial state.
fn forged_race(
    case: &Case,
    path: &[Hop],
    location: Location,
    earlier: u64,
    later: u64,
) -> TemporalCounterexample<ProtocolTransition> {
    forged_race_under(case, &ra(), Ra::default(), path, location, earlier, later)
}

/// [`forged_race`] over `memory`, the model `selection` resolves to.
fn forged_race_under<M: MemoryModel>(
    case: &Case,
    selection: &[MemorySelection],
    memory: M,
    path: &[Hop],
    location: Location,
    earlier: u64,
    later: u64,
) -> TemporalCounterexample<ProtocolTransition> {
    let system = case.system(selection, memory).expect("builds");
    let mut state = system.initial().remove(0);
    let mut prefix = Vec::new();
    for hop in path {
        let (transition, next) = system
            .successors(&state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| match hop {
                Hop::Fork => transition.kind() == StepKind::Fork,
                Hop::Kind(kind) => transition.kind() == *kind,
                Hop::Node(node) => is_at(transition, node),
                Hop::At(node, read) => {
                    is_at(transition, node)
                        && transition
                            .memory_choice()
                            .and_then(|choice| choice.read_depth)
                            == *read
                }
            })
            .unwrap_or_else(|| panic!("a step for the hop of the path is enabled"));
        let (_, digest) = SubjectSystem::state_key(&system, &next).expect("a key");
        prefix.push(ModelStep {
            transition: system.reported(transition, &next).expect("reports"),
            post_state: digest,
            delay: None,
        });
        state = next;
    }
    TemporalCounterexample {
        initial_state: 0,
        prefix,
        over_binding: None,
        fairness: Vec::new(),
        kind: CounterexampleKind::Race(Race {
            location,
            earlier,
            later,
        }),
        lasso: None,
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
    }
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

/// `message_passing`'s branches `a` and `b` over the extra contracts `extra`
/// and the channel members `members`.
fn unit_with(members: &str, extra: &str, a: &str, b: &str) -> Case {
    let run = litmus_run(&[("a", a.to_owned()), ("b", b.to_owned())], "");
    Case::with_letters(
        &(contracts() + extra + &protocol_with("SB", members, &run) + NO_BOTH_ZERO),
        &MP,
    )
}

/// The reader of `message_passing`: it loads `data` after a flag load with
/// `flag_load` that read 1, then `after`.
fn reader(flag_load: &str, before_data: &str) -> String {
    format!(
        "sequence B {{ {} choice Pick by c visible (bLy) {{ case yes when {{ bLy }} \
         sequence Yes {{ {before_data} {} }} \
         case no when {{ not bLy }} sequence Skip {{ }} }} }}",
        attempt("Ly", "c", "Core::look", flag_load, "LooksY, Looked"),
        load_ordered("Lx", "x", 2, ""),
    )
}

/// Whether `outcome` proved the item over its subject.
fn holds(outcome: &ModelCheckOutcome<ProtocolTransition>) -> bool {
    matches!(outcome, ModelCheckOutcome::Holds { .. })
}

/// FR-227-AC-3 (TC-672), the fence branches of replay: in fence-based message
/// passing (`data := 1; fence release; flag := 1 relaxed` against `r1 := flag
/// relaxed; fence acquire; r2 := data`) the fences order the accesses, so the
/// item proves, and a race forged on `data` is refused. The release fence
/// sets `rel` for the `flag` store although the location was first touched
/// after the fence (RC11, ADR-025 RA-5).
#[trace("TC-672", "FR-227-AC-3")]
#[test]
fn a_forged_race_through_release_and_acquire_fences_is_refused() {
    let writer = steps(
        "A",
        &[
            put("Sx", "x", ""),
            "fence F1 ordering release;".to_owned(),
            put("Sy", "y", "relaxed"),
        ],
    );
    let case = unit_with(
        "",
        "",
        &writer,
        &reader("relaxed", "fence F2 ordering acquire;"),
    );
    let outcome = case.check(&ra(), ModelCheckItem::RaceFreedom, None);
    assert!(holds(&outcome), "{outcome:?}");
    let path = [
        Hop::Fork,
        Hop::At("Sx", None),
        Hop::Node("F1"),
        Hop::At("Sy", None),
        Hop::At("Ly", Some(0)),
        Hop::Node("F2"),
        Hop::At("Lx", Some(0)),
    ];
    let forged = forged_race(&case, &path, location_of(&case, "x"), 2, 7);
    disagreement(&case, forged);

    // Without the acquire fence the same shape races, and the race replays.
    let racy = unit_with("", "", &writer, &reader("relaxed", ""));
    let counterexample = violated(racy.check(&ra(), ModelCheckItem::RaceFreedom, None));
    assert!(matches!(counterexample.kind, CounterexampleKind::Race(_)));
    let replayed = racy.replay_race(&ra(), counterexample).expect("replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
}

/// FR-227-AC-3 (TC-672), the read-modify-write branch of replay: a `release`
/// `add` on the flag publishes `data` to the `acquire` flag load, so the item
/// proves and a forged race is refused.
#[trace("TC-672", "FR-227-AC-3")]
#[test]
fn a_forged_race_through_a_release_read_modify_write_is_refused() {
    let extra = "pre AddY using v on M::Cell::add { self.id = 1 and self.v < 3 }\n";
    let writer = format!(
        "sequence A {{ {}{} }}",
        put("Sx", "x", ""),
        attempt("Ay", "w", "Cell::add", "release", "AddY, AddOne"),
    );
    let case = unit_with("", extra, &writer, &reader("acquire", ""));
    let outcome = case.check(&ra(), ModelCheckItem::RaceFreedom, None);
    assert!(holds(&outcome), "{outcome:?}");
    let path = [
        Hop::Fork,
        Hop::At("Sx", None),
        Hop::Node("Ay"),
        Hop::At("Ly", Some(0)),
        Hop::At("Lx", Some(0)),
    ];
    let forged = forged_race(&case, &path, location_of(&case, "x"), 2, 5);
    disagreement(&case, forged);
}

/// FR-227-AC-3 (TC-672), the nested `fork` and `join` branches of replay: a
/// branch stores `data` in an inner `parallel` and joins it, beside a branch
/// that loads `data` unsynchronised. The item refutes, and the race replays
/// over a prefix that forks and joins the inner `parallel`.
#[trace("TC-672", "FR-227-AC-3")]
#[test]
fn a_race_replays_through_a_nested_fork_and_join() {
    let writer = format!(
        "sequence A {{ parallel Inner {{ branch i sequence I {{ {} }} branch j sequence J {{ }} }} \
         join all [i, j]; }}",
        put("Sx", "x", ""),
    );
    let reader = steps("B", &[load_ordered("Lx", "x", 2, "")]);
    let case = unit_with("", "", &writer, &reader);
    let counterexample = violated(case.check(&ra(), ModelCheckItem::RaceFreedom, None));
    let kinds: Vec<StepKind> = counterexample
        .prefix
        .iter()
        .map(|step| step.transition.kind())
        .collect();
    assert!(kinds.iter().filter(|kind| **kind == StepKind::Fork).count() >= 2);
    let replayed = case
        .replay_race(&ra(), counterexample)
        .expect("the race replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
}

/// The channel `ch` whose delivery is unknown, so `duplicate` and `lose`
/// steps exist.
const LOSSY: &str = "channel ch from w to w carries Boolean ordering unordered \
                     delivery unknown capacity 2 overflow block;";

/// `data := 1` before or after the `send`, beside a branch that receives
/// and loads `data`.
fn channel_case(store_before_send: bool) -> Case {
    let send = "send Tx via ch as (tx: Boolean) { tx };";
    let store = put("Sx", "x", "");
    let sender = if store_before_send {
        format!("sequence Sender {{ {store} {send} }}")
    } else {
        format!("sequence Sender {{ {send} {store} }}")
    };
    let receiver = format!(
        "sequence Recv {{ receive Rx via ch of s::Sender::Tx as (rx: Boolean) {{ true }}; {} }}",
        load_ordered("Lx", "x", 2, ""),
    );
    let run = format!(
        "sequence Main {{ parallel Both {{ branch s {sender} branch r {receiver} }} \
         join all [s, r]; }}"
    );
    Case::with_letters(
        &(contracts() + &protocol_with("SB", LOSSY, &run) + NO_BOTH_ZERO),
        &MP,
    )
}

/// FR-227-AC-3 (TC-672), the `send` and `receive` branches of replay, and the
/// receive-side join of the race summary: `data` stored before the `send`
/// happens before the load after the `receive`, so the item proves; a race
/// forged on `data` is refused, also over a path through a `duplicate` of the
/// queued entry on the lossy channel.
#[trace("TC-672", "FR-227-AC-3")]
#[test]
fn a_forged_race_through_a_lossy_channel_is_refused() {
    let case = channel_case(true);
    let outcome = case.check(&ra(), ModelCheckItem::RaceFreedom, None);
    assert!(holds(&outcome), "{outcome:?}");
    let location = location_of(&case, "x");
    let direct = [
        Hop::Fork,
        Hop::At("Sx", None),
        Hop::Node("Tx"),
        Hop::Node("Rx"),
        Hop::At("Lx", Some(0)),
    ];
    disagreement(&case, forged_race(&case, &direct, location.clone(), 2, 5));
    let duplicated = [
        Hop::Fork,
        Hop::At("Sx", None),
        Hop::Node("Tx"),
        Hop::Kind(StepKind::Duplicate),
        Hop::Node("Rx"),
        Hop::At("Lx", Some(0)),
    ];
    disagreement(&case, forged_race(&case, &duplicated, location, 2, 6));
}

/// The queued-send carrier (`carry_after`, `carried_after`): `data` stored
/// after the `send` is not ordered before the load after the `receive`, so
/// the item refutes with a race that replays. The queued entry's view must
/// not learn of the later store. It asserts no acceptance criterion: FR-224-AC-5's
/// channel variant, which would back it, is deferred to QSL-995.
#[test]
fn a_store_after_the_send_races_with_the_receivers_load() {
    let case = channel_case(false);
    let counterexample = violated(case.check(&ra(), ModelCheckItem::RaceFreedom, None));
    assert_eq!(
        counterexample.kind,
        CounterexampleKind::Race(Race {
            location: location_of(&case, "x"),
            earlier: position_of(&counterexample, "Sx"),
            later: position_of(&counterexample, "Lx"),
        })
    );
    let replayed = case.replay_race(&ra(), counterexample).expect("replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
}

/// The race-freedom item names a protocol subject: EN-1 refuses it over a
/// model subject, and so do the SMT facade and the replay of a race
/// counterexample, all with `invalid-value`. It asserts no acceptance
/// criterion; it backs a refusal no AC names.
#[test]
fn the_race_freedom_item_over_a_model_subject_is_refused() {
    use crate::counter::{
        package_with_clauses, subject as model_subject, Documents, CONTRACT, UNLIMITED,
    };
    let documents = Documents::new(crate::counter::document(false));
    let (package, _) = package_with_clauses(&documents.document, CONTRACT, &[]);
    let mut documents = documents;
    let start = documents.start("s0", &[("c", 0)]);
    let second = start.clone();
    let subject = model_subject(&package, &documents, vec![start], &["c"], UNLIMITED);
    let refusal = qsl_analyze::model_check::check_model(
        qsl_analyze::model_check::ModelCheckRequest {
            subject: model_subject(&package, &documents, vec![second], &["c"], UNLIMITED),
            item: ModelCheckItem::RaceFreedom,
            max_depth: None,
            limits: ModelCheckLimits::default(),
            reductions: Default::default(),
        },
        &Cancel::new(),
    )
    .expect_err("refused");
    assert!(
        matches!(refusal, ModelCheckRefusal::RaceOverModel),
        "{refusal:?}"
    );

    let refused = qsl_replay::encode_smt_query(
        &subject,
        ModelCheckItem::RaceFreedom,
        qsl_replay::SmtQueryKind::InductionBase { depth: 1 },
        &qsl_replay::SmtEncodingLimits::default(),
    )
    .expect_err("refused");
    assert_eq!(refused.code(), Some("invalid-value"));
}

/// FR-224 with the closure checker: a certificate that holds every reachable
/// state of the relaxed-flag subject under the race-freedom key is rejected
/// `BadState` at the post-state of the step that completes the race. The
/// certificate is the deadlock-freedom run's states with the key an honest
/// race-freedom proof carries. It asserts no acceptance criterion; FR-224's
/// ACs name no closure-certificate rule.
#[test]
fn a_closure_certificate_over_a_racy_edge_is_rejected_bad_state() {
    let relaxed = unit(&message_passing("", "relaxed", "relaxed"), &MP);
    let ordered = unit(&message_passing("", "release", "acquire"), &MP);
    let certificate = |outcome: ModelCheckOutcome<ProtocolTransition>| match outcome {
        ModelCheckOutcome::Holds {
            certificate: ProofCertificate::Closure(certificate),
            ..
        } => certificate,
        other => panic!("{other:?}"),
    };
    let honest = certificate(ordered.check(&ra(), ModelCheckItem::RaceFreedom, None));
    let mut forged = certificate(relaxed.check(&ra(), ModelCheckItem::DeadlockFreedom, None));
    let key = honest.states[0].automaton_state.clone();
    for state in &mut forged.states {
        state.automaton_state = key.clone();
    }
    let checked = check_protocol_closure(
        &ProtocolCertificateRequest {
            subject: relaxed.subject(&ra()),
            item: ModelCheckItem::RaceFreedom,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
        },
        &forged,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the request is checked");
    let TemporalCertificateCheck::Rejected(rejection) = checked else {
        panic!("{checked:?}");
    };
    assert_eq!(rejection.rule, CertificateRule::BadState);
}

/// A sender that stores `data` then sends the same payload, twice in a
/// `repeat`, beside a receiver that receives once and loads `data`, over a
/// channel of capacity `capacity` with `overflow`. The second
/// send is after the second store; the first is not.
fn repeat_case(ordering: &str, capacity: u32, overflow: &str, store_first: bool) -> Case {
    let store = put("Sx", "x", "");
    let send = "send Tx via ch as (tx: Boolean) { tx };";
    let body = if store_first {
        format!("{store} {send}")
    } else {
        format!("{send} {store}")
    };
    let sender = format!(
        "sequence Sender {{ repeat Loop by w visible () max 2 while {{ true }} \
         sequence Body {{ {body} }} exhausted sequence Out {{ }} }}"
    );
    let receiver = format!(
        "sequence Recv {{ receive Rx via ch of s::Sender::Loop::Body::Tx as (rx: Boolean) \
         {{ true }}; {} }}",
        load_ordered("Lx", "x", 2, ""),
    );
    let run = format!(
        "sequence Main {{ parallel Both {{ branch s {sender} branch r {receiver} }} \
         join all [s, r]; }}"
    );
    let channel = format!(
        "channel ch from w to w carries Boolean ordering {ordering} delivery unknown \
         capacity {capacity} overflow {overflow};"
    );
    Case::with_letters(
        &(contracts() + &protocol_with("SB", &channel, &run) + NO_BOTH_ZERO),
        &MP,
    )
}

/// QSL-932 gap analysis FND-005: a `repeat` sends one payload twice, the
/// second entry displacing the first (`overflow loss head`, capacity 1), and
/// the receiver takes the second, which carries both stores. A race forged
/// on the second store is refused; replay used to match the receive to the
/// displaced first entry and accept it.
#[trace("TC-672", "FR-227-AC-3")]
#[test]
fn a_forged_race_after_a_displaced_send_is_refused() {
    let case = repeat_case("fifo by (m: Boolean) { true }", 1, "loss head", true);
    let path = [
        Hop::Fork,
        Hop::Node("Sx"),
        Hop::Node("Tx"),
        Hop::Node("Sx"),
        Hop::Node("Tx"),
        Hop::Node("Rx"),
        Hop::At("Lx", Some(0)),
    ];
    let forged = forged_race(&case, &path, location_of(&case, "x"), 4, 7);
    disagreement(&case, forged);
}

/// A `duplicate` keeps the original queued and adds one copy after the equal
/// entries of an `unordered` queue. Taking the first of the equal entries
/// after it is the older send, which carries only the first store, so the
/// second store races with the load and replay reproduces the race.
#[trace("TC-672", "FR-227-AC-3")]
#[test]
fn a_duplicate_leaves_the_original_in_place_and_adds_one_copy_after_it() {
    let case = repeat_case("unordered", 3, "block", true);
    let path = [
        Hop::Fork,
        Hop::Node("Sx"),
        Hop::Node("Tx"),
        Hop::Node("Sx"),
        Hop::Node("Tx"),
        Hop::Kind(StepKind::Duplicate),
        Hop::Node("Rx"),
        Hop::At("Lx", Some(0)),
    ];
    let race = forged_race(&case, &path, location_of(&case, "x"), 4, 8);
    let replayed = case.replay_race(&ra(), race).expect("replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
}

/// Entries of equal bytes (same send, thread and payload) are one choice of
/// the engine, which takes the first. A `repeat` sends one payload before and
/// after `data := 1` over a lossy channel; the earlier entry does not carry
/// the store, so the item refutes with a race, and the race replays. It asserts
/// no acceptance criterion; it guards the single-receiver case of FND-012.
#[test]
fn equal_queue_entries_with_different_views_still_expose_the_race() {
    let case = repeat_case("unordered", 2, "block", false);
    let counterexample = violated(case.check(&ra(), ModelCheckItem::RaceFreedom, None));
    assert!(matches!(counterexample.kind, CounterexampleKind::Race(_)));
    let replayed = case.replay_race(&ra(), counterexample).expect("replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
}

/// FND-012 of QSL-932: queued entries of equal bytes but different carried
/// views are different choices. `s` sends one payload twice from a `repeat`
/// and, after each send, receives an ack that `w` sends after storing `data`,
/// so its first entry does not carry the store and its second does. `r1`
/// receives, then sets a `relaxed` flag; `r2` receives only after reading
/// that flag as 1, which orders nothing, and is the only reader of `data`.
/// The engine must also give `r1` the later entry and `r2` the earlier one,
/// where `r2` races with the store; the item refutes with a race that
/// replays. It asserts no acceptance criterion.
#[test]
fn a_second_receiver_of_an_older_equal_entry_races_with_the_store() {
    let sender = "sequence S { repeat Loop by w visible () max 2 while { true } \
         sequence Body { send Tx via ch as (tx: Boolean) { tx }; \
         receive Ak via ch2 of w::W::Acks::Ack as (ak: Boolean) { true }; } \
         exhausted sequence Out { } }";
    let writer = format!(
        "sequence W {{ {} repeat Acks by w visible () max 2 while {{ true }} \
         send Ack via ch2 as (ack: Boolean) {{ ack }}; exhausted sequence OutW {{ }} }}",
        put("Sx", "x", ""),
    );
    let first = format!(
        "sequence R1 {{ receive Ra via ch of s::S::Loop::Body::Tx as (ra: Boolean) \
         {{ true }}; {} }}",
        put("Sy", "y", "relaxed"),
    );
    let second = format!(
        "sequence R2 {{ {} choice Pick2 by c visible (bLy) {{ case yes when {{ bLy }} \
         sequence Y {{ receive Rb via ch of s::S::Loop::Body::Tx as (rb: Boolean) \
         {{ true }}; {} }} case no when {{ not bLy }} sequence Skip2 {{ }} }} }}",
        attempt("Ly", "c", "Core::look", "relaxed", "LooksY, Looked"),
        load_ordered("Lx", "x", 2, ""),
    );
    let run = format!(
        "sequence Main {{ parallel Both {{ branch w {writer} branch s {sender} \
         branch r1 {first} branch r2 {second} }} join all [w, s, r1, r2]; }}"
    );
    let channels = "channel ch from w to w carries Boolean ordering unordered \
                    delivery exactly-once-premise capacity 2 overflow block; \
                    channel ch2 from w to w carries Boolean ordering unordered \
                    delivery exactly-once-premise capacity 2 overflow block;";
    let case = Case::with_letters(
        &(contracts() + &protocol_with("SB", channels, &run) + NO_BOTH_ZERO),
        &MP,
    );
    let counterexample = violated(case.check(&ra(), ModelCheckItem::RaceFreedom, None));
    assert!(matches!(counterexample.kind, CounterexampleKind::Race(_)));
    let replayed = case.replay_race(&ra(), counterexample).expect("replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
}

/// FND-012 of QSL-932, under `tso`: queued entries of equal bytes but different carried
/// views are different choices. `s` sends one payload twice from a `repeat`
/// and, after each send, receives an ack that `w` sends after storing `data`,
/// so its first entry does not carry the store and its second does. `r1`
/// receives, then sets a `relaxed` flag; `r2` receives only after reading
/// that flag as 1, which orders nothing, and is the only reader of `data`.
/// The engine must also give `r1` the later entry and `r2` the earlier one,
/// where `r2` races with the store; the item refutes with a race that
/// replays. It asserts no acceptance criterion.
#[test]
fn a_second_receiver_of_an_older_equal_entry_races_with_the_store_under_tso() {
    let sender = "sequence S { repeat Loop by w visible () max 2 while { true } \
         sequence Body { send Tx via ch as (tx: Boolean) { tx }; \
         receive Ak via ch2 of w::W::Acks::Ack as (ak: Boolean) { true }; } \
         exhausted sequence Out { } }";
    let writer = format!(
        "sequence W {{ {} repeat Acks by w visible () max 2 while {{ true }} \
         send Ack via ch2 as (ack: Boolean) {{ ack }}; exhausted sequence OutW {{ }} }}",
        put("Sx", "x", ""),
    );
    let first = format!(
        "sequence R1 {{ receive Ra via ch of s::S::Loop::Body::Tx as (ra: Boolean) \
         {{ true }}; {} }}",
        put("Sy", "y", "relaxed"),
    );
    let second = format!(
        "sequence R2 {{ {} choice Pick2 by c visible (bLy) {{ case yes when {{ bLy }} \
         sequence Y {{ receive Rb via ch of s::S::Loop::Body::Tx as (rb: Boolean) \
         {{ true }}; {} }} case no when {{ not bLy }} sequence Skip2 {{ }} }} }}",
        attempt("Ly", "c", "Core::look", "relaxed", "LooksY, Looked"),
        load_ordered("Lx", "x", 2, ""),
    );
    let run = format!(
        "sequence Main {{ parallel Both {{ branch w {writer} branch s {sender} \
         branch r1 {first} branch r2 {second} }} join all [w, s, r1, r2]; }}"
    );
    let channels = "channel ch from w to w carries Boolean ordering unordered \
                    delivery exactly-once-premise capacity 2 overflow block; \
                    channel ch2 from w to w carries Boolean ordering unordered \
                    delivery exactly-once-premise capacity 2 overflow block;";
    let case = Case::with_letters(
        &(contracts() + &protocol_with("SB", channels, &run) + NO_BOTH_ZERO),
        &MP,
    );
    let counterexample = violated(case.check(&tso(), ModelCheckItem::RaceFreedom, None));
    assert!(matches!(counterexample.kind, CounterexampleKind::Race(_)));
    let replayed = case.replay_race(&tso(), counterexample).expect("replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
}

/// FR-224-AC-2 (TC-669), the `tso` half: with a `relaxed` flag the item
/// settles `refuted` under `tso` with kind `Race{location: data, earlier,
/// later}`, `earlier` the position of `data := 1` and `later` that of
/// `r2 := data`, and the counterexample replays to `reproduced`.
#[trace("TC-669", "FR-224-AC-2")]
#[test]
fn a_relaxed_flag_refutes_race_freedom_under_tso_and_the_race_replays() {
    let case = unit(&message_passing("", "relaxed", "relaxed"), &MP);
    assert_eq!(race_freedom_item_count(&case, &tso()), 1);
    let outcome = case.check(&tso(), ModelCheckItem::RaceFreedom, None);
    let counterexample = violated(outcome.clone());
    assert_eq!(
        counterexample.kind,
        CounterexampleKind::Race(Race {
            location: location_of_under(&case, "x", &tso()),
            earlier: position_of(&counterexample, "Sx"),
            later: position_of(&counterexample, "Lx"),
        })
    );
    let record = case.settle_race(&tso(), &outcome);
    assert_eq!(record.label, ResultLabel::Refuted, "{record:?}");
    let replayed = case
        .replay_race(&tso(), counterexample)
        .expect("the race replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
}

/// FR-227 "For `kind: Race`" under `tso` (no acceptance criterion states it
/// for `tso`): a race forged on the `release`/`acquire` shape, whose `data` load runs after a flag load that read the flushed
/// `release` store, settles `inconclusive`, `ReplayParity`: the store buffer
/// and `flush` give the load a happens-before the forged race contradicts.
#[test]
fn a_forged_race_settles_inconclusive_under_tso() {
    let case = unit(&message_passing("", "release", "acquire"), &MP);
    let path = [
        Hop::Fork,
        Hop::At("Sx", None),
        Hop::At("Sy", None),
        Hop::Kind(StepKind::Memory),
        Hop::Kind(StepKind::Memory),
        Hop::At("Ly", None),
        Hop::At("Lx", None),
    ];
    let location = location_of_under(&case, "x", &tso());
    let forged = forged_race_under(&case, &tso(), Tso::default(), &path, location, 2, 7);
    disagreement_under(&case, &tso(), forged);
}

/// Walk `case`'s `tso` system along `path` and return the JSON of the state
/// key after each hop.
fn tso_keys_after(case: &Case, path: &[Hop]) -> Vec<Json> {
    let system = case.system(&tso(), Tso::default()).expect("builds");
    let mut state = system.initial().remove(0);
    let mut keys = Vec::new();
    for hop in path {
        let (_, next) = system
            .successors(&state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| match hop {
                Hop::Fork => transition.kind() == StepKind::Fork,
                Hop::Kind(kind) => transition.kind() == *kind,
                Hop::Node(node) | Hop::At(node, _) => is_at(transition, node),
            })
            .unwrap_or_else(|| panic!("a step for the hop of the path is enabled"));
        let (key, _) = system.state_key(&next).expect("a key");
        keys.push(serde_json::from_slice::<Json>(&key).expect("JSON"));
        state = next;
    }
    keys
}

/// The identity of the object a location of the unit names: its key's
/// characters in hexadecimal.
fn identity(name: &str) -> String {
    name.bytes().map(|byte| format!("{byte:02x}")).collect()
}

/// The value of the entry of the map member `member` whose key has `kind`
/// and which `pick` accepts.
fn entry_of(member: &Json, kind: &str, pick: impl Fn(&Json) -> bool) -> Json {
    member["entries"]
        .as_array()
        .expect("a map")
        .iter()
        .find(|entry| entry["key"]["type"] == json!(kind) && pick(&entry["key"]))
        .unwrap_or_else(|| panic!("an entry of {kind}"))["value"]
        .clone()
}

/// The race entry of `kind` for the location `name`.
fn race_entry(member: &Json, kind: &str, name: &str) -> Json {
    entry_of(member, kind, |key| {
        key["location"]["object"]["identity"] == json!(identity(name))
    })
}

/// The thread entry of `kind` of the branch `branch`.
fn thread_entry(member: &Json, kind: &str, branch: &str) -> Json {
    entry_of(member, kind, |key| {
        key["thread"]["steps"][0]["branch"] == json!(branch)
    })
}

/// The write Boolean a race map holds for the location `name`.
fn write_of(map: &Json, name: &str) -> bool {
    map["entries"]
        .as_array()
        .expect("a map")
        .iter()
        .find(|entry| entry["key"]["object"]["identity"] == json!(identity(name)))
        .unwrap_or_else(|| panic!("a map entry for {name}"))["value"]["write"]
        .as_bool()
        .expect("a Boolean")
}

/// Every queue entry in `key` that carries a view, with the member naming it.
fn carried(key: &Json, found: &mut Vec<(String, Json)>) {
    match key {
        Json::Object(members) => {
            if members.contains_key("entry") {
                for member in ["race", "view"] {
                    if let Some(value) = members.get(member) {
                        found.push((member.to_owned(), value.clone()));
                    }
                }
            }
            members.values().for_each(|value| carried(value, found));
        }
        Json::Array(values) => values.iter().for_each(|value| carried(value, found)),
        _ => {}
    }
}

/// FR-224-AC-6 (TC-669): in the message-passing shape under `tso`, the state
/// after the writer's `data := 1` holds `data`'s race-summary entry, the
/// writer's `race-summaries` entry mapping `data` to `"write":true` in `cur`
/// and `acq` and `"write":false` in `rel` for `flag`, and the buffered `data`
/// element's `race` member mapping it to `"write":false`. The buffered `flag`
/// element, and after `flush` the `race-memory` entry for `flag`, map `data` to
/// `"write":true` with a `release` store and `"write":false` with a `relaxed`
/// one. The item settles `proved` with the `release` store and `acquire` load
/// and with the flag passed by a `send` and `receive`, whose queue entry holds
/// the writer's `cur` map under the member `race`, and `refuted` with the
/// `relaxed` flag.
#[trace("TC-669", "FR-224-AC-6")]
#[test]
fn the_tso_race_summary_holds_the_typed_values_of_the_message_passing_shape() {
    let path = [
        Hop::Fork,
        Hop::Node("Sx"),
        Hop::Node("Sy"),
        Hop::Kind(StepKind::Memory),
        Hop::Kind(StepKind::Memory),
    ];
    for (flag, carries) in [("release", true), ("relaxed", false)] {
        let case = unit(&message_passing("", flag, "acquire"), &MP);
        let keys = tso_keys_after(&case, &path);
        let stored = &keys[1]["memory"];
        let summary = race_entry(stored, "race-summary", "x");
        assert_eq!(summary["last_write"]["atomic"], json!(false));
        assert_eq!(
            summary["last_write"]["thread"]["steps"][0]["branch"],
            json!("a")
        );
        assert_eq!(summary["last_reads"]["entries"], json!([]));
        let writer = thread_entry(stored, "race-summaries", "a");
        assert!(write_of(&writer["cur"], "x"), "{flag}");
        assert!(write_of(&writer["acq"], "x"), "{flag}");
        let rel = &writer["rel"]["entries"]
            .as_array()
            .expect("a map")
            .iter()
            .find(|entry| entry["key"]["object"]["identity"] == json!(identity("y")))
            .expect("the release summary for flag")["value"];
        assert!(!write_of(rel, "x"), "{flag}");
        let buffer = thread_entry(stored, "store-buffer", "a");
        assert!(!write_of(&buffer["elements"][0]["race"], "x"), "{flag}");
        let flagged = thread_entry(&keys[2]["memory"], "store-buffer", "a");
        assert_eq!(
            write_of(&flagged["elements"][1]["race"], "x"),
            carries,
            "{flag}"
        );
        let memory = race_entry(&keys[4]["memory"], "race-memory", "y");
        assert_eq!(write_of(&memory, "x"), carries, "{flag}");
    }

    let case = unit(&message_passing("", "release", "acquire"), &MP);
    let outcome = case.check(&tso(), ModelCheckItem::RaceFreedom, None);
    assert!(holds(&outcome), "{outcome:?}");
    assert_eq!(
        case.settle_race(&tso(), &outcome).label,
        ResultLabel::Proved
    );

    let sent = channel_case(true);
    let outcome = sent.check(&tso(), ModelCheckItem::RaceFreedom, None);
    assert!(holds(&outcome), "{outcome:?}");
    assert_eq!(
        sent.settle_race(&tso(), &outcome).label,
        ResultLabel::Proved
    );
    let keys = tso_keys_after(
        &sent,
        &[
            Hop::Fork,
            Hop::Node("Sx"),
            Hop::Kind(StepKind::Memory),
            Hop::Node("Tx"),
        ],
    );
    let mut entries = Vec::new();
    carried(&keys[3], &mut entries);
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0].0, "race");
    assert!(write_of(&entries[0].1, "x"));

    let relaxed = unit(&message_passing("", "relaxed", "relaxed"), &MP);
    let outcome = relaxed.check(&tso(), ModelCheckItem::RaceFreedom, None);
    assert_eq!(
        relaxed.settle_race(&tso(), &outcome).label,
        ResultLabel::Refuted
    );
}

/// FR-224-AC-7: the RMW is not enabled initially; it becomes enabled only
/// after the other branch stores `x := 1`.
fn delayed_rmw_case(ordering: &str) -> Case {
    let extra = "pre RmwAfterStore using v on M::Cell::add { self.id = 0 and self.v = 1 }\n";
    let writer = format!(
        "sequence A {{ {}{} }}",
        load_ordered("Lx", "x", 2, ""),
        put("Sx", "x", "release"),
    );
    let rmw = attempt("Rmw", "w", "Cell::add", ordering, "RmwAfterStore, AddOne");
    let reader = format!("sequence B {{ {rmw} }}");
    unit_with("", extra, &writer, &reader)
}

/// FR-224-AC-7 (TC-669): the RMW and its shared location count when enabled
/// in a reachable state, even though the initial state disables it.
#[trace("TC-669", "FR-224-AC-7")]
#[test]
fn a_later_enabled_rmw_creates_the_race_freedom_item_under_ra_and_tso() {
    for selection in [&ra()[..], &tso()[..]] {
        let acquire = delayed_rmw_case("acquire");
        let reachable = if selection[0].model == MemoryModelName::Ra {
            has_reachable_non_atomic_access(&acquire, selection, Ra::default())
        } else {
            has_reachable_non_atomic_access(&acquire, selection, Tso::default())
        };
        assert!(reachable, "{selection:?}");
        assert_eq!(
            race_freedom_item_count_for_access(&acquire, selection, reachable),
            1
        );
        let outcome = acquire.check(selection, ModelCheckItem::RaceFreedom, None);
        assert!(holds(&outcome), "{selection:?}: {outcome:?}");

        let relaxed = delayed_rmw_case("relaxed");
        let reachable = if selection[0].model == MemoryModelName::Ra {
            has_reachable_non_atomic_access(&relaxed, selection, Ra::default())
        } else {
            has_reachable_non_atomic_access(&relaxed, selection, Tso::default())
        };
        assert!(reachable, "{selection:?}");
        assert_eq!(
            race_freedom_item_count_for_access(&relaxed, selection, reachable),
            1
        );
        let outcome = relaxed.check(selection, ModelCheckItem::RaceFreedom, None);
        let counterexample = violated(outcome.clone());
        let CounterexampleKind::Race(race) = &counterexample.kind else {
            panic!("the delayed RMW counterexample is a race: {counterexample:?}");
        };
        assert!(
            matches!(
                &race.location,
                Location::Field { object, field }
                    if object.object().as_str() == "x" && field.node.ends_with("/Cell/v")
            ),
            "{selection:?}: {:?}",
            race.location
        );
        assert_eq!(
            race.earlier,
            position_of(&counterexample, "Lx"),
            "{selection:?}"
        );
        assert_eq!(
            race.later,
            position_of(&counterexample, "Rmw"),
            "{selection:?}"
        );
        assert_eq!(
            relaxed.settle_race(selection, &outcome).label,
            ResultLabel::Refuted,
            "{selection:?}"
        );
    }
}

/// FR-224-AC-3 (TC-669), the `tso` half: a subject whose shared accesses are
/// all atomic carries no `RaceFreedom` item under `tso`, nor does the AC-1
/// shape under `sc`; two items over one subject give one item, whose identity
/// differs between the `tso` and `ra` requests. The AC-1 shape's clause has
/// no node, so its item is settled `unsupported` with no request item and
/// is counted there; the identities are compared over two plain stores whose
/// clause has one.
#[trace("TC-669", "FR-224-AC-3")]
#[test]
fn the_race_freedom_item_is_derived_only_for_a_weak_subject_with_a_non_atomic_access() {
    let atomic = unit(&message_passing("relaxed", "release", "acquire"), &MP);
    assert_eq!(race_freedom_item_count(&atomic, &tso()), 0);
    let plain = unit(&message_passing("", "release", "acquire"), &MP);
    let sc = [select("SB::Main::Both", MemoryModelName::Sc)];
    assert_eq!(race_freedom_item_count(&plain, &sc), 0);
    assert_eq!(race_freedom_item_count(&plain, &tso()), 1);
    let emitted = two_plain_stores();
    assert_ne!(
        race_freedom_obligation(&emitted, &tso()),
        race_freedom_obligation(&emitted, &ra())
    );
}

/// FR-227 "For `kind: Race`" under `tso`, over the fence and the locked
/// paths of replay (no acceptance criterion states them for `tso`): in
/// fence-based message passing the fences order the accesses, and in a
/// `release` read-modify-write on the flag the locked access does, so the item
/// proves and a race forged on `data` is refused.
#[test]
fn a_forged_race_through_tso_fences_and_a_locked_access_is_refused() {
    let writer = steps(
        "A",
        &[
            put("Sx", "x", ""),
            "fence F1 ordering release;".to_owned(),
            put("Sy", "y", "relaxed"),
        ],
    );
    let case = unit_with(
        "",
        "",
        &writer,
        &reader("relaxed", "fence F2 ordering acquire;"),
    );
    let outcome = case.check(&tso(), ModelCheckItem::RaceFreedom, None);
    assert!(holds(&outcome), "{outcome:?}");
    let path = [
        Hop::Fork,
        Hop::Node("Sx"),
        Hop::Node("F1"),
        Hop::Node("Sy"),
        Hop::Kind(StepKind::Memory),
        Hop::Kind(StepKind::Memory),
        Hop::Node("Ly"),
        Hop::Node("F2"),
        Hop::Node("Lx"),
    ];
    let location = location_of_under(&case, "x", &tso());
    let forged = forged_race_under(&case, &tso(), Tso::default(), &path, location, 2, 9);
    disagreement_under(&case, &tso(), forged);

    let extra = "pre AddY using v on M::Cell::add { self.id = 1 and self.v < 3 }\n";
    let writer = format!(
        "sequence A {{ {}{} }}",
        put("Sx", "x", ""),
        attempt("Ay", "w", "Cell::add", "release", "AddY, AddOne"),
    );
    let locked = unit_with("", extra, &writer, &reader("acquire", ""));
    let outcome = locked.check(&tso(), ModelCheckItem::RaceFreedom, None);
    assert!(holds(&outcome), "{outcome:?}");
    let path = [
        Hop::Fork,
        Hop::Node("Sx"),
        Hop::Kind(StepKind::Memory),
        Hop::Node("Ay"),
        Hop::Node("Ly"),
        Hop::Node("Lx"),
    ];
    let location = location_of_under(&locked, "x", &tso());
    let forged = forged_race_under(&locked, &tso(), Tso::default(), &path, location, 2, 6);
    disagreement_under(&locked, &tso(), forged);
}

/// FR-227 "For `kind: Race`" under `tso`, over replay's `send` and `receive`
/// (no acceptance criterion states it for `tso`): `data` stored and flushed
/// before the `send` happens before the load after the `receive`, so a race
/// forged on it is refused; stored after the `send` it races, and the race
/// replays.
#[test]
fn a_forged_race_through_a_tso_channel_is_refused_and_a_real_one_replays() {
    let case = channel_case(true);
    let path = [
        Hop::Fork,
        Hop::Node("Sx"),
        Hop::Kind(StepKind::Memory),
        Hop::Node("Tx"),
        Hop::Node("Rx"),
        Hop::Node("Lx"),
    ];
    let location = location_of_under(&case, "x", &tso());
    let forged = forged_race_under(&case, &tso(), Tso::default(), &path, location, 2, 6);
    disagreement_under(&case, &tso(), forged);

    let racy = channel_case(false);
    let counterexample = violated(racy.check(&tso(), ModelCheckItem::RaceFreedom, None));
    assert!(matches!(counterexample.kind, CounterexampleKind::Race(_)));
    let replayed = racy.replay_race(&tso(), counterexample).expect("replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
}

/// QSL-995 review FND-001, under `tso` (no acceptance criterion states it):
/// `tso` has no counterpart of the global fence view `S` (ADR-025 DR-3), so
/// `seq_cst` fences order nothing between threads. In `a: x := 1; fence
/// seq_cst; y := 1` against `b: fence seq_cst; r1 := y; r2 := x` the item
/// refutes and the race replays.
#[test]
fn seq_cst_fences_do_not_order_threads_under_tso() {
    let writer = steps(
        "A",
        &[
            put("Sx", "x", ""),
            "fence F1 ordering seq_cst;".to_owned(),
            put("Sy", "y", "relaxed"),
        ],
    );
    let reader = format!(
        "sequence B {{ fence F2 ordering seq_cst; {} choice Pick by c visible (bLy) {{ \
         case yes when {{ bLy }} {} case no when {{ not bLy }} sequence Skip {{ }} }} }}",
        attempt("Ly", "c", "Core::look", "relaxed", "LooksY, Looked"),
        load_ordered("Lx", "x", 2, ""),
    );
    let case = unit_with("", "", &writer, &reader);
    let counterexample = violated(case.check(&tso(), ModelCheckItem::RaceFreedom, None));
    assert!(matches!(counterexample.kind, CounterexampleKind::Race(_)));
    let replayed = case.replay_race(&tso(), counterexample).expect("replays");
    assert!(
        matches!(replayed.settlement, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
}

/// QSpec FR-368-AC-14 and AC-17 with FR-429 and FR-438, the producer half:
/// over two threads that each store `x` with no `ordering` under `ra`, in a
/// unit selecting the protocol profile, a request whose
/// caller items are a temporal item over the protocol subject `SB`, one over
/// the model subject of `SB`'s own model, and a second over `SB`, holds those
/// three first in that order. The derived items follow, grouped by the
/// subject's first occurrence: `SB`'s `deadlock-freedom` and then its
/// `race-freedom` item, and then the model subject's `deadlock-freedom`
/// item, since a protocol subject and a model subject over one model are two
/// subjects. `SB`'s two derived items name its protocol clause node's
/// `declaration` occurrence at ordinal 0, which the emitted package's source
/// map holds, not the `model_root` the model subject's item names. Both are
/// unbounded `temporal-satisfaction` items routed as `ReachableInvariant`
/// with the untimed candidates EN-1 and EN-6, and the race-freedom item's
/// identity holds `SB`'s resolved models.
#[trace("QSpec-TC-431", "QSpec-FR-368-AC-14")]
#[trace("QSpec-TC-302", "QSpec-FR-368-AC-17")]
#[trace("QSpec-TC-392", "QSpec-FR-438-AC-2")]
#[test]
fn a_protocol_subjects_derived_items_follow_the_callers_at_its_clause_node() {
    let case = two_plain_stores();
    assert!(!case.accesses(&ra()).non_atomic().is_empty());
    let subject = case.subject(&ra());
    let models = models_of(&subject);
    let graph = case.checked().graph();
    let protocol = graph.protocol("SB").expect("SB");
    let clause = node_occurrence(&graph.temporal_clauses()[0]);
    let other = other_claim();
    let protocol_item = |occurrence| ProtocolTemporalItem {
        occurrence,
        subject: &subject.protocol,
        protocol,
        models: &models,
        non_atomic_access: true,
    };
    let model_subject = "SB's model".to_owned();
    let selections: BTreeSet<&str> = graph
        .model_roots()
        .map(|(selection, _)| selection.identity.as_str())
        .collect();
    let record = Requirements::new(Capability::TemporalSatisfaction, ClaimExtent::Bounded);
    let mut request = ModelRequest::new();
    request.protocol(&protocol_item(&clause), &record);
    request.authored(
        &ModelTemporalItem {
            occurrence: &clause,
            subject: &model_subject,
            package: graph,
        },
        &record,
        &selections,
    );
    request.protocol(&protocol_item(&other), &record);
    let written = request.derived(&registry(), None, &DerivedLimits::default());

    let kinds: Vec<_> = written.items().iter().map(RequestItem::item_kind).collect();
    assert_eq!(
        kinds,
        [
            Some(ItemKind::Formula),
            Some(ItemKind::Formula),
            Some(ItemKind::Formula),
            Some(ItemKind::DeadlockFreedom),
            Some(ItemKind::RaceFreedom),
            Some(ItemKind::DeadlockFreedom),
        ]
    );
    let subjects: Vec<_> = written
        .items()
        .iter()
        .map(|item| {
            written
                .obligation(item.index())
                .expect("its obligation")
                .subject
                .clone()
        })
        .collect();
    let sb = subject.protocol.clone();
    assert_eq!(
        subjects,
        [
            sb.clone(),
            model_subject.clone(),
            sb.clone(),
            sb.clone(),
            sb,
            model_subject
        ]
    );
    for (position, item) in written.items().iter().enumerate() {
        assert_eq!(item.index().get(), position);
    }

    let node = protocol.node().expect("SB's clause has a node");
    let anchor = OccurrenceKey::declaration(WireNodeId::from_digest(*node.as_bytes()));
    let emission = qsl_package::emit_checked(case.checked()).expect("the package emits");
    assert!(emission.source_map().regions(&anchor).is_some());
    for item in &written.items()[3..5] {
        assert_eq!(item.occurrence(), &anchor);
        assert_eq!(item.kind(), Capability::TemporalSatisfaction);
        assert_eq!(item.property_form(), Some(PropertyForm::ReachableInvariant));
        assert_eq!(
            ids(item.candidates().expect("a derived item's candidates")),
            [en1::BACKEND_ID, en6::BACKEND_ID]
        );
    }
    assert_ne!(written.items()[5].occurrence(), &anchor);
    assert_eq!(
        written
            .obligation(written.items()[4].index())
            .map(|obligation| &obligation.kind),
        Some(&ObligationKind::RaceFreedom { models })
    );
    assert!(written.unanchored().is_empty());
}

/// QSpec FR-425 with FR-438: the resolved models are part of the protocol
/// subject, so two caller items over one subject `SB`, one under `ra` and one
/// under `tso`, give one race-freedom item for each set of models, in the
/// order each first occurs after `SB`'s one deadlock-freedom item; neither
/// set's item is dropped.
#[trace("QSpec-TC-392", "QSpec-FR-438-AC-2")]
#[test]
fn each_set_of_resolved_models_of_one_subject_gets_its_race_freedom_item() {
    let case = two_plain_stores();
    let under_ra = case.subject(&ra());
    let under_tso = case.subject(&tso());
    let (ra_models, tso_models) = (models_of(&under_ra), models_of(&under_tso));
    assert_ne!(ra_models, tso_models);
    let graph = case.checked().graph();
    let protocol = graph.protocol("SB").expect("SB");
    let clause = node_occurrence(&graph.temporal_clauses()[0]);
    let other = other_claim();
    let item = |occurrence, models| ProtocolTemporalItem {
        occurrence,
        subject: &under_ra.protocol,
        protocol,
        models,
        non_atomic_access: true,
    };
    let written = protocol_request(&[item(&clause, &ra_models), item(&other, &tso_models)]);
    let derived: Vec<_> = written.items()[2..]
        .iter()
        .map(|item| {
            let obligation = written.obligation(item.index()).expect("its obligation");
            (item.item_kind(), obligation.kind.clone())
        })
        .collect();
    assert_eq!(
        derived,
        [
            (
                Some(ItemKind::DeadlockFreedom),
                ObligationKind::DeadlockFreedom
            ),
            (
                Some(ItemKind::RaceFreedom),
                ObligationKind::RaceFreedom { models: ra_models }
            ),
            (
                Some(ItemKind::RaceFreedom),
                ObligationKind::RaceFreedom { models: tso_models }
            ),
        ]
    );
    assert!(written.unanchored().is_empty());
}

/// QSpec FR-368-AC-22's settlement for a protocol subject whose clause has
/// no node (its unit selects no protocol profile): its deadlock-freedom and
/// race-freedom items are written as no request item and settled
/// `unsupported` with `unsupported_projection`/
/// `unsupported-requested-capability`, cause `NoProtocolNode`, each detail
/// naming the item kind and the missing node. QSpec FR-368 states no anchor
/// for such a clause; this is QSL's reading, by FR-331's unemitted package.
#[trace("QSpec-TC-431")]
#[test]
fn a_protocol_subject_whose_clause_has_no_node_gets_no_derived_request_item() {
    let case = unit(&message_passing("", "release", "acquire"), &MP);
    let subject = case.subject(&ra());
    let models = models_of(&subject);
    let graph = case.checked().graph();
    let protocol = graph.protocol("SB").expect("SB");
    assert_eq!(protocol.node(), None);
    let clause = node_occurrence(&graph.temporal_clauses()[0]);
    let written = protocol_request(&[ProtocolTemporalItem {
        occurrence: &clause,
        subject: &subject.protocol,
        protocol,
        models: &models,
        non_atomic_access: true,
    }]);
    let kinds: Vec<_> = written.items().iter().map(RequestItem::item_kind).collect();
    assert_eq!(kinds, [Some(ItemKind::Formula)]);
    let unanchored: Vec<_> = written
        .unanchored()
        .iter()
        .map(|item| {
            (
                item.subject.as_str(),
                item.kind,
                item.cause,
                item.catalog_code(),
                item.detail(),
            )
        })
        .collect();
    let code = CatalogCode::new("unsupported_projection", "unsupported-requested-capability");
    assert_eq!(
        unanchored,
        [
            (
                "SB",
                ItemKind::DeadlockFreedom,
                NoAnchor::NoProtocolNode,
                code,
                "the deadlock-freedom item's protocol subject's clause has no node, \
                 so it has no anchor"
                    .to_owned()
            ),
            (
                "SB",
                ItemKind::RaceFreedom,
                NoAnchor::NoProtocolNode,
                code,
                "the race-freedom item's protocol subject's clause has no node, \
                 so it has no anchor"
                    .to_owned()
            ),
        ]
    );
}
