---
id: SR-12889
title: "Scope-boundary review of STD-162 state-graph S1 grammar (PR #981)"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; PR #981 (git diff origin/main...HEAD at 12778aab4); FR-165 Parsing (S1) and Inputs against the QSpec owner (agent-ix/quire-specification proposals/quire-v1/shared-grammar.md and spec/functional/analysis/FR-390-analyze-state-graph-claims.md at origin/main 967ee5a)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: reviews
---

## Summary

Ticket: STD-162. Boundary review of the S1 rules that FR-165 now states while
QSpec owns the grammar (ADR-022 QS-1, FR-165 Dependencies). Examined: both
FR-165 Parsing (S1) bullets and the FR-165 Inputs sentence. Context only:
QSpec shared grammar `block`, `expr`, `temporal-expr`, `interval`,
`state-graph-clause` and `graph-over` (shared-grammar.md:123,158-174,473,573-583),
its state-graph recognition rules and source-fragment table (lines 671-714),
QSpec FR-390 table, Admission and AC-1/AC-2, and PR #968's
`StateGraphClaim`/`StateGraphBody`/`StateGraphPredicate`/`StateGraphFormula`/`StateGraphTerm`
productions.

Author's open point (2), what QSpec needs:

- Optional `over`: nothing. QSpec already has it: `[ graph-over ]` in all
  three productions (shared-grammar.md:575-583), `[over (x: T)]` in FR-390's
  table, FR-390-AC-1 ("its optional `over` parameter"), and the source
  fragment `always possible Rec using v from (not x.lost) { x.ok }` with no
  `over` (line 713). FR-165 can cite these instead of stating its own rule.
- Temporal operator in a predicate: a QSpec follow-up is required (FND-001).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-165 Parsing says S1 SHALL parse a temporal operator anywhere in a predicate "with the full clause grammar". QSpec, which owns the grammar, rejects this. The three state-graph productions take `block = '{', expr, '}'` and `'(', expr, ')'`, `expr` has no temporal operator, and `eventually` is a reserved word. So `{ c.versionNumber = 2 and eventually c.versionNumber = 3 }` is a QSpec syntax error. The QSpec temporal grammar would also reject `eventually c.versionNumber = 3`, because `temporal-unary` needs an `interval` and a `holds(...)` or parenthesised operand. QSpec is itself inconsistent: FR-390 Admission and FR-390-AC-2 require an `expression-form` refusal at the operator, which its own grammar cannot reach. #968 adds a bespoke widening: temporal operators with an optional interval over `Expression` operands, and `Fairness*` in the body. "Full clause grammar" therefore names no real grammar, and two implementers would parse different languages. Fix in QSL: describe the widening (unary and binary temporal operators, optional interval, at any operand position, and fairness constraints at the start of the body) by reference to a QSpec production. QSpec follow-up (STD ticket linked from STD-162): add a `graph-predicate` production to shared-grammar.md's state-graph section, used by the three bodies and by `from (...)` and `to (...)`. It admits those forms for recognition only, and FR-390 refuses them at admission. Add source-fragment rows showing that the form parses and is refused by FR-390. | spec/functional/FR-165-check-state-graph-claims-at-s3.md:49-52,84-91; spec/functional/FR-165-check-state-graph-claims-at-s3.md:169-170 | wrong-requirement |

## Verdict

No high finding. FND-001 can be deferred only with a filed QSpec ticket:
QSL would then specify the agreed widening ahead of the owner's grammar, and
FR-165 should name that ticket. The QSL wording fix ("full clause grammar")
belongs in this PR.

Checked and sound: the split of responsibilities is right. Recognition is at
S1, the refusal is at S3 with a span at the operator, and S1 never refuses.
This matches QSpec FR-390's intent (refusal at admission, located at the
operator) and #968's behaviour. The parameter rule (`missing-name`, resolved
against the claim's own scope) needs no QSpec change: FR-390 lists only
anchor and pre-state reads as `expression-form`, so a parameter falls to
ordinary name resolution.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 2fbe63ee5: FR-165 Parsing now defines the widening concretely: unary and binary temporal operators, each with an optional interval, at any operand position, and fairness constraints at body start, recognition only, with the refusal in QSpec FR-390. It names the missing QSpec `graph-predicate` production, and the QSpec follow-up is filed as STD-171. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | medium | FR-165:95-97 says that until QSpec's `graph-predicate` lands "QSL cannot reach the S3 refusal below from source text". That contradicts FR-165-AC-2 and AC-4 ("none is an S1 refusal"; S3 refuses at `eventually`) and the Parsing bullet's own SHALL. PR #968 already parses these predicates in QSL's S1 (`StateGraphPredicate`/`StateGraphFormula`/`StateGraphTerm`) and tests the S3 refusal. Failure: a test author reads AC-2 and AC-4 as unreachable and skips them, or a coder removes #968's widening to match "QSpec's production". Fix: say QSL's S1 implements this widening now, ahead of QSpec, and that STD-171 brings QSpec's shared grammar and FR-390 in line. Also cite STD-171 by id; it is in no spec file today. | spec/functional/FR-165-check-state-graph-claims-at-s3.md:87-97 | wrong-requirement |
