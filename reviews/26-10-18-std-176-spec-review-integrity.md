---
id: SR-12893
title: "Integrity review of PR #986: FR-165 and ADR-022 SG-4 against QSpec FR-390 and TC-330"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@2cb28aa4b; FR-165 Use case, Inputs, Parsing (S1), Predicates (S3), AC-4, AC-6; ADR-022 SG-4 and the paragraph after the SG table; TC-590 steps 4 and 5; agent-ix/quire-specification@origin/main 4f65ed2 proposals/quire-v1/shared-grammar.md lines 574-601 and 721-743, FR-390 lines 112-137 and AC-1/AC-2, TC-330 SG-R09 to SG-R18"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: reviews
---

## Summary

Ticket: STD-176. PR quire-spec-language#986. This review compares each S1
case in the PR with merged QSpec, read only from origin/main 4f65ed2:

- the `graph-body`, `graph-predicate` and `graph-relation` productions;
- the general rule;
- FR-390 admission;
- TC-330 SG-R09 to SG-R18.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QSpec places the error for a fairness constraint after the predicate, and QSL does not. QSpec SG-R13 puts `{ c.done; fair weak C::Cfg::step }` "at the `;` after `c.done`: a body is a predicate and ends there". FR-165 line 101, AC-6 and TC-590 step 5 say only "at the offending token". They pin a token for the flat chain alone (the second `until`). An S1 that reports at `fair` passes AC-6 but disagrees with SG-R13. The same gap applies to the `from (c.a; fair weak ...)` vector in SG-R15. PR #968's matching test checks only that a diagnostic exists. | spec/functional/FR-165-check-state-graph-claims-at-s3.md:101,180; spec/test-cases/TC-590-s3-checks-state-graph-claims.md:60-61 |
| FND-002 | low | The recognised placements are described as "an operand of a `graph-relation`". In the grammar, `c.n = 1 and eventually c.n = 2` puts `eventually` as an operand of `and`, which is a `graph-conjunction`; it is a `graph-relation` itself, not an operand of one. Operands of `or` and `implies` are not named at all. The example covers `and`, but the noun is wrong. ADR-022 line 98 repeats it. | spec/functional/FR-165-check-state-graph-claims-at-s3.md:95-97; spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:98 |

## Verdict

Every PR vector matches QSpec's verdict (parses then refused, or syntax
error) except for FND-001's location. There is no contradiction. QSpec
agrees with the PR on each of these:

- SG-R09 and SG-R10's `and`, `not` and paren vectors, and `until`: AC-4.
- SG-R11 `from`: AC-4.
- SG-R12 leading fairness: AC-4.
- SG-R13: AC-6, apart from the location (FND-001).
- SG-R15's three syntax errors: AC-6.
- SG-R16 `not c.n = 2` read as `(not c.n) = 2`: Parsing line 110.
- SG-R17 `if` branch: AC-4.
- SG-R18 flat chain at the second `until` and the parenthesised chain at
  the first `until`: AC-4 and AC-6.

The `let` binding, call argument and quantifier body cases match the
grammar text ("a temporal operator in an `if` condition, a `let` binding, or
an operand of a primary, such as a call argument or a quantifier body") and
FR-390 lines 130-131. The general rule matches shared-grammar.md lines
725-731. The PR's "phrase" generalises QSpec's "predicate or parenthesised
group" with no change in meaning. ADR-022 SG-4 now agrees with FR-390. The
old line saying S1 parses an operator "at any position" is gone. The
interval and `to` coverage gaps are recorded in the base review.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The location rule for a fairness constraint inside a `from` or `to` operand is "at the `;` that follows the predicate". That holds for `from (c.versionNumber = 1; fair weak ...)`, but a constraint written first, `from (fair weak Config::ConfigVersion::attemptUpdate; c.versionNumber = 1)`, has no predicate before it; a parser fails at `fair` there, and the rule names no token. QSpec is silent on this placement. | spec/functional/FR-165-check-state-graph-claims-at-s3.md:109-110 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 0a710fa5e names the token for every S1 case. Fairness after a body predicate or inside `from`/`to` is at the `;` after the predicate, matching QSpec TC-330 SG-R13. AC-6 and TC-590 step 5 now say "at the token FR-165-AC-6 names". |
| FND-002 | fixed | 0a710fa5e reads "an operand of `and`, `or` or `implies`, of a binary temporal operator, of a unary operator, of a parenthesised group or of a branch of an `if` or the body of a `let`" in FR-165, and ADR-022 line 98 matches. |
