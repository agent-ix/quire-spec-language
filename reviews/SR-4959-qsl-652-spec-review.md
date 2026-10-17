---
id: SR-4959
title: "QSL-652 PR 682 scoped cleanup review"
type: SpecReview
analysis: base
scope: "Specification Status cleanup at 653e51d0a13af16846cd7009ac1c7376ea74a8b7; all changed path inventory and remaining Status sections"
review_set: subset
---

## Summary

Static scope and cleanup inspection finds two remaining Planned Status sections. The requested literal deletion-only condition also does not describe this branch: QSL-652's broader ticket allows relocation of unique design/trace prose. Index checking currently fails on inherited TC-984; structural spec validation passes with warnings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-914 still contains a Planned Status section. | TC-914:70; QSL-652 |
| FND-002 | medium | TC-931 still contains a Planned Status section. | TC-931:47; QSL-652 |

## Verdict

FINDINGS; no mergeability PASS at 653e51d0a13af16846cd7009ac1c7376ea74a8b7. Resolve both remaining cleanup sections and rerun the requested static checks on the resulting head. Final index result must include the upstream TC-984 correction. The literal deletion-only scope condition needs reconciliation with the ticket-authorized relocation, not blind deletion of retained design content.

## Static checks

- GitHub/local frozen head agree; worktree clean before review. Against base `edd8deb864627c1ee55ada381b9143b70b8d001b`, 283 files change, all under spec and all Markdown: 494 insertions, 3651 deletions. No production code, executable tests, schemas, workflows, dependency or index edits. `git diff --check` exits 0.
- Strict 'only Status-section deletions': FAIL. ADR-009 moves scoping text to Context; FR-001 moves implementation/debt prose to its statement and below acceptance criteria; TC-451 changes the procedure/expected fixture name from origin to corner; spec/spec.md changes qualification prose and removes Lifecycle Status. The ticket explicitly allows moving unique design and trace information, and these changes must not be represented as deletion-only.
- `rg -n '^## Status' spec` returns exactly the two findings above. They were introduced by newer main spec work and were not removed by this branch.
- `make check-index-completeness`: exit 2, missing TC-984. The base index also lacks TC-984, and this branch does not modify it. This is an inherited gate failure; the known index correction belongs to QSL-673 PR 719. It is not a new defect attributed to this cleanup.
- `quire validate --scope . 'spec/**/*.md'`: exit 0. Warnings identify existing module duplicate-archetype/inline-schema issues and EARS/wording diagnostics; no validation error was emitted. Full output preserved in scratch spec-validation.log.
- No build, Cargo command or test suite launched. Earlier coder Rust receipts are not presented as this review's execution evidence.

## Scope and method

Applied the reviewer workflow and a scoped spec base/cleanup inspection, using the requested diff inventory, index check and structural validation. This is not a full semantic re-review of all 283 specifications. No applicable AssuranceProfile found. Rust/code review and production gap analysis are inapplicable to this documentation-only change; additional specification analyses are not claimed. No source edits made by reviewer; only separately authorized trailing review-document custody is permitted.

## Finding data

```yaml
clean: false
scope:
- id: TC-914
  path: spec/test-cases/TC-914-registry-allocation-preserves-native-units.md
  role: examined
  excerpt: '## Status


    Planned consumer verification after the reviewed QSL and QSV contracts are

    merged and IR-713 delivers its fallible production storage and carrier. This

    test case specifies the required evidence; it does not claim an allocator

    failure has already been executed or the QSL-500 allocation path delivered.'
- id: TC-931
  path: spec/test-cases/TC-931-format-setting-integration.md
  role: examined
  excerpt: '## Status


    Planned. This case is the QSL-673 integration contract for QSL-605; the

    `format.output_bytes` row and FormatLimits/settings mapping remain pending

    until that implementation lands.'
findings:
- fnd: FND-001
  severity: medium
  confidence: high
  method: spec-review/base
  check: coverage
  artifact_id: TC-914
  related_ids:
  - QSL-652
  path: spec/test-cases/TC-914-registry-allocation-preserves-native-units.md
  lines: '70'
  excerpt: '## Status


    Planned consumer verification after the reviewed QSL and QSV contracts are

    merged and IR-713 delivers its fallible production storage and carrier. This

    test case specifies the required evidence; it does not claim an allocator

    failure has already been executed or the QSL-500 allocation path delivered.'
  finding: This Planned run-status section remains after the removal campaign. Remove the Status heading and run-status prose; preserve unique dependency or scope information in the appropriate existing section, as QSL-652 requires. This artifact entered through newer main changes but is present at the reviewed frozen head.
- fnd: FND-002
  severity: medium
  confidence: high
  method: spec-review/base
  check: coverage
  artifact_id: TC-931
  related_ids:
  - QSL-652
  path: spec/test-cases/TC-931-format-setting-integration.md
  lines: '47'
  excerpt: '## Status


    Planned. This case is the QSL-673 integration contract for QSL-605; the

    `format.output_bytes` row and FormatLimits/settings mapping remain pending

    until that implementation lands.'
  finding: This Planned run-status section remains after the removal campaign. Remove the Status heading and run-status prose; preserve unique dependency or scope information in the appropriate existing section, as QSL-652 requires. This artifact entered through newer main changes but is present at the reviewed frozen head.
bindings: []
```
