---
id: SR-16345
title: "spec-review of FR-120 encodes every FR-181 value family in state keys and transition identities (quire-spec-language PR 1317)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ccbd7f79; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md; spec/test-cases/TC-1195-state-keys-encode-every-fr-181-value-family.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1195
    type: references
---

# Spec review of FR-120 encodes every FR-181 value family in state keys and transition identities (quire-spec-language PR 1317)

## Summary

Ticket: STD-363. One spec-review pass over `git diff origin/main...ccbd7f79`: standing rule (no drop, narrowing or deferral), testable ACs with hand-derived vectors, id uniqueness across all open PR heads, QSpec name cross-check, mergeability, and the repo gate.

Examined, clean:

- FR-120-AC-16/17 derived by hand against QSpec FR-181-AC-19/20/21 on main: UUID, Timestamp -1, decimal 1.50 (150/2), quantity 3/2 rational with `unit`, enum/record/tuple/nullary-union shapes; set element 1.50 normalizes to 15/1; Timestamps {2,10} order `10` before `2` by JCS bytes; keys sorted (denominator, numerator, type, unit).
- Name and unit spellings (checked node id, 64 hex unit digest) match FR-070's witness forms; population-typed value refuses `EncodingRefusal` as FR-181 requires.
- Main has FR-120-AC-19..21 (#1345) and leaves AC-16..18 free, so no id collision.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

Mergeable after origin/main is merged: FR-120 conflicts with main (AC table append against #1345/#1389). No defect in the delta.
