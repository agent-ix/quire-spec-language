---
id: SR-12140
title: "QSL-850 PR #963 gap analysis: FR-381 (TC-1057, TC-1058) and FR-218 role models"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@49d2f331c508a7574315d3852f8fc1aedf06d1a7; FR-381-AC-1..AC-15, FR-218-AC-24, TC-1057, TC-1058; qsl-semantics/src/check/protocol_obligation.rs, qsl-semantics/src/check/protocol_obligation/tests.rs, qsl-semantics/src/check/check/typing.rs, qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-forms/tests/it/protocol_member_forms.rs"
review_set: subset
---

## Summary

Ticket: QSL-850. PR quire-spec-language#963. Planless, diff-scoped gap
analysis. Plan completion: not assessed. Criteria were traced to tagged tests
by reading. No matrix or test was run (static brief).

Trace bindings in the diff (`#[trace]`): FR-381-AC-2, AC-3, AC-4, AC-14,
AC-15 (TC-1057); FR-381-AC-8, AC-9, AC-10 (twice) (TC-1058). Each was read
against its test. Each asserts the code, the cause, the span text and the
retained members, and each would fail if the behaviour were wrong, within
the limit SR-12139 FND-001 states. FR-381-AC-1, AC-5, AC-6, AC-7, AC-12 and
AC-13 are named only in doc comments, with no `#[trace]`. No test claims
FR-381-AC-11, FR-381-AC-12 or TC-1058 row 15c, which matches the PR's stated
exclusions (the recover-binder `missing-name`, and STD-158's inline
`attempts` type). FR-218-AC-24 is named in a doc comment on the assembler
scalar test only.

Clean units: the stage table (AC-1 refusal half, AC-2, AC-4, AC-14, AC-15),
capture typing and totality (AC-3), the forward binder, role and operation
refusals (AC-5 refusal half), zero attempts (AC-7 refusal half), deadline
(AC-8), retry binders (AC-9), and the retry relation with the Int-sum
equality and its range obligation (AC-10).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-381-AC-11's `unsupported_construct`/`declaration-form` refusal (`commit-precedes-effect`, a commit that dominates the forward effect) is not implemented. `template()` only resolves the commit anchor. The ticket assigns AC-1..12 and the commit-dominance edit to this slice, and the PR body does not list it as excluded. | qsl-semantics/src/check/protocol_obligation.rs:899-904 |
| FND-002 | medium | FR-381-AC-5's binding of the template to the operation's one shared anchor node and frame node (TC-1058 row 5) is not implemented. Only a `ClauseOperation` is recorded, and nothing ties it to the attempt's frame. This is not disclosed. | qsl-semantics/src/check/protocol_obligation.rs:950-1003 |
| FND-003 | medium | No test asserts any `CheckedCompensation`, because no protocol with a template can compile while its `effect`/`commit` nodes are uncovered. So the recorded-output halves of FR-381-AC-1, AC-5, AC-6, AC-7 (the exact count), AC-11 (`Never`/`Node`) and AC-12 (two compensations) are untested. | qsl-semantics/src/check/protocol_obligation/tests.rs:7-11 |
| FND-004 | medium | FR-381-AC-13's variant-type case (`attempts 3 of M::Mode`) is a named, reachable form, but it was dropped together with row 15c's inline type. The fixture has no `Mode`. | qsl-semantics/src/check/protocol_obligation/tests.rs:833-851; qsl-semantics/tests/fixtures/protocol_obligations.rs |

## Verdict

CHANGES REQUESTED on FND-001. The commit-dominance refusal is an
acceptance criterion this slice owns, and no code satisfies it. Either
implement it, or have the lead defer it explicitly to a named ticket and say
so in the PR body. FND-002 to FND-004 are recorded and do not block alone.

## Dispositions

Round 2, reviewed at 379bf1dbdf19902daadc11834ae19e20573d641f (fix range `49d2f331c..379bf1dbd`).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | deferred | An honest deferral to C1b2 (the lead names it as QSL-849's follow-up). FR-381-AC-11 is defined over the compiled causal edge relation, and S3 builds none yet: there is no edge relation in `protocol_controls` or `protocol_clause`. While `content` refuses `effect`/`commit` nodes as not-yet-implemented, no protocol holding a template compiles, so a dominating commit cannot reach a package. The PR body now lists it under "Not in this PR", and no trace tag claims AC-11. |
| FND-002 | deferred | Named in the PR body under "Not in this PR": the template records its `ClauseOperation` only, and tying it to the attempt's shared anchor and frame lands with C1b2. No template protocol compiles until then. |
| FND-003 | deferred | Named in the PR body under "Not in this PR": no `CheckedCompensation` can be asserted until C1b2 covers `effect`/`commit` nodes. None of the affected criteria is trace-tagged. |
| FND-004 | fixed 379bf1dbd | The fixture adds the variant type `Mode` (Enum plus Variant records), and `an_attempt_type_must_be_a_record_or_an_event` asserts `invalid_model_binding`/`malformed-declaration` at `M::Mode`. |
