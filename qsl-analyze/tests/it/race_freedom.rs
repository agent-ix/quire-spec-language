// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-224 and FR-227 (TC-669, TC-672): the `ra` race summary, the derived
//! race-freedom item it decides and the replay of its counterexample, over
//! ADR-025 §10's message-passing shape: the writer stores the plain field
//! `data` then the `flag`, and the reader loads `flag` and loads `data` only
//! in the case of a `choice` where the flag load read 1.

use ix_trace_rs::trace;
use qsl_analyze::model_check::ModelCheckRefusal;

use qsl_eval::model_subject::ModelLetters;
use qsl_eval::simulation::protocol::{
    MemoryModel, MemorySelection, ProtocolTransition, Ra, StepKind, Tso,
};
use qsl_eval::simulation::{Location, SubjectSystem, TransitionSystem};
use qsl_foundation::MemoryModelName;
use qsl_replay::{
    check_protocol_closure, CertificateRule, CounterexampleKind, DisagreementCause, ModelCheckItem,
    ModelCheckLimits, ModelCheckOutcome, ModelStep, ModelTraceSettlement, ProofCertificate,
    ProtocolCertificateRequest, Race, ResultLabel, TemporalCertificateCheck,
    TemporalCounterexample,
};
use qsl_route::deadlock::{race_freedom_items, ProtocolMemoryItem};
use quire_exact::Cancel;
use quire_semantic_value::catalog::Category;

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

/// FR-224-AC-1 (TC-669), the `tso` half: ADR-025 §10's message-passing shape
/// with `data` non-atomic and a `release` flag store and `acquire` flag load
/// carries one `RaceFreedom` item under `tso`, which settles `proved`.
#[trace("TC-669", "FR-224-AC-1")]
#[test]
fn a_release_acquire_flag_proves_race_freedom_under_tso() {
    let case = unit(&message_passing("", "release", "acquire"), &MP);
    assert_eq!(race_freedom_item_count(&case, &tso()), 1);
    let outcome = case.check(&tso(), ModelCheckItem::RaceFreedom, None);
    assert!(holds(&outcome), "{outcome:?}");
    let record = case.settle_race(&tso(), &outcome);
    assert_eq!(record.label, ResultLabel::Proved, "{record:?}");
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

/// FR-224-AC-4 (TC-669), the `tso` half: a race changes no authored claim's
/// verdict. `tso` forbids ADR-025 §10's message-passing outcome, so the claim
/// `always holds(not (r1 = 1 and r2 = 0))` holds with `data` non-atomic and
/// with `data` atomic and `relaxed`, the same verdict.
#[trace("TC-669", "FR-224-AC-4")]
#[test]
fn a_race_changes_no_authored_claims_verdict_under_tso() {
    for data in ["", "relaxed"] {
        let case = unit(&message_passing(data, "relaxed", "relaxed"), &MP);
        let outcome = case.claim(&tso(), None);
        assert!(holds(&outcome), "data {data:?}: {outcome:?}");
    }
}

/// FR-227-AC-3 (TC-672) for `tso`: a race forged on the `release`/`acquire`
/// shape, whose `data` load runs after a flag load that read the flushed
/// `release` store, settles `inconclusive`, `ReplayParity`: the store buffer
/// and `flush` give the load a happens-before the forged race contradicts.
#[trace("TC-672", "FR-227-AC-3")]
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
