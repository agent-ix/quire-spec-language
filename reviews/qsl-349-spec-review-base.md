---
id: SR-931
title: "QSL-349 spec review of PR 549: FR-093 generated-occurrence placement rule, FR-093-AC-18, TC-416 step 10"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@de0ff633d58bb5b3e3434d62e3b0b5ad3458ba21; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (generated-occurrence bullet, AC-18 row, TC-416 backing sentence); spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md (Scope, step 10, expected result, Status); spec/tests.md (TC-416 row, context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-416
    type: reviews
---
## Summary

Ticket: QSL-349. PR: quire-spec-language#549 at de0ff633.

FR-093 now states the placement rule for a `generated` occurrence: at the least
function body, measure, state clause or protocol attempt that reaches the node, where an
enum declaration reaches its members; failing that, at the least declared type name that
reaches it. This matches `enclosing_declarations`. AC-18 is a single testable
behaviour, given as concrete inputs and outputs. TC-416 step 10 and its expected result
match AC-18 clause for clause, and TC-416's Scope adds AC-18. The status names the four
tests. It states what is and does not name unsupported alternatives.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | "least" is undefined in the new placement rule ("the least function body, measure, state clause or protocol attempt", "the least declared type name"). Two implementers could read it as source order or as name order. The code uses the derived `Ord` of `Origin`: variant order `Body` < `Measure` < `TypeDeclaration` < `StateClause` < `ProtocolAttempt`, then the declared name in UTF-8 byte order, then the declaration index. Source order plays no part (measured: `Q` declared before `P`, and the shared node is placed at `P`). State the order in the bullet. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:93-97 |
| FND-002 | medium | The Test Matrix row for TC-416 in `spec/tests.md` still lists only FR-093-AC-7, AC-9, AC-12, AC-13, AC-16 and AC-17. The PR adds AC-18 to TC-416's Scope and to FR-093's backing sentence, but not to this row. A reader of the matrix, or a coverage count over it, sees AC-18 as unbacked. Add AC-18 and its status (three passing tests, read-back as per SR-930 FND-001). | spec/tests.md:202 |

## Verdict

The rule and the AC are sound, testable and consistent with the code. There are two
medium gaps: the ordering the rule depends on is unstated, and the matrix row was not
updated. Both are spec-only fixes for the fix round.
