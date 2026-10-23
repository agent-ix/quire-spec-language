---
id: SR-16316
title: "spec-review of an HP-5 item settles through settle_existential (quire-spec-language PR 1327)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f1320e41b9; spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md; spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md; spec/functional/FR-182-settle-a-hyper-verdict.md; spec/functional/FR-181-check-a-single-existential-claim-through-the-possible-family.md; spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md; spec/test-cases/TC-1198-an-hp-5-item-settles-through-settle-existential.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-163
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-181
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-282
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1198
    type: references
---

# Spec review of an HP-5 item settles through settle_existential (quire-spec-language PR 1327)

## Summary

Ticket: STD-359. One spec-review pass over `git diff origin/main...f1320e41b9` (agent-ix/quire-spec-language#1327), checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (derived by hand), id uniqueness across open PR heads, and mergeability. Gate: `QSPEC_DIR=<worktree of QSpec origin/main> make check-spec-validation` exit 0.

Examined, clean:

- The circle in the ticket is closed: FR-163 Description now says `Witnessed` for HP-5, FR-163/FR-181/FR-182 all point HP-5 settlement at FR-169's `settle_existential`, FR-182's `settle_hyper_check` settles no HP-5 item, and FR-181's "Remaining work" paragraph is removed because FR-169-AC-13, FR-163-AC-8 and FR-181 Replay now specify every half it listed (a deferral removed, not added).
- Hand-derived vectors: FR-169-AC-13 settlements follow the state-graph rows by name (Accepted to V-9 proved, Rejected to V-6 `CertificateRejected`, PackageMismatch to V-6 `StaleDependency`, Stopped at `max_states` 1 to V-7 `LimitReached`); the loop-removed witness follows FR-181-AC-4 and FR-128; `Trapped` follows FR-181-AC-2 to `refuted`, `closed-scope`. FR-163-AC-8 puts the package comparison first, so the unsorted-keys `NonInterference` certificate returns `PackageMismatch`, not `Malformed` (FR-282 order). FR-182-AC-13 matches the new V-6 row.
- No deferral or follow-up wording in the diff.
- Ids FR-163-AC-8, FR-169-AC-13, FR-182-AC-13 and TC-1198 are unused on origin/main and on every other open head.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The `proved` vector writes the basis as `Proved{basis: Witness{[Explored, …]}, certification: Certified}` and TC-1198 step 1 repeats the ellipsis. The sentence "one source per initial state" fixes the length only by reference to FR-181-AC-1, so a test cannot compare the record literally. Fix: write the list for the vault fixture (one `Explored` per initial state, with the number of initial states) or say the test asserts length and element kind. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md | missing-requirement |

## Verdict

Mergeable: only low findings remain.

Conflicts with open heads: #1311 (FR-169), #1346 (FR-169), #1330 (FR-163), each textual in rows or neighbouring hunks; clean against origin/main. The PR body mentions #1311 only.
