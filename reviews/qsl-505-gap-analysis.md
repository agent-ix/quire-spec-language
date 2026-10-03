---
id: SR-1253
title: "QSL-505 gap analysis of PR #612 (FR-106-AC-9, TC-464 step 6, FR-106-AC-3, TC-465 row 38)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5402fab068aaf55dc893edf0d9bbd329f30410fd; PR #612 diff against origin/main (merge base c54e3595); spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/test-cases/TC-464-snapshots-and-invocations-admit.md; spec/test-cases/TC-465-admission-refuses-each-input-defect.md; spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
---
## Summary

Ticket: QSL-505 (slice PC1). PR: quire-spec-language#612. This is a manual
check of each AC against its tests (the quoin gap-analysis method, with no
plan bundle for this slice). It covers only the QSL-505 change. The #610
revert the branch also carries is SR-1252 FND-001.

Trace, per unit:
- FR-106-AC-9 / TC-464 step 6. Four tests carry
  `#[trace("TC-464", "FR-106-AC-9")]`:
  `pre_call_invocation::a_precondition_over_an_invocation_reads_only_the_pre_side`
  (verdict equals `PreCall`, documents read are exactly `[invocation, pre]`,
  no frame check),
  `pre_call_invocation::a_precondition_over_an_invocation_ignores_a_missing_or_malformed_post`
  (post snapshot absent, and post snapshot bytes not JSON),
  `state_clauses::attempt_update_precondition_admits_an_authorized_change`
  (`post` is `None`), and
  `state_clauses::precondition_does_not_require_self_in_the_post_snapshot`
  (self deleted outside the frame still admits, with no post and no delta).
  The bindings are correct, and they cover every sentence of the AC. The
  oracle is strong: reading post, running check 11 or admitting the post
  snapshot each makes the first test fail (see SR-1252).
- FR-106-AC-3 / TC-465 row 38.
  `tc465_row38_result_for_a_no_result_operation_refuses_unknown_member` now
  selects `ProbeHolds`, and the AC and row 38 match the code. The binding is
  correct.
- FR-106-AC-5. The four moved `attempt_update_refuses_*` tests still fail at
  check 11 with the same code, cause and object, now through a postcondition.
  The bindings are correct.
- FR-106 Behavior, the precondition-over-`Invocation` paragraph, and check
  1.5's carve-out. These are implemented, but the member half has no test
  (FND-001).
- FR-106 check 9's "(for a postcondition)" qualifier, as FR-115 uses it for a
  `Frame` run. Its only test moved off the path that implements it (FND-002).
- Underspecified code: none. `DocumentKind::InvocationCall`,
  `admit_pre_call_invocation` and `check_operation` each trace to FR-106's
  Behavior or to check 5.

## Verdict

Changes requested: two medium findings. Both need tests, and FND-001 also
needs a sentence added to AC-9. AC-9 itself is fully and strongly covered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-106 Behavior says admission "SHALL neither require, read nor check the invocation's `post`, `result`, `created` or `deleted` members", and check 1.5 says those members are not required for a precondition selected by `Invocation`. No test selects a precondition by `Invocation` with any of those members missing or ill-formed. AC-9 names only the post snapshot (absent or malformed), not the members. So if `post` were put back into `DocumentKind::InvocationCall`'s required list, or `read_invocation_call` read `result`, every test would still pass. The case row 38 used to cover (`ReachesTarget` over a `probe` invocation whose `result` is `{"boolean": true}`) now admits, and no test asserts that either. Fix: add a sentence to AC-9 and TC-464 step 6 and back it with tests. The forbidden-parent-change invocation with `post`, `result`, `created` and `deleted` removed, and the probe invocation with `result` `{"boolean": true}` selected for `ReachesTarget`, should each admit and give the `PreCall` verdict. | qsl-semantics/src/model/observation/document.rs:282-291; qsl-semantics/src/model/observation/document.rs:645-687; spec/functional/FR-106-admit-snapshots-and-invocations.md:163-172; spec/functional/FR-106-admit-snapshots-and-invocations.md:201-205 |
| FND-002 | medium | The `post_self = false` branch of `admit_invocation_documents` lost its only test. Before this PR, `precondition_does_not_require_self_in_the_post_snapshot` reached it through `admit_operation` for a precondition. The test now goes through `admit_pre_call_invocation` and never calls `admit_invocation_documents`. The branch is now reached only by `admit_frame_invocation`, and every `Frame`-run fixture (spine/clause/tests/frame.rs, frame_replay.rs, witness.rs) keeps self (`child`) in post. So changing `false` to `true` at observation.rs:1562 would pass every test, even though FR-106 check 9 requires self in post only for a postcondition and FR-115 runs checks 1 and 3 to 10 for a `Frame` run whose operation may delete self. Fix: add a TC-514 `Frame`-run case whose post deletes `child` (self). It should reach S6a and report the deletion (a `FrameViolation` naming `child` under `attemptUpdate`'s empty `deletes`), not `wrong-role-mapping` at admission. | qsl-semantics/src/model/observation.rs:1034; qsl-semantics/src/model/observation.rs:1076-1085; qsl-semantics/src/model/observation.rs:1562; qsl-semantics/tests/it/state_clauses.rs:4702 |
