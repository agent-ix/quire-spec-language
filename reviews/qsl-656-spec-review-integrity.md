---
id: SR-7820
title: "QSL-656 specification integrity review at c8fe420b"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@c8fe420b3969357081be9cca623762c2102f9482; edited spec paths in PR #665 against origin/main 65daf24a988e3e49d844d24855df1ff0d9342b2f; FR-255, FR-003, FR-100, FR-098, FR-263, FR-286, FR-357, ADR-030 and their changed test cases"
review_set: subset
relationships:
  - {target: ix://agent-ix/quire-spec-language/FR-255, type: reviews}
  - {target: ix://agent-ix/quire-spec-language/FR-003, type: references}
---

## Summary

Checked completeness, cross-document consistency, atomicity and testability for the changed requirements. One cross-reference inconsistency remains: the formatter's setting row was removed from FR-255 even though FR-003 and QSL still depend on it.

## Verdict

**CONDITIONAL** — FR-255 and FR-003 disagree about ownership and continued availability of `format.output_bytes`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-255 removes the `format.output_bytes` row and says its table is the set of accepted setting names, while FR-003-AC-10 and its References section still require that row and QSL still accepts the setting. Restore a coherent owner/table contract or update the dependent FR-003 contract. | FR-255; FR-003-AC-10 |

## Coverage

Reviewed changed requirement relationships, behavior clauses, acceptance criteria, test-case references and the changed table/data shape. The acceptance criteria remain concrete and observable. The finding is cross-artifact consistency, not a missing test assertion.

## New findings (disposition pass 1)

Reviewed exact frozen head d7c7ea72c7899a7d2ce973da0473034f4a79b033 against origin/main fe7d883ade8ed3d12c1152fa49dad8a48b325680. The prior FR-255/FR-003 formatter-table inconsistency is fixed. The PR's FR-312 edit creates an incomplete cross-requirement contract.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | FR-312 changes the clause request field to `accounting` and delegates its shape to FR-100, but FR-100's defined wire member is `call.work_units`; it does not define the clause member. The source still decodes `ClauseRun.work_units` and rejects unknown fields. A request conforming to the changed FR-312 wording cannot be decoded. Define and implement the clause accounting shape as part of the owning change, or keep FR-312 unchanged. | FR-312; FR-100; src/command/wire.rs:95-102 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d7c7ea72c7899a7d2ce973da0473034f4a79b033 |
