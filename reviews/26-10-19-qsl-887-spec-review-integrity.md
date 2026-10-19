---
id: SR-13076
title: "Integrity review of PR #1023: internal-list terminology and model-subject consistency"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@54b78f5f7; git diff origin/main...HEAD: ADR-020 RM-9/QS-11, FR-135, FR-147, FR-310, TC-552; dependents FR-148, FR-214"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-310
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: reviews
---

## Summary

Ticket: QSL-887. Checked that the PR's move from "the profile's internal
list" to "the refinement's internal list", supplied as the compilation's
`refinement_internals` input, is applied consistently, and that FR-147's
statements and criteria agree with each other. A grep for "profile's" and
"internal list" across `spec/` finds the old wording left in an edited file
and in dependents. FR-135, FR-147's body and TC-552's procedure use the new
wording.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-310 S2 rows bullet | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md | examined |
| FR-310 abstract-subject bullet | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md | examined |
| ADR-020 RM-9 | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md | examined |
| ADR-020 QS-11 | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md | examined |
| FR-135 Refinement features the build does not check | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-147-AC-2 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147 Class spellings table | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-148 Closure | spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md | context_only |
| FR-214 | spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-310, which this PR edits, still says an abstract step is internal "only when the refinement profile's list names it (QSpec FR-432)". QSpec FR-432 and the new FR-147 say the list is not the profile's: it is the compilation's `refinement_internals` input. Say "the refinement's internal list (the compilation's `refinement_internals` input) names it". | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md:93-96 |
| FND-002 | low | ADR-020 RM-9's new text keeps "the refinement profile's explicit list names the abstract step classes that are internal" and two sentences later says a profile selection carries an identity only and the list is the `refinement_internals` input; QS-11 (edited) still lists "the profile's list of internal abstract step classes". MC-1 (line 322) carries the same phrase. | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md:150,560 |
| FND-003 | low | Dependents outside the diff still name the profile as the list's owner: FR-148's internal closure ("the refinement profile's internal list names (`abstract_internal`)") and FR-214 ("the profile's explicit list of internal abstract steps"); ADR-027 PR-2 and line 598 likewise. FR-148 is normative and decides the steps from this list. | spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md:65; spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md:44 |
| FND-004 | medium | FR-147-AC-2 says without condition "A non-empty list on `CasRefinesCounter` is AC-7's, not a binding refusal." FR-147's Defects table and QSpec FR-432 say that once `refinement-internal-list` is built the same list refuses `invalid_model_binding`/`malformed-declaration` at the object's pointer, which is a binding refusal. Condition the sentence on a build that omits the feature. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:249 |
| FND-005 | low | The Alias-free column of the class-spelling table lists one form per row where the Spelling column lists two (`fork` only for fork/join, `lose` only for duplicate/lose, no template form in the first row), so it can be read as saying `duplicate`'s alias-free spelling is `lose …`. List both forms, or state the rule once ("the Spelling with `<A>::` removed") and drop the column. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:126-133 |

## Verdict

Mergeable after FND-001 and FND-004 (medium). FR-135's support-set
paragraph, FR-310's new `abstract-subject` bullet and FR-147's body are
consistent with each other and with QSpec. FND-002, FND-003 and FND-005 are
wording; FND-003 is outside the diff and may be deferred to a follow-up.

## Dispositions

Round 1, against head 40892cef1 (fix commit edced4cd7).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | edced4cd7: FR-310 now reads "the refinement's internal list (the compilation's `refinement_internals` input) names it (QSpec FR-432)". |
| FND-002 | fixed | edced4cd7: ADR-020 RM-9, MC-1 and QS-11 say "the refinement's internal list". |
| FND-003 | fixed | edced4cd7: FR-148 closure, FR-214 (two places and AC-4), ADR-027 PR-2 and line 598, and TC-659 step 4 use "the refinement's internal list"; grep finds no "profile's … list" left in spec/. |
| FND-004 | fixed | edced4cd7: FR-147-AC-2 conditions the sentence on a build that omits `refinement-internal-list` and sends the built case to the new AC-12 (`malformed-declaration` at `/0`). |
| FND-005 | fixed | edced4cd7: the column is headed "Alias-free (the spelling with `<A>::` removed)" and lists both forms in every two-form row. |
