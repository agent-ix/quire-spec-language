---
id: SR-11756
title: "Code review of quire-spec-language PR #913: link replay tests and FR-115 trace tags"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@749b893a54c358601d912ab99aae6601654feeac; qsl-replay/src/spine/clause/tests.rs, qsl-replay/src/spine/clause/tests/link_replay.rs, qsl-semantics/tests/it/relationship_snapshots.rs, spec/functional/FR-116-replay-a-frame-counterexample.md"
review_set: subset
---
# Code review of quire-spec-language PR #913

## Summary

Ticket: QSL-797 (closing slice). Code review with the rust-review lane over
`git diff origin/main...HEAD`: a new test module `link_replay.rs` (TC-1037
row 15), trace tags added to four existing tests, and one FR-116 Description
sentence. Read-only; no build or test run was made by this review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS, no major findings.

- `link_replay.rs` drives the real `replay_frame` over a compiled package
  whose `rehome` frame node is resolved through `resolve_operation` and
  `operation_frame`, so anchor, frame and occurrence come from the package,
  not from literals.
- The three tests discriminate: the same envelope settles
  `reproduced-with-evaluated-witness` with `modifies []` and `inconclusive`,
  `Verdicts { violation, success }` with `modifies [Pump-housing-Tank]`. The
  only variable is the frame, so the test cannot pass vacuously.
- The added-link test asserts the claim is kept as `p1`-`t2` `added` while the
  found change is the replay's own `p1`-`t1` `removed`, which is what "both
  changes as found" requires.
- The copied orders fixture is within the same repo, uses the same
  relationship identity form as `relationship_snapshots.rs`
  (`ix://test/orders/relationship/Pump-housing-Tank`) and the same pre/post
  housing instances as TC-1037's invocation I. It omits `RehomePre` and
  `Sys-composition-Pump`; neither affects replay of row 15. Not wrong.
- Rust lane: test-only code; `expect`/`unwrap`/`panic!` are confined to tests;
  no unsafe, no integer conversions, no async. The module is registered in
  `tests.rs`. Imports reuse `frame_replay` helpers rather than duplicating them.
