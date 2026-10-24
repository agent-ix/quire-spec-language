---
id: SR-9107
title: "Gap analysis of quire-spec-language PR #839: Population<T>[N] emitted-package fixtures"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@facab37ff15fc5b874e806b8ce529cd91483b715; QSL-768 AC1-AC3, FR-093-AC-19, TC-416 step 11; qsl-package/src/emit/tests/admission_corpus.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #839

## Summary

Ticket: QSL-768. Diff-scoped gap analysis. This is a planless check of the
ticket ACs and the traced criterion against the test that claims them.

Examined:
- QSL-768 AC1 (N = 0, 1, 3 and two distinct bounds, each admitted through
  the IR reader): cases `[0]`, `[1]`, `[3]`, `[2, 5]`. Each case is read
  through `read_checked_package_v2` at the emitted `package_id` and must
  return Ok (`unwrap_or_else` panics on a refusal). N = 0 is emitted
  (`omitted == []`, one population node with `max` 0) and admitted (one
  admitted `ModelPopulation` kind). It is not skipped (met)
- QSL-768 AC2 (the population type node carries role Type): each population
  node's occurrence roles must equal exactly `["type"]` (met)
- QSL-768 AC3 (vectors produced by the emitter, not hand-written): packages
  are built from forms, then checked, linked and emitted by `emit`. Nothing
  is checked in or copied into another repo. Forms, not source text, are
  used because source text cannot spell `Population` (met)
- FR-093-AC-19 (each family QSL writes is admitted at its emitted
  package_id): the test adds admitted fixtures for the `model_population`
  family, and the `FR-093-AC-19` tag fits. The `TC-416` tag fits: the TC-416
  row in spec/tests.md lists FR-093-AC-19, and step 11 now names these
  fixtures (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS.** All three ticket ACs are met by a test that can fail.

Recorded, not reported: the ticket asks for a "published V2 package corpus"
that IR-760 can consume. This PR delivers it as an emitter-produced test
corpus inside QSL, as the brief intended, with no checked-in artifact. IR
consumes it by running QSL's emitter. IR FR-053 asking for role Expression
instead of Type is IR's change to make. It is recorded only.
