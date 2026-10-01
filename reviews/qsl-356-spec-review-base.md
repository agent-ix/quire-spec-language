---
id: SR-945
title: "QSL-356 spec review of PR 554, FR-059 leaf classification, ADR-011 FB-05 and X-10, ADR-013 §2 and O-15"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f085d7a307f1ffd3504ebb6b3a52d7978b451051; spec/functional/FR-059-check-backend-dependency-direction.md; spec/test-cases/TC-156-check-backend-dependency-direction.md; spec/tests.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md; spec/functional/FR-061-check-duplicate-ecosystem-revisions.md (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-059
    type: reviews
---
## Summary

Ticket: QSL-356. Base review of the spec diff.

- **FR-059 Behavior and FR-059-AC-8.** The new SHALL is testable and names one package;
  AC-8 states both the passing and the failing case and is backed by TC-156 step 8 and
  its expected result, which match the test.
- **TC-156.** Scope widened to AC-8; `tests.md` row matches.
- **ADR-011 FB-05.** The added sentence is consistent with §6.1 "K is a leaf" and the
  §7.1 edge table (RT -> quire-exact and CG -> quire-exact are "New" edges).
- **ADR-011 X-10.** Accurate against the crate (see SR-944).
- **ADR-013 §2.** No longer claims IR-274 is adopted.
- **ADR-013 O-15 Invariants.** Correct against the code; QSpec ids carry the `QSpec`
  prefix in prose, per the repo's style.
- **Cross-FR consistency: found.** FR-061 says its classification is the one shared
  function `graph::classify` and that a package classifies "primarily by its `source`
  string". FR-059 now says `quire-exact` classifies as no repository wherever it is
  sourced from. Read together, FR-061 now excludes `quire-exact` without saying so, and
  its own Behavior text contradicts that. See FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-059's new "classify `quire-exact` as no ecosystem repository, wherever it is sourced from" contradicts FR-061 Behavior, which requires the shared `graph::classify` to classify by `source` (so a QSL-sourced `quire-exact` is QSL). Scope the FR-059 sentence to FR-059's edge extraction and state in FR-061 that `quire-exact` is still checked for duplicate revisions. | spec/functional/FR-059-check-backend-dependency-direction.md:46-49, spec/functional/FR-061-check-duplicate-ecosystem-revisions.md |

## Verdict

The ADR edits are accurate. FR-059's new classification rule needs scoping so it does not
silently amend FR-061; this goes with SR-943 FND-001.
