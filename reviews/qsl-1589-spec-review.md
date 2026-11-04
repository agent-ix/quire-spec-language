---
id: SR-2299002
title: "spec-review of FR-035-AC-7 and TC-113 reserved declaration names (quire-spec-language PR 2299)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-035-parse-composed-native-units.md; spec/test-cases/TC-113-parse-composed-native-syntax.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-035
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-113
    type: references
---

# Spec review of QSL-1589 FR-035-AC-7 and TC-113 (quire-spec-language PR 2299)

## Summary

Ticket: QSL-1589. Checked the FR-035-AC-7 and TC-113 step 5 and expected-result edits against
FR-035's Behavior, the AC-7 text merged in #2255, and the QSpec authorities FR-035 depends on.
Those are QSpec FR-032 and FR-339. FR-339 incorporates the shared grammar as the normative syntax
contract, and the shared grammar lists the common and complete-V1 reserved-word inventories.

The correction is accurate. 0-draft has no `function` declarations: `function` is a complete-only
token, and `Kind::historical` unreserves it. The four spellings keep `unsupported_construct` in
0-draft, which the root-crate test `complete_compound_words_do_not_change_historical_tokenization`
asserts. The #2255 wording, "retain their historical identifier treatment and parse as function
declaration names", was untrue. The correction does not contradict QSpec. QSpec says nothing about
0-draft function names, and the shared grammar's rule that a 1-draft keyword "cannot steal an
identifier admitted by 0-draft" is not engaged, because these spellings were never identifiers in
0-draft.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-7 reserves `helper`, `rec`, `cast` and `Tuple` as 1-draft function and predicate names, but neither QSpec shared-grammar inventory (common or complete-V1) lists them. The complete-V1 list has `tuple`, not `Tuple`. QSpec FR-339 says the parser recognizes exactly that grammar, with reserved words "fixed by the linked language profile". This refusal is therefore a QSL-local reservation with no QSpec owner. AC-7 also reserves only two of the many `ident` positions: a record, field, enum, member, parameter, binder or alias with these spellings is still admitted by S1 and then fails at S2 (or is dropped silently, see SR-2299001 FND-001). Either get a QSpec ruling that adds these four spellings to the 1-draft reserved inventory for every `ident` position, and widen AC-7 to "any native declaration or binder name", or make them ordinary identifiers in 1-draft as QSpec reads today. Both remove the S1/S2 disagreement at its root. | spec/functional/FR-035-parse-composed-native-units.md:76 | wrong-requirement |

## Verdict

The 0-draft correction and the predicate extension are right, and TC-113 step 5 matches the tests
that carry the tags. One medium finding remains: AC-7's scope (two declaration positions) and its
authority (not in QSpec's reserved inventory) need a ruling. Non-blocking for this PR if a ticket is
filed. The silent alias drop it allows is tracked as a code bug in SR-2299001.
