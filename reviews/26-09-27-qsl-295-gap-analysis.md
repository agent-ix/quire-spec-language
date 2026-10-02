---
id: SR-756
title: "QSL-295 gap analysis of PR 502 (FR-109 I3 input, TC-468 step 6)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; spec/functional/FR-109-run-a-state-clause-through-the-spine.md; spec/test-cases/TC-468-spine-clause-run-reports-typed-dispositions.md; qsl-replay/src/spine/clause.rs; qsl-replay/src/spine/clause/tests.rs; qsl-replay/Cargo.toml; Cargo.toml"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-468
    type: reviews
---
## Summary

Ticket: QSL-295. PR: quire-spec-language#502.

Trace:
- FR-109 Inputs :49-51 (I3 source "with its original document identity"):
  `ClauseRunSource::Extracted` (clause.rs:93-94). The original identity comes
  from `map.original().identity()`.
- FR-109 Outputs :81-83 (source identity and digest; the extraction's original
  identity and digest): `ClauseRunProvenance.source`, `.extraction`, and
  `ClauseRunReport.source_digest`.
- FR-109-AC-6: `run_clause_compiles_an_extracted_body_and_reports_its_original`
  (tests.rs:2458) covers success, exit 0, the body's `package_id`, the body
  identity and digest, and the original identity and digest.
  `run_clause_reports_the_extraction_original_on_violation_and_compile_refusal`
  (tests.rs:2509) covers violation with exit 10, and a compile refusal with the
  extraction kept. `run_clause_evaluates_a_clause_selection` (tagged AC-1 and
  AC-6) covers "program source carries no extraction".
- TC-468 step 6 matches these tests. Both tests ran and passed in the
  all-features gate (scratchpad/qsl-295-ci-r1.log, exit=0).
- The tests assert real values. The expected `package_id` comes from an
  independent `compile` of the body, and the digest is checked to differ from
  the body's. They are not tautologies.

No production code without an owning requirement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No requirement or test covers the extracted clause's declared language (see SR-755 FND-001). A non-`ix:native` fence is untested and its result is unspecified. Fix: add the rule to FR-109 (see SR-757 FND-001), then a TC-468 step 6 case and a test for it. | qsl-replay/src/spine/clause.rs:119-132; qsl-replay/src/spine/clause/tests.rs:2399-2527 |

## Verdict

Request changes on FND-001 only. AC-6 is fully traced and tested otherwise.

## Dispositions

Round 2.

| FND | Outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `run_clause_refuses_an_extracted_fence_that_is_not_ix_native` (qsl-replay/src/spine/clause/tests.rs:2546-2581, feature-gated, traced to TC-468 and FR-109-AC-6) extracts the same unit from an `ix:formal` fence. It asserts `UnknownLanguage { language: "ix:formal" }`, stage `compile`, `Refusal`, no truth, exit 20, no `package_id`, the body as the source, the original identity and digest as the extraction, and no documents. It passed in qsl-295-ci-r2.log. |
