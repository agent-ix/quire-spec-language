---
id: SR-20261104074841
title: "Gap analysis — QSL-1384 authenticated temporal provenance"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2279 changed temporal emission, checked-spine evaluation and temporal tests; FR-050-AC-5; FR-050-AC-6; TC-150"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-050", type: reviews }
  - { target: "ix://agent-ix/quire-spec-language/TC-150", type: reviews }
---

## Summary

Ticket: QSL-1384 (child QSL-1446). Quoin and Quire matrix commands stalled without producing results, so this artifact records a manual acceptance-criteria-to-tests check for the changed temporal code and criteria; it does not claim a repository-wide matrix or reverse-gap result. Plan completion was not assessed.

## Verdict

**CONDITIONAL** — the changed success result does not retain the exact source locus required to verify the declaration join, and its new test does not assert that locus.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | The temporal provenance test checks only a nonempty declaration path, not the exact source locus required by the checked-route test case. | qsl-replay/src/spine/clause/tests/temporal.rs:2720-2743; TC-150 |

## Coverage

- Reconciliation: manual acceptance-criteria-to-tests check for the changed temporal path; `quoin matrix --repo . --json` and `quire matrix --scope . --format json` stalled without output and were interrupted.
- Plan completion: not assessed
- Criteria examined: FR-050-AC-5, FR-050-AC-6, and TC-150's checked-successor procedure and expected results.
- Evidence: no run evidence read.
- Changed behaviors inventoried: strict checked-package read-back, exact declaration identity join, trace binding authentication, authenticated selection reporting, and added temporal tests; reverse-gap inventory outside the PR diff was not completed.
- Semantic review: skipped.

## Scope examined

- `qsl-package/src/emit.rs:489-520, 6060-6095`
- `qsl-package/src/emit/tests/temporal_clause.rs:102-170`
- `qsl-replay/src/spine/clause.rs:900-920, 1049-1205`
- `qsl-replay/src/spine/clause/temporal.rs:393-400, 432-530, 536-565, 680-865`
- `qsl-replay/src/spine/clause/tests/temporal.rs:2643-2743`
- `spec/functional/FR-050-publish-authenticated-temporal-artifacts.md` (FR-050-AC-5 and FR-050-AC-6)
- `spec/test-cases/TC-150-preserve-authenticated-temporal-selections-on-checked-route.md` (checked-successor procedure and expected results)

## Manual acceptance check

FR-050-AC-5 has an exact byte-limit, one-short refusal and fresh-retry test. FR-050-AC-6 has a checked declaration identity mapping, strict package read-back and authenticated profile/clock assertions. The implementation resolves the source-map entry in the admitted temporal subject but does not include that locus in `AuthenticatedTemporalSelection`; the new test only checks that the declaration path is nonempty. TC-150 requires an exact source locus, declaration name and byte span and requires retained admitted definition/clock evidence, so the current evidence is incomplete for that exact-locus claim. Existing temporal tests cover missing/mismatched binding refusals and zero position work.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The regression test now asserts the admitted source authority and identity and calculates the expected declaration-name start/end byte offsets from the fixture source. |

After excerpt:

```rust
assert_eq!(source_region.source.authority.as_ref(), "agent-ix");
assert_eq!(source_region.source.identity.as_ref(), "clause-run-fixture");
assert_eq!((source_region.start, source_region.end), (expected_start, expected_end));
```
