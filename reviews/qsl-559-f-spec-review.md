---
id: SR-12321
title: "Spec review of the QSL-559 PR #1202 fix-round edits to FR-209 and FR-216"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4d558634dbe71761b79097ebaedb2a0c83a5d87a; spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md, spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
---
# SR-12321: Spec review of the PR #1202 fix-round edits to FR-209 and FR-216

## Summary

Ticket: QSL-559. PR quire-spec-language#1202, fix commit 4d558634d. The fix round added one non-normative paragraph to each of two specs. Each paragraph sits between the AC table and Dependencies and cites QSL-968:

- **FR-209:** "A record-typed trigger `T` refuses `UnenumerableRecord` in the protocol program, so the ACs run over an `Int` range trigger until it lands."
- **FR-216:** "FR-216-AC-6's record-binder guard and its `lookup` membership read are not yet tested ..."

No SHALL statement or AC changed. Both files pass `quire validate`. A `Remaining work: <ticket>` paragraph in a spec has precedent in this repository (FR-093, FR-094, FR-100, FR-147). The review checked whether each note is accurate, and whether it changes the meaning of any criterion.

## Verdict

**PASS**, with one low finding. The FR-216 note is accurate: QSL-968 lists both clauses, and the test no longer cites the AC. The FR-209 note reads as a statement about the ACs themselves (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-209 note says "the ACs run over an `Int` range trigger until it lands". FR-209-AC-1 and AC-2 are written over the record trigger `Tick`. The note reads as narrowing those criteria to an `Int` trigger, when what it means is that the tests substitute one. It should say what is unbuilt: "the record trigger `Tick` of FR-209-AC-1 and AC-2 refuses `UnenumerableRecord`, and the tests use the `Int` range `Level` until QSL-968 lands." The criteria stay as written. | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:124-126 |
