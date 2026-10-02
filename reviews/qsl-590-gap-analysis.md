---
id: SR-1251
title: "QSL-590 gap analysis of PR #610 (FR-285, FR-100 amended, FR-109, TC-769, TC-786)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@8464165c04450cc813f10637fdcdf80d42e94466; PR #610 diff against origin/main; spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/functional/FR-109-run-a-state-clause-through-the-spine.md; spec/test-cases/TC-769-*.md; spec/test-cases/TC-786-*.md; spec/tests.md; spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: reviews
---
## Summary

Ticket: QSL-590. PR: quire-spec-language#610.

Trace, per AC:
- FR-285-AC-1: `tc_769_exit_function_maps_every_table_row` (qsl-foundation)
  maps the bare categories, and `tc_769_refuted_terminal_record_is_violation_exit_10`
  (proof_result.rs) covers the FR-281-AC-2 refuted record. The AC's
  `analyze` item with cause `CertificateRejected` and FR-283-AC-1's pending
  clause are not built, so this AC is only partly covered (FND-001).
- FR-285-AC-2: `tc_769_multi_item_outcomes_exit_with_the_most_severe_code`
  covers all six sets, each in both orders, plus the empty set. The binding is correct.
- FR-285-AC-3: `tc_769_stage_failures_exit_by_their_category` and
  `tc_769_call_failures_exit_by_their_category` cover `Refused` (21/20),
  `Limit`, `CallFailure::Input` and `CallFailure::Fault`.
  `StageFailure::Cancelled`, `StageFailure::Fault` and `CallFailure::Cancelled`
  do not exist. Both the spec.md and tests.md rows say "partial", which is
  honest. The owner is only partly named (FND-002).
- FR-285-AC-4: `undefined_kernel_reasons_render_and_exit_10` includes
  `sum-out-of-domain` and asserts the JSON and exit 10. The binding is correct.
- FR-100-AC-10 and AC-11 (TC-786): `tc_786_sum_over_pos_is_sum_out_of_domain_exit_10_or_completes_exit_0`
  and `tc_786_run_exit_statuses_come_from_the_exit_function` (binary and
  library on the same input). The bindings are correct.
- QSpec half (position 5): QSpec FR-301 on quire-specification origin/main
  has the exit table with undefined at 10 and the order 30, 20, 21, 22, 10, 0.
  FR-285 cites "QSpec FR-301-AC-2 (STD-141)". STD-143 is the depth-rule and
  v2-body-grammar ticket and has nothing to do with exits. The coder is right
  that the link on QSL-590 is wrong. The matching ticket is STD-141, though
  that ticket still reads Backlog while its FR-301 text is on main. QSpec
  FR-301 has no row for a pending supplied-trace clause (exit 0), but FR-285
  does. That difference predates this PR.
- Underspecified code: none found. Every new `category()` method traces to
  FR-285's "every outcome reports the one Category" or to FR-100/FR-109.

## Verdict

Changes requested: two medium findings and one low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The TC-769 row in tests.md says "steps 1, 2 and 4 pass locally". Step 1 also needs an inconclusive `analyze` item with cause `CertificateRejected` and FR-283-AC-1's pending clause, and neither is built. The test maps bare `Category` values only. FR-285's own spec.md row admits that analyze and the monitor are not built. Fix: mark step 1 partial in the TC-769 row and name the two missing items. | spec/tests.md:1031 |
| FND-002 | medium | The FR-285-AC-3 deferral has no owner. The spec.md and tests.md rows say that `StageFailure::Cancelled`/`Fault` and `CallFailure::Cancelled` "do not exist yet" but name no ticket. QSL-489 (LC1) covers `StageFailure::Cancelled` only. No ticket covers `StageFailure::Fault`, `CallFailure::Cancelled` or finishing TC-769 step 3. Fix: add these to QSL-489 (or the LC3 ticket), and cite that ticket in both rows. | spec/spec.md:1280; spec/tests.md:1031 |
| FND-003 | low | FR-109's reworded exit bullet covers compile refusals, argument refusals, admit failures and evaluate outcomes. It drops the select-stage dispositions (`MissingName`, `NotAPredicate`), which the old "compile, select or admit result" wording covered. They still exit 20 in code. Fix: name the select stage (refusal, 20) in the bullet. | spec/functional/FR-109-run-a-state-clause-through-the-spine.md:143-150 |

## Dispositions

Round 1, reviewed at e4f2770ceef6ddfaed0c9b087f4028723026a5b9 (fix commit bc464d1d6).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | bc464d1d6: the TC-769 row now says that step 1 passes for the bare categories and the `refuted` record only, and names the missing `analyze` item and FR-283 pending clause. |
| FND-002 | fixed | bc464d1d6: the spec.md FR-285 row and the TC-769 row both name slice LC1 as owner of `StageFailure::Cancelled`/`Fault`, `CallFailure::Cancelled` and the rest of step 3. This matches plan v2's type table, which assigns both enums to LC1. QSL-489's body still names only `StageFailure::Cancelled`, so its scope should be widened in Linear. I could not check Linear because the keyring is locked. |
| FND-003 | fixed | bc464d1d6: FR-109's exit bullet names "refusal (20) for a `select` result (`missing_declaration`, `ill_typed`)". |
