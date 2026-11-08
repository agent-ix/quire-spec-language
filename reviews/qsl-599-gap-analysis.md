---
id: SR-599002
title: "gap analysis of QSL-599 (FR-299-AC-1..4, FR-285-AC-6, FR-276-AC-4)"
type: SpecReview
analysis: gap-analysis
date: 2026-11-08
scope: "agent-ix/quire-spec-language; branch qsl-599-lc11 git diff ff0b7551d..1477a62db; FR-299-AC-1 to AC-4 (TC-784), FR-285-AC-6 (TC-769), FR-276-AC-4 (TC-757); qsl-analyze/src/analyze.rs run/settle"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-299
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: reviews
---
# SR-599002: gap analysis of QSL-599 (FR-299-AC-1..4, FR-285-AC-6, FR-276-AC-4)

## Summary

Ticket: QSL-599. Plan completion: not assessed. Every claimed AC now carries a tag: FR-299-AC-1..4 in `value_family_end_to_end.rs`, FR-285-AC-6 on three tests in `execution_tests.rs`, and FR-276-AC-4 in `analyze.rs`. Semantic review was done by hand: each tagged test was read against its AC and against the production path. FR-299-AC-1..4 are asserted as written. Known pending items, not defects: FR-286-AC-7 (the spec is being amended) and FR-280-AC-2 (EN-1).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Code gap against FR-276. `run` checks `cancel.cause()` only before an item's work starts (`analyze.rs:1131`). After the work returns it settles the engine's report through `settle` (`analyze.rs:1138`) without reading the handle's cause, and `EngineReport` has no cancelled variant. So an item that is in flight when the handle is cancelled keeps whatever record its engine returns, not `incomplete` with cause cancelled. FR-276 says "each item still open when the handle is cancelled settles FR-331 incomplete with the cancellation cause" and "the operation that holds the handle shall read the handle's tripped() cause". FR-276-AC-4 as written, where the engine blocks on the second item until the cancel, would therefore fail. The QSL-599 test passes only because its variant moves the block into item 0. | qsl-analyze/src/analyze.rs:1131-1138 |
| FND-002 | medium | FR-285-AC-6's `CallFailure::Limit` naming `prepare.compile_work_units` exits 22: no test maps that setting. The tagged test uses `prepare.code_bytes`, and `execution.rs:477` (the CompileWorkUnits arm) is not covered by any exit assertion. | qsl-replay/src/spine/call/execution_tests.rs:387 |

## Verdict

**CONDITIONAL / not mergeable while FND-001 stands.** FR-299-AC-1..4 have no gap. FR-285-AC-6 needs a compile_work_units case, for example a stand-in whose prepare refuses `Setting::PrepareCompileWorkUnits`. FR-276-AC-4 needs the post-work cancel read in `run`, plus a literal test in which the second item's work blocks until the cancel.

## Coverage

Plan completion: not assessed. `quoin matrix` was not available within the review window (the full-repo matrix run exceeded 300 s), so tag presence was checked by grep over `#[trace(..)]` and the semantic check was done by hand. No stubs were added. The diff adds no production code, so no ownerless code.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The new post-work read in `run` uses `cancel.cause()`, not `cancel.tripped()`. FR-276 Behavior says the operation "shall read the handle's `tripped()` cause and return its cancelled failure". quire-exact documents `tripped` as follows: "A handle cancelled after the last charge of an operation is not tripped, and that operation's result stands." With `cause()`, an engine that finished its last charge before the cancel landed has its settled verdict (proved, refuted with a counterexample) replaced by `incomplete`/Cancelled. That item was settled, not open, so FR-276 Outputs ("items already settled keep their records") says it keeps its record. The same crate already uses `tripped()` for this decision (`qsl-analyze/src/contract_check/mod.rs:168`). Fix: read `cancel.tripped()` after work. Keep `cause()` for the pre-work check, because an already-cancelled handle must refuse before the first charge. Make the FR-276-AC-4 test's blocking engine charge via `cancel.poll()` instead of spinning on `cause()`, so the second item trips the handle the way a real engine's meter does. | qsl-analyze/src/analyze.rs:1138-1148 |

## Dispositions

Round 1, against fix commit 14ebd238a (head; be2d3d5a2 and 6d649c8d5 are `=` to the reviewed commits under `git range-diff`).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
