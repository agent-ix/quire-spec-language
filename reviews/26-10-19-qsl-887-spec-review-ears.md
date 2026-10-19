---
id: SR-13077
title: "EARS review of PR #1023: FR-147 internal-list rules"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@54b78f5f7; git diff origin/main...HEAD: FR-135, FR-147, FR-310 statement text"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
---

## Summary

Ticket: QSL-887. Checked the changed requirement text for EARS form. FR-135's
support-set refusal stays a SHALL bullet with an optional-feature trigger
("A declaration that uses a feature outside the support set SHALL refuse").
FR-310's new bullet is a plain SHALL. FR-147's "Internal abstract steps"
bullets are SHALL statements. `quire validate` reports no EARS diagnostic.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-147 The refinement's internal list (Resolution, Defects, Precedence) | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147 Internal abstract steps bullets | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-135 support-set bullets | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-310 abstract-subject bullet | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The PR removed the one SHALL bullet that bound resolution ("Each entry of the profile's internal list SHALL name … refusing `missing_declaration`/`missing-name` otherwise"). Resolution, the defect refusals and the precedence rule are now indicative prose ("S3 resolves", "the refinement check refuses it", "every defect above is reported") with no SHALL. Restore one SHALL statement, e.g. "S3 SHALL resolve each refinement's entries as QSpec FR-432 states before the feature check, and SHALL refuse each defect in the Defects table at its (input, pointer) locus." | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:142-200 |

## Verdict

Mergeable on this lens; FND-001 is wording. The rules themselves are
determinate, with named codes, causes and loci.

## Dispositions

Round 1, against head 40892cef1 (fix commit edced4cd7).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | edced4cd7: Resolution, Defects, Precedence and the model-subject rule now read "S3 SHALL resolve", "S3 SHALL refuse each defect below", "SHALL report every defect", "SHALL refuse it", "SHALL check … SHALL NOT resolve". |
