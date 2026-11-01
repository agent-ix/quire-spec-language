---
id: SR-1988006
title: "Gap analysis — QSL-588 timed statistical sampling PR 1998"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-253-AC-1..4, FR-254-AC-1..4; qsl-analyze/src/statistical/, qsl-analyze/tests/it/statistical_model.rs, qsl-eval/src/simulation/model/timed/, qsl-eval/src/simulation/sampled/timed.rs, qsl-eval/tests/it/timed_subject.rs, qsl-semantics/tests/it/timed_declarations.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-253", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-254", type: references }
---

## Summary

Focused gap analysis against FR-253/254 and the TC-708/709 implementation tests. The timed sampling behavior is present for stochastic delays, but free-delay races still lack the FR-253 scheduler-choice/MDP behavior and FR-253-AC-2 has no criterion trace binding.

## Verdict

**FAIL** — FR-253-AC-4 behavior is missing, and FR-253-AC-2 is untagged in the computed test mapping.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A racing free delay exits as `NotStochastic` instead of yielding the scheduler-choice race required by FR-253-AC-4; no test exercises the required MDP outcome. | FR-253-AC-4, qsl-eval/src/simulation/model/timed/race.rs:342-346 |
| FND-002 | high | FR-253-AC-2's behavior is asserted in the AC-1 test, but that test lacks an FR-253-AC-2 tag; TC-708 alone does not bind a criterion. | FR-253-AC-2, qsl-eval/tests/it/timed_subject.rs:460-462 |

## Coverage

- Reconciliation: manual focused check after `quire matrix --scope . --format json` and `quoin matrix --repo . --json` produced no output for more than three minutes; both were interrupted. The grep-backed check of local `#[trace]` tags confirms FR-253-AC-2 and FR-253-AC-4 have no criterion tag.
- Focused criteria: FR-253-AC-1 tested; FR-253-AC-2 behavior present in the test but untagged; FR-253-AC-3 tested; FR-253-AC-4 not implemented/tested. FR-254-AC-1..4 each have tests tagged under TC-709 and their FR criterion IDs.
- Focused binding review: `stochastic_rpc_exposes_checked_delay_windows` asserts FR-253-AC-1 and AC-2, but binds only AC-1; FR-253-AC-3 has declaration and race refusal tests; FR-254 tests bind AC-1..4.
- Repository-wide matrix counts, stale tags, reverse-gap inventory and stub counts: unavailable because the required matrix computation did not return. The master requirements root has no `id`, so no root `references` relationship was asserted. This artifact makes no whole-repository completion claim.
- Plan completion: not assessed
- Semantic review: skipped; not requested.

## Bindings

- `TC-708` → `FR-253-AC-1`: correct.
- `TC-708` → `FR-253-AC-2`: missing criterion tag; the AC-1 test body includes the path probability assertions.
- `TC-708` → `FR-253-AC-3`: correct.
- `TC-709` → `FR-254-AC-1`, `FR-254-AC-2`, `FR-254-AC-3`, `FR-254-AC-4`: correct.
