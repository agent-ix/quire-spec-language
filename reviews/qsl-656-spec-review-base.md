---
id: SR-7818
title: "QSL-656 changed-spec base review at c8fe420b"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c8fe420b3969357081be9cca623762c2102f9482; changed spec paths in PR #665 against origin/main 65daf24a988e3e49d844d24855df1ff0d9342b2f; FR-255, FR-100, FR-098, FR-109, FR-263, FR-286, FR-312, FR-357, ADR-030, TC-450, TC-451, TC-770, TC-921, spec/tests.md"
review_set: subset
relationships:
  - {target: ix://agent-ix/quire-spec-language/FR-255, type: reviews}
  - {target: ix://agent-ix/quire-spec-language/FR-003, type: references}
---

## Summary

Reviewed the changed requirements and test procedures at exact head c8fe420b against origin/main 65daf24a. The accounting-setting contract is consistent overall, with one contradiction where FR-255 removes the formatter setting row while FR-003 still requires that row and the implementation continues to accept it.

## Verdict

**CONDITIONAL** — one medium consistency finding remains in the changed spec.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-255 removes `format.output_bytes` from its setting table, but FR-003-AC-10 still says it maps through that row and the settings operation; QSL continues to expose `Setting::FormatOutputBytes` and accepts the operand. The edited FR-255 also says its table contains every accepted setting, so the two requirements and implementation disagree. | FR-255; FR-003-AC-10 |

## Coverage

Reviewed changed requirement statements, acceptance criteria, test procedures, trace tags listed in `spec/tests.md`, and cross-references to formatter accounting. No full test matrix was available: both `quoin matrix --repo . --json` and `quire matrix --scope . --format json` remained silent beyond 30 seconds and were stopped. No tests or build gates were run.

## New findings (disposition pass 1)

Reviewed exact frozen head d7c7ea72c7899a7d2ce973da0473034f4a79b033 against origin/main fe7d883ade8ed3d12c1152fa49dad8a48b325680. The prior formatter-row inconsistency is fixed: FR-255 again contains `format.output_bytes`, and its behavior and FR-003 reference remain coherent. The added FR-312 wire wording introduces a new unresolved contract mismatch.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | FR-312 now says a clause request has an optional `accounting` member but only points to FR-100's accounting object; FR-100 defines `call.work_units`, not a clause-run accounting object, and the implementation's closed `ClauseRun` wire type still accepts `work_units`. Thus the changed spec names an undefined member and a request following it is rejected. Define the member shape/behavior and implement it, or do not change FR-312 here. | FR-312; FR-100 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d7c7ea72c7899a7d2ce973da0473034f4a79b033 |
