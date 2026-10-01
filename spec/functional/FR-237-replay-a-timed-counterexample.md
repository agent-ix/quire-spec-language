---
id: FR-237
title: "Replay a timed counterexample through ModelSystem"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-232
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-236
    type: depends_on
---
# FR-237: Replay a timed counterexample through ModelSystem

## Description

`qsl_replay::replay_model_trace` (FR-128) SHALL replay a counterexample
over a timed subject in exact rational arithmetic (ADR-026 CT-3, TD-3): it
starts every clock at 0, checks each delay is admissible, checks each
transition is enabled after its delay, selects the successor by the
post-state digest, applies resets, checks time invariants, checks the timed
lasso's closure and fairness, and evaluates the formula over the timed
lasso. A time-lock counterexample replays by its own arm. Replay needs no
zone engine.

## Use case

An auditor with only the source, the subject's snapshots and a refutation
of a timing claim replays it. A counterexample whose delay violates a time
invariant, or whose lasso does not let time diverge, does not count as a
refutation.

## Inputs

- FR-128's request and a `WitnessEnvelope<TemporalCounterexample>` over a
  timed subject (FR-236).
- For a non-local time-lock: the request's exploration limits.

## Outputs

- An FR-072 replay result on the `ModelTrace` arm, or a typed
  `ReplayRefusal` with no partial result, as FR-128 states.

## Behavior

- Replay SHALL apply FR-128's recompile, identity, fairness-set and
  admission rules unchanged.
- The executor SHALL start every clock of the subject at 0.
- For each step, the executor SHALL:
  - check the delay is admissible at the current timed state (FR-231);
    if not, refuse `invalid_runtime_input`/`invalid-value`, naming the step
    and the time invariant or urgency that forbids it;
  - apply the delay, then check the transition identity is enabled at the
    delayed state, refusing `invalid_runtime_input`/`invalid-value`
    naming the step and the guard when it is not;
  - select the successor whose post-state digest equals the recorded one,
    refusing `stale_dependency`/`revision-mismatch` naming the step and
    both digests when none does;
  - apply the resets and check the target's time invariants, refusing
    `invalid_runtime_input`/`invalid-value` when one fails.
- Every comparison and sum SHALL be exact.
- **Lasso.** Replay SHALL check the loop's closure (FR-236) and that its
  total delay is positive, refusing `invalid_runtime_input`/`invalid-value`
  otherwise. It SHALL check fairness over time (FR-232) and refuse an
  unfair lasso as FR-128 does.
- **Evaluation.** For `kind: Formula`, replay SHALL evaluate the formula by
  FR-231 over the timed lasso, unrolling the loop until the unrolled time
  span covers the formula's time reach. `false` SHALL settle
  `reproduced-with-evaluated-witness`; `true` or no value, `inconclusive`.
- **Local time-lock.** For `kind: TimeLock`, after the prefix and its
  `final_delay`, when no positive delay is admissible at the last timed
  state, replay SHALL check that no transition identity is enabled there;
  agreement SHALL settle `reproduced-with-evaluated-witness`, and an
  enabled identity SHALL settle `inconclusive`, `ReplayParity`.
- **Non-local time-lock.** When a positive delay is admissible at the last
  timed state, replay SHALL explore that state's forward closure afresh,
  unreduced, under the request's limits, with the subject's timed
  exploration (FR-101 `explore` for a `Tick` source, the zone successor
  relation of FR-238 from the point zone for a `Dense` source), looking for
  a quiescent state or a cycle with positive total delay. An exploration
  that completes with none SHALL settle `reproduced-with-evaluated-witness`;
  one that finds one SHALL settle `inconclusive`, `ReplayParity`; one a
  limit stops SHALL settle the item V-7.
- Replay SHALL read no path, environment variable, clock or search
  location, and SHALL give the same result for the same request and
  envelope.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-237-AC-1 | FR-236-AC-1's counterexample replays to `reproduced-with-evaluated-witness`, checking the delay 3 against `x <= 3` and `x <= T` and the guards `x >= 3` and `x >= 1`. `Settles` with `eventually[0 ms, 3 ms)` over `Rpc` with `T = 4 ms` is refuted by `send` at 0 then `reply` after delay 3, and that counterexample reproduces. | Test (TC-692) |
| FR-237-AC-2 | Refusals settle no result: the `timeout` delay changed to `5/2` (guard `x >= 3` fails); the delay changed to `7/2` (`x <= 3` forbids it); one post-state digest altered (`stale_dependency`/`revision-mismatch`); a lasso whose loop has total delay 0; a lasso whose loop does not return to its entry clock values. | Test (TC-692) |
| FR-237-AC-3 | The strict-guard variant's local time-lock counterexample reproduces; the same payload replayed against the variant with `reply` guarded by `x >= 3 ms` settles `inconclusive`, `ReplayParity`, since `reply` is enabled at the last state. | Test (TC-692) |
| FR-237-AC-4 | The `Stall` model's non-local time-lock counterexample (stem empty, `final_delay` `1/2`) reproduces by fresh exploration finding no quiescent state and no cycle with positive delay; the same payload against a variant with `ping` resetting `x` settles `inconclusive`, `ReplayParity`; with an exploration limit of one state it settles V-7. Replaying one envelope twice gives equal results. | Test (TC-692) |

## Dependencies

- ADR-026 §7 CT-3, §4 TD-3; ADR-018 CX-3 and SM-8; ADR-022 GX-3.
- [FR-128](FR-128-replay-a-model-counterexample.md),
  [FR-098](FR-098-execute-a-replay-request.md),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md),
  [FR-231](FR-231-read-a-timed-subject-s-behaviours-as-timed-traces.md),
  [FR-232](FR-232-derive-the-time-lock-freedom-item-and-read-deadlocks-over-time.md),
  [FR-236](FR-236-carry-exact-rational-delays-in-a-timed-counterexample.md).

## References

- QSpec half: Linear STD-139 owns the replay rules of a timed counterexample
  and of the time-lock trap on the wire (ADR-026 OV-6, OV-9); this
  requirement cites it until those QSpec FRs merge.
