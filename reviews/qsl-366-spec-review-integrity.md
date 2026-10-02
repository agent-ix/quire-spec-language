---
id: SR-962
title: "QSL-366 integrity review of ADR-018 and its amendments to ADR-011, ADR-013, ADR-014 and ADR-016"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@9b546eb069048b46d0c365f85c4481c0a3012115; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md; spec/decisions/ADR-016-state-model-finite-execution-mapping.md; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md; spec/functional/FR-123..FR-128; spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---

## Summary

Ticket: QSL-366. This review checks cross-artifact consistency:
- the local item ids (TP, V-1 to V-8, SM, EN, FA-1 to FA-6, CX, DS, QS, RU, DL, IV);
- FR-123 to FR-128 against the ADR;
- the four in-place ADR amendments.

**Consistent:**
- Every local id cited inside ADR-018 and FR-123 to FR-128 resolves to a defined row.
- V-1 to V-8 agree between ADR-018 §1, FR-127's table and the ADR-013 O-16 amendment.
- FA-1 to FA-6 agree with FR-123 and FR-126.
- DL-1 to DL-7 agree with FR-124, FR-126 and FR-128.
- IV-1 to IV-6 agree with FR-123, FR-125 and FR-126.
- QS-1 to QS-13 cover every item the FRs defer to QSpec.
- The ADR-014 TR-3, A-2, A-4, B-1 and B-5 amendments match IV-1, IV-4 and §1.
- The ADR-016 §6 amendment matches §3 and §5.
- ADR-013 O-16 and O-24 agree with FR-127 on `ProofBasis`, `Checks{0}` mapping to `kani_vacuous_proof`, the four new causes and the stopped-run row.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The ADR-011 amendment adds S6c and E10 but leaves the rows that S6c's refutation path runs through unchanged. (a) The S8 row still says parity is "compared with the S6b result". (b) The E9 row still "selects the function by `QualifiedName`, then calls the S6a executor", and a model trace replays through `ModelSystem`. (c) The new S6c to S7 arrow has no edge id, where every other arrow has one. (d) T-13, the driver row, describes only the Kani path, while DS-4 gives the driver EN-1. (e) The admission invariant "S5, S6a and S6b admit only S4 outputs" omits S6c. (f) The module table has a `simulation` row and no `model_check` row. (g) The S6c row says S6c outputs "one FR-331 terminal record per item", but FR-127 settles a `Violated` item `refuted` only after E9 replay, so the record of a refutation is written after S8, not at S6c. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:180,197,199,265,582,896,1408 |
| FND-002 | low | ADR-013 O-24's Public type cell is amended for `ProofBasis`, but its list of `ProofResultEnvelope` typed inconclusive causes still reads "`kani_vacuous_proof`, `replay_parity` or `replay_refused`" without the four new causes. The O-16 amendment also never says that a model counterexample whose replay disagrees or refuses maps to `replay_parity` or `replay_refused`, which ADR-018 V-6 and FR-127 state. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:49-59,826 |
| FND-003 | low | ADR-018 §5 and §7 describe model-trace replay as a new `ReplaySource::ModelTrace` arm of layer-6 `replay`, and ADR-011 E9 has the driver call `qsl_replay::replay`. FR-128 specifies a separate entry, `qsl_replay::replay_model_trace`. State which it is: an arm dispatched by `replay`, or a third facade entry beside `replay` and `call_site`. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:259,263-272; spec/functional/FR-128-replay-a-model-counterexample.md:31-33 |
| FND-004 | low | ADR-018 cites ADR-021 twice ("ADR-021 specifies it", and IV-7's "Reductions are specified in ADR-021"). ADR-021 exists only on the stacked branch of #568. If #562 merges first, main carries a reference to an ADR that does not exist. Cite the ticket in References until #568 lands, or merge in stack order. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:407,474 |
| FND-005 | low | The `spec/spec.md` index row for ADR-018 lists property forms, semantics, engines, fairness, counterexamples and sequencing. It omits deadlocks (§10) and interval operators under infinite-trace (§11), two of the record's decisions. | spec/spec.md:564 |

## Verdict

The ids are internally consistent. FND-001 is the one real gap: the
ADR-011 stage DAG is half-amended, and its S6c row disagrees with FR-127 on
where a refutation's terminal record is written. The rest are small
alignment fixes.

## New findings (disposition pass 1)

Reviewed at ed7bcc8b (first seen at f7180085; unchanged at the head).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The ADR-013 O-16 amendment says a model-check run stopped by a run limit, including `max_automaton_states`, "maps to incomplete with `ResourceExhausted`, `TimedOut` or `Cancelled`", and the O-16 incomplete row keeps that cause. ADR-018 IV-6 and FR-127's V-7 row settle a reached limit as `Incomplete(LimitReached{limit, value, setting})` and a cancelled run as `Incomplete(Cancelled)`, written `cancelled`. `ModelCheckLimits` has no time member, so `TimedOut` has no producer. Two readers would write different causes. State the two causes FR-127 uses. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:428-431 |

## Dispositions

Round 1, reviewed at ed7bcc8b (re-checked at the #562 head; first drafted at f7180085). This file's findings were posted on QSL-366 under the id SR-950.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 204b4db1 |
| FND-002 | fixed | 204b4db1 |
| FND-003 | fixed | 204b4db1 |
| FND-004 | fixed | 204b4db1 |
| FND-005 | fixed | 204b4db1 |
