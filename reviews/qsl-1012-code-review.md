---
id: SR-16633
title: "Code review of QSL-1012 Quantity<ms> probabilistic encoding (PR #1431)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language PR #1431 at 0122d306f; qsl-eval/src/simulation/model/encode.rs; qsl-semantics/src/model/domain_package.rs, intake.rs, index.rs, check/assemble.rs; qsl-analyze/tests/it/prob_world.rs and statistical_model.rs; qsl-semantics/src/check/assemble/tests.rs; qsl-replay/src/spine.rs and spine/refinement.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: reviews
---

## Summary

Independent Rust review inspected the changed codec, intake, assembler and test paths against FR-056-AC-42, FR-120-AC-16, FR-185-AC-8 and FR-189-AC-15. The exact rational/unit encoding and quantity integration passed review. The first fix round added a typed path but initially omitted the declaration identity; the second round added the member URI, included it in the diagnostic message and added a test for the member/path. The reviewer confirmed that finding resolved. DomainPackage records do not retain a typeRef source span; the refusal identifies the member by URI and path.

Verification on the rebased head: fmt check and Clippy pass; FR-120/FR-189 validation passes; the scoped matrix reports the two selected criteria tagged; focused Quantity tests pass. The affected crate test command passed qsl-analyze (736 tests), qsl-eval (704), qsl-replay (843 unit tests plus integration targets), and qsl-semantics (765 passed, 1 ignored), after correcting the one stale expected diagnostic message exposed by the full run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The first implementation reported an unresolved unit typeRef at the whole model selection and did not identify the declaring member/path. | qsl-semantics/src/check/assemble.rs; FR-056-AC-42 |

## Verdict

PASS after the second fix round. The refusal now includes the exact member declaration URI and typed member path, and `assembly_message` emits both with the missing unit.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Commits `ea0f880c1` and `4a2e705c7` add the member path and declaration identity; independent re-review of the resolved implementation found no remaining issue. |
