---
id: SR-1988013
title: "Code review — QSL-588 timed statistical sampling PR 1998 rebased on #2006"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language PR #1998 at 691b46ad8e8cf48fb10830d729c79f21c4750bb5 over base a8090549bb371d8b0330d013dab9462b36d78d37; changed Rust implementation/tests; FR-253-AC-1..4, FR-254-AC-1..4"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-253", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-254", type: references }
---

## Summary

Reviewed the rebased implementation and Rust changes, including the overlapping exact-probabilistic integration test and qsl-replay outcome/proof_result changes introduced alongside #2006. FR-253/254 test oracles and typed-fairness integration remain behaviorally aligned. No code-review findings.

## Verdict

**PASS** — no code or Rust findings in this scoped review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- FR-253-AC-1/2: the timed race integration test directly traces both ACs. It verifies the bounded windows and exact path probabilities (`9/10`, `1/20`, `1/20`) in a `BTreeMap<Vec<(String, String)>, Rational>` keyed by immutable path values.
- FR-253-AC-3/4: declaration refusals remain tested. The race still represents an undeclared timeout delay as `distribution: None` and a scheduler choice; sampled execution refuses the same state as `NotStochastic`.
- FR-254-AC-1..4: exact statistical decisions, rational delay draws, witness replay and tamper rejection, elapsed reading, timed regenerative bound, exact-route refusal, free-delay refusal, sample budget and local time-lock are asserted by directly tagged tests.
- #2006 overlap: `exact_probabilistic.rs` uses the current `ProductRequest`/`ExactProbLimits::default()` API and asserts the strict-guard timed request is refused as `ProfileNotTranslated`. New typed state/transition fields in qsl-replay are omitted when absent, retaining the prior wire shape; a new serialization test covers populated typed fields. The new refusal variants settle to the intended wire causes.
- Typed fairness integration: `every_scheduler_carries_its_fairness_set` remains present and passed.
- EN-5: no protected exact-probabilistic implementation, certificate/witness, or CI/workflow paths were changed.
- Gates: `cargo fmt --check`, strict all-target/all-feature workspace Clippy, and focused exact-probabilistic (26), statistical-model (29), witness-replay (12), timed-subject (73), probabilistic-declarations (52), typed replay-outcome (1) test passed, and `cargo check --no-default-features` passed. `cargo deny check bans` passed; full `cargo deny check` reports existing AGPL license-policy failures in unchanged dependencies.
- Quoin matrix produced no output and was interrupted after extended runtime; repository-wide matrix results are unavailable and are not claimed here.

## Scope reviewed

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-253-AC-1 | examined | ADR-026 §11's stochastic variant checks: `reply` `Discrete[(1, 90), (3, 10)]`, `timeout` `Uniform`, `send` and `reset` `Exponential{1/1000}` per ms. In the state just after `send`, `reply`'s window is `[1, 3]` and `timeout`'s (with `T = 3 ms`) is `[3, 3]`. |
| FR-253-AC-2 | examined | In that state the race gives `reply` at 1 with probability `9/10`; at 3 it ties with `timeout`, ordered evenly by the workload `Even`, so `reply` then `timeout` and `timeout` then `reply` each have probability `1/20`, and in the second order `reply` is still enabled at its turn. |
| FR-253-AC-3 | examined | Refusals at the span: `delay M::o ~ exponential(0)`; `delay M::o ~ discrete { 1 ms: 1, 1 ms: 2 }`; `delay M::o ~ uniform[3 ms, 1 ms]`; `delay M::o ~ uniform` in an untimed model (`invalid_probabilistic_model`/`delay-on-untimed-model`); a second `delay M::o ~ uniform;` of one operation (`invalid_model_binding`/`malformed-declaration`, at the second declaration). A state where an identity with unbounded window and `uniform` races is disposed `Unsupported(NotStochastic)` naming the state and identity. |
| FR-253-AC-4 | examined | The §11 stochastic variant with no `delay` declaration on `timeout` checks `timeout` with `None`. In the state just after `send` the race is a Markov decision process: after `reply`'s draw the scheduler chooses `timeout`'s delay in `[3, 3]`, and no distribution is assigned to it. |
| FR-254-AC-1 | examined | `RareLate` (`probability <= 0.01 [eventually[0 ms, 3 ms] holds(c.late)]`) over FR-253-AC-1's model, whose exact event probability is `1/20`, settles `measured`, `Rejected`, with `q` 64 in its basis; with `reply` under `uniform` it settles `measured`, `Accepted`. Two runs with one seed give equal verdicts. |
| FR-254-AC-2 | examined | Every sampled delay is an exact rational; a sampled witness replays through `replay_sampled_witness` (FR-193) with FR-237's timed steps, and its draws recompute from the seed; the same witness with one delay changed fails draw recomputation. With `q` 8, the basis records 8 and each sampled `uniform` or `exponential` delay equals the rounded-up conditioned inverse distribution function at a multiple of `2^-8`. |
| FR-254-AC-3 | examined | `elapsed from holds(c.phase = Waiting) until holds(c.phase != Waiting) within 3 ms` reads each sample's time-stamp difference; a model that never returns to its initial discrete state with every clock 0 settles `incomplete`, `limit-reached` naming `statistical.max_cycle_steps` under the regenerative method; an `exact` request over the strict-guard `Rpc` variant settles `unsupported`. `RareLate` with no `delay` declaration on `timeout` settles `unsupported`, `NotStochastic`, naming the state just after `send` and `timeout`. `RareLate` with `max_sample_steps` 1 stops `Incomplete(LimitReached{... |
| FR-254-AC-4 | examined | A stochastic `Lock` variant (one object `c` with `phase: {A, B}` and a clock `x` never reset; `go` moves `A` to `B`, with `delay Lock::go ~ discrete { 1/2 ms: 1 };`; `time invariant when phase = B { x <= 1 ms }`; no operation enabled in `B`) samples `go` at `1/2`, delays to `(B, x = 1)`, where no positive delay is admissible and no identity is enabled, and settles `unsupported`, `TimeLockedSample` naming that state, with no witness kept. |

## Bindings

- `TC-708` directly binds FR-253-AC-1..4; the exact race and free-delay MDP/refusal tests assert those behaviors.
- `TC-709` directly binds FR-254-AC-1..4, including the additional AC-3 exact-route and regenerative-bound tests.
