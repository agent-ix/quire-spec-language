---
id: SR-14011
title: "EARS review of STD-234 new and changed requirement statements (PR #1190)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@b8b3c38bd2e3ca0d6073db552ab5fc315911ddfc; spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md, spec/functional/FR-252-hand-timing-obligations-to-code-generation.md, spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-391-name-a-population-in-a-state-clause.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-252
    type: reviews
---
# SR-14011: EARS review of STD-234 (PR #1190)

## Summary

Ticket: STD-234. Diff: QSL `git diff origin/main...b8b3c38`. Statements
examined:

- FR-251 Overflow: "An event that would exceed a buffer SHALL never be
  dropped: the obligations it affects SHALL settle ..." (unwanted-behaviour
  form, unchanged shape, conforms).
- FR-252 MonitorAgreement bullet (rename only, conforms).
- FR-252 TimestampContract bullet: "When a contract's clauses relate integer
  parameters that `timestamp_parameters` names, QSL SHALL write ..." (event
  form, conforms).
- FR-252 timing-clause bullet (FND-001).
- FR-391 Lowering: "Package emission (S4) SHALL emit the term as QSpec
  FR-322's population reference ..." (ubiquitous, conforms).
- FR-255 exception paragraph (descriptive, no SHALL, not a requirement
  statement).

`quire validate` over the seven touched files reports the two warnings
below at head and none for them on main.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-252 bullet opens with a definition ("A **timing clause** is a comparison `m <op> d` ...") and continues in the same sentence run into "QSL SHALL dispose a timing obligation ...". quire reports `ears:unclassifiable` and `ears:missing-subject` at line 79, both new in this PR. Fix: give the definition its own paragraph or sentence and start the requirement with "QSL SHALL dispose ...". | spec/functional/FR-252-hand-timing-obligations-to-code-generation.md:75-83 |

## Verdict

The new and changed SHALL statements conform except the FR-252 timing-clause
bullet, which mixes a definition into the requirement sentence and raises two
new validator warnings.
