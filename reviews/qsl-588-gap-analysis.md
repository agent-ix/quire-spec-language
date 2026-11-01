---
id: SR-1988014
title: "Gap analysis — QSL-588 timed statistical sampling PR 1998 rebased on #2006"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language PR #1998 at 691b46ad8e8cf48fb10830d729c79f21c4750bb5 over base a8090549bb371d8b0330d013dab9462b36d78d37; focused FR-253/254 implementation and TC-708/709 trace review"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-253", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-254", type: references }
---

## Summary

Focused gap and trace review of FR-253/254, TC-708/709, the tagged tests, and the changed sampler, statistical analyzer and replay code found direct bindings and behavior-oracle coverage for all eight acceptance criteria. The repository-wide computed matrix did not return, so this artifact does not claim repository-wide matrix, reverse-gap, or stub completeness.

## Verdict

**CONDITIONAL** — no scoped FR-253/254 gap was identified, but Quoin matrix evidence was unavailable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-000 | low | No scoped FR-253/FR-254 findings identified (schema-required empty-list sentinel) | — |

## Coverage

- FR-253-AC-1: `TC-708` checks exact conditioned delay windows and the three exact race outcome probabilities.
- FR-253-AC-2: a direct trace-tagged test binds the race/tie-order probability oracle; its BTreeMap key is an immutable ordered vector of operation/delay pairs.
- FR-253-AC-3: tests cover invalid and duplicate delay declarations, untimed declaration refusal, and unbounded uniform refusal with the state/identity.
- FR-253-AC-4: the no-law delay remains a scheduler choice with `distribution: None`; timed sampling separately refuses that MDP state as `NotStochastic`.
- FR-254-AC-1/2: tests cover expected statistical decisions, deterministic seeded verdicts, exact rational grid draws, timed witness replay and rejection after changing a delay.
- FR-254-AC-3: direct tagged tests assert elapsed timestamp subtraction, timed `max_cycle_steps` settlement, strict-guard timed exact-route refusal, free-delay refusal, and the `max_sample_steps` limit.
- FR-254-AC-4: the local time-lock test checks the named state and that no witness is retained.
- Typed fairness integration test remains present and passed. EN-5 protected paths remain unchanged.
- Matrix limitation: `quoin matrix --repo . --json` emitted no output during an extended run and was interrupted. Manual focused source/tag inspection confirms the eight FR tags, but no computed repository-wide untagged/stale-tag, ignored-test, reverse-gap, or stub report is available. No repository-wide completeness claim is made.
- Plan completion: not assessed.

## Scope

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

- `TC-708` → FR-253-AC-1..4: direct test tags and behavior assertions were found.
- `TC-709` → FR-254-AC-1..4: direct test tags and behavior assertions were found, including all bundled FR-254-AC-3 outcomes.
