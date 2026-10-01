---
id: FR-131
title: "Replay checks strong fairness on a model counterexample"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-016
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-019
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-129
    type: depends_on
---
# FR-131: Replay checks strong fairness on a model counterexample

## Description

`qsl_replay::replay_model_trace` (FR-128) SHALL check every constraint of a
model counterexample's fairness set against its loop, weak and strong, with
enabledness recomputed from the model, and refuse a lasso that fails a
strong constraint as an unfair lasso (ADR-019 SR-8, SF-4). The settlement
in `model_check` SHALL settle an EN-1 counterexample that replay refuses as
unfair `inconclusive`, `ReplayParity`, since EN-1 builds only fair lassos
(ADR-019 SV-4).

## Use case

An auditor replays a lasso against a claim under strong fairness. Replay
does not trust the engine's claim that the lasso is fair: it rebuilds the
model, computes which transitions are enabled at each loop state, and
refuses the lasso when a strongly fair operation is enabled in the loop and
never taken.

## Semantic authority and boundary

QSpec owns the counterexample wire with each constraint's kind and the rule
that replay recomputes enabledness and refuses an unfair lasso (ADR-019
QS-5, QS-6; References). This requirement specifies QSL's replay check and
its settlement.

## Inputs

- FR-128's replay request and `WitnessEnvelope<TemporalCounterexample>`,
  whose fairness set carries each constraint's kind, operation and
  granularity (FR-129).

## Outputs

- FR-128's replay result or `ReplayRefusal`.
- For FR-127's settlement: `Inconclusive(ReplayParity)` for an EN-1
  counterexample refused as unfair.

## Behavior

- For a `kind: Formula` counterexample with a loop, the executor SHALL
  compute, from `ModelSystem`, the transition identities enabled at every
  loop state.
- The executor SHALL check a weak constraint by ADR-018 FA-3: taken
  somewhere in the loop or disabled somewhere in the loop.
- The executor SHALL check a strong constraint by ADR-019 SF-4: when it is
  enabled at some loop state, some loop step takes it.
- If any constraint fails, then the executor SHALL refuse
  `invalid_runtime_input`/`invalid-value`, naming the first failing
  constraint of the fairness set, with no result.
- The executor SHALL treat a loop that is the terminal stutter step as fair
  under every strong constraint, since no operation is enabled at a
  terminal state.
- When a `Violated` outcome of EN-1 is refused by replay as an unfair
  lasso, `model_check` SHALL settle it `inconclusive`, `ReplayParity`
  (ADR-019 SV-4); every other replay refusal settles by FR-127.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-131-AC-1 | ADR-019 §6's mutex: the lasso `0 -acq(2)-> 2 -rel-> 0` reproduces in an envelope for the `weak each` clause and for the `strong` (whole) clause; in an envelope for the `strong each` clause, carrying that clause's identities and fairness set, it refuses `invalid_runtime_input`/`invalid-value` naming `{Strong, acquire, Each}`, with no result. | Test (TC-532) |
| FR-131-AC-2 | FR-130-AC-2's `Handoff` lasso reproduces under its `strong each` clause, since `acquire` is enabled at no loop state. FR-126-AC-3's terminal stutter lasso over `Counter` reproduces in an envelope for the same claim with `strong` on `inc` added. | Test (TC-532) |
| FR-131-AC-3 | A `Violated` outcome given to `model_check`'s settlement as EN-1's, carrying AC-1's lasso for the `strong each` clause, whose replay refuses it as an unfair lasso, settles `inconclusive`, `ReplayParity`, category inconclusive. | Test (TC-532) |

## Dependencies

- ADR-019 §2 SF-4, §3 SR-8, §5 SV-4, AM-6; ADR-018 §4 FA-3, §5 CX-3.
- [FR-128](FR-128-replay-a-model-counterexample.md) (the replay entry it
  extends), [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (settlement), [FR-129](FR-129-check-strong-fairness-constraints.md).

## References

- QSpec half of ADR-019, carrying ADR-019 QS-5 and QS-6: Linear STD-132.
