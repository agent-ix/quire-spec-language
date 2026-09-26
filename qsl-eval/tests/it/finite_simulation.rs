// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-101: finite exploration and seeded sampling through the public
//! entries, `explore_request` and `sample_request`. Traces to TC-453,
//! TC-454 and TC-455.
//!
//! `EdgeGraph` is a toy `TransitionSystem` over `String`-labelled states: its
//! key is the label itself, and its transitions are an explicit, authored
//! edge list the graph filters by source label. The engine orders both
//! initial states and each parent's successors itself (FR-101-AC-1,
//! FR-101-AC-9); nothing here is pre-sorted.

use std::cell::Cell;

use ix_trace_rs::trace;
use qsl_eval::simulation::{
    explore_request, replay, sample_request, Limit, Limits, NotSimulated, Outcome, ReplayError,
    StopReason, Trace, TransitionSystem,
};
use qsl_foundation::diagnostic::{LimitExceeded, LimitKind};
use qsl_foundation::digest::{DigestDomain, DigestRecord, WireNodeId};
use qsl_foundation::selection::{DefinitionDigest, DefinitionRef};
use qsl_foundation::{ByteDigest, CatalogCode, CatalogCoded, InternalFault};
use qsl_semantics::family::{ClassifyFailure, DomainKind};
use qsl_semantics::value::declaration::TypeEnvironment;
use quire_exact::{Integer, IntegerInterval, ValueType};
use serde::Serialize;

/// A `{"type":"transition","operation":"<qualified-name>",
/// "arguments":[<encoded>, ...]}` transition identity (QSpec FR-181's typed
/// canonical form).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct Transition {
    #[serde(rename = "type")]
    kind: &'static str,
    operation: String,
    arguments: Vec<Argument>,
}

/// One typed argument value; only the `integer` form this file's fixtures
/// need.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
enum Argument {
    #[serde(rename = "integer")]
    Integer { value: String },
}

/// A transition named `name` with no arguments.
fn op(name: &str) -> Transition {
    Transition {
        kind: "transition",
        operation: name.to_owned(),
        arguments: vec![],
    }
}

/// A transition named `name` applied to one decimal integer argument.
fn op_int(name: &str, value: i64) -> Transition {
    Transition {
        kind: "transition",
        operation: name.to_owned(),
        arguments: vec![Argument::Integer {
            value: value.to_string(),
        }],
    }
}

/// The `quire.simulation.state-key/v1` digest a `String`-keyed `EdgeGraph`
/// state computes: the same RFC 8785 JCS + SHA-256 the engine itself runs
/// (FR-101), duplicated here as the fixed spelling test fixtures assert
/// against, exactly as the previous toy `Graph::key` duplicated its own
/// encoding.
fn key(label: &str) -> DigestRecord {
    let limits = quire_canonical::Limits::new(u64::MAX, quire_canonical::Limits::MAX_DEPTH)
        .expect("MAX_DEPTH is within MAX_DEPTH");
    let digest = quire_canonical::sha256(&label.to_owned(), limits).expect("a string encodes");
    DigestRecord::mint(DigestDomain::SimulationStateKeyV1, *digest.as_bytes())
}

/// An authored, unsorted edge list over `String`-labelled states: the
/// engine's own job is to order both `initial()` and each state's
/// `successors()` (FR-101), so this fixture preserves no order of its own.
struct EdgeGraph {
    initial: Vec<String>,
    edges: Vec<(String, Transition, String)>,
}

impl EdgeGraph {
    /// Build from short-lived label references, owning a copy of each.
    fn new(initial: Vec<&str>, edges: Vec<(&str, Transition, &str)>) -> Self {
        Self {
            initial: initial.into_iter().map(str::to_owned).collect(),
            edges: edges
                .into_iter()
                .map(|(from, transition, to)| (from.to_owned(), transition, to.to_owned()))
                .collect(),
        }
    }

    /// Build from already-owned labels, for a fixture that formats them.
    fn from_owned(initial: Vec<String>, edges: Vec<(String, Transition, String)>) -> Self {
        Self { initial, edges }
    }
}

impl TransitionSystem for EdgeGraph {
    type State = String;
    type TransitionId = Transition;
    type Key = String;

    fn initial(&self) -> Vec<String> {
        self.initial.clone()
    }

    fn key(&self, state: &String) -> String {
        state.clone()
    }

    fn successors(&self, state: &String) -> Vec<(Transition, String)> {
        self.edges
            .iter()
            .filter(|(from, _, _)| from == state)
            .map(|(_, transition, to)| (transition.clone(), to.clone()))
            .collect()
    }
}

/// An FR-181 `simulation-state` envelope whose `semantic` member is a bare
/// `float64` and whose other five members are empty maps (FR-101-AC-2).
fn float64_envelope(bits: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "simulation-state",
        "semantic": {"type": "float64", "bits": bits},
        "control": {"type": "map", "entries": []},
        "queues": {"type": "map", "entries": []},
        "roles": {"type": "map", "entries": []},
        "observations": {"type": "map", "entries": []},
        "bounds": {"type": "map", "entries": []},
    })
}

/// A `TransitionSystem` over pre-built envelope states, addressed by index.
struct EnvelopeGraph {
    states: Vec<serde_json::Value>,
    edges: Vec<(usize, usize)>,
}

impl TransitionSystem for EnvelopeGraph {
    type State = usize;
    type TransitionId = Transition;
    type Key = serde_json::Value;

    fn initial(&self) -> Vec<usize> {
        vec![0]
    }

    fn key(&self, state: &usize) -> serde_json::Value {
        self.states[*state].clone()
    }

    fn successors(&self, state: &usize) -> Vec<(Transition, usize)> {
        self.edges
            .iter()
            .filter(|(from, _)| from == state)
            .map(|&(_, to)| (op("next"), to))
            .collect()
    }
}

/// A `TransitionSystem` whose one initial state has no successors, and
/// which counts every `TransitionSystem` method call it receives
/// (TC-454-step-8, TC-455-step-4).
struct RecordingSystem {
    calls: Cell<u32>,
}

impl RecordingSystem {
    fn new() -> Self {
        Self {
            calls: Cell::new(0),
        }
    }

    fn calls(&self) -> u32 {
        self.calls.get()
    }
}

impl TransitionSystem for RecordingSystem {
    type State = ();
    type TransitionId = Transition;
    type Key = ();

    fn initial(&self) -> Vec<()> {
        self.calls.set(self.calls.get() + 1);
        vec![()]
    }

    fn key(&self, _state: &()) {
        self.calls.set(self.calls.get() + 1);
    }

    fn successors(&self, _state: &()) -> Vec<(Transition, ())> {
        self.calls.set(self.calls.get() + 1);
        vec![]
    }
}

/// A `TransitionSystem` whose state key is a bare integer (FR-101-AC-11): a
/// `u64` above `2^53` has no exact `f64` representation, so `quire-
/// canonical` refuses to encode it as an RFC 8785 JCS number.
struct BigIntKeyGraph {
    key: u64,
}

impl TransitionSystem for BigIntKeyGraph {
    type State = ();
    type TransitionId = Transition;
    type Key = u64;

    fn initial(&self) -> Vec<()> {
        vec![()]
    }

    fn key(&self, _state: &()) -> u64 {
        self.key
    }

    fn successors(&self, _state: &()) -> Vec<(Transition, ())> {
        vec![]
    }
}

fn empty_types() -> TypeEnvironment {
    TypeEnvironment::new([], []).expect("an empty type environment admits")
}

fn generous_limits() -> Limits {
    Limits {
        max_states: usize::MAX,
        max_depth: usize::MAX,
        max_transitions: usize::MAX,
    }
}

fn never_cancels() -> bool {
    false
}

const NO_DOMAINS: &[(WireNodeId, &ValueType)] = &[];

/// The pinned `quire.simulation.sampler/v1` `1-draft.1` `DefinitionRef`
/// TC-454's vectors run under. QSL pins no raw-byte digest of the
/// definition file (FR-101); this digest is an arbitrary fixture value,
/// recorded as given.
fn sampler_ref() -> DefinitionRef {
    DefinitionRef::new(
        "quire.simulation.sampler/v1",
        "1-draft.1",
        DefinitionDigest::from_digest(ByteDigest::of(b"fixture-sampler-digest")),
    )
    .expect("a valid DefinitionRef")
}

// ---------------------------------------------------------------------------
// TC-453: canonical successor order, initial-state order and the state key.
// ---------------------------------------------------------------------------

/// TC-453 step 1, FR-101-AC-1: successors are visited in ascending
/// transition-identity byte order, not the system's own authored order: `z`
/// is authored before `a`, but `a` sorts first.
#[trace("TC-453", "FR-101-AC-1")]
#[test]
fn canonical_order_sorts_successors_by_transition_identity_bytes() {
    // `a`'s post-state key ("9") sorts greater than `z`'s ("3"): a frontier
    // of [key("9"), key("3")] can only come from ordering by transition
    // identity (a before z), never from sorting by post-state key (which
    // would put z's smaller key first).
    let system = EdgeGraph::new(vec!["0"], vec![("0", op("z"), "3"), ("0", op("a"), "9")]);
    let outcome = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_depth: 1,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        outcome,
        Outcome::Bounded {
            stats: qsl_eval::simulation::Stats {
                states: 3,
                transitions: 2,
                depth: 1,
            },
            frontier: vec![key("9"), key("3")],
            limit: Limit::Depth,
        }
    );
}

/// TC-453 step 2, FR-101-AC-1: an integer argument's decimal-string bytes
/// decide the order, not its numeric value: `step(10)`'s argument
/// `"value":"10"` sorts before `step(9)`'s `"value":"9"`.
#[trace("TC-453", "FR-101-AC-1")]
#[test]
fn canonical_order_sorts_integer_arguments_by_decimal_string_bytes() {
    let system = EdgeGraph::new(
        vec!["0"],
        vec![
            ("0", op_int("step", 9), "nine"),
            ("0", op_int("step", 10), "ten"),
        ],
    );
    let outcome = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_depth: 1,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    let Outcome::Bounded { frontier, .. } = outcome else {
        panic!("expected Bounded, got {outcome:?}");
    };
    assert_eq!(frontier, vec![key("ten"), key("nine")]);
}

/// TC-453 step 3, FR-101-AC-1: two successors with the same transition
/// identity are ordered by ascending post-state key bytes, even when the
/// system lists the larger key first.
#[trace("TC-453", "FR-101-AC-1")]
#[test]
fn canonical_order_breaks_ties_by_ascending_state_key_bytes() {
    let system = EdgeGraph::new(vec!["0"], vec![("0", op("x"), "b"), ("0", op("x"), "a")]);
    let outcome = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_depth: 1,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    let Outcome::Bounded { frontier, .. } = outcome else {
        panic!("expected Bounded, got {outcome:?}");
    };
    assert_eq!(frontier, vec![key("a"), key("b")]);
}

/// TC-453 step 4, FR-101-AC-9: several distinct initial states are all
/// admitted when the cap allows, and a duplicate coalesces into one state.
#[trace("TC-453", "FR-101-AC-9")]
#[test]
fn several_initial_states_are_all_admitted_when_the_cap_allows() {
    let system = EdgeGraph::new(vec!["0", "1", "2"], vec![]);
    let outcome = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        generous_limits(),
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        outcome,
        Outcome::Exhaustive(qsl_eval::simulation::Stats {
            states: 3,
            transitions: 0,
            depth: 0,
        })
    );
}

/// TC-453 step 4, FR-101-AC-9: a duplicate initial state coalesces with its
/// first occurrence.
#[trace("TC-453", "FR-101-AC-9")]
#[test]
fn duplicate_initial_states_coalesce_to_one_state() {
    let system = EdgeGraph::new(vec!["0", "0", "1"], vec![]);
    let outcome = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        generous_limits(),
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        outcome,
        Outcome::Exhaustive(qsl_eval::simulation::Stats {
            states: 2,
            transitions: 0,
            depth: 0,
        })
    );
}

/// TC-453 step 4, FR-101-AC-9, as written: three distinct initial states
/// listed in descending state-key order, with one repeated. `max_states` 3
/// admits all three distinct states; `max_depth` 0 then stops the run
/// before any of them expands, `Bounded` at `Limit::Depth` (not
/// `Limit::States`, which `max_states` 3 never reaches), with the frontier
/// in ascending key order regardless of the system's descending listing.
#[trace("TC-453", "FR-101-AC-9")]
#[test]
fn several_initial_states_in_descending_order_stop_bounded_at_depth_zero() {
    let system = EdgeGraph::new(vec!["c", "b", "a", "a"], vec![]);
    let outcome = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_states: 3,
            max_depth: 0,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        outcome,
        Outcome::Bounded {
            stats: qsl_eval::simulation::Stats {
                states: 3,
                transitions: 0,
                depth: 0,
            },
            frontier: vec![key("a"), key("b"), key("c")],
            limit: Limit::Depth,
        }
    );
}

/// TC-453 step 5: level-2 parents keep their FIFO discovery order, not a
/// re-sort by key -- even though `key(1) > key(2)` and `key(3) > key(4)`,
/// cancellation at the fourth expansion still stops after state 1's own
/// child (discovered through transition `a`, which sorts before `b`).
#[trace("TC-453", "FR-101-AC-1")]
#[test]
fn cancellation_frontier_keeps_fifo_order_not_key_order() {
    // 0 -a-> "d" -> "b"   (key("d") > key("c"), key("b") > key("a"))
    // 0 -b-> "c" -> "a"
    let system = EdgeGraph::new(
        vec!["0"],
        vec![
            ("0", op("a"), "d"),
            ("0", op("b"), "c"),
            ("d", op("x"), "b"),
            ("c", op("x"), "a"),
        ],
    );
    let mut polls = 0usize;
    let outcome = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        generous_limits(),
        || {
            polls += 1;
            polls == 4
        },
    )
    .expect("bounded domains explore");
    let Outcome::Cancelled {
        frontier, cause, ..
    } = outcome
    else {
        panic!("expected Cancelled, got {outcome:?}");
    };
    assert_eq!(frontier, vec![key("b"), key("a")]);
    assert_eq!(cause, CatalogCode::new("cancelled", "caller-cancelled"));
}

/// TC-453 step 6, FR-101-AC-2: the pinned digests of the signed-zero and
/// distinct-NaN-payload float64 states, and that the two of each pair
/// remain two states.
#[trace("TC-453", "FR-101-AC-2")]
#[test]
fn float64_state_keys_pin_their_digests_and_stay_distinct() {
    let vectors = [
        (
            "0000000000000000",
            "943ae638f84583f2a35a7a92f1eac7f58c380c045298892fb1412756a1cc95e6",
        ),
        (
            "8000000000000000",
            "92a3e9557f9aaf672a61aaab71eb2bfad13a0d88662953998de4057ed54e4b01",
        ),
        (
            "7ff8000000000000",
            "a3d5ecff68c7cfb60a687aa72b743a04e1dc8513e348b9c3f64393dd96f4bdec",
        ),
        (
            "7ff8000000000001",
            "62c344cca9a4942b80644ca8527bc7ccced905f01bf67262a9bd5e824356a955",
        ),
    ];
    for (bits, expected_hex) in vectors {
        let system = EnvelopeGraph {
            states: vec![float64_envelope(bits)],
            edges: vec![],
        };
        let outcome = explore_request(
            &system,
            NO_DOMAINS,
            &empty_types(),
            1000,
            Limits {
                max_depth: 0,
                ..generous_limits()
            },
            never_cancels,
        )
        .expect("bounded domains explore");
        let Outcome::Bounded { frontier, .. } = outcome else {
            panic!("expected Bounded, got {outcome:?}");
        };
        assert_eq!(frontier.len(), 1);
        assert_eq!(frontier[0].domain(), DigestDomain::SimulationStateKeyV1);
        assert_eq!(frontier[0].hex(), expected_hex);
    }

    for (a, b) in [
        ("0000000000000000", "8000000000000000"),
        ("7ff8000000000000", "7ff8000000000001"),
    ] {
        let system = EnvelopeGraph {
            states: vec![float64_envelope(a), float64_envelope(b)],
            edges: vec![(0, 1)],
        };
        let outcome = explore_request(
            &system,
            NO_DOMAINS,
            &empty_types(),
            1000,
            generous_limits(),
            never_cancels,
        )
        .expect("bounded domains explore");
        let Outcome::Exhaustive(stats) = outcome else {
            panic!("expected Exhaustive, got {outcome:?}");
        };
        assert_eq!(stats.states, 2, "{a} and {b} must remain two states");
    }
}

/// TC-453 step 9, FR-101-AC-11: a `TransitionSystem::Key` with no RFC 8785
/// encoding -- a `u64` above the exact-double range, `2^53` -- refuses
/// `NotSimulated::KeyEncoding` from `explore_request` and `sample_request`,
/// and `ReplayError::KeyEncoding` from `replay`, instead of aborting the
/// process. `TransitionSystem` is a public trait; an implementer can hand
/// the engine a value it cannot canonically encode.
#[trace("TC-453", "FR-101-AC-11")]
#[test]
fn a_key_with_no_rfc_8785_encoding_refuses_instead_of_panicking() {
    let system = BigIntKeyGraph {
        key: (1u64 << 53) + 1,
    };
    let explore_error = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        generous_limits(),
        never_cancels,
    )
    .expect_err("an unencodable key refuses");
    assert!(matches!(explore_error, NotSimulated::KeyEncoding(_)));

    let sample_error = sample_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        0,
        0,
        1,
    )
    .expect_err("an unencodable key refuses");
    assert!(matches!(sample_error, NotSimulated::KeyEncoding(_)));

    let trace: Trace<Transition> = Trace {
        initial: key("anything"),
        steps: vec![],
        provenance: None,
    };
    let replay_error = replay(&system, &trace).expect_err("an unencodable key refuses");
    assert!(matches!(replay_error, ReplayError::KeyEncoding(_)));
}

/// TC-453 step 7, FR-101-AC-2: two distinct paths that reach an equal state
/// key coalesce into one state.
#[trace("TC-453", "FR-101-AC-2")]
#[test]
fn key_equal_coalescing_merges_two_paths_to_the_same_state() {
    let system = EdgeGraph::new(
        vec!["0"],
        vec![
            ("0", op("left"), "1"),
            ("0", op("right"), "2"),
            ("1", op("merge"), "3"),
            ("2", op("merge"), "3"),
        ],
    );
    let outcome = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        generous_limits(),
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        outcome,
        Outcome::Exhaustive(qsl_eval::simulation::Stats {
            states: 4,
            transitions: 4,
            depth: 2,
        })
    );
}

/// TC-453 step 8, FR-101-AC-9: initial states are admitted in ascending
/// state-key order, whatever order the system lists them in; a `max_states`
/// cap during admission puts every initial state's digest in the frontier,
/// in canonical order.
#[trace("TC-453", "FR-101-AC-9")]
#[test]
fn state_limit_on_initial_states_admits_in_canonical_order() {
    let system = EdgeGraph::new(vec!["d", "b", "a", "c"], vec![]);
    let outcome = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_states: 2,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        outcome,
        Outcome::Bounded {
            stats: qsl_eval::simulation::Stats {
                states: 2,
                transitions: 0,
                depth: 0,
            },
            frontier: vec![key("a"), key("b"), key("c"), key("d")],
            limit: Limit::States,
        }
    );

    let zero_cap = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_states: 0,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        zero_cap,
        Outcome::Bounded {
            stats: qsl_eval::simulation::Stats {
                states: 0,
                transitions: 0,
                depth: 0,
            },
            frontier: vec![key("a"), key("b"), key("c"), key("d")],
            limit: Limit::States,
        }
    );

    let no_initial = EdgeGraph::new(vec![], vec![]);
    let empty = explore_request(
        &no_initial,
        NO_DOMAINS,
        &empty_types(),
        1000,
        generous_limits(),
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        empty,
        Outcome::Exhaustive(qsl_eval::simulation::Stats {
            states: 0,
            transitions: 0,
            depth: 0,
        })
    );
}

// ---------------------------------------------------------------------------
// TC-455: stopped explorations stay incomplete; requires-bound.
// ---------------------------------------------------------------------------

/// TC-455 step 1, FR-101-AC-6: a run the poll cancels returns `Cancelled`
/// with the caller-cancelled cause and the unexplored frontier, never
/// `Exhaustive` or `Bounded`.
#[trace("TC-455", "FR-101-AC-6")]
#[test]
fn cancellation_stops_the_run_and_returns_the_frontier() {
    let system = EdgeGraph::new(
        vec!["0"],
        vec![
            ("0", op("a"), "1"),
            ("0", op("b"), "2"),
            ("1", op("c"), "3"),
        ],
    );
    let mut polls = 0usize;
    let outcome = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        generous_limits(),
        || {
            polls += 1;
            polls == 2
        },
    )
    .expect("bounded domains explore");
    assert_eq!(
        outcome,
        Outcome::Cancelled {
            stats: qsl_eval::simulation::Stats {
                states: 3,
                transitions: 2,
                depth: 1,
            },
            frontier: vec![key("1"), key("2")],
            cause: CatalogCode::new("cancelled", "caller-cancelled"),
        }
    );
    assert_eq!(
        outcome.category(),
        qsl_foundation::diagnostic::Category::Incomplete
    );
}

/// TC-455 step 2, FR-101-AC-7: each smaller limit on the chain 0 -> 1 -> 2
/// returns `Bounded` with its `Limit`; each larger limit completes it
/// `Exhaustive` with the same counts.
#[trace("TC-455", "FR-101-AC-7")]
#[test]
fn depth_limit_n_stops_bounded_and_n_plus_one_is_exhaustive() {
    let system = EdgeGraph::new(vec!["0"], vec![("0", op("t1"), "1"), ("1", op("t2"), "2")]);

    let stopped = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_depth: 2,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        stopped,
        Outcome::Bounded {
            stats: qsl_eval::simulation::Stats {
                states: 3,
                transitions: 2,
                depth: 2,
            },
            frontier: vec![key("2")],
            limit: Limit::Depth,
        }
    );

    let completed = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_depth: 3,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        completed,
        Outcome::Exhaustive(qsl_eval::simulation::Stats {
            states: 3,
            transitions: 2,
            depth: 2,
        })
    );
}

/// TC-455 step 2, FR-101-AC-7: the same chain, bounded on distinct states.
#[trace("TC-455", "FR-101-AC-7")]
#[test]
fn state_limit_n_stops_bounded_and_n_plus_one_is_exhaustive() {
    let system = EdgeGraph::new(vec!["0"], vec![("0", op("t1"), "1"), ("1", op("t2"), "2")]);

    let stopped = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_states: 2,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        stopped,
        Outcome::Bounded {
            stats: qsl_eval::simulation::Stats {
                states: 2,
                transitions: 2,
                depth: 1,
            },
            frontier: vec![key("1"), key("2")],
            limit: Limit::States,
        }
    );

    let completed = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_states: 3,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        completed,
        Outcome::Exhaustive(qsl_eval::simulation::Stats {
            states: 3,
            transitions: 2,
            depth: 2,
        })
    );
}

/// TC-455 step 2, FR-101-AC-7: the same chain, bounded on explored
/// transitions.
#[trace("TC-455", "FR-101-AC-7")]
#[test]
fn transition_limit_n_stops_bounded_and_n_plus_one_is_exhaustive() {
    let system = EdgeGraph::new(vec!["0"], vec![("0", op("t1"), "1"), ("1", op("t2"), "2")]);

    let stopped = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_transitions: 1,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        stopped,
        Outcome::Bounded {
            stats: qsl_eval::simulation::Stats {
                states: 2,
                transitions: 1,
                depth: 1,
            },
            frontier: vec![key("1")],
            limit: Limit::Transitions,
        }
    );

    let completed = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_transitions: 2,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        completed,
        Outcome::Exhaustive(qsl_eval::simulation::Stats {
            states: 3,
            transitions: 2,
            depth: 2,
        })
    );
}

/// TC-455 step 3, FR-101-AC-7: when the state cap trips on a successor
/// while the queue is non-empty, the blocked key follows the queue.
#[trace("TC-455", "FR-101-AC-7")]
#[test]
fn state_limit_mid_successors_places_the_blocked_key_after_the_queue() {
    let system = EdgeGraph::new(
        vec!["0"],
        vec![
            ("0", op("a"), "1"),
            ("0", op("b"), "2"),
            ("1", op("c"), "3"),
        ],
    );
    let outcome = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_states: 3,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        outcome,
        Outcome::Bounded {
            stats: qsl_eval::simulation::Stats {
                states: 3,
                transitions: 3,
                depth: 1,
            },
            frontier: vec![key("1"), key("2"), key("3")],
            limit: Limit::States,
        }
    );
}

/// TC-455 step 4, FR-101-AC-8: a request over an unbounded `Integer` domain
/// refuses `RequiresBound`, calling no `TransitionSystem` method, under
/// either a tight or a generous `Limits`; a `position_limit` of 0 refuses
/// `Extent` with a node-count stage limit instead.
#[trace("TC-455", "FR-101-AC-8")]
#[test]
fn requires_bound_refuses_before_any_transition_system_call() {
    let node = WireNodeId::from_digest([7; 32]);
    let unbounded = ValueType::Integer;
    let domains = [(node, &unbounded)];
    let types = empty_types();

    // A tight `Limits` first, then a generous one: AC-8's "raising
    // exploration `Limits` does not change a `RequiresBound` result" is
    // only exercised if the two values actually differ.
    for limits in [
        Limits {
            max_states: 1,
            max_depth: 1,
            max_transitions: 1,
        },
        generous_limits(),
    ] {
        let system = RecordingSystem::new();
        let error = explore_request(&system, &domains, &types, 1000, limits, never_cancels)
            .expect_err("an unbounded Integer domain requires a bound");
        let NotSimulated::RequiresBound(requires_bound) = error else {
            panic!("expected RequiresBound, got {error:?}");
        };
        assert_eq!(requires_bound.domains.len(), 1);
        let (domain_key, domain_kind) = requires_bound
            .domains
            .iter()
            .next()
            .expect("one unbounded domain");
        assert_eq!(domain_key.node(), node);
        assert_eq!(domain_key.path(), &[] as &[u32]);
        assert_eq!(domain_kind, DomainKind::Integer);
        assert_eq!(system.calls(), 0, "no TransitionSystem method ran");

        let sample_system = RecordingSystem::new();
        let sample_error = sample_request(
            &sample_system,
            &domains,
            &types,
            1000,
            &sampler_ref(),
            0,
            0,
            10,
        )
        .expect_err("an unbounded Integer domain requires a bound");
        assert!(matches!(sample_error, NotSimulated::RequiresBound(_)));
        assert_eq!(sample_system.calls(), 0, "no TransitionSystem method ran");
    }

    let bounded = ValueType::Int(
        IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64))
            .expect("0..=9 is a valid interval"),
    );
    let bounded_domains = [(node, &bounded)];
    let system = RecordingSystem::new();
    let extent_error = explore_request(
        &system,
        &bounded_domains,
        &types,
        0,
        generous_limits(),
        never_cancels,
    )
    .expect_err("position_limit 0 stops at the first position");
    let NotSimulated::Extent(ClassifyFailure::Limit(exceeded)) = extent_error else {
        panic!("expected Extent(ClassifyFailure::Limit(_)), got {extent_error:?}");
    };
    assert_eq!(exceeded.kind(), LimitKind::NodeCount);
    assert_eq!(system.calls(), 0, "no TransitionSystem method ran");
}

/// TC-455 step 5, FR-101-AC-7: an exhaustive run over a small branching
/// graph reports the exact state and transition counts.
#[trace("TC-455", "FR-101-AC-7")]
#[test]
fn exhaustive_small_graph_reports_exact_state_and_transition_counts() {
    let system = EdgeGraph::new(
        vec!["0"],
        vec![
            ("0", op("a"), "1"),
            ("0", op("b"), "2"),
            ("1", op("c"), "3"),
            ("2", op("d"), "4"),
        ],
    );
    let outcome = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        generous_limits(),
        never_cancels,
    )
    .expect("bounded domains explore");
    assert_eq!(
        outcome,
        Outcome::Exhaustive(qsl_eval::simulation::Stats {
            states: 5,
            transitions: 4,
            depth: 2,
        })
    );
}

/// TC-455 step 6, FR-101-AC-8: a request over a bounded domain (`Int[0,9]`)
/// explores instead of refusing.
#[trace("TC-455", "FR-101-AC-8")]
#[test]
fn bounded_domain_request_explores() {
    let node = WireNodeId::from_digest([9; 32]);
    let bounded = ValueType::Int(
        IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64))
            .expect("0..=9 is a valid interval"),
    );
    let domains = [(node, &bounded)];
    let system = EdgeGraph::new(vec!["0"], vec![]);
    let outcome = explore_request(
        &system,
        &domains,
        &empty_types(),
        1000,
        generous_limits(),
        never_cancels,
    )
    .expect("a bounded domain explores");
    assert!(matches!(outcome, Outcome::Exhaustive(_)));
}

// ---------------------------------------------------------------------------
// TC-439 (unchanged): `Outcome::category()` is FR-097-AC-5's O-16 map.
// ---------------------------------------------------------------------------

/// TC-439 (ADR-014 §7, §10 scenario 4): an exhaustive run is O-16 success; a
/// run a bound stops or the caller cancels is incomplete, and keeps its
/// frontier and the bound that stopped it.
#[trace("TC-439", "FR-097-AC-5")]
#[test]
fn tc_439_explore_outcomes_map_to_their_o16_category() {
    use qsl_foundation::diagnostic::Category;
    let system = EdgeGraph::new(vec!["0"], vec![("0", op("a"), "1"), ("1", op("b"), "2")]);
    let exhaustive = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        generous_limits(),
        never_cancels,
    )
    .expect("bounded domains explore");
    assert!(matches!(exhaustive, Outcome::Exhaustive(_)));
    assert_eq!(exhaustive.category(), Category::Success);

    let bounded = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        Limits {
            max_depth: 1,
            ..generous_limits()
        },
        never_cancels,
    )
    .expect("bounded domains explore");
    let Outcome::Bounded {
        ref frontier,
        limit,
        ..
    } = bounded
    else {
        panic!("expected Bounded, got {bounded:?}");
    };
    assert_eq!(limit, Limit::Depth);
    assert_eq!(frontier, &vec![key("1")]);
    assert_eq!(bounded.category(), Category::Incomplete);

    let cancelled = explore_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        generous_limits(),
        || true,
    )
    .expect("bounded domains explore");
    let Outcome::Cancelled { ref frontier, .. } = cancelled else {
        panic!("expected Cancelled, got {cancelled:?}");
    };
    assert_eq!(frontier, &vec![key("0")]);
    assert_eq!(cancelled.category(), Category::Incomplete);
}

// ---------------------------------------------------------------------------
// TC-454: the pinned sampler, sampled-trace reproducibility, and replay.
// ---------------------------------------------------------------------------

/// A system where every state has five distinct-identity successors, used
/// by TC-454 step 1's vector.
fn five_way_chain() -> EdgeGraph {
    branching_chain(5)
}

/// A system where every state has three distinct-identity successors, used
/// by TC-454 step 2's third vector.
fn three_way_chain() -> EdgeGraph {
    branching_chain(3)
}

/// A chain of six levels, each state offering `branches` distinct-identity
/// successors `t0..t<branches>`, labelled by depth and branch so every state
/// is distinct.
fn branching_chain(branches: usize) -> EdgeGraph {
    let mut edges = Vec::new();
    let mut frontier = vec!["0".to_owned()];
    // Five levels: enough for every trace this file samples (`max_steps`
    // never exceeds 5), covering every path the sampler could choose.
    for _ in 0..5 {
        let mut next_frontier = Vec::new();
        for parent in &frontier {
            for branch in 0..branches {
                let child = format!("{parent}-{branch}");
                edges.push((parent.clone(), op(&format!("t{branch}")), child.clone()));
                next_frontier.push(child);
            }
        }
        frontier = next_frontier;
    }
    EdgeGraph::from_owned(vec!["0".to_owned()], edges)
}

/// The 0-based branch index a step's transition selected: `t3` at step 2
/// selects index 3.
fn selected_index(transition: &Transition) -> usize {
    transition
        .operation
        .trim_start_matches('t')
        .parse()
        .expect("fixture transitions are named t<index>")
}

/// TC-454 step 1, FR-101-AC-3: the pinned sampler reproduces QSpec TC-210's
/// vector, seed 424242 trace 0 over five-way branches: indices 0, 0, 4, 4,
/// 4.
#[trace("TC-454", "FR-101-AC-3")]
#[test]
fn pinned_sampler_reproduces_tc_210_five_way_vector() {
    let system = five_way_chain();
    let trace = sample_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        424_242,
        0,
        5,
    )
    .expect("a bounded, generator-matched sample");
    let indices: Vec<usize> = trace
        .steps
        .iter()
        .map(|step| selected_index(&step.transition))
        .collect();
    assert_eq!(indices, vec![0, 0, 4, 4, 4]);
}

/// TC-454 step 2, FR-101-AC-3: seed 424242 trace 1 over five-way branches
/// draws 1, 4, 3, 4, 0; seed 424242 trace 0 over three-way branches draws
/// 2, 1, 0, 2, 1.
#[trace("TC-454", "FR-101-AC-3")]
#[test]
fn pinned_sampler_reproduces_tc_210_further_vectors() {
    let five_way = five_way_chain();
    let trace_one = sample_request(
        &five_way,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        424_242,
        1,
        5,
    )
    .expect("a bounded, generator-matched sample");
    let indices: Vec<usize> = trace_one
        .steps
        .iter()
        .map(|step| selected_index(&step.transition))
        .collect();
    assert_eq!(indices, vec![1, 4, 3, 4, 0]);

    let three_way = three_way_chain();
    let trace_three = sample_request(
        &three_way,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        424_242,
        0,
        5,
    )
    .expect("a bounded, generator-matched sample");
    let indices: Vec<usize> = trace_three
        .steps
        .iter()
        .map(|step| selected_index(&step.transition))
        .collect();
    assert_eq!(indices, vec![2, 1, 0, 2, 1]);
}

/// TC-454 step 3, FR-101-AC-3, FR-101-AC-10: a one-successor chain always
/// selects index 0 with no digest computed; a system with `n = 1` at steps
/// 0 and 1 and `n = 5` at step 2 selects index 4 at step 2, the step
/// counter having advanced over the no-digest steps.
#[trace("TC-454", "FR-101-AC-3")]
#[test]
fn pinned_sampler_advances_the_step_counter_across_no_digest_steps() {
    let chain = EdgeGraph::new(
        vec!["0"],
        vec![
            ("0", op("t0"), "1"),
            ("1", op("t0"), "2"),
            ("2", op("t0"), "3"),
        ],
    );
    let trace = sample_request(
        &chain,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        424_242,
        0,
        3,
    )
    .expect("a bounded, generator-matched sample");
    for step in &trace.steps {
        assert_eq!(selected_index(&step.transition), 0);
    }

    let mut edges = vec![
        ("0".to_owned(), op("t0"), "1".to_owned()),
        ("1".to_owned(), op("t0"), "2".to_owned()),
    ];
    for branch in 0..5 {
        edges.push((
            "2".to_owned(),
            op(&format!("t{branch}")),
            format!("2-{branch}"),
        ));
    }
    let mixed = EdgeGraph::from_owned(vec!["0".to_owned()], edges);
    let mixed_trace = sample_request(
        &mixed,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        424_242,
        0,
        3,
    )
    .expect("a bounded, generator-matched sample");
    assert_eq!(selected_index(&mixed_trace.steps[2].transition), 4);
}

/// TC-454 step 4, FR-101-AC-3: a state with no successors ends the trace on
/// its very first step, with `StopReason::NoSuccessors` and no steps.
#[trace("TC-454", "FR-101-AC-3")]
#[test]
fn sample_on_a_state_with_no_successors_ends_immediately() {
    let system = EdgeGraph::new(vec!["0"], vec![]);
    let trace = sample_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        1,
        0,
        5,
    )
    .expect("a bounded, generator-matched sample");
    assert!(trace.steps.is_empty());
    assert_eq!(
        trace.provenance.expect("sampled trace").stopped,
        StopReason::NoSuccessors
    );
}

/// TC-454 step 5, FR-101-AC-4: two runs with equal seed and trace index
/// give equal traces and provenance; a different seed or trace index gives
/// different provenance.
#[trace("TC-454", "FR-101-AC-4")]
#[test]
fn seeded_sampling_reproduces_the_same_trace() {
    let system = EdgeGraph::new(
        vec!["0"],
        vec![
            ("0", op("a"), "1"),
            ("0", op("b"), "2"),
            ("1", op("c"), "3"),
            ("2", op("d"), "4"),
        ],
    );
    let first = sample_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        42,
        0,
        5,
    )
    .expect("a bounded, generator-matched sample");
    let second = sample_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        42,
        0,
        5,
    )
    .expect("a bounded, generator-matched sample");
    assert_eq!(first, second);
    let first_provenance = first.provenance.as_ref().expect("sampled trace");
    assert_eq!(first_provenance.seed, 42);
    assert_eq!(first_provenance.trace, 0);
    assert_eq!(first_provenance.sampler, sampler_ref());
    assert_eq!(replay(&system, &first), Ok(()));

    let different_seed = sample_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        43,
        0,
        5,
    )
    .expect("a bounded, generator-matched sample");
    assert_ne!(first.provenance, different_seed.provenance);

    let different_trace = sample_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        42,
        1,
        5,
    )
    .expect("a bounded, generator-matched sample");
    assert_ne!(first.provenance, different_trace.provenance);
}

/// TC-454 step 6, FR-101-AC-4: with `m = 3` distinct initial states, trace
/// `t` starts at canonical initial state `t mod m`, with no draw selecting
/// the start.
#[trace("TC-454", "FR-101-AC-4")]
#[test]
fn multi_initial_sample_starts_at_trace_index_mod_m() {
    let system = EdgeGraph::new(vec!["b", "a", "c", "a"], vec![]);
    let canonical = ["a", "b", "c"];
    for trace_index in 0u64..4 {
        let trace = sample_request(
            &system,
            NO_DOMAINS,
            &empty_types(),
            1000,
            &sampler_ref(),
            0,
            trace_index,
            5,
        )
        .expect("a bounded, generator-matched sample");
        let expected = canonical[(trace_index % 3) as usize];
        assert_eq!(trace.initial, key(expected), "trace {trace_index}");
    }
}

/// TC-454 step 7, FR-101-AC-5: an unaltered trace replays; a shared-identity
/// step replays by taking the successor whose digest matches; a non-first
/// initial state is found by its digest; and each of a tampered initial
/// digest, transition and step digest refuses at that exact step.
#[trace("TC-454", "FR-101-AC-5")]
#[test]
fn sample_then_replay_round_trips_and_refuses_tampered_traces() {
    let duplicate_ids = EdgeGraph::new(
        vec!["0"],
        vec![("0", op("recv"), "1"), ("0", op("recv"), "2")],
    );
    // Seed 1 draws index 1 (recomputed independently), the second-listed
    // `recv` edge to "2": this is what makes the test discriminate a
    // replay that only takes the first matching-identity successor from a
    // replay that matches by digest, per FR-101-AC-5.
    let sampled = sample_request(
        &duplicate_ids,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        1,
        0,
        1,
    )
    .expect("a bounded, generator-matched sample");
    assert_eq!(sampled.steps[0].key, key("2"));
    assert_eq!(replay(&duplicate_ids, &sampled), Ok(()));

    let multi_initial = EdgeGraph::new(vec!["a", "b"], vec![("b", op("from_b"), "z")]);
    let trace: Trace<Transition> = Trace {
        initial: key("b"),
        steps: vec![qsl_eval::simulation::Step {
            transition: op("from_b"),
            key: key("z"),
        }],
        provenance: None,
    };
    assert_eq!(replay(&multi_initial, &trace), Ok(()));

    let chain = EdgeGraph::new(vec!["0"], vec![("0", op("t1"), "1"), ("1", op("t2"), "2")]);
    let good: Trace<Transition> = Trace {
        initial: key("0"),
        steps: vec![
            qsl_eval::simulation::Step {
                transition: op("t1"),
                key: key("1"),
            },
            qsl_eval::simulation::Step {
                transition: op("t2"),
                key: key("2"),
            },
        ],
        provenance: None,
    };
    assert_eq!(replay(&chain, &good), Ok(()));

    let mut wrong_initial = good.clone();
    wrong_initial.initial = key("999");
    assert_eq!(
        replay(&chain, &wrong_initial),
        Err(ReplayError::UnknownInitial {
            expected: key("999")
        })
    );

    let mut wrong_transition = good.clone();
    wrong_transition.steps[0].transition = op("bogus");
    assert_eq!(
        replay(&chain, &wrong_transition),
        Err(ReplayError::MissingTransition {
            step: 0,
            transition: op("bogus"),
        })
    );

    let mut wrong_key = good.clone();
    wrong_key.steps[1].key = key("999");
    assert_eq!(
        replay(&chain, &wrong_key),
        Err(ReplayError::KeyMismatch {
            step: 1,
            transition: op("t2"),
            expected: key("999"),
            actual: key("2"),
        })
    );
}

/// TC-454 step 8, FR-101-AC-10: `sample_request` refuses a `DefinitionRef`
/// naming a different identity or a different version, before any
/// `TransitionSystem` method and any draw, and the refusal's catalog code
/// is `invalid_runtime_input`/`invalid-value` (FR-101 Behavior).
#[trace("TC-454", "FR-101-AC-10")]
#[test]
fn sample_request_refuses_a_generator_mismatch_before_any_call() {
    let wrong_identity = DefinitionRef::new(
        "quire.simulation.sampler/v2",
        "1-draft.1",
        DefinitionDigest::from_digest(ByteDigest::of(b"fixture")),
    )
    .expect("a valid DefinitionRef");
    let system = RecordingSystem::new();
    let error = sample_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &wrong_identity,
        0,
        0,
        5,
    )
    .expect_err("a different identity refuses");
    assert_eq!(
        error,
        NotSimulated::GeneratorMismatch {
            supplied: wrong_identity
        }
    );
    assert_eq!(
        error.catalog_code(),
        Some(CatalogCode::new("invalid_runtime_input", "invalid-value"))
    );
    assert_eq!(system.calls(), 0);

    let wrong_version = DefinitionRef::new(
        "quire.simulation.sampler/v1",
        "1-draft.2",
        DefinitionDigest::from_digest(ByteDigest::of(b"fixture")),
    )
    .expect("a valid DefinitionRef");
    let system = RecordingSystem::new();
    let error = sample_request(
        &system,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &wrong_version,
        0,
        0,
        5,
    )
    .expect_err("a different version refuses");
    assert_eq!(
        error,
        NotSimulated::GeneratorMismatch {
            supplied: wrong_version
        }
    );
    assert_eq!(
        error.catalog_code(),
        Some(CatalogCode::new("invalid_runtime_input", "invalid-value"))
    );
    assert_eq!(system.calls(), 0);
}

/// R1-FND-003 (PR #467 review): `NotSimulated::catalog_code` and
/// `catalog_fields` are total, and this asserts every variant literally,
/// including `Extent(ClassifyFailure::Limit(_))`'s delegation to
/// `LimitExceeded`'s own fields. `RequiresBound` has no catalog code: it is
/// a negotiation disposition, not a refusal (FR-101 Behavior).
#[test]
fn not_simulated_catalog_code_and_fields_cover_every_variant() {
    let node = WireNodeId::from_digest([9; 32]);
    let unbounded = ValueType::Integer;
    let requires_bound_domains = [(node, &unbounded)];
    let system = RecordingSystem::new();
    let requires_bound = explore_request(
        &system,
        &requires_bound_domains,
        &empty_types(),
        1000,
        generous_limits(),
        never_cancels,
    )
    .expect_err("an unbounded Integer domain requires a bound");
    assert!(matches!(requires_bound, NotSimulated::RequiresBound(_)));
    assert_eq!(requires_bound.catalog_code(), None);
    assert_eq!(requires_bound.catalog_fields(), None);

    let exceeded = LimitExceeded::new(LimitKind::NodeCount, 0, 1);
    let extent_limit = NotSimulated::Extent(ClassifyFailure::Limit(exceeded.clone()));
    assert_eq!(extent_limit.catalog_code(), Some(exceeded.catalog_code()));
    assert_eq!(extent_limit.catalog_fields(), exceeded.catalog_fields());
    assert_eq!(
        extent_limit.catalog_fields(),
        Some(std::collections::BTreeMap::from([
            ("kind", "node-count-exceeded".to_owned()),
            ("bound", "0".to_owned()),
            ("actual", "1".to_owned()),
        ]))
    );

    let fault = InternalFault::new("S3", "test-fixture-invariant");
    let extent_fault = NotSimulated::Extent(ClassifyFailure::Fault(fault));
    assert_eq!(extent_fault.catalog_code(), Some(fault.catalog_code()));
    assert_eq!(extent_fault.catalog_fields(), None);

    let generator_mismatch = NotSimulated::GeneratorMismatch {
        supplied: sampler_ref(),
    };
    assert_eq!(
        generator_mismatch.catalog_code(),
        Some(CatalogCode::new("invalid_runtime_input", "invalid-value"))
    );
    assert_eq!(generator_mismatch.catalog_fields(), None);

    assert_eq!(
        NotSimulated::EmptyInitial.catalog_code(),
        Some(CatalogCode::new("invalid_runtime_input", "invalid-value"))
    );
    assert_eq!(NotSimulated::EmptyInitial.catalog_fields(), None);

    let unencodable = BigIntKeyGraph {
        key: (1u64 << 53) + 1,
    };
    let key_encoding = explore_request(
        &unencodable,
        NO_DOMAINS,
        &empty_types(),
        1000,
        generous_limits(),
        never_cancels,
    )
    .expect_err("an unencodable key refuses");
    assert!(matches!(key_encoding, NotSimulated::KeyEncoding(_)));
    assert_eq!(
        key_encoding.catalog_code(),
        Some(CatalogCode::new("invalid_runtime_input", "invalid-value"))
    );
    assert_eq!(key_encoding.catalog_fields(), None);
}

/// TC-454 step 9, FR-101-AC-10: `sample_request` refuses `EmptyInitial` when
/// the system has no initial state; `max_steps` shorter than the available
/// path stops on the step limit, whether or not the current state has
/// further successors.
#[trace("TC-454", "FR-101-AC-10")]
#[test]
fn sample_request_reports_empty_initial_and_the_step_limit() {
    let empty = EdgeGraph::new(vec![], vec![]);
    let error = sample_request(
        &empty,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        0,
        0,
        3,
    )
    .expect_err("no initial state to sample from");
    assert_eq!(error, NotSimulated::EmptyInitial);

    let chain = EdgeGraph::new(vec!["0"], vec![("0", op("t1"), "1"), ("1", op("t2"), "2")]);
    let trace = sample_request(
        &chain,
        NO_DOMAINS,
        &empty_types(),
        1000,
        &sampler_ref(),
        0,
        0,
        1,
    )
    .expect("a bounded, generator-matched sample");
    assert_eq!(trace.steps.len(), 1);
    assert_eq!(
        trace.provenance.expect("sampled trace").stopped,
        StopReason::StepLimit
    );
}
