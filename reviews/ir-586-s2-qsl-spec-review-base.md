---
id: SR-16361
title: "IR-586 S2 PR 1384 spec review: DeclarationKey path text in FR-273 and ADR-013, and the FR-400 trace into TC-390"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@182875cb5810b62451c4be2643bcda0e8893810b; diff origin/main...HEAD; spec/functional/FR-273-resolve-each-canonical-type-namesake.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md; context: FR-400, FR-060 T12-G, FR-059, FR-272, TC-390"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-273
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---

## Summary

Ticket: IR-586. This is the base checklist over the two spec files the PR
changes. It also covers the trace from the PR's new FR-400 tests into the
test cases they tag. PR #1384 changes no FR-060, FR-400 or FR-059 text: the
T12-G row, the FR-059 `construct` paragraph and the FR-272 shared-leaf list
already exist on `main`. They were read for context only.

Examined:

- FR-273 rows 36 and 48, and FR-273-AC-48.
- ADR-013 O-03's Public type row, the `linking::DeclarationKey` note after
  O-03, and DA-01.

`quire validate` passes on both files. No old path
(`qsl_semantics::model::key::DeclarationKey`,
`qsl_semantics::value::AdmittedIntegerDivision`) is left in `spec/`, except
in historical review records, ADR-010's baseline, ADR-011's X-13 move list
and FR-400's "before the move" text. All of these correctly name the old
location.

## Verdict

PASS with two low findings. The path updates are consistent with each
other, with FR-400 and with the code. Row 48 and AC-48 match the
`qsl-replay` facade `pub use quire_checked::key::DeclarationKey`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | FR-400-AC-1 says "TC-390's table check asserts that `qsl-semantics` is the only crate that enables `quire-checked`'s `construct` feature", and the PR's new tests carry `#[trace("FR-400-AC-1", "TC-390")]`. TC-390 has no step and no expected result for that check, and its `relationships` verify only FR-090. The test case does not describe the test now bound to it. Fix: add a TC-390 step and expected result for the `construct` table check and its fixture case, with an FR-400 `verifies` edge. | spec/test-cases/TC-390-family-outcome-and-refusal-layering.md:5-7,57-77; spec/functional/FR-400-separate-checked-terms-from-the-checker.md:342 | correct-requirement-no-evidence |
| FND-002 | low | ADR-013 O-03's Owner row still reads "QSL `model::key`, intake stage", but its Public type row now names `quire_checked::key::DeclarationKey` in the `agent-ix/quire-eval` repository. R-01 asks for one repository, one stage and one public type per owner, so a reader cannot tell which repository owns the key. O-04 shows the established wording ("QSL, check stage ... The type lives in the `quire-exact` kernel"). Fix: reword O-03's Owner the same way: QSL, intake stage, with `model::key` keeping the preimage encoding, and the type living in `quire-checked`. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:167-169 | wrong-requirement |

## Dispositions

Disposition pass 1 at `65d4950ca1da122411926b393e58fadde9758958`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | TC-390 gains an FR-400 `verifies` edge, step 5 (the `construct` table check over every workspace member and a fixture in which `qsl-eval` enables the feature), its expected result, and the `#[trace("FR-400-AC-1", "TC-390")]` tag line. |
| FND-002 | fixed | ADR-013 O-03's Owner row now reads "QSL, intake stage: `model::key` keeps the preimage encoding. The type lives in `quire-checked` (`agent-ix/quire-eval`, ADR-011 layer C)." |
