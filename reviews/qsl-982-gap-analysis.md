---
id: SR-17101
title: "Gap analysis of quire-spec-language PR 1491 (QSL-982 PR 1): safety-branch key algebra"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@aab93e02ac2fe85f735c2e7622ac1250e96fbaf4; PR #1491 diff against origin/main; FR-338 Canonical translation state and Canonical octet vectors, FR-338-AC-4, FR-338-AC-5, FR-338-AC-7, TC-525 steps 5, 6 and 8; qsl-eval/src/automaton/key.rs, qsl-eval/src/automaton/key_state.rs; trace tags at the reviewed sha"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/quire-spec-language/FR-338
---
# Gap analysis of quire-spec-language PR #1491

## Summary

Ticket: QSL-982. This is a planless audit of the requirement surface the PR
touches, checked against `quoin matrix` (quoin 0.29.1), the trace tags, the
tests and the source. The PR does not change the spec. Plan completion: not
assessed. It merged as `5d37fc8dd`, with the reviewed files unchanged, so these
are follow-ups.

Examined:

- FR-338-AC-4 (examined). `quoin matrix` shows it `tagged`, through one test:
  `automaton_key::safety_keys_are_the_authored_octets_and_a_function_of_the_facts`.
  That test asserts the deadline, offsets, rejecting-state and
  two-occurrence vectors, plus insertion-order and inequality relations. It
  does not assert the bounded, deadlines-1-and-3, TP-4 or guarded-buchi
  vectors, or the full deadlock oracle. See FND-001.
- FR-338-AC-5 (examined). `tagged` before this PR, through
  `automaton_key_bounded`. The PR adds safety-algebra admission negatives
  (missing members, unsorted/duplicate sets, `"03"`, `lower>upper` offsets,
  wrong counter strategy, foreign context, other forms). They are tagged only
  `TC-525`, and no interval-obligation negative is present. See FND-002.
- FR-338-AC-7 (context_only). It is an Analysis criterion
  (`method-without-symbol`, before this PR). The PR's b+1 boundary test is
  evidence that analysis could cite.
- TC-525 steps 5, 6 and 8 (examined).

Reverse gap: every new production path is owned by FR-338's Canonical
translation state section: `SafetyStates`, `admit_envelope`, `FormulaShape`,
`Reader` and `decimal_wide`. None of the new code lacks a requirement. The
semantic disagreements between code and text are recorded in SR-17100
(FND-002 to FND-007).

## Verdict

Findings. FR-338-AC-4's only tag claims more than its test asserts. The new
admission-negative tests are not bound to AC-5. No reverse gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-338-AC-4 shows as `tagged` in `quoin matrix` only through `safety_keys_are_the_authored_octets_and_a_function_of_the_facts`. That test covers the safety rows of the vector table, but not the 2 bounded rows, the deadlines-1-and-3 row, the TP-4 `buchi` row, the 2 `guarded-buchi` rows, or the full `K_C` deadlock oracle. The tag overstates coverage until PR 2 lands the Büchi vectors and the remaining rows are tagged or added. | qsl-eval/tests/it/automaton_key.rs:355-363; spec/functional/FR-338-check-an-en-1-closure-certificate.md:744-765 |
| FND-002 | low | The new admission-negative tests assert FR-338-AC-5's listed negatives for the safety algebra but carry only `#[trace("TC-525")]`, so the matrix binds them to no criterion: `past_memory_is_typed_by_the_occurrence_interval_and_complete`, `window_and_acceptance_members_must_resolve_in_the_formula` and `safety_states_refuse_foreign_contexts_and_other_forms`. AC-5's "lower>upper" and "offset outside the checked interval" have no test for the `[o,s,lower,upper]` obligation form. | qsl-eval/tests/it/automaton_key.rs:549-551; qsl-eval/tests/it/automaton_key.rs:711-713; qsl-eval/tests/it/automaton_key.rs:843-845 |
