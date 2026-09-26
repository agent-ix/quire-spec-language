---
id: SR-641
title: "QSL-272 base checklist review of the finite simulation spec (FR-101, TC-453 to TC-455)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@84a691bf8beb9df40aa945c48e1e05d7f5fb070b; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md; spec/test-cases/TC-453-exploration-orders-successors-canonically-and-keys-states-by-jcs-bytes.md; spec/test-cases/TC-454-the-pinned-sampler-reproduces-its-vectors-and-sampled-traces-replay.md; spec/test-cases/TC-455-stopped-explorations-stay-incomplete-and-unbounded-requests-require-a-bound.md; spec/test-cases/TC-390-family-outcome-and-refusal-layering.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/spec.md; spec/tests.md; spec/usecase/US-003-evaluate-bounded-state.md; qsl-eval/tests/it/finite_simulation.rs (unchanged, retag target)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: reviews
---

## Summary

Ticket: QSL-272 (PR agent-ix/quire-spec-language#457, spec only). Base
checklist over the nine changed files: ID formats, AC testability and
oracles, index rows, relationships, and the retag and replacement plan for
the existing simulation tests. QSpec reference: agent-ix/quire-specification
at origin/main 0d53cf2d (FR-181, FR-201, TC-210,
`proposals/quire-v1/definitions/simulation-sampler.md`).

What holds:

- IDs are new and free (FR-101, TC-453 to TC-455); `quire validate` on the
  nine changed files exits 0.
- spec.md, tests.md and US-003 index FR-101 and its TCs; tests.md's AC
  columns match each TC's scope.
- Every AC names a concrete oracle. AC-2 and AC-3 carry byte-exact vectors,
  and all of them recompute (see the vector table below).
- The leader decisions (tie-break by state-key bytes, canonical coalesced
  initial states, trace `t` starting at initial `t mod m`, QSL-owned pending
  STD-109, the QSL-274 split) are stated as QSL's choices and are not raised.

Vector recomputation (independent Python, `hashlib.sha256` over sorted-key
compact JSON, which equals RFC 8785 for these ASCII-only, number-free
objects; sampler per simulation-sampler.md `1-draft.1`):

| Vector | Spec value | Recomputed |
| --- | --- | --- |
| seed 424242, trace 0, n=5, steps 0-4 | 0,0,4,4,4 | 0,0,4,4,4 |
| step-0 preimage | `{"draw":"0","seed":"424242","step":"0","trace":"0"}` | same bytes |
| step-0 digest | cb7d4b3b…6682622 | cb7d4b3b8b8491d8310ccc1e07c3ee236d3fe86cf713d1851533a85ef6682622 |
| seed 424242, trace 1, n=5 | 1,4,3,4,0 | 1,4,3,4,0 |
| seed 424242, trace 0, n=3 | 2,1,0,2,1 | 2,1,0,2,1 |
| +0 state key digest | 943ae638…a1cc95e6 | 943ae638f84583f2a35a7a92f1eac7f58c380c045298892fb1412756a1cc95e6 |
| -0 state key digest | 92a3e955…d54e4b01 | 92a3e9557f9aaf672a61aaab71eb2bfad13a0d88662953998de4057ed54e4b01 |
| NaN 7ff8000000000000 (not in spec) | differ | a3d5ecff68c7cfb60a687aa72b743a04e1dc8513e348b9c3f64393dd96f4bdec |
| NaN 7ff8000000000001 (not in spec) | differ | 62c344cca9a4942b80644ca8527bc7ccced905f01bf67262a9bd5e824356a955 |
| `step(10)` before `step(9)` | bytewise | `{"arguments":[{"type":"integer","value":"10"}],…}` < `…"9"…` holds |

No draw in any vector hit rejection sampling.

Two defects, below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The retag and replacement plan leaves eight existing tests unmapped, and FR-101's traceability rule cannot hold for them. FR-101 says "Every simulation test in `qsl-eval/tests/it/` traces to this requirement's ACs and to TC-453 to TC-455". The TC Status sections name only `canonical_order_is_the_systems_authored_successor_order`, `key_equal_coalescing_…`, "the replay tests", "the limit and frontier tests" and "the `CounterSampler`-seeded tests". Not mapped: `exhaustive_small_graph_reports_exact_state_and_transition_counts` (:73), `several_initial_states_are_all_admitted_when_the_cap_allows` (:325), `state_limit_on_initial_states_orders_admitted_then_blocked_then_remaining` (:343), `duplicate_initial_states_coalesce_to_one_state` (:371), `max_states_zero_admits_no_initial_state` (:388), `cancellation_stops_the_run_and_returns_the_frontier` (:416), `sample_on_a_system_with_no_initial_states_returns_an_error` (:511, `EmptyInitial`) and `max_steps_shorter_than_the_path_stops_on_the_step_limit` (:497, `StopReason::StepLimit`). Four of these behaviours are in no FR-101 AC: `EmptyInitial`, `StepLimit`, `max_states` 0, and initial-state coalescing (Behavior only). The failure: the implementer either deletes those tests, losing coverage, or tags them to an AC that does not state what they assert. Fix: name each test's disposition (retag to TC-45n step, replace, or delete), and add the four behaviours to an AC and a TC step, or narrow the traceability rule. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:122-125; spec/test-cases/TC-453-exploration-orders-successors-canonically-and-keys-states-by-jcs-bytes.md:57-60; spec/test-cases/TC-454-the-pinned-sampler-reproduces-its-vectors-and-sampled-traces-replay.md:53-56; spec/test-cases/TC-455-stopped-explorations-stay-incomplete-and-unbounded-requests-require-a-bound.md:46-49; qsl-eval/tests/it/finite_simulation.rs:73-416,497-549 |
| FND-002 | medium | Three TC-453 oracles do not tell the rule under test from a wrong rule, because the fixtures do not fix post-state keys against the order being tested. Step 1 (`z` then `a`) and step 2 (`step(9)` then `step(10)`) pass for an engine that sorts successors by post-state key, when `a`'s and `step(10)`'s post-states happen to have the smaller keys. Step 5 (0 → {1, 2}, 1 → 3, 2 → 4, frontier 3 then 4) passes for an engine that re-sorts each level by state key, when key(3) < key(4). Step 3 already covers post-state order for equal transition identities, so a sort-by-post-state engine meets steps 1 to 3 on a generous fixture. Fix: require post-state keys in the opposite order to the transition order in steps 1 and 2 (for example, `a`'s post-state key greater than `z`'s), and in step 5 require key(1) > key(2) and key(3) > key(4), so only FIFO order gives the stated frontier. | spec/test-cases/TC-453-exploration-orders-successors-canonically-and-keys-states-by-jcs-bytes.md:18-28,39-45 |

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@be1851894b7dbe9f47a606186f181900e5e6eb3b` (fix commit `be185189`, "QSL-272 spec: fix PR #457 review findings (SR-641 to SR-645)"). I re-checked each outcome against the spec at that head. I did not take any outcome from the commit message. Vectors re-run: the mixed-n vector (n=1 at steps 0 and 1, n=5 at step 2, seed 424242, trace 0) selects 4, and a counter that skips no-draw steps selects 0. The NaN digests for 7ff8000000000000 and 7ff8000000000001 are a3d5ecff68c7cfb60a687aa72b743a04e1dc8513e348b9c3f64393dd96f4bdec and 62c344cca9a4942b80644ca8527bc7ccced905f01bf67262a9bd5e824356a955, and they match TC-453 step 6.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed be185189 | FR-101 has a new "Existing test disposition" table (FR-101:221-252). It covers all 23 `#[test]` functions in finite_simulation.rs, and I counted 23. Each has a retag, replace or delete target at a TC step and an AC. `multi_initial_sample_picks_the_sampler_selected_start` is replaced by TC-454 step 6. New AC-9 covers initial admission, coalescing, `max_states` 0 and exploring with no initial state. New AC-10 covers `GeneratorMismatch`, `EmptyInitial` and `StepLimit`. |
| FND-002 | fixed be185189 | TC-453 step 1 now makes `a`'s post-state key greater than `z`'s. Step 2 makes `step(10)`'s greater than `step(9)`'s. Step 5 requires key(1) > key(2) and key(3) > key(4). Only FIFO order and transition-identity order give the stated frontiers. Step 4 adds a duplicate initial state under `max_states` 3, which also catches an engine that does not coalesce. |
| FND-003 | fixed 561abc41 | FR-101 makes `explore`, `sample` and the `Sampler` trait `pub(crate)`, so `explore_request` and `sample_request` are the only public entries (Behavior and Status). Every step in TC-453, TC-454 and TC-455 names its entry, with a bounded or empty `domains` set; TC-454 steps 1b and 3b are named in-crate unit tests. |

## New findings (disposition pass)

These were found at `be185189`. They are new, not dispositions of the rows above.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | FR-101 adds `explore_request` and `sample_request` as the checked entries, but does not say whether today's public `explore`, `sample` and the `Sampler` trait (qsl-eval/src/simulation/mod.rs:23-25) stay public. FR-101 says only that `CounterSampler` is removed. If they stay, a caller can explore without the `requires-bound` check that FR-181 puts "before exploration begins", or sample without the `GeneratorMismatch` check. TC-453 and TC-455 steps 1 to 3 and 5 say "Explore a system…" with no domains, so the spec reads either way. Fix: state that `explore`/`sample` become private, or that they stay as engine-only entries that callers holding a request never use, and name which one the TC steps call. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:74-119,180-181; qsl-eval/src/simulation/mod.rs:23-25 |
