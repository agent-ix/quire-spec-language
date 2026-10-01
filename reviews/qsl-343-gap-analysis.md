---
id: SR-921
title: "QSL-343 gap analysis of PR 548 (drop lock-vs-evidence artifact digests)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3e95fb6a6f9d749b8fcd815f61d2814d7f41ea27; qsl-package/src/emit.rs; qsl-package/src/checked_v2.rs; qsl-package/src/checked_v2/tests.rs; qsl-package/src/emit/tests.rs; tests/it/config_version_spine.rs; FR-093 (AC-16, deleted AC-17); FR-087-AC-3; FR-087-AC-14; ADR-015 D-1; TC-253; TC-416; TC-421"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: reviews
---
## Summary

Ticket: QSL-343. PR: quire-spec-language#548 at 3e95fb6a.

The ticket's items each have a matching change. own_evidence's digest loop is
removed. `diagnostics_catalog` is made private and its re-export dropped. The
`insert_dependency_package` call is fixed. Tests that only existed for
staleness, and their helpers, are deleted. The remaining tests supply only
required features (plus domain packages and dependencies on the import-view
path). The only test deleted is `diagnostics_catalog_matches_the_emitted_reference`,
which backed AC-17, and AC-17 is deleted with it. No surviving AC lost its
test.

The remaining stale_dependency coverage stays bound and meaningful:
- FR-087-AC-14 (ADR-015 D-1 step 5): e4_refuses_a_stale_dependency_and_a_conflicting_diamond, trace correct.
- FR-087-AC-3 condition 3: refuses_when_the_pinned_package_id_disagrees and
  refuses_when_the_pinned_version_disagrees, trace correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | After AC-17 is deleted, no AC or test states which `diagnostics.catalog` the v2 emission writes. The value is QSpec `quire.native.diagnostics/v1` at `1-draft.8`, and only the emit.rs doc comment states it. IR's reader checks only its shape and domain. The FR-093 fixture comparison excludes `diagnostics`. AC-17's accessor half was ceremony, but its catalog-revision half pinned a real wire value, and that value is now unowned. A wrong revision would ship unnoticed. Fix: one FR-093 AC sentence ("the emission writes `diagnostics.catalog` = QSpec `quire.native.diagnostics/v1` at `1-draft.8`") plus one assertion in an existing TC-416 emit test, or a ruling that the member is outside spec. | qsl-package/src/emit.rs:32-33; qsl-package/src/emit.rs:103-115; qsl-package/src/emit.rs:920 |

## Verdict

Complete against the ticket. One low gap: the emitted catalog revision
became an unowned value when AC-17 was deleted.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The FND-001 fix asserts the catalog's `digest` literal (`94580e10...d95d`) in `the_lock_selects_the_catalog_definitions`. FR-093-AC-17 deliberately leaves the digest out. emit.rs:115 calls the digest "Informational; no reader verifies it yet", and after IR-457 nothing verifies it. The assertion fails only when someone updates the digest. It is a digest pin tested for its own sake, which the owner's rules forbid. Fix: drop the `digest` member from the assertion, and assert authority, identity, revision and digest_domain, the four members AC-17 names. | qsl-package/src/emit/tests.rs:327-336 |

## Dispositions

Round 1, reviewed at 985bb85bcfd85683f898a4131f3dceebc52678fb.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 05051fa3: FR-093 states the emitted `diagnostics.catalog`, new FR-093-AC-17 and TC-416 step 9 back it, and `the_lock_selects_the_catalog_definitions` (traced TC-416, FR-093-AC-7, FR-093-AC-17) asserts the wire member and the Verified read-back. Reusing the id AC-17 is acceptable: the old AC was never released, no live spec or code references it, and the new AC pins the same catalog-revision value the old AC's last sentence asserted. |

Round 2, reviewed at c0991f5d6fe6cf914bcac23ba5a1ca489a64f447. No new findings.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-002 | fixed | c0991f5d: `the_lock_selects_the_catalog_definitions` asserts `authority`, `identity`, `revision` and `digest_domain` separately, which are exactly AC-17's members. The digest literal is gone. |
