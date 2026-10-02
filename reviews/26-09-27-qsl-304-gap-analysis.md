---
id: SR-766
title: "QSL-304 gap analysis of PR 506 (ConfigVersion fixture on the bound VersionNumber scalar)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1d58b150d62709540b21adcf474ead696251aebb; qsl-semantics/tests/it/model_operations.rs; qsl-semantics/tests/it/state_clauses.rs; spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md; spec/functional/FR-104-check-state-clauses.md; spec/test-cases/TC-458-spine-admits-model-operations-and-frames.md; spec/test-cases/TC-459-s3-checks-configversion-state-clauses.md (unchanged); spec/test-cases/TC-465-admission-refuses-each-input-defect.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-465
    type: references
---
## Summary

Ticket: QSL-304. PR: quire-spec-language#506 at 1d58b150. There is no plan
bundle; the scope is the ticket's own three asks.

1. **Flip the shared fixture to the bound `VersionNumber` scalar.** Done. See
   SR-765 item 1: all five `ConfigVersion` builders are retyped, plus the
   AC-3 `delta` parameter.
2. **Re-verify TC-458/459/460/462/464/466/469.** Their tests all run in the
   `it` binary, and my own fresh `make ci` at 1d58b150 exited 0 (93
   `test result: ok` lines, 0 FAILED, 229 `it` tests). Only one test's
   fixture had to change: `Sub::version`, a real widening that the flip made
   reachable (SR-765 item 2). No asserted outcome in those TCs changed, apart
   from `ValueType::Integer` becoming `Int[0, 1000]` where the spec already
   says `Int[0, 1000]`.
3. **Clear the deferral markers in FR-103/FR-104.** Done, and also cleared in
   TC-458, TC-465 rows 20/30 and the TC-459 row of `spec/tests.md`. A
   repo-wide sweep of `spec/` and `plan/` for `QSL-289`, `Unverified`,
   "still substitutes", "stand-in" and "deferred separately" finds no stale
   marker. Every remaining `QSL-289` mention is current (FR-120, TC-471,
   tests.md:638, spec.md:543). The TC-459 file itself never carried the
   marker. The deferral text left behind is in Rust doc comments (SR-765
   FND-001/FND-002), not in `spec/`.

TC-465 rows 20/30: the three new tests carry
`#[trace("TC-465", "FR-106-AC-3")]`, the same tag as the sibling row 19. They
cover both halves of row 20 ("root then child") and row 30's walk order
(root reported first). They go through the `ValueType::Int` interval arm, not
the lexical parse (SR-765 item 3).

Underspecified code: none added. The diff is test and spec only.

Semantic review: done inline with SR-765, because the diff is two test files
plus status text.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The ticket's deferral is cleared in `spec/` but not in code. The same "still substitutes the native `Integer`" deferral stays in two test-file doc comments, so the traceability picture disagrees with itself: the spec says the fixture is bound, while the test header says it is not. This is tracked as code fixes in SR-765 FND-001 and FND-002, and recorded here so the gap view is complete. | qsl-semantics/tests/it/state_clauses.rs:6-16; qsl-semantics/tests/it/model_operations.rs:41-43; qsl-semantics/tests/it/model_operations.rs:943-951 |

## Verdict

PASS with one low finding. The ticket's asks are all delivered and backed by
tests, and every spec marker the PR claims to clear is gone. The remaining gap
is stale code comments, fixed with SR-765.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e59e1ff8 — the stale code-comment deferrals are removed (SR-765 FND-001/FND-002), so code and spec now agree |
