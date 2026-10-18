---
id: SR-12126
title: "Gap analysis of quire-spec-language PR #954: TemporalTrace evaluator (FR-327, FR-328, FR-329, FR-125 SM-8)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a193cde9cde16886dd343352eafe7b3f38d93336; PR #954 diff scope: qsl-eval/src/temporal/**, qsl-eval/tests/it/temporal_trace.rs, qsl-semantics/src/check/temporal_clause.rs; spec FR-327, FR-328, FR-329, FR-125, FR-338, ADR-018 IV-2"
review_set: subset
---
# Gap analysis of quire-spec-language PR #954

## Summary

Ticket: QSL-509 (T3a). Plan completion: not assessed (planless). This
review is read-only. Each AC was checked against its tagged test and the
code path by reading them. No test was run.

AC to test bindings, all checked against the AC text:

- FR-327-AC-1 to AC-5: TC-837 and TC-847.
- FR-328-AC-1 to AC-6: TC-838, TC-847 and TC-848.
- FR-329-AC-1, AC-2, AC-5, AC-6 and AC-7: TC-839, TC-847 and TC-848.
- FR-125-AC-3, AC-5 and AC-6: TC-520 and TC-537.

FR-329-AC-3 is half covered. The empty-loop test is untagged and the
`Lasso::Model` half is not exercised. FR-329-AC-4 is not covered. Both are
correctly left untagged.

FR-125-AC-1, AC-2 and AC-4 (model behaviours) are outside this slice.

The ticket asks for "Add its FR-255 row (temporal.*)". This is not needed:
the evaluator adds no temporal limit of its own and charges the existing
`work_units` accounting counter, which FR-255 already lists.

## Verdict

No blocking (high) gaps. One medium and two lows, recorded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-329's `Lasso::Model` input is not built: model-step reconstruction, the fairness check by QSpec FR-362's lasso rule, and the unfair-lasso refusal. Its ACs (FR-329-AC-3 second half, FR-329-AC-4) have no code or test. `TemporalTrace` has only `Finite` and `Lasso{prefix, loop_len}`, and the only fairness behaviour is `MissingFairnessPremise`. QSL-509 says it delivers "a fair lasso". QSL-858 (T3b) covers only the `src/temporal` deletion. No ticket is named as owning the `Lasso::Model`/fairness half. Name an owning ticket, or land it here. | qsl-eval/src/temporal/input.rs:75-94; qsl-eval/src/temporal/engine.rs:271-279 |
| FND-002 | low | FR-327 "Undefined evaluation": a window aggregate atom with `insufficient-data` settles O-16 `undefined`, never `UndefinedEvaluation`. This has no path. `AtomReading` has only `True`/`False`/`Undefined(cause)`, so an aggregate letter could only report the cause as an `UndefinedEvaluation` violation. FR-327 has no AC for it. | qsl-eval/src/temporal/letters.rs:145-154 |
| FND-003 | low | `Incomplete` reports `ChargePoint::GraphExpand` for every temporal visit (`const VISIT`, "the nearest graph.expand"). FR-327 returns "`Incomplete` with its charge point". A temporal budget stop therefore names the model-exploration point, which misleads anyone reading the charge point. quire-exact has no temporal point, so it needs one, or the spec should say temporal work reports `graph.expand`. | qsl-eval/src/temporal/engine.rs:41-44 |
