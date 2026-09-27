---
id: SR-729
title: "QSL-245 delta integrity analysis of PR 487 fix round"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@e0d028afd95e05baf23062ffe7d22b9bc5f251a2; delta 913375e2..e0d028af; spec/functional/FR-001-read-exact-source.md; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md; spec/spec.md; spec/tests.md; spec/test-cases/TC-424, TC-428, TC-444; read-only context spec/functional/FR-100-run-a-named-function-through-the-spine.md; code src/complete/edit.rs, src/complete/editor.rs, qsl-cst/src/cst.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: reviews
---

## Summary

Ticket: QSL-245 (PR agent-ix/quire-spec-language#487). This reviews only the
new content in fix commit e0d028af. The merge of main (#484) is not in scope.

- New FR-001-AC-12 and the "Edit and binding refusals" section match the
  catalog. The catalog keeps `invalid_source_map` as a retained host code,
  its causes are not closed in `1-draft.8`, and its pinned meaning is
  "Correspondence, source binding or queried range is invalid".
- They also match the three call sites: `src/complete/edit.rs:110-124`
  (`EditPredecessor`), `qsl-cst/src/cst.rs:376-386` (`ForeignNode`) and
  `src/complete/editor.rs:302-310` (`RequestRevision`). All three emit a
  region at byte 0 today, and the PR marks the fix as planned.
- FR-001-AC-12 collides with nothing on origin/main or in open PRs #485 and
  #486.
- `quire validate` exits 0 on every file touched in 913375e2..e0d028af.
- The five committed SR files are byte-identical to the reviewer's.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-096 now says `Undefined::SumOutOfDomain` renders as `sum-out-of-domain` "in the table that spells the kernel undefined reasons (FR-100)". That table (FR-100:171-176) has no such row. FR-100:167 also still says "FR-096 spells no kernel undefined reason", which this text contradicts. FR-100 is agent-a's lane. Either route the row to the FR-100 owner, or reword FR-096 so it defers the spelling to FR-100. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:276-283; spec/functional/FR-100-run-a-named-function-through-the-spine.md:167-176 |

## Verdict

Approve. The one new finding is low: a cross-reference to a row that does not
exist yet in another lane's artifact.
