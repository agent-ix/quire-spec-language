---
id: SR-725
title: "QSL-245 integrity analysis of PR 487 (catalog 1-draft.8 adoption)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@913375e2255351626fdcb6f318b130938d351df9; spec/functional/FR-001-read-exact-source.md; spec/functional/FR-010-report-native-outcomes.md; spec/functional/FR-018-construct-native-runtime-inputs.md; spec/functional/FR-026-run-standalone-native-workflow.md; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md; spec/spec.md; spec/tests.md; spec/test-cases/TC-424, TC-425, TC-428, TC-430, TC-431, TC-500; read-only context spec/functional/FR-106-admit-snapshots-and-invocations.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
---

## Summary

Ticket: QSL-245 (PR agent-ix/quire-spec-language#487). This analysis checked
consistency, completeness and atomicity across the touched artifacts.

- Every new or changed AC has one TC, and the TC scope lines agree:
  FR-001-AC-11 is TC-424 step 6, FR-096-AC-13 is TC-428 step 5, and
  FR-096-AC-14 is TC-500.
- TC-500 has a `tests.md` row, and its `verifies` edge targets FR-096.
- The "twelve kernel causes" count matches the key table: two existing rows
  plus ten new ones.
- `quire validate` over the 13 touched files exits 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-096-AC-7 says "For each cause in the key table, `catalog_fields()` holds exactly the keys". The table now has twelve kernel `Refusal` rows, and `Refusal` does not implement `CatalogCoded`. The kernel fields come from `kernel_refusal_record` (AC-8). AC-7 is still reported as passing locally. Scope AC-7 to the family causes. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:325; spec/spec.md:510 |
| FND-002 | low | FR-106 condition 7 says a blank label moves to `invalid_source_identity` "when the catalog adds the code". Revision `1-draft.8` has now done that. FR-106 is outside this PR's lane and correctly not edited, but the follow-up has no owner. Route it to the FR-106 lane. | spec/functional/FR-106-admit-snapshots-and-invocations.md:174-181 |

## Verdict

Approve. Both findings are low. FND-001 is a small wording fix to AC-7 that
can go in this PR. FND-002 needs routing, not an edit here.

## Dispositions

Verified against `git diff 913375e2..e0d028af` on 2026-09-26.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e0d028af: FR-096-AC-7 and TC-428 step 3 now cover only the `CatalogCoded` rows; AC-8 covers the twelve kernel rows. |
| FND-002 | deferred | FR-106 is in agent-a's lane and this PR correctly leaves it alone. The move of condition 7 to `invalid_source_identity`/`blank-label` still needs routing to the FR-106 owner. |
