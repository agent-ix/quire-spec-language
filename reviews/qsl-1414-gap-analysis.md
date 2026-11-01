---
id: SR-202610310138
title: "Gap analysis — QSL-1414 checked-package v2 AC-6/7 partial claim"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-package/src/emit/tests/protocol_requirements/fixture_w.rs; FR-048-AC-6; FR-048-AC-7; TC-134"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-048", type: references }
---

## Summary

Performed a focused, repository-driven audit of the PR's FR-048-AC-6/7 static checked-package successor slice against FR-048 and TC-134. The positive artifact is strictly read back, but the focused negative cases terminate in source checking and do not exercise checked-package reader refusal; plan completion and whole-repository coverage were not assessed.

## Verdict

**CONDITIONAL** — the checked-package positive has strict read-back evidence, while the reader-side adverse path required by this review brief is not exercised.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The tagged TC-134 tests provide positive checked-package v2 emission and strict read-back, but each adverse case uses `check_tc134` and returns before emission/reader admission. Thus the checked reader's adverse provenance path has no test in this PR. The additional ordering mutation also moves commit before the forward effect rather than before recovery, leaving the named AC-7 commit-before-recovery case uncovered. | FR-048-AC-6; FR-048-AC-7; TC-134; qsl-package/src/emit/tests/protocol_requirements/fixture_w.rs:435-479 |

## Coverage

- Reconciliation: focused manual AC/test/code check; `quire matrix --scope . --format tsv` was interrupted after 60 seconds before producing matrix output.
- Plan completion: not assessed
- Criteria: FR-048-AC-6 and FR-048-AC-7 examined for the PR's explicitly limited static-provenance claim; AC-8 clock configuration remains blocked and outside this PR's claim.
- Evidence: no run evidence read by matrix
- Untraced behaviors / stubs: repository-wide count not assessed; in the reviewed slice, reader-side adverse provenance is untested.
- Semantic review: skipped; the audit was limited to criterion-to-test/code evidence and the supplied partial-claim boundary.
