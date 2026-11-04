---
id: SR-2273002
title: "QSL-1633 gap analysis of PR #2273"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2273 head f1e3e4c40; qsl-eval/src/value/expression/evaluate.rs; qsl-eval/src/value/expression/evaluate/node_source.rs; qsl-eval/src/value/expression/evaluate/invariants.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: references
---

## Summary

Ticket: QSL-1633; PR 2273. Planless, diff-scoped. Plan completion: not assessed.

This is a refactor-only slice with no new requirement behaviour. The changed callee paths are `Call`, `ImportedCall`, the dispatch precondition, the selected name and the body, and the `DispatchGuard` body. The existing qsl-eval lib and `it` suites already cover them, through the dispatch, imported-call and precondition probes in invariants.rs and the it suites. The PR adds no criterion, and no criterion's coverage changes. The new code is `NodeSource`/`Callee`, which serves step 1 of QSL-1625 under FR-390. Its one implementor is exercised by every machine test, and no stub or unreachable branch was added. FR-390-AC-16's reader path is still unimplemented, as expected for step 1, and its gated tests remain `#[ignore]` under QSL-1625. That is open work, not a gap introduced here.

## Verdict

**PASS**: no coverage gaps or orphaned code introduced. The trait-shape concern against FR-390-AC-16 is recorded in the code-review artifact, SR-2273001 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
