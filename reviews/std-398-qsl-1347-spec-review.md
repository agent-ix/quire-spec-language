---
id: SR-16295
title: "Spec review of quire-spec-language PR 1347: STD-398"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@edbe57f5; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md; spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md; spec/test-cases/TC-419-quantity-type-nodes-match-golden-vectors.md; spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md"
review_set: subset
date: 2026-10-23
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-419
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-483
    type: references
---

# SR-16295: Spec review of quire-spec-language PR 1347 (STD-398)

## Summary

Ticket: STD-398. Head edbe57f5 (PR 1347, base main). Own delta is `git diff origin/main...edbe57f5` (one non-merge commit, e67639f3): FR-093 (division paragraph, zero-divisor refusal, AC-34 `q` clause moved, new AC-36 and AC-37), FR-094 (quotient text, AC-11), TC-419 step 6, TC-483 step 9. origin/main is 4 commits ahead and is not an ancestor; `git merge-tree` with main is clean.

Hand derivation of the vectors: `a / 3 ms` over `a: Quantity<m>` has the compound unit m^1 s^-1 and the value 1000/3 times `a`; AC-36 p2 (`6 m` over `3 ms` is `2000 mps`), p3 (`-2000 mps`), p4 (`6 m / 0.5 s` is `12 mps`), p1 and p5 (equal operands equal, unequal operands unequal) all hold. The compound-unit node of `a / 3 ms` equals that of `a / 0.5 s` (m^1 s^-1) and the dimensionless node is shared by `d / 3 ms`, `d / 3 s` and `d / rational(-3, 1) ms`. The z1 to z9 refusals each name a reachable code and the first-division span. The QSpec side (FR-146 literal divisor, FR-142 quotient) is in QSpec PR 405, which is in the QSpec stack used for the gate.

Also checked:

- No deferral, narrowing or follow-up language in the added lines (grep over the delta). The cross-repo QSpec FR-146 amendment is STD-405 / QSpec PR 405, filed as its own ticket and outside this ticket's scope.
- Ids FR-093-AC-36, FR-093-AC-37 and FR-094-AC-11 appear on no other open PR head; FR-094-AC-10 is held by PR 706, which explains the numbering gap.
- Gate `QSPEC_DIR=<QSpec stack> make check-spec-validation`: exit 0, 19 warnings, identical to origin/main.
- Mergeability: main is not an ancestor (4 commits ahead) but merges cleanly. Textual conflicts with open PR 1333 (FR-093) and PR 706 (FR-094).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

**Mergeable.** No findings.
