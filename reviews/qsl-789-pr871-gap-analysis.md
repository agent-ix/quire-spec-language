---
id: SR-11269
title: "Gap analysis — QSL-789 FR-363 AC-1 to AC-6 (PR 871)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f80c8544db29f10047636e047510c61179c765e7; FR-363-AC-1..6, TC-1031 N01-N12, TC-1032, TC-1033, TC-1034; qsl-semantics/src/check, qsl-semantics/tests/it/relationship_navigation.rs, qsl-package/src/emit/tests/navigation.rs"
review_set: subset
---

## Summary

Ticket: QSL-789. A planless gap analysis of FR-363 against the tagged tests and the code in PR 871. No spec file changed in the diff, so spec-review was not run.

## Verdict

**PASS.** Each AC is backed by a test that asserts it:

- **AC-1:** N01 (Option and `present`), N02 (Reference), N03 (subtype receiver), N04 (Set, inverse end under `bidirectional`, a target end with a role), N05 (Bag), and the TC-235 M21 run 2 check.
- **AC-2:** N07 (two ends, then a field and an end, each naming every candidate) and N08.
- **AC-3:** N09 (both directions are refused; the inverse of `target-to-source` navigates) and N06 (`undirected`).
- **AC-4:** N10 (unbounded, ordered and undeclared ends, each with its typed bound) and N11 (direction is checked before the bound).
- **AC-5:** N12. Every refusal vector also asserts the StateModel family and the `state-model` catalog prefix.
- **AC-6:** the TC-1032 emission test (f1 to f3: member, argument, result form, no deref or project, IR reader Verified).

No stubs. The evaluator's `NavigateUnsupported` refusal is a deliberate, typed gap: QSL-795 covers evaluation.

The cross-check asked for finds no mismatch. On origin/main, TC-1033 and TC-1034 follow the near-end rule. `parts` and `home` are source-end roles navigated forward from `s1` and `p1`. `pump` is the target-end role navigated inverse from `t1` under housing-undirected. The bounds in rows 6 to 8 are the destination end's. The old review note about destination-role naming no longer applies. QSpec main includes the near-end alignment (#258), and its TC-197 Y08 ambiguity row, Y11 no-candidate row and Y12 lowering row agree with this implementation.

## Coverage

- Reconciliation: each AC was checked by reading FR-363, TC-1031, TC-1032 and the tagged tests directly. No `quoin matrix` was run (no-build review).
- Plan completion: not assessed.
- Changed criteria: FR-363-AC-1 to AC-6 are all covered.
- Evidence: the coder's logs show the all-features runs passing on this head: relationship_navigation (12 passed), tc235_m21, emission and qsl-eval. Nothing was re-run.
- Reverse gap: the routing of non-model field reads to the hook has no owning AC. It is recorded in SR-11268 FND-001.
- Semantic review: skipped (not opted in).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
