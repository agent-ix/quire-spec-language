---
id: SR-1302
title: "Gap analysis of PR #634 (QSL-351: DeclineCode::Std001)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6725aa7a610aa33a18f3c2d76443018dd10259ff; qsl-replay/src/proof_result.rs, qsl-replay/tests/terminal_record_facade.rs, spec/functional/FR-069-implement-typed-proof-result-envelope.md, spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md"
review_set: subset
---

## Summary

Ticket: QSL-351. PR agent-ix/quire-spec-language#634. Manual
AC-to-test-to-code check for the `Std001` arm.

- `tc_177_every_fr331_value_maps_to_its_exact_category` gains a `Declined`
  row with a `Std001` code (category `refusal`, no inconclusive cause), and
  the facade test builds two `Std001` declines through root paths and checks
  category, `request_index`, value and that a `Qsl` and a `Std001` decline
  differ. Both oracles are independent literals.
- `a_declined_value_carries_a_std001_code_without_remapping_it` checks that a
  run-time and a literal code compare equal, that a malformed code does not
  build, that an unregistered well-formed code does, and that a `Std001`
  decline differs from a `Qsl` decline with the same cause.
- The rule itself (a code is never remapped; no issuer; no unregistered
  refusal) is stated in FR-121's statement and ADR-013 O-16/O-24, with no
  AC. All three tests trace to FR-069-AC-1, whose text names no
  `DeclineCode`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `Std001` arm has no AC: the three tests trace to FR-069-AC-1, which covers category mapping and names no `DeclineCode`, and half of the new unit test asserts `Std001Code`'s own form check (IR's behaviour, not QSL's). Add the arm to FR-069-AC-1's list (a `declined` with a `Std001` code maps to `refusal` and keeps its code) and drop the IR form-check asserts, or trace them to an AC that states them | qsl-replay/src/proof_result.rs:719-748 |

## Verdict

Approve with one low finding. The arm is built and covered by
independent oracles; it lacks an AC of its own.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | TC-177 step 1 still lists one `declined` record and says "Eleven records total"; since this PR the test builds a QSL and a STD-001 `declined` (twelve records), which only the new Expected Results bullet mentions. Separately, the amended FR-069-AC-1 lacks "and" after "keyed by its `request_index`)" before "a `declined` result" | spec/test-cases/TC-177-proof-result-category-preserving-map.md:32-42, spec/functional/FR-069-implement-typed-proof-result-envelope.md:82 |

## Dispositions

Disposition pass 1, reviewed at `00ad945c1` (range `6725aa7a..00ad945c`), no build.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 00ad945c: FR-069-AC-1 lists a `declined` with a QSL code and one with a STD-001 code, even unregistered (category `refusal`, never remapped); TC-177 adds the matching expected result; the IR form-check asserts (`std001_code!` equality, `Bad-Code` refusal) are dropped, leaving the test on QSL behaviour traced to the AC that now states it |
