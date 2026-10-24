---
id: SR-15740
title: "Spec review of quire-spec-language PR #1238: STD-304, an operation constant in hyper match"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@683f154c310a8c9e0afb3ecbdfe523658432c30c; PR #1238 (git diff origin/main...683f154c3): spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md, spec/test-cases/TC-597-s3-checks-hyper-and-relation-clauses.md; read for context: FR-176, FR-177-AC-7, ADR-023 §1 HS-2 HS-5 CS-1 PA-1, agent-ix/quire-specification main 5ee0071fc FR-395"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-597
    type: reviews
---
# SR-15740: Spec review of quire-spec-language PR #1238: STD-304

## Summary

Ticket: STD-304. Sub-analyses folded in: integrity, EARS, failure-domain. The head 683f154c3 is behind main 43f4e2086 but merges cleanly. `make check-spec-validation` on the merge result exits 0 with no diagnostic on FR-172 or TC-597. FR-172-AC-5 is new and no other open PR touches FR-172 or TC-597. The files do not overlap #1239 or #1241.

The PR does what it claims. Inside `match`, a qualified operation name compared by `=` or `!=` with `t.step.op` of the same alias is the label constant. Any other alias, a bare conjunct, or a comparison with `t.step.receiver` refuses `ill_typed`/`type-mismatch`. An unresolved name refuses `missing_declaration`/`missing-name`. The statement that the constant is false at the terminal stutter step matches QSpec FR-395 ("`stutter` for a terminal stutter step"). It also matches FR-176-AC-2, where `a.step.op = V::Vault::step and b.step.op = stutter` is vacuous over a vault with no terminal state. The lead defaults match the ticket's default. The QSpec follow-up is stated correctly: FR-395's step-label paragraph does not say what a qualified operation name means as a `match` operand.

## Verdict

**MERGEABLE after a sync with main**, with two minor findings to fix or accept. FND-001 is the only one a test author would trip over.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | [minor] The AC-5 refusal `a.step.op = N::Device::tick` with `a` of `V` gives `ill_typed`/`type-mismatch` only when the clause's unit also declares alias `N`. ADR-023 §1 declares `V` (vault, `NonInterference`) and `S`/`N` (device models, `SavesPower`) in separate examples, and TC-597's fixtures do not say they share a unit. If `N` is undeclared, the name resolves to nothing, and the third rule gives `missing_declaration`/`missing-name`. State that the fixture unit declares `V` and `N`. Planner note: an operation of another alias now gets three different codes in three places: fairness gives `missing_declaration`/`missing-name` (FR-172 Fairness), `align skip` gives `unsupported_construct` (PA-1) and `match` gives `ill_typed`/`type-mismatch`. The ticket's lead default picks the third, so this is a planner decision, not a defect in the PR. | spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:126-129,180 |
| FND-002 | low | [minor] Two loose ends in AC-5. First, "checks the same as its mirror" does not say what is compared: the checked conjunct, its `mu_universal` placement, or the node identity, which ADR-023 CS-1's normal form sorts `=`/`!=` operands for. Second, the rule that a qualified name resolving to a model member "SHALL keep the state-read refusal" has no AC. Name the compared output and add one member case, for example `a.step.op = V::Vault::l` giving `unsupported_construct`/`expression-form`. | spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:130-132,180 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Head ba0bbff27: AC-5 opens "In a unit that declares alias `V` (the vault model ...) and alias `N` (the device model, object type `Device` with operation `tick`)", and TC-597 step 5 checks "In a unit declaring aliases `V` and `N`". The planner kept the three codes for another alias's operation (fairness, `align skip`, `match`). |
| FND-002 | fixed | Head ba0bbff27: the mirror case now compares the `mu_universal` placement and the value at every joint step against `match { a.step.op != V::Vault::step }`. The member case `a.step.op = V::Vault::l` refuses `unsupported_construct`/`expression-form`. TC-597 step 5 and its expected result match. |
