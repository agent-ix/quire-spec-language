---
id: SR-7868
title: "QSL-734 PR 797 variant-type export: integrity"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@0223a174423f74e61ce7314fdf7c18745151701b; spec/functional/FR-056-admit-domain-package-model-declarations.md (Variant types, AC-24..27)"
review_set: subset
---

## Summary

Ticket: QSL-734. Consistency of the new FR-056 "Variant types" rules with QSpec (checkout at 1498c1e) FR-154 declaration refusals, FR-208, FR-150, FR-141, TC-195 and TC-235, with FCD semantic-ir (b3defd7), and with QSL TC-145 and FR-355-AC-3.

Consistent: the clause-refused, declaration-admitted split (FR-208:43, FR-150:109, QSpec TC-195 N04, QSL FR-355-AC-3, TC-145); the source shape (FCD semantic-ir.schema.json `variant` = required `identity`, `name` (minLength 1), `origin`, optional `payloadType`; construct-vocabulary shape `enumeration` requires `variants`); case identity `variant/<Name>-<value>` (QSpec FR-154:138, FCD FR-095, slug is case-preserving so `Gold` and `gold` are distinct); case order as a sorted unordered set (QSpec FR-141: unordered enum members sorted by case name, member identity is (declaration, case)).

## Verdict

Findings: one high, three medium. The duplicate-case cause contradicts FR-154. Three rules (member typeRef, payload, empty variant type) state intake rules QSpec does not have or contradict its precedent.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Two cases of one variant type with the same name both mint `ix://<pkg>/variant/Tier-Gold`, which the PR itself calls the case node's declaration identity. QSpec FR-154's declaration refusal table gives "Two nodes share one identity, including two states or two steps of one owner with one name" `invalid_model_binding`/`conflicting-binding`, and FR-208's Names rule does the same for states and steps. FR-056 and AC-26 instead give `malformed-declaration`. | FR-056-AC-26; FR-056:450-454; QSpec FR-154:86 |
| FND-002 | medium | A member `typeRef` naming a variant type refuses `unsupported_construct`/`declaration-form`, citing FR-150 "a variant type referenced by source". FR-150:109 covers native source, not a model member `typeRef`. For a model reference naming a node of a meaning it may not name, QSpec FR-154 row 3 and TC-235 M10 (field `Order/ref` typed by a business meaning) give `invalid_model_binding`/`malformed-declaration` at `Field.typeRef`. FR-208 lists which meanings a field may name (value type, record value type) and does not list variant type. Either align with that precedent, or have the QSpec owner decide it. | FR-056-AC-27; FR-056:456-461; QSpec FR-150:109; QSpec FR-154:82; QSpec TC-235 M10 |
| FND-003 | medium | "The variant type's own declaration and exports are unaffected" by a member typeRef refusal contradicts FR-056's all-or-nothing rule: "If any declaration refusal occurs, then the compiler SHALL report every declaration refusal in node order and SHALL admit no declaration of the package." AC-27 and TC-991 step 3 don't say whether the package admits anything. | FR-056:456-461; FR-056 Declarations; FR-056-AC-27 |
| FND-004 | medium | The payload refusal (`unsupported_construct`/`declaration-form`) and the empty-variant-type refusal are intake rules that QSpec does not state. FR-154's `unsupported_construct` row covers only business-model meanings. FR-208 defines a variant type as "a closed set of named cases" with no case minimum. FCD's schema allows an empty `variants` array. Both need a QSpec FR-154/FR-208 row from the owner, or this PR states QSL-only refusals that FR-154's table contradicts. The precedents for `unsupported_construct` here (Text profile, Rational) are native-type rules, not meaning rules. | FR-056-AC-25; FR-056-AC-26; QSpec FR-154:85; QSpec FR-208:43 |

## Examined obligations

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-056 Variant types: source shape | examined | In Semantic IR it is a type definition whose construct has shape `enumeration`, with a `variants` list. Each list item has `identity`, `name`, `origin` and an optional `payloadType`. |
| FR-056 Variant types: exports | examined | one `enum`, keyed by the type's declaration identity ... `cases` ... ascending UTF-8 byte order; one `variant` per case, keyed by the owner's declaration identity and the case's declared `name` |
| FR-056 clause refusal paragraph | examined | The refusal belongs to the clause that names the variant type, never to the variant type's declaration |
| FR-056-AC-24 | examined | one `enum` keyed by `Tier`'s declaration identity with `name` `Tier` and `cases` `[Gold, Standard]` |
| FR-056-AC-25 | examined | refuses `unsupported_construct`/`declaration-form` at that case ... admits no declaration of the package |
| FR-056-AC-26 | examined | two cases named `Gold` refuses `invalid_model_binding`/`malformed-declaration` on the second |
| FR-056-AC-27 | examined | A field whose `typeRef` names `Tier` refuses `unsupported_construct`/`declaration-form` at the member |
| QSpec FR-208:43 | context_only | Normalization admits it as an effective type; a clause naming it is `unsupported_construct`/`declaration-form` (FR-150). |
| QSpec FR-141 enum identity | context_only | `members` is the declaration-ordered list of semantic case names (sorted by case name for an unordered enum) |
| QSL TC-145 | context_only | one `enum` with its `variant` records |
