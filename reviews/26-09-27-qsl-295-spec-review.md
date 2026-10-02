---
id: SR-757
title: "QSL-295 spec review of PR 502 (FR-109-AC-6, TC-468 step 6)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-109-run-a-state-clause-through-the-spine.md; spec/test-cases/TC-468-spine-clause-run-reports-typed-dispositions.md; context read: spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:204,711,717,867"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-468
    type: reviews
---
## Summary

Ticket: QSL-295. PR: quire-spec-language#502.

Sound:
- The coder was right that main had no I3 AC. FR-109-AC-1 to AC-5 on
  main never mention the I3 source.
- FR-109-AC-6 restates Outputs :81-83 faithfully. The body's identity and
  digest are the source, and the original's identity and digest are the
  extraction. It adds that a program source carries none. Each clause is
  concrete and testable, and each has a test.
- TC-468 step 6 and its expected result match AC-6, and its scope line now
  reads AC-1 to AC-6.
- The Status edit (:173-175) replaces the pending note with where the input
  lives. It is accurate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-109 does not say what happens when the extracted clause's declared language is not `ix:native`. ADR-011:867 puts the native join, including the language, in the caller, and the root join refuses `unknown_language` (src/mapped.rs:185-194). Without a rule, FR-109 lets the replay facade and the CLI give different results for the same document. Fix: add a Behavior bullet. If the unit is an I3 extracted source whose declared language is not `ix:native`, the entry SHALL report stage `compile`, `unknown_language`, carrying the extraction's original. Extend AC-6 and TC-468 step 6 with that case. | spec/functional/FR-109-run-a-state-clause-through-the-spine.md:49-51,157; spec/test-cases/TC-468-spine-clause-run-reports-typed-dispositions.md:38-41,67-71 |

## Verdict

Request changes on FND-001. AC-6 and TC-468 step 6 are otherwise correct and
testable.

## Dispositions

Round 2.

| FND | Outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-109 has a new Behavior bullet (:94-98): a non-`ix:native` I3 source reports stage `compile`, `refusal`, `unknown_language`, carries the extraction's original, and compiles, admits and evaluates nothing. FR-109-AC-6 (:162) and TC-468 step 6 (:38-42, :68-74) add the `ix:formal` case with exit 20 and no `package_id`. The case is testable and tested. `unknown_language` is the catalog code the CLI join emits (qsl-foundation/src/diagnostic.rs:192; docs/native-error-codes.md:40). |
