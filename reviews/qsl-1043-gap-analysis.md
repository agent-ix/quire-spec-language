---
id: SR-17010
title: "Gap analysis of QSL-1043 FR-127-AC-7 and FR-255 setting names"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #1436 (branch qsl1043-max-states, head 98b0baeb3); FR-127, FR-255, TC-522, TC-1161; qsl-replay/src/model_check/limits.rs, qsl-analyze/tests/it"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: references
---

## Summary

Ticket: QSL-1043. This is a planless gap analysis scoped to the PR's
criteria. The matrix was computed by `quire matrix --scope <worktree> --format
json` at head 98b0baeb3. FR-127-AC-7 is now `tagged`, bound to
`a_certificate_check_settles_accepted_rejected_or_stopped_with_the_named_setting`
(closure_certificate.rs:818). That trace carries both `TC-522` and
`FR-127-AC-7`, and TC-522's scope lists FR-127-AC-7. The doc comment's AC ids
name only FR-127-AC-7, the AC the test is tagged with. The other edited tests
changed only expected strings; their tags and doc-comment ids are unchanged.

Reverse gap: the code change is one match block whose names FR-255's engine
table owns. There is no unowned behaviour and no stub. The test is not
hollow: two mutations turned it red (see SR-17009).

## Verdict

**CONDITIONAL.** The PR's one criterion, FR-127-AC-7, is traced and backed by
a passing test that can fail. One low spec-drift finding sits outside the diff.
The quality of the test's binding to the AC is covered in SR-17009 (FND-001
to FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-203's Use case says an exact run's `max_states` stop names "`max_states`, its value and `max_states`", and TC-638 step 2 expects the setting `max_states`. FR-255's engine table gives EN-5's product bound the setting `exact_prob.max_states`. The rename makes this drift visible: the spec text should name `exact_prob.max_states` | spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md:37-38; spec/test-cases/TC-638-exact-limits-and-settlement.md:32 |

## Coverage

- FR-127-AC-7: tagged (1 binder), test passes. In scope.
- FR-127-AC-1, AC-4, AC-16: tagged. Context only.
- FR-255-AC-12, AC-13, AC-14: untagged. Context only, not scored against this
  PR. AC-13's settings-operation clause (`model_check.max_states=3` raises the
  limit) needs the `model_check.*` rows that QSL-1026 adds to
  qsl-foundation's `Setting`. Tagging AC-13 here would overclaim.
- Semantic review: covered by the code-review method (SR-17009), not run
  separately.
- Plan completion: not assessed
