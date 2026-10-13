---
id: SR-8705
title: "QSL-740 PR 804 spec review (integrity): FR-056 variant effective identity, FR-081-AC-16, TC-1003, TC-1004"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@2dff29e41112bae401b988287d5a4db06e9fd632; spec/functional/FR-056-* (\"Variant types\" as merged in #799 and the new \"Effective identity of a variant type\", AC-26, AC-29..31), spec/functional/FR-081-* (new paragraph, AC-16), spec/test-cases/TC-996, TC-1003, TC-1004, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: reviews
---
# QSL-740 PR 804 spec review (integrity)

## Summary

Ticket: QSL-740. Base spec review of the spec hunks, using the integrity
sub-analysis (consistency, completeness, ids). The planner asked one question:
does the new mis-minted-case statement match FR-056 "Variant types" and TC-996
as merged in #799, which say a mis-minted case refuses at the case node with
path `Variant.identity` at intake?

The two statements describe different stages, so they do not contradict in
effect. Under FR-056's reader-first rule the reader's `DUPLICATE_IDENTITY`
refuses the package before intake runs, so the intake `Variant.identity` rule
is never reached when the mis-minted identity equals a sibling's. Two
problems remain. The merged rule is still written unconditionally, and its
reader-first sentence covers only cases with equal names. And the new
paragraph's justification is false in one of the two orders it tests
(FND-001, FND-002).

## Verdict

**Findings: two medium, one low.** The effective-identity text itself
(qualify fact, owner, case original equal to the case node key, reorder
invariance, `Gold`/`gold` distinct, `M::<artifact id>` in byte order, FR-150
unchanged, QSL-738 placeholder kept) is consistent with QSpec FR-208:43 and
FR-154 and with the code. The numbering does not collide: FR-056-AC-29..31,
FR-081-AC-16 and TC-1003/TC-1004 appear on no other remote branch or open PR.
The tests.md rows match the TC files.

Follow-up for the QSpec spec owner, not a finding here: QSpec model-complete.md
does not list variant cases in its qualify list. The QSL spec does not claim
that it does. It cites FR-208:43 and FR-081 only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new mis-minted paragraph ends "Intake never reads the later case, so the refusal names the case that causes the conflict, not the first holder of the identity." That is false when the mis-minted case comes first. With `[Silver (carrying .../variant/Tier-Gold), Gold]`, which TC-1003 step 5 and the test at variant_type.rs:905 exercise, the refusal is at `variants[1]`, Gold's span. Gold is correctly minted. The mis-minted Silver is never named. The reader always refuses the second holder of the identity, whichever case is mis-minted. Also, intake does not run at all, so it reads neither case. QSL-740 AC 3's wording ("the later case node, the node that causes the conflict") has the same conflation. State only what holds: the refusal is at the later list position, and it may name the correctly minted case. | spec/functional/FR-056-admit-domain-package-model-declarations.md:561-567 |
| FND-002 | medium | The paragraph sits in the effective-identity section, three lines after that section says "This section adds no refusal of its own: every refusal of a variant type is typed under 'Variant types' above". So the section contradicts itself. Meanwhile "Variant types" (merged in #799) still says unconditionally that intake SHALL refuse a mis-minted case at the case node with `Variant.identity`, and TC-996 step 5 says the same. The reader-first sentence there covers only two cases "with equal names". Fix it in "Variant types": widen the reader-first sentence to any two cases that share one identity, whatever their names, and scope the `Variant.identity` rule to an identity the reader admitted. Then move the paragraph there, or reduce it to a pointer. | spec/functional/FR-056-admit-domain-package-model-declarations.md:503-509; spec/functional/FR-056-admit-domain-package-model-declarations.md:557-567; spec/test-cases/TC-996-refuse-malformed-and-duplicate-variant-cases.md:30 |
| FND-003 | low | The paragraph fixes the cause of two case nodes sharing one identity as `malformed-declaration` (the reader's generic mapping). FR-056 says a two-node identity collision is the one "FR-154 refuses" as `conflicting-binding` (line 503). QSL also classifies duplicate process step, state and transition identities from the same `DUPLICATE_IDENTITY` diagnostic as `conflicting-binding` (`duplicate_process_step_refusal`). Pinning `malformed-declaration` for variant cases makes them the odd one out. Either state why variant cases differ, or map them as the others are mapped. This is a judgement call for the spec owner. | spec/functional/FR-056-admit-domain-package-model-declarations.md:561-565; qsl-semantics/src/model/intake.rs:1636-1700 |
