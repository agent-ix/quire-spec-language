---
id: SR-11178
title: "QSL-502 quire-spec-language PR 857 spec review: FR-360 item 5, FR-360-AC-5 and TC-1007 step 3"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9290f614ae36ee7c4986a8024dd3917294443667; spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md, spec/test-cases/TC-1007-check-variant-values-and-refuse-mistyped-uses.md"
review_set: subset
---

## Summary

Ticket: QSL-502. Spec review of the FR-360 item 5, FR-360-AC-5 and TC-1007
step 3 edit. Read against FR-318 Behavior "Order" and FR-318-AC-4.

The edit replaces the retired `unknown_required_feature`/`unsupported-feature`
placeholder with FR-318's scrutinee refusal, `ill_typed`/`type-mismatch` at
the scrutinee. This matches FR-318 ("a scrutinee of an enum type reports the
same") and the code: `UnionRef::of` returns `None` for `ValueType::Enum`.
TC-1007 step 3 agrees with the AC. No other spec text in the repository
still names the deleted placeholder for `case` or union.

Recorded, not blocking (medium): FR-313 Behavior still says "The FR-151
dispatch rewrite ... SHALL NOT rename an arm binder". The PreserveBind fix
in this PR now renames an arm binder when a rename target would capture
it, as `let` does. The code is correct. FR-313's sentence should say
capture-avoiding renaming. Recorded here, not as a defect of the FR-360 edit.

Recorded (low): item 5's sentence "The scrutinee refusal above is the one
FR-318 gives" repeats the sentence before it.

## Verdict

PASS for this method: the edited text is correct and agrees with FR-318 and
the code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
