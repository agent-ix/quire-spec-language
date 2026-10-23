---
id: SR-16213
title: "Spec review of quire-spec-language PR #1304: STD-356, bare native Text, Rational and Decimal admitted and refused where used"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@697c4826f95be1dfc158c6b6aa8db630303cde24; PR #1304 (git diff origin/main...697c4826): spec/functional/FR-056-admit-domain-package-model-declarations.md, spec/test-cases/TC-897-text-is-the-native-text-name.md; read for context: agent-ix/quire-specification main FR-154 (native typeRef paragraph, FR-154-AC-17), FR-322 Model-owned members step 4 and FR-322-AC-31, FR-208; QSL FR-040 (rational literal), FR-106"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-897
    type: reviews
---
# SR-16213: Spec review of quire-spec-language PR #1304: STD-356

## Summary

Ticket: STD-356. Sub-analyses folded in: integrity, EARS, criterion strength (manual, no Jev), cross-repo fidelity, id uniqueness, scope-deferral check. Examined: the rewritten bare-native paragraph, the use-site refusal paragraph, the carve-out in the no-kernel-type sentence, FR-056-AC-11 (amended), FR-056-AC-43 (new), TC-897 Scope, step 3, steps 8-9 and their expected results. Main is an ancestor of the head. FR-056-AC-43 is unique across all open QSL PR heads. The three decisions are flagged in the PR body as lead defaults.

Fidelity: QSpec FR-154 admits `typeRef` `Text`, `Rational` and `Decimal` at intake and decides usability "where it is used, at member resolution"; FR-322 step 4 gives a declared type outside the table no type and refuses the application `ill_typed`/`operator-ineligible`. The PR's intake admission, no-checked-type rule at every multiplicity and the application refusal match that. QSpec names a parameter or result refusal at "the operation node and `Operation.params[i].typeRef`".

## Verdict

**NOT MERGEABLE.** FND-001 is a high scope finding under the standing rule: the FR-106 rule refuses every snapshot or invocation that holds an object whose type merely declares such a field, which QSpec does not require and which removes the ability to check any unit whose state holds such an object, even one that never reads the field. FND-002 to FND-004 are medium.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | scope-deferral (narrowing): the FR-106 rule refuses `ill_typed`/`operator-ineligible` for an object whose type declares a bare-native field, "also when `memo` is absent", before missing/unknown-member checks. QSpec FR-154/FR-322 refuse only the application. With this rule a unit the paragraph above calls admitted, whose clauses never read the member, still cannot take any snapshot holding such an object, including one with an `optional` bare-native field left absent. State what admission does with an absent optional member and an unread required member instead of refusing the whole object, or bring the narrower rule to the owner as a QSpec question. | spec/functional/FR-056-admit-domain-package-model-declarations.md:824-833 |
| FND-002 | medium | FR-056-AC-43 says the `scale` call refuses "naming `Note` and ... `Operation.params[0].typeRef`". QSpec FR-154 names a parameter refusal at the operation node, and the FR text says "the member's declaring node", which for a parameter is the operation `scale`, not `Note`. Two implementers would name different nodes. Say which node, consistent with QSpec. | spec/functional/FR-056-admit-domain-package-model-declarations.md:952 |
| FND-003 | medium | The `scale` call vector passes `rational(1, 2)`. FR-040 requires a rational literal to have a unique expected rational type, and the parameter it fills has no checked type. The literal can refuse on its own before the call refuses, with a different cause, so the vector does not reliably reach the stated refusal. Use an argument whose type does not depend on the slot (a typed binder), or state which refusal comes first. | spec/functional/FR-056-admit-domain-package-model-declarations.md:952 |
| FND-004 | medium | The FR states refusals for an operation result typed by a bare native, a post-state read, and FR-106 invocation arguments. FR-056-AC-43 and TC-897 steps 8-9 test only fields, one parameter (`scale`'s result is `Integer`) and a snapshot. An implementation that does not refuse a bare-native result, a post-state read, or an invocation argument passes. Add a result-typed operation, a postcondition read, and an invocation vector. | spec/functional/FR-056-admit-domain-package-model-declarations.md:952 |
