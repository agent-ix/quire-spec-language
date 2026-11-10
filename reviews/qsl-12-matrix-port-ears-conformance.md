---
id: SR-20261110040709820014
title: "QSL-12 explicit source label ears-conformance review"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; QSL-12; PR #2649; FR-565 source-identity alignment and TC-1394 matching procedure"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-565
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1394
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: references
---

## Summary

Independent narrow ears-conformance review of the owning source-label specification alignment at `5f2f92bfa77e24802615b8b9cd2ef3827949965b`. No findings.

## Verdict

PASS for this exact specification delta; no full specification campaign or QSL qualification is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review Evidence

Both changed normative Behavior statements are ubiquitous EARS obligations with an explicit actor (MatrixCase or run_case), one shall, and an observable response. They do not confuse an event with a continuous state or use a vague response. The descriptive source-identity paragraph supplies the concrete field mapping; TC-1394 is test procedure context, not an EARS requirement. The changed acceptance criterion is checked for semantic consistency rather than forced into a shall template.

Exact changed FR-565 and TC-1394 validation passed with installed duplicate-module notices only and no EARS diagnostics. No applicable AssuranceProfile was found. Direct review is appropriate; no enforced review workflow is claimed. No Cargo or source changes. Plan completion: not assessed.

## Examined Scope

- `FR-565-AC-2` — role: examined; `spec/functional/FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md`; excerpt: For every `Case`, `run_case` returns `Ok` with a typed document (or, for `prove`, an item) whose category and exit code equal the case's expected category and exit code, two `run_case` calls give equal `OutcomeDocument` values, and every authored source identity in the document equals `SourceIdentity::new(case.parameters.source_authority, case.parameters.source_identity)`, independently of the input's physical file name. A case with an input the test removes returns `Err(CaseError)` whose `stage` is `CaseStage::Read`, the `Copy` stage value, and no `CaseOutcome`.

- `FR-565` — role: examined; `spec/functional/FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md`; excerpt: - Each `MatrixCase` shall state its authored source authority and identity
  separately from the input file names.

- `FR-565` — role: examined; `spec/functional/FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md`; excerpt: - The `run_case` operation shall read the authored source under the case's
  `SourceIdentity::new(source_authority, source_identity)`.

- `TC-1394` — role: examined; `spec/test-cases/TC-1394-the-published-matrix-cases-give-their-expected-documents.md`; excerpt: identity against the case's explicit `source_authority` and
   `source_identity` labels, independently of its physical input file name;

- `FR-287` — role: context_only; `spec/functional/FR-287-reach-qsl-through-the-driver-cli.md`; excerpt: The user binary is `quire`, built from the driver repository (ADR-029 CB-1).
QSL builds no user binary. Its only executable targets are the developer
tools, the packages that run against source trees and measurements rather
than serve a user and that no QSL library's build depends on: `xtask`, the
packages whose manifest directory is under `tools/`, `qsl-string-edge` (the
standalone string-edge scanner FR-064 requires as a binary) and `qsl-bench`
(whose `qsl-bench-probe` binary MP-006's probe and the Makefile's
`BENCH_PROBE` run). The request
handling FR-027 and FR-100 specify is a QSL library

