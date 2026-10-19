---
id: SR-12900
title: "Integrity review of PR #992: FR-310 without internal and visible rows against FR-147, FR-148, TC-552 and ADR-020"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@91bf1ee1d; FR-310 Behavior and AC-2; FR-147 Inputs, Internal abstract steps, AC-1, AC-2, AC-4 and the CasTwice fixture; FR-148 internal closure; TC-552 steps 1 and 4; ADR-020 section 1 lines 135-138, RM-9, MC-1, QS-11; FR-137 and TC-542; agent-ix/quire-specification@origin/main FR-375, FR-432, shared-grammar protocol-step-row"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-310
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: reviews
---

## Summary

Ticket: STD-178. PR quire-spec-language#992. Checks the alignment of FR-310
with QSpec FR-375 and FR-432 field by field. Also checks whether the rest of
the QSL corpus that reads the refinement form is still consistent with it:
FR-137, FR-138, FR-147, FR-148, TC-542, TC-552, TC-889 and ADR-020.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | After this PR, two contradictory rules for abstract-step visibility sit in the corpus, and FR-310's own AC-2 cannot be met. FR-310 now says there is no `internal` or `visible` row and that an abstract step is internal only when the profile's list names it (QSpec FR-432: "an abstract step the list does not name is visible"). FR-147 still requires `internal`/`visible` rows and refuses `missing-name` for an abstract class that none names. Its fixture `CasTwice` carries `internal Spec::Twice::finish`, and AC-1, AC-2 and AC-4 check `abstract_internal = [finish]`, refusals on `internal`/`visible` rows, and `-> Spec::Twice::a1`. That target has neither `node` nor parentheses, so it is not a valid `step-target` or `protocol-step-target`. FR-148's internal closure reads "every class an `internal` row names". TC-552 steps 1 and 4 and ADR-020 RM-9 (and section 1 lines 137-138) carry the same rows, while ADR-020 MC-1 already uses the profile list. What breaks: FR-310-AC-2 builds `CasTwice`, and S1 refuses its `internal` row, so the AC fails as written. An S3 coder who follows FR-147-AC-4 needs rows that S2 never builds. FR-147 calling the spelling "illustrative" does not save it, because its refusal rule is semantic, not a matter of spelling. Both issues the author flagged are real contradictions. | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md:91-95,114; spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:33-34,49-53,91-111,137,141-144; spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md:63-65; spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:23-27,33; spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md:137-138,152 |

## Verdict

Not mergeable until FND-001 is fixed. The minimal fix is in this PR (or a
sibling PR merged together with it):

- rewrite FR-147's Inputs and Internal abstract steps, AC-1, AC-2, AC-4 and
  the fixture to FR-432's rule (abstract internal = the profile list; no
  rows; right sides `-> node Spec::Twice::a1`);
- rewrite FR-148's closure to read the profile list;
- update TC-552 and ADR-020 RM-9 and section 1 lines 137-138 to match.

Everything else lines up with QSpec. `HistoryLiteralForm` covers exactly the
seven `history-literal` spellings (`true`/`false` as `Boolean`).
`StepTargetForm::Node` matches `'node', qualified-name, [ '(' ... ')' ]`.
`Memory(Name)` matches `'memory', ident`. The `Class` keywords are the
protocol-step-source set. FR-138's `initial: Literal` is consistent.
FR-137-AC-4, TC-542 step 4 and ADR-020 lines 135-136 agree with the
`weak | strong` fairness row.

Open code PRs that follow from this:

- #988 (QSL-866) still refuses `assume fair strong` in its CST test.
- #970 (QSL-543) reads kindless `fair` as `Weak`, which QSpec FR-375-AC-10
  now refuses. Its `StepRowForm{concrete: NameForm}` has no
  `History`/`StepSource`/`Node` form.

Both have to be reshaped to the merged spec.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | FR-147 keeps a QSL-only refusal. An abstract class that the profile's internal list names and a step row also targets refuses `conflicting-binding`, and so does FR-147-AC-4's `finish` vector. QSpec FR-432 has no such refusal; it only reads a list-named step as internal. QSL therefore refuses a declaration QSpec admits. Either QSpec should state the refusal, or FR-147 should cite it as a QSL addition. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:99-103,139,141 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | aea8326f7 removes the `internal`/`visible` rows from FR-147 (Inputs, Behavior, fixture, AC-1, AC-2, AC-4), FR-148's closure, TC-552 steps 1 and 4, ADR-020 RM-9, ADR-020 section 1 lines 137-138, and ADR-020 QS-11. They now use the profile's internal list, and an unnamed abstract class is visible. Right sides are written `-> node Spec::Twice::a1(self)`. FR-310-AC-2's `CasTwice` no longer carries a row that S1 refuses. |
