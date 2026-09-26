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
(operation × finite argument domains × frame post-states, QSpec FR-013-AC-1),
the recording of an invariant-violating successor (FR-181-AC-4's last
clause) or the refusal of evaluator effects (FR-181-AC-6). A
`TransitionSystem` implementation supplies the successor relation; the
engine orders, keys, explores and samples it.

## Inputs

- A `TransitionSystem`: its initial states and, per state, its successors,
  each a transition identity and a post-state, both in the typed canonical
  form of QSpec FR-181's exploration contract.
- For exploration: `Limits{max_states, max_depth, max_transitions}` and a
  cancellation poll.
- For sampling: a `u64` seed, a trace index, a step ceiling and the
  sampler's `DefinitionRef` from the ecosystem lock's `definitions`.
- The simulation request's parameter-domain and population types, each with
  the wire id of the checked node that carries it.

## Outputs

- `Outcome::Exhaustive(Stats)`, `Outcome::Bounded{stats, frontier, limit}`
  or `Outcome::Cancelled{stats, frontier, cause}`.
- A sampled `Trace` with its `SampleProvenance{seed, trace, sampler, stopped}`.
- `RequiresBound` with each unbounded domain's key and kind, returned before
  any state is explored.
- A replay result: success, or the first step that does not replay.

## Behavior

**Canonical order.** The engine orders successors itself. It does not keep
the order a `TransitionSystem` lists them in. Breadth-first exploration
keeps parent states in FIFO discovery order within a level. Each parent's
successors are visited in ascending unsigned-lexicographic order of the
RFC 8785 JCS UTF-8 bytes of their transition identity,
`{"type":"transition","operation":"<qualified-name>","arguments":[<encoded>, ...]}`.
FR-181 does not order two successors with equal transition identities and
different post-states. QSL orders them by the post-state's state-key bytes,
ascending. Initial states are admitted in ascending state-key byte order,
with equal keys coalesced.

**State key.** The state key is FR-181's
`{"type":"simulation-state","semantic":…,"control":…,"queues":…,"roles":…,"observations":…,"bounds":…}`
typed canonical form, serialized with RFC 8785 JCS through
`quire-canonical`, ADR-013 §2's one RFC 8785 encoder. Two states are the
same state exactly when their full key bytes are equal. The state-key digest
is SHA-256 over those bytes, with no domain label in the preimage, recorded
as a `DigestRecord` under the `quire.simulation.state-key/v1` domain
(`DigestDomain::SimulationStateKeyV1`, QSpec FR-201). The digest identifies
a state in traces and frontiers; it is never used to coalesce.

**Sampler.** Sampling draws from the canonically ordered successor set with
`quire.simulation.sampler/v1`: draw `d` at trace `t`, step `s` under seed
`k` is SHA-256 of the JCS object `{"draw":"d","seed":"k","step":"s","trace":"t"}`,
each member a decimal string, encoded through `quire-canonical`. With `n`
successors and `v` the digest read as a big-endian `u256`, `v` is accepted
when `v < n * floor(2^256 / n)` and selects index `v mod n`; a rejected `v`
increments `d`. `d` restarts at 0 at every step. `n = 1` selects index 0 with
no digest. `n = 0` ends the trace with no draw. FR-181 names one initial
state; when a `TransitionSystem` has `m > 1` distinct initial states, QSL
starts trace `t` from the initial state at index `t mod m` in canonical
order, with no draw. Provenance records the seed, the trace index and the
lock's `DefinitionRef` (identity, revision and raw-byte digest) in place of
today's `sampler_version: String` (ADR-014 TR-1).

**Stopped outcomes.** A run that stops before its frontier empties never
reports `Exhaustive`. `Bounded` carries the limit that stopped it.
`Cancelled` carries cause `cancelled`/`caller-cancelled`. Both carry the
unexplored frontier as state-key digests, in the order the run would have
expanded them next. `Outcome::category()` stays FR-097-AC-5's map.

**Requires-bound.** Before exploration begins, the simulation entry
classifies the request's parameter-domain and population types with the
ADR-014 §4 extent rule (`qsl_semantics::family::classify_extent`). An
`Unbounded` extent returns `RequiresBound` and explores nothing.
`requires-bound` is never an `Outcome` variant. No exploration `Limits`
value answers it, because an accounting limit is not a proof bound (ADR-014
§1).

**Traceability.** Every simulation test in `qsl-eval/tests/it/` traces to
this requirement's ACs and to TC-453 to TC-455. None carries QSpec's
`TC-210` or `FR-181-AC-*` ids, which name different artifacts in this
repository (this repository's TC-210 is a witness-envelope case).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-101-AC-1 | Exploration visits each parent's successors in ascending JCS byte order of their transition identity, whatever order the `TransitionSystem` lists them in: a state listing `z` before `a` is expanded `a` first, and `step(10)` is visited before `step(9)`, because `{"arguments":[{"type":"integer","value":"10"}],…}` precedes `{"arguments":[{"type":"integer","value":"9"}],…}` bytewise. Successors with equal transition identities are visited in ascending state-key byte order. Initial states are admitted in ascending state-key byte order. Parent states keep FIFO discovery order within a level. | Test (TC-453) |
| FR-101-AC-2 | The state key is the JCS encoding of FR-181's typed canonical form, and exploration coalesces two states exactly when their full key bytes are equal. The state whose `semantic` member is `float64` bits `0000000000000000` and every map empty has digest `943ae638f84583f2a35a7a92f1eac7f58c380c045298892fb1412756a1cc95e6`; with bits `8000000000000000` (negative zero) it has digest `92a3e9557f9aaf672a61aaab71eb2bfad13a0d88662953998de4057ed54e4b01`, and the two remain two states. Two NaNs with different payloads are two states. Each digest is recorded under `quire.simulation.state-key/v1`. | Test (TC-453) |
| FR-101-AC-3 | The sampler reproduces QSpec TC-210's vector: seed `424242`, trace `0`, `n = 5` draws indices `0, 0, 4, 4, 4` at steps 0 to 4, and the step-0 preimage `{"draw":"0","seed":"424242","step":"0","trace":"0"}` hashes to `cb7d4b3b8b8491d8310ccc1e07c3ee236d3fe86cf713d1851533a85ef6682622`. Seed `424242`, trace `1`, `n = 5` draws `1, 4, 3, 4, 0`; seed `424242`, trace `0`, `n = 3` draws `2, 1, 0, 2, 1`. `n = 1` selects index 0 and computes no digest; `n = 0` ends the trace with `StopReason::NoSuccessors`. | Test (TC-454) |
| FR-101-AC-4 | Two sampled runs with equal seeds, trace indices and `DefinitionRef`s over the same `TransitionSystem` produce identical traces, including provenance. The provenance records the seed, the trace index and the lock's `DefinitionRef`. With `m > 1` initial states, trace `t` starts at canonical initial state `t mod m`. | Test (TC-454) |
| FR-101-AC-5 | A trace replays against the `TransitionSystem` that produced it without the simulator. A trace whose initial key matches no initial state, whose step names a transition the current state does not offer, or whose recorded key differs from every matching successor's key refuses at that step with `UnknownInitial`, `MissingTransition` or `KeyMismatch`. | Test (TC-454) |
| FR-101-AC-6 | A run the poll cancels returns `Outcome::Cancelled` with cause `cancelled`/`caller-cancelled`, category incomplete, and the unexplored frontier as state-key digests in next-expansion order. It never returns `Exhaustive` or `Bounded`. | Test (TC-455) |
| FR-101-AC-7 | A run that reaches `max_states`, `max_depth` or `max_transitions` before its frontier empties returns `Outcome::Bounded` with that `Limit` and the unexplored frontier as state-key digests in next-expansion order, category incomplete. The same model under limits one larger returns `Exhaustive`. `Exhaustive` is returned only when the frontier is empty. | Test (TC-455) |
| FR-101-AC-8 | A simulation request whose parameter-domain or population types include an unbounded domain under the ADR-014 §4 extent rule returns `RequiresBound` naming each unbounded domain's key and kind, calls no `TransitionSystem` method, and returns no `Outcome`. The same request with every domain bounded explores. Raising exploration `Limits` does not change a `RequiresBound` result. | Test (TC-455) |

## Dependencies

- QSpec FR-181, its exploration contract and typed canonical form, and QSpec TC-210's
  sampler vector.
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

Specified under QSL-272. Not implemented. Today `explore` keeps the
authored successor order (`qsl-eval/src/simulation/explore.rs:192`), the
state key is caller-owned bytes, `CounterSampler` is the only sampler,
`Outcome::Cancelled` carries no cause, the frontier holds full key bytes and
no `RequiresBound` exists. `qsl-eval/tests/it/finite_simulation.rs` still
carries QSpec's `TC-210` and `FR-181-AC-*` tags.
