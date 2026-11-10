---
id: SR-20261109172912000109
title: "QSL-1813 source diagnostic wire spec-review/base review"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1813; PR #2634; FR-286 and TC-770"
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-286, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-770, type: references }
---
## Summary

Independent spec-review/base review of PR #2634. No in-scope defect found.

## Assurance Context

No applicable AssuranceProfile exists under spec/. Direct base, EARS, integrity and failure-domain lenses were selected for the changed projection contract. No enforced workflow or human acceptance is claimed.

## Verdict

PASS for specification readiness after artifact custody. Runtime projection, full FR-287 CLI behavior, command-owner extraction and QSL-1813 completion are not qualified by this review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Analysis

FR-286-AC-15 is unique and uses a method-only Test verification cell. Existing TC-770 is extended with a corresponding step and expected results. Cases cover both label names, absence of label on other causes, absent region, original located source/endpoints, message and code/cause preservation, and equivalence across source, compile, parse and format producers. Existing US-029/ADR-029 trace context remains. The API is needed by the current driver pre-I/O blank-label refusal, rather than only by tests. This spec-only PR intentionally precedes tagged implementation tests; no full matrix or implementation completion is claimed.

## Verification and Limits

Both changed documents validate with exit 0; full-tree validation reports 28 existing advisory grammar findings, none in the changed documents. Diff check and frozen-head checks pass. No changed Markdown is embedded by a Rust include. No Cargo, Kani, runtime test, full matrix probe, source edit or push was performed. Security/object/dependency lenses were not separately run: the change introduces no authorization boundary, domain entity or relationship edge; identity failure handling is addressed by the selected failure-domain lens. Criterion-strength calibration was not requested or claimed.

## Reviewed Units

```json
{
  "clean": true,
  "scope": [
    {
      "id": "FR-286",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "This public producer-owned projection SHALL preserve the diagnostic's\ncatalog code, typed cause tag, optional source region and human message."
    },
    {
      "id": "FR-286-AC-15",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "An external caller projects a pre-admission blank-label source diagnostic through `from_source_diagnostic` and serializes a document: its code is `invalid_source_identity`, cause `blank-label`, `label` the exact typed `authority` or `identity`, `locus` null and message unchanged."
    },
    {
      "id": "TC-770",
      "path": "spec/test-cases/TC-770-library-outcomes-serialize-to-one-outcome-document-with-the-undefined-label-on-non-proof-outcomes-only.md",
      "role": "examined",
      "excerpt": "Repeat for an empty-path refusal, a byte-budget refusal with no region and an invalid-UTF-8 refusal with its original region."
    },
    {
      "id": "FR-001-AC-10",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "context_only",
      "excerpt": "Admission with an empty identity, and admission of five bytes under a four-byte ceiling each refuse with no region, not a region at byte 0."
    },
    {
      "id": "FR-001-AC-11",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "context_only",
      "excerpt": "With both the identity and the authority blank, admission refuses `invalid_source_identity`/`blank-label` with `label` `authority`."
    },
    {
      "id": "FR-287-AC-3",
      "path": "spec/functional/FR-287-reach-qsl-through-the-driver-cli.md",
      "role": "context_only",
      "excerpt": "A `parse` or `format` call with a blank label and a missing file exits 20 with `invalid_source_identity`, cause `blank-label`, the blank label's name in `label`, not a file error."
    }
  ],
  "findings": []
}
```
