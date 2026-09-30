---
id: SR-734
title: "QSL-245 spec review of PR 489 status edits (blank-label and empty-path)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-001-read-exact-source.md; spec/functional/FR-010-report-native-outcomes.md; spec/functional/FR-018-construct-native-runtime-inputs.md; spec/functional/FR-026-run-standalone-native-workflow.md; spec/spec.md; spec/tests.md; spec/test-cases/TC-424, TC-425, TC-430, TC-431, TC-444"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: reviews
---
## Summary

Ticket: QSL-245. PR: quire-spec-language#489. The spec diff is
limited to Status text and status rows. No requirement statement or AC
changed. Each edit was checked against the code and tests and
against the QSpec catalog
(`proposals/quire-v1/definitions/native-diagnostics.md`, the
`invalid_source_identity` row at line 102).

The FR-001 debt statement is accurate: "the native-v1 `Diagnostic` ... for a
refusal with no region it keeps rendering byte 0" (lines 99-104). The PR does
not add to that debt for complete-V1. `CompleteDiagnostic` already has an
optional region, and the three moved host causes now build `region: None`.
The foundation `Diagnostic.span` is still required, so a region-less
refusal still renders at byte 0, exactly as recorded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-001 Status says "Built against catalog revision `1-draft.8`", but the build's definition lock still claims `1-draft.7`. The catalog forbids a `1-draft.7` producer from emitting these causes (see SR-732 FND-001). The Status should say the claim is still `1-draft.7` until the bump lands, or the PR should land with the bump. | spec/functional/FR-001-read-exact-source.md:228; src/linking/composed/definition_source.rs:248 |
| FND-002 | low | The FR-001 S0-location bullet still says "An unnamed source (an empty or blank label or path) ... The first has no label set to name the source by." After this PR's split, `empty-path` refuses when all four labels are set. The stated reason is therefore false for `empty-path`, and the old "unnamed source" wording no longer matches `blank-label` and `empty-path`. | spec/functional/FR-001-read-exact-source.md:95-98 |
| FND-003 | low | The run output now pairs `cause: blank-label` with the required byte-0 `span` of the foundation `Diagnostic`. The catalog says neither cause names a source region. FR-001 records this as debt, and the record is honest. But the FR-026 Status and TC-430 flip `blank-label` to backed without pointing to that debt, and no test asserts the span on this path. Add a pointer in the FR-026 Status so the byte-0 span is not taken for a location. | spec/functional/FR-026-run-standalone-native-workflow.md:124; src/command/output/types.rs:111; src/command/output.rs:96 |
| FND-004 | low | The TC-430 row moves from Partial to "✅ Passed locally", but step 1 still passes only with the documented deviation (`draft:1`, not `1`). The deviation is disclosed. Even so, marking a TC fully passed while its literal step 1 fails is a judgement the row should state explicitly (for example, "Passed with the step 1 deviation") or leave Partial. | spec/tests.md:216; spec/test-cases/TC-430-native-requests-and-outputs-carry-the-four-source-labels.md:54 |

## Verdict

Approve with findings. The status edits match the code and tests: TC-424
steps 2, 6 and 7, TC-425 step 3, TC-430 step 2, TC-431 step 2 and TC-444
step 5 all pass and were mutation-checked in SR-733. None of the four
low-severity findings blocks a merge.

## Dispositions

Checked against the fix diff on 2026-09-26.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | FR-001 Status says the revision claim still reads `1-draft.7` and that the bump lands with #490. The merge-order constraint stays open under SR-732 FND-001. |
| FND-002 | fixed | The FR-001 S0 bullet (lines 95-98) now reads "A blank label or an empty path ... no complete, non-blank label set to name the source by, and an empty path names no location." |
| FND-003 | fixed | FR-026 Status, TC-430 Status and a new FR-001 Status paragraph each point to the byte-0 debt. |
| FND-004 | fixed | The TC-430 Status and its spec/tests.md row are back to Partial, and say that step 1 passes only with the deviation. |
