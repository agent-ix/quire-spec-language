---
id: FR-101
title: "Explore finite models with canonical order, typed state keys and the pinned sampler"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-201
    type: depends_on
---
# FR-101: Explore finite models with canonical order, typed state keys and the pinned sampler

## Description

QSpec FR-181 is the normative source for finite simulation. This requirement
states how `qsl_eval::simulation` implements it. It adds no rule to FR-181;
where FR-181 leaves a choice open, this requirement names QSL's choice and
says so.

QSL implements these parts of FR-181:

- the exploration contract's canonical successor order;
- the simulator state key, its typed canonical form and its digest;
- seeded sampling with the pinned `quire.simulation.sampler/v1` generator
  (QSpec `proposals/quire-v1/definitions/simulation-sampler.md`, revision
  `1-draft.1`);
- trace replay;
- the stopped outcomes: exhaustion of an exploration limit, and caller
  cancellation with cause `cancelled`/`caller-cancelled`;
- the `requires-bound` disposition returned before exploration begins.

This requirement does not implement the model-level successor relation
(operation × finite argument domains × frame post-states, QSpec
FR-013-AC-1), the recording of an invariant-violating successor
(FR-181-AC-4's last clause) or the refusal of evaluator effects
(FR-181-AC-6); QSL-274 owns those. A `TransitionSystem` implementation
supplies the successor relation; the engine orders, keys, explores and
samples it.

## Inputs

- A `TransitionSystem`: its initial states and, per state, its successors,
  each a transition identity and a post-state, both in the typed canonical
  form of QSpec FR-181's exploration contract.
- The simulation request's parameter-domain and population types, as
  `domains: &[(WireNodeId, &ValueType)]`: each type with the wire id of the
  checked node that carries it.
- The package's `TypeEnvironment` (`qsl_semantics`) and the extent walk's
  `position_limit: u64`, both passed to `classify_extent`.
- For exploration: `Limits{max_states, max_depth, max_transitions}` and a
  cancellation poll `impl FnMut() -> bool`.
- For sampling: a `u64` seed, a `u64` trace index, a `usize` step ceiling
  and the sampler's `DefinitionRef` (`qsl_foundation::selection`) from the
  ecosystem lock's `definitions`.

## Outputs

- `Outcome::Exhaustive(Stats)`, `Outcome::Bounded{stats, frontier, limit}`
  or `Outcome::Cancelled{stats, frontier, cause}`.
- A sampled `Trace` with its `SampleProvenance{seed, trace, sampler, stopped}`.
- `NotSimulated`, returned before any state is explored or sampled.
- A replay result: success, or the first step that does not replay.

## Behavior

**Entry points and owners.** Every type below is owned by `qsl-eval`
(`qsl_eval::simulation`) unless another owner is named. `qsl-eval` reaches
only the crates ADR-011 X-8 and TC-390 list; it gains no dependency on
`qsl-replay` or `qsl-cst`.

```rust
pub fn explore_request<S: TransitionSystem>(
    system: &S,
    domains: &[(WireNodeId, &ValueType)],
    types: &TypeEnvironment,
    position_limit: u64,
    limits: Limits,
    poll: impl FnMut() -> bool,
) -> Result<Outcome, NotSimulated>;

pub fn sample_request<S: TransitionSystem>(
    system: &S,
    domains: &[(WireNodeId, &ValueType)],
    types: &TypeEnvironment,
    position_limit: u64,
    sampler: &DefinitionRef,
    seed: u64,
    trace: u64,
    max_steps: usize,
) -> Result<Trace<S::TransitionId>, NotSimulated>;

pub enum NotSimulated {
    /// The request's extent is unbounded (ADR-014 §4).
    RequiresBound(RequiresBound),
    /// `classify_extent` stopped: a node-count stage limit or an
    /// internal fault (`qsl_semantics::family::ClassifyFailure`).
    Extent(ClassifyFailure),
    /// `sampler` names a generator other than the one that runs.
    GeneratorMismatch { supplied: DefinitionRef },
    /// `system.initial()` returned no state to sample from.
    EmptyInitial,
    /// A state or transition identity reached during the walk has no RFC
    /// 8785 encoding under `quire-canonical`.
    KeyEncoding(EncodingRefusal),
}

pub struct RequiresBound {
    /// Every unbounded domain, by key, with its kind
    /// (`qsl_semantics::family::UnboundedDomains`).
    pub domains: UnboundedDomains,
}

/// A `TransitionSystem::Key` or `TransitionId` has no RFC 8785 encoding: for
/// example an integer outside the exact-double range (`2^53`), a
/// non-finite float, a non-string map key, or nesting past
/// `quire_canonical::Limits::MAX_DEPTH`.
pub struct EncodingRefusal(String);
```

`explore_request` and `sample_request` are the only public entries that
explore or sample: `explore`, `sample` and the `Sampler` trait are
`pub(crate)`, so no caller skips the requires-bound check or the
`GeneratorMismatch` check. `replay` stays public; it explores nothing new.
Integration tests in `qsl-eval/tests/it/` call the two entries with a
bounded or empty `domains` set; a check that needs draw internals is an
in-crate unit test.

`explore_request` never returns `GeneratorMismatch` or `EmptyInitial`. Any
entry, including `replay` (`ReplayError::KeyEncoding`), can refuse a state or
transition identity reached mid-walk that has no RFC 8785 encoding: a
`TransitionSystem` is a public trait, and its `Key` and `TransitionId` are
implementer-supplied, not engine-built, so an encoding failure is a caller
defect the engine refuses rather than a broken internal invariant.

`RequiresBound` is not a refusal: ADR-014 §4 and QSpec FR-181 treat
requires-bound as a negotiation disposition the caller answers with a
bounded request, the same disposition `qsl-route`'s own
`Disposition::RequiresBound` carries with no catalog code
(`qsl-route/src/routing.rs:29-31`). `NotSimulated` gives `catalog_code` and
`catalog_fields` as inherent methods, not a `CatalogCoded` impl -- a
`CatalogCoded` cause is always `Category::Refusal`, which `RequiresBound`
is not -- and `catalog_code` returns `None` for `RequiresBound`. It is
`Some(invalid_runtime_input/invalid-value)` for `EmptyInitial` and
`KeyEncoding`, the same category ADR-014 TR-2 gives a malformed runtime
request; `GeneratorMismatch` reads the same code for a different reason
(FR-101-AC-10). `Extent` delegates to whichever of `classify_extent`'s own
two failure types stopped it: `LimitExceeded`'s own
`stage_limit_exceeded`/`<kind>-exceeded`, or `InternalFault`'s own
`runtime_invariant`/`established-invariant-broken`. No code is invented for
`NotSimulated` itself, and none for `RequiresBound`.

**Canonical order.** The engine orders successors itself. It does not keep
the order a `TransitionSystem` lists them in. Breadth-first exploration
keeps parent states in FIFO discovery order within a level. Each parent's
successors are visited in ascending unsigned-lexicographic order of the
RFC 8785 JCS UTF-8 bytes of their transition identity,
`{"type":"transition","operation":"<qualified-name>","arguments":[<encoded>, ...]}`.
FR-181 does not order two successors with equal transition identities and
different post-states. QSL orders them by the post-state's state-key bytes,
ascending. Initial states are admitted in ascending state-key byte order,
with equal keys coalesced into one state. Both are QSL's choices; STD-109
settles them upstream in FR-181.

**Initial states.** A `max_states` cap reached while admitting initial
states stops the run `Bounded` at `Limit::States` before any expansion. Its
frontier is the admitted initial states, then the one the cap refused, then
the rest, each in canonical order; `max_states` 0 admits none, and every
initial state is in the frontier. Exploring a system with no initial state
returns `Exhaustive` with zero states, transitions and depth: its reachable
set is empty and fully visited.

**State key.** The state key is FR-181's
`{"type":"simulation-state","semantic":…,"control":…,"queues":…,"roles":…,"observations":…,"bounds":…}`
typed canonical form, serialized with RFC 8785 JCS through
`quire-canonical`, ADR-013 §2's one RFC 8785 encoder. Two states are the
same state exactly when their full key bytes are equal. The state-key digest
is SHA-256 over those bytes, with no domain label in the preimage, held as
a `DigestRecord` (`qsl_foundation::digest`) under the
`quire.simulation.state-key/v1` domain (`DigestDomain::SimulationStateKeyV1`,
QSpec FR-201). Coalescing compares full key bytes, never digests.

**Traces and frontiers record digests.** Following FR-181, every state a
trace or frontier names is its state-key digest, not its key bytes:
`Trace.initial` and each `Step.key` are `DigestRecord`s, a `Frontier` is a
`Vec<DigestRecord>`, and `ReplayError::UnknownInitial.expected` and
`KeyMismatch.{expected, actual}` are `DigestRecord`s. Replay recomputes each
candidate state's digest and compares it with the recorded digest.

**Sampler.** Sampling runs `quire.simulation.sampler/v1` at revision
`1-draft.1`, and only that generator. `sample_request` first compares the
supplied `DefinitionRef`'s identity and version with
`quire.simulation.sampler/v1` and `1-draft.1`, and refuses any other with
`NotSimulated::GeneratorMismatch`, before any draw. QSL pins no raw-byte
digest of the definition file; the supplied digest is recorded as given.
Draw `d` at trace `t`, step `s` under seed `k` is SHA-256 of the JCS object
`{"draw":"d","seed":"k","step":"s","trace":"t"}`, each member a decimal
string, encoded through `quire-canonical`. `s` is the 0-based index of the
transition within the trace: it advances once per transition taken, whether
or not that step computed a digest. With `n` successors in canonical order
and `v` the digest read as a big-endian `u256`, `v` is accepted when
`v < n * floor(2^256 / n)` and selects index `v mod n`; a rejected `v`
increments `d`. `d` restarts at 0 at every step. `n = 1` selects index 0
with no digest. `n = 0` ends the trace with `StopReason::NoSuccessors` and
no draw. Reaching `max_steps` ends it with `StopReason::StepLimit`. FR-181
names one initial state; when a `TransitionSystem` has `m > 1` initial
states after coalescing equal keys, QSL starts trace `t` from the initial
state at index `t mod m` in canonical order, with no draw. This is QSL's
choice; STD-109 settles it upstream. Provenance records the seed, the trace
index and the supplied `DefinitionRef` (identity, version and digest) in
place of today's `sampler_version: String` (ADR-014 TR-1). `CounterSampler`
is removed.

**Stopped outcomes.** A run that stops before its frontier empties never
reports `Exhaustive`. `Bounded` carries the limit that stopped it.
`Cancelled` carries `cause: CatalogCode` (`qsl_foundation::diagnostic`),
always `CatalogCode::new("cancelled", "caller-cancelled")`. Both carry the
unexplored frontier in the order the run would have expanded it next.
`Outcome::category()` stays FR-097-AC-5's map.

**Requires-bound.** Before any `TransitionSystem` method is called,
`explore_request` and `sample_request` classify `domains` with the ADR-014
§4 extent rule (`qsl_semantics::family::classify_extent(domains, types,
position_limit)`). `ClaimExtent::Unbounded(domains)` returns
`NotSimulated::RequiresBound`. A `ClassifyFailure` returns
`NotSimulated::Extent` with it, which is neither `RequiresBound` nor an
exploration. `requires-bound` is never an `Outcome` variant. No
exploration `Limits` value answers it, because an accounting limit is not a
proof bound (ADR-014 §1).

**Traceability.** Every simulation test in `qsl-eval/tests/it/` traces to
this requirement's ACs and to TC-453 to TC-455. None carries QSpec's
`TC-210` or `FR-181-AC-*` ids, which name different artifacts in this
repository (this repository's TC-210 is a witness-envelope case). The
disposition of each existing test is in the table below.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-101-AC-1 | Exploration visits each parent's successors in ascending JCS byte order of their transition identity, whatever order the `TransitionSystem` lists them in and whatever their post-states' keys: a state listing `z` before `a` is expanded `a` first, and `step(10)` is visited before `step(9)`, because `{"arguments":[{"type":"integer","value":"10"}],…}` precedes `{"arguments":[{"type":"integer","value":"9"}],…}` bytewise. Successors with equal transition identities are visited in ascending state-key byte order. Parent states keep FIFO discovery order within a level, whatever their keys. | Test (TC-453) |
| FR-101-AC-2 | The state key is the JCS encoding of FR-181's typed canonical form, and exploration coalesces two states exactly when their full key bytes are equal. The state whose `semantic` member is `float64` bits `0000000000000000` and every map empty has digest `943ae638f84583f2a35a7a92f1eac7f58c380c045298892fb1412756a1cc95e6`; with bits `8000000000000000` (negative zero) it has digest `92a3e9557f9aaf672a61aaab71eb2bfad13a0d88662953998de4057ed54e4b01`, and the two remain two states. Two NaNs with different payloads are two states. Each digest is a `DigestRecord` under `quire.simulation.state-key/v1`. | Test (TC-453) |
| FR-101-AC-3 | The sampler reproduces QSpec TC-210's vector: seed `424242`, trace `0`, `n = 5` draws indices `0, 0, 4, 4, 4` at steps 0 to 4, and the step-0 preimage `{"draw":"0","seed":"424242","step":"0","trace":"0"}` hashes to `cb7d4b3b8b8491d8310ccc1e07c3ee236d3fe86cf713d1851533a85ef6682622`. Seed `424242`, trace `1`, `n = 5` draws `1, 4, 3, 4, 0`; seed `424242`, trace `0`, `n = 3` draws `2, 1, 0, 2, 1`. `n = 1` selects index 0 and computes no digest, and still advances the step: seed `424242`, trace `0`, with `n = 1` at steps 0 and 1 and `n = 5` at step 2, selects index `4` at step 2. `n = 0` ends the trace with `StopReason::NoSuccessors`. | Test (TC-454) |
| FR-101-AC-4 | Two sampled runs with equal seeds, trace indices and `DefinitionRef`s over the same `TransitionSystem` produce identical traces, including provenance. The provenance records the seed, the trace index and the supplied `DefinitionRef`; a different seed or trace index gives different provenance. With `m > 1` initial states after coalescing equal keys, trace `t` starts at canonical initial state `t mod m`, and no draw selects the start. | Test (TC-454) |
| FR-101-AC-5 | A trace records its initial state and each step's state as state-key digests, and replays against the `TransitionSystem` that produced it without the simulator, by recomputed digest. When several successors share a transition identity, replay takes the one whose digest equals the recorded digest. A trace whose initial digest matches no initial state, whose step names a transition the current state does not offer, or whose recorded digest differs from every matching successor's digest refuses at that step with `UnknownInitial`, `MissingTransition` or `KeyMismatch`. | Test (TC-454) |
| FR-101-AC-6 | A run the poll cancels returns `Outcome::Cancelled` with `cause` `CatalogCode::new("cancelled", "caller-cancelled")`, category incomplete, and the unexplored frontier as state-key digests in next-expansion order. It never returns `Exhaustive` or `Bounded`. | Test (TC-455) |
| FR-101-AC-7 | A run that reaches `max_states`, `max_depth` or `max_transitions` before its frontier empties returns `Outcome::Bounded` with that `Limit` and the unexplored frontier as state-key digests in next-expansion order, category incomplete. The same model returns `Exhaustive` once `max_states` is at least its reachable state count, `max_transitions` at least its transition count, and `max_depth` greater than its deepest state's depth. `Exhaustive` is returned only when the frontier is empty, and its `Stats` count distinct states, explored transitions and the deepest depth. | Test (TC-455) |
| FR-101-AC-8 | `explore_request` or `sample_request` over `domains` that include an unbounded domain under the ADR-014 §4 extent rule returns `NotSimulated::RequiresBound` naming each unbounded domain's key and kind, calls no `TransitionSystem` method, and returns no `Outcome`. A `classify_extent` stage limit returns `NotSimulated::Extent`. The same request with every domain bounded explores. Raising exploration `Limits` does not change a `RequiresBound` result. | Test (TC-455) |
| FR-101-AC-9 | Initial states are admitted in ascending state-key byte order, and equal keys coalesce into one state. A `max_states` cap reached during admission returns `Bounded` at `Limit::States` with frontier: admitted, then refused, then the rest, in canonical order; `max_states` 0 admits none and puts every initial state in the frontier. Exploring a system with no initial state returns `Exhaustive` with zero stats. | Test (TC-453) |
| FR-101-AC-10 | `sample_request` refuses before any draw with `NotSimulated::GeneratorMismatch` when the supplied `DefinitionRef`'s identity is not `quire.simulation.sampler/v1` or its version is not `1-draft.1`, and with `NotSimulated::EmptyInitial` when the system has no initial state. A run that reaches `max_steps` stops with `StopReason::StepLimit`, whether or not the current state has successors. `GeneratorMismatch`'s catalog code is `invalid_runtime_input`/`invalid-value`. | Test (TC-454) |
| FR-101-AC-11 | A `TransitionSystem::Key` or `TransitionId` with no RFC 8785 encoding -- for example a `u64` above `2^53` -- refuses `NotSimulated::KeyEncoding` from `explore_request` and `sample_request`, and `ReplayError::KeyEncoding` from `replay`, instead of aborting the process. | Test (TC-453) |

## Existing test disposition

The tests in `qsl-eval/tests/it/finite_simulation.rs` at `6938db3d`:

| Test | Disposition |
| --- | --- |
| `exhaustive_small_graph_reports_exact_state_and_transition_counts` | Retag: TC-455 step 5, AC-7 |
| `canonical_order_is_the_systems_authored_successor_order` | Replace: TC-453 step 1, AC-1 |
| `key_equal_coalescing_merges_two_paths_to_the_same_state` | Retag: TC-453 step 7, AC-2 |
| `depth_limit_n_stops_bounded_and_n_plus_one_is_exhaustive` | Retag: TC-455 step 2, AC-7 |
| `state_limit_n_stops_bounded_and_n_plus_one_is_exhaustive` | Retag: TC-455 step 2, AC-7 |
| `transition_limit_n_stops_bounded_and_n_plus_one_is_exhaustive` | Retag: TC-455 step 2, AC-7 |
| `state_limit_mid_successors_places_the_blocked_key_after_the_queue` | Retag: TC-455 step 3, AC-7 |
| `several_initial_states_are_all_admitted_when_the_cap_allows` | Retag: TC-453 step 4, AC-9 |
| `state_limit_on_initial_states_orders_admitted_then_blocked_then_remaining` | Retag: TC-453 step 8, AC-9 |
| `duplicate_initial_states_coalesce_to_one_state` | Retag: TC-453 step 4, AC-9 |
| `max_states_zero_admits_no_initial_state` | Retag: TC-453 step 8, AC-9 |
| `cancellation_stops_the_run_and_returns_the_frontier` | Retag: TC-455 step 1, AC-6 |
| `seeded_sampling_reproduces_the_same_trace` | Retag: TC-454 step 5, AC-4 |
| `different_seeds_can_sample_different_traces` | Retag: TC-454 step 5, AC-4 |
| `max_steps_shorter_than_the_path_stops_on_the_step_limit` | Retag: TC-454 step 9, AC-10 |
| `sample_on_a_system_with_no_initial_states_returns_an_error` | Retag: TC-454 step 9, AC-10 |
| `sample_then_replay_round_trips_with_duplicate_transition_ids` | Retag: TC-454 step 7, AC-5 |
| `multi_initial_sample_picks_the_sampler_selected_start` | Replace: TC-454 step 6, AC-4. It asserts a drawn start, which AC-4 forbids |
| `multi_initial_replay_matches_the_recorded_initial_key` | Retag: TC-454 step 7, AC-5 |
| `replay_refuses_a_trace_whose_initial_key_matches_no_state` | Retag: TC-454 step 7, AC-5 |
| `replay_accepts_a_good_trace_and_refuses_a_tampered_one` | Retag: TC-454 step 7, AC-5 |
| `counter_sampler_produces_a_pinned_index_sequence` | Delete with `CounterSampler`; TC-454 steps 1 to 3 replace it |
| `tc_439_explore_outcomes_map_to_their_o16_category` | Unchanged: TC-439, FR-097-AC-5 |

Each retagged test keeps its assertion and moves to digests, canonical order
and the new signatures where this requirement changes them.

## Dependencies

- QSpec FR-181, its exploration contract and typed canonical form, and QSpec
  TC-210's sampler vector.
- QSpec `quire.simulation.sampler/v1`, revision `1-draft.1`
  (`proposals/quire-v1/definitions/simulation-sampler.md`).
- QSpec FR-201: the `quire.simulation.state-key/v1` digest domain.
- [ADR-014](../decisions/ADR-014-temporal-trace-and-boundedness-architecture.md)
  §1, §4, §7 and TR-1, TR-6, TR-7.
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  §2: `quire-canonical` is the one RFC 8785 encoder.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  X-8: `qsl-eval`'s `[dependencies]` gain `quire-canonical` and `serde` for
  the state key and the sampler preimage. SHA-256 comes through
  `quire-canonical`; `sha2` stays a dev dependency.
- [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md): the
  extent rule and `Outcome::category()`.

## Status

Implemented under QSL-272. `qsl-eval::simulation` exposes `explore_request`
and `sample_request` as its only public entries; `explore`, `sample` and
`Sampler` are `pub(crate)`. Exploration orders successors by ascending JCS
transition-identity bytes, tie-broken by ascending post-state key bytes, and
admits initial states in ascending state-key byte order. The state key is
the JCS encoding of a `TransitionSystem`-supplied typed view, through
`quire-canonical`; `Trace.initial`, `Step.key` and every `Frontier` entry
hold `DigestRecord`s under `quire.simulation.state-key/v1`, and replay
compares recomputed digests. Sampling runs the pinned
`quire.simulation.sampler/v1` `1-draft.1` generator; `CounterSampler` is
deleted. `Outcome::Cancelled` carries `cause: CatalogCode::new("cancelled",
"caller-cancelled")`, asserted literally by both cancellation tests.
`explore_request` and `sample_request` classify `domains` before calling any
`TransitionSystem` method, returning `NotSimulated::RequiresBound`,
`NotSimulated::Extent`, `NotSimulated::GeneratorMismatch` or
`NotSimulated::EmptyInitial` as this requirement specifies.
`NotSimulated::KeyEncoding` and `ReplayError::KeyEncoding` (FR-101-AC-11)
refuse a `TransitionSystem::Key` or `TransitionId` with no RFC 8785 encoding
instead of panicking. `NotSimulated` gives `catalog_code` and
`catalog_fields` as inherent methods returning `Option`, `None` for
`RequiresBound` (a negotiation disposition, never a refusal).
`qsl-eval/tests/it/finite_simulation.rs` traces to FR-101, TC-453, TC-454
and TC-455, and carries none of QSpec's `TC-210` or `FR-181-AC-*` tags.

Review round (SR-672, SR-673): fixed in this PR. `Outcome::Cancelled`
gained its `cause` field (SR-672/SR-673 FND-001); `NotSimulated` gained
`CatalogCoded` and `KeyEncoding` (SR-672 FND-002, SR-673 FND-002, this
requirement's AC-11); the duplicate-transition replay test and TC-453
step 1's fixture were fixed to discriminate what they claim to test (SR-672
FND-003, FND-004); the sampler gained `n = 2`/`n = 7` byte-order vectors and
a `U256` arithmetic unit test (SR-672 FND-005); `state_key` hashes its
already-produced bytes once (SR-672 FND-006); the sampler's successor draw
no longer has an `unreachable!` path (SR-672 FND-007); replay reports
`KeyMismatch.actual` in canonical order (SR-672 FND-008); TC-455 step 4 and
TC-453 step 4 are now tested as written, and AC-4's provenance is asserted
literally with a replay (SR-673 FND-003 to FND-005); the FIFO-order test is
retagged to AC-1 (SR-673 FND-006).

Disposition-pass round (R1): fixed in this PR. `RequiresBound` is no longer
`CatalogCoded`; no requires-bound catalog code or disposition exists on
`main` (checked `qsl-route`'s `Disposition::RequiresBound`, which itself
carries none), so `NotSimulated` gives `catalog_code`/`catalog_fields` as
inherent `Option`-returning methods instead, `None` for `RequiresBound`
(R1-FND-002). The sampler's acceptance test is split into `accepts`, tested
at the real `n = 7` threshold `floor(2^256/7)*7` and one below it, and the
`u256_divmod_and_mul_on_a_synthetic_value` comment no longer calls its
synthetic `98` the sampler's rejection bound (R1-FND-001). `GeneratorMismatch`'s
catalog code moved from AC-11 to AC-10 and TC-454 step 8's expected results;
a new test asserts `catalog_code` and `catalog_fields` for every
`NotSimulated` variant, including `Extent(Limit)`'s delegation to
`LimitExceeded`'s fields (R1-FND-003).

The ACs use no EARS keyword. They state behaviour declaratively, as FR-097
does, and each names its oracle (SR-642 FND-002, no change).
