---
id: SR-17181
title: "QSL-1072 PR #1523 integrity review of FR-487 and TC-1264"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md, spec/test-cases/TC-1264-hyper-atoms-and-match-conjuncts-are-lowered-and-evaluated.md, spec/functional/FR-315-encode-the-smt-lib-transition-relation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1264
    type: reviews
---

## Summary

Ticket: QSL-1072. PR: quire-spec-language#1523 (head <sha removed>). This lens checks completeness, consistency and atomicity inside the change, and whether TC-1264 covers each criterion. Because criterion-strength could not run (no Jev client), each AC was also read for whether it can fail.

Examined units: FR-487 (statement, Outputs, Behavior), FR-487-AC-1 to FR-487-AC-8, FR-315-AC-43, and TC-1264 (procedure steps 1 to 9 and expected results). Coverage map: AC-1 is step 1; AC-2 is steps 1 and 2; AC-3 is step 3; AC-4 is step 4; AC-5 is step 5; AC-6 is step 6; AC-7 is step 7; AC-8 is step 8; FR-315-AC-43 is step 9. Every criterion has a procedure step and an expected result. Each AC names concrete inputs and outputs, and none is vacuous except where noted below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `evaluate_step_predicate` faults when "`labels` does not hold one label per trace variable that `part` reads". `StepMatch::universal` always passes one label per universal variable, and `existential` one per variable of the clause, whether or not the part reads each one. The same bullet's own "as `StepMatch` receives them for `part`" says so too. Read literally, the condition faults on valid calls where a part reads fewer variables than it is given labels for, for example `match { a.step.op = V::Vault::step }` over two traces. State the length as one label per universal variable for `Universal` and one per variable for `Existential`, with slot `k` read at its variable's quantifier index. | FR-487:219-224, qsl-eval/src/hyper/seam.rs |
| FND-002 | medium | FR-487-AC-5's third case, "conjunct 2 instead refused by its evaluation", names no construct. A conjunct reads only step labels, literals and calls, so it is not obvious what refuses at evaluation. A test author must invent one (an authored bound a declared function exceeds? an evaluator refusal?), and two tests could pass for different reasons. Name the refusing conjunct (for example a call to a declared function whose evaluation is refused, with the refusal's `ClauseOutcome`) in the AC and in TC-1264 step 5. | FR-487-AC-5, TC-1264 step 5 |
| FND-003 | low | "The products ... keep the conjunct for a refusal locus" sets no observable result. FR-176 `MatchUndetermined`, FR-177 and FR-178 carry no locus field, and no AC checks that the conjunct reaches any output. Either name the output field that carries it or drop the clause. | FR-487:269-273 |
| FND-004 | low | FR-487-AC-4 says `(step(0), step(1))` gives `Fails` "from conjunct 1", but `MatchReading::Fails` carries no conjunct. Only the charge log can show which conjunct decided, and the AC asks for the log only in the `(step(0), stutter)` case. Ask for the charge log there too, or drop "from conjunct 1". The same applies to AC-5's "gives `Fails` from conjunct 1". | FR-487-AC-4, FR-487-AC-5 |

## Verdict

Conditionally acceptable on integrity: the change is internally consistent, ids are unique, and TC-1264 covers all nine criteria step by step. FND-001 and FND-002 should be fixed before coding, because a literal implementation would fault, or a test would not pin the refusal.

## Dispositions

Round 1, head <sha removed> (fix commit <sha removed>).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The label-vector length is now stated per part: one label per universal variable for `Universal`, and every variable, universal ones first, for `Existential`. The fault is "not the length this section gives for `part`", and slot `k` is read at its variable's quantifier index. |
| FND-002 | fixed | AC-5's refusing conjunct is named: `quire::value::ieee::sqrt` under `Float64[exact]` of a non-square constant (FR-397-AC-20), with outcome the kernel `Refused` `ieee_not_exact`. TC-1264 step 5 follows. |
| FND-003 | fixed | Replaced by "This requirement adds no conjunct field to those outcomes. The conjunct is observable through `StepMatch` and through FR-315's `match-conjunct` refusal." |
| FND-004 | fixed | AC-4 now asks for the charge log over `(step(0), step(1))` (conjuncts 0 and 1), and AC-5 for "a charge log holding no charge of conjunct 2". |
