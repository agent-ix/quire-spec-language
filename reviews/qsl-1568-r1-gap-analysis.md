---
id: SR-20261103161226
title: "QSL-1568 r1 gap analysis of PR #2204"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2204 head bb13ab56e; FR-501 refusal bullet and AC-26; qsl-eval/src/temporal/finite_window/run.rs; qsl-eval/tests/it/finite_window.rs; qsl-replay/src/spine/clause/tests/temporal.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: references
---

## Summary

Ticket: QSL-1568. This is a planless gap analysis scoped to the PR diff. Plan completion was not assessed. The `quoin matrix` command was not run for this diff-scoped pass. Instead I checked by hand that each AC the PR touches has a test that asserts it:

- **FR-501-AC-26.** `a_repeated_key_of_one_authority_is_not_a_usable_order`, tagged `#[trace("TC-1295", "FR-501-AC-26")]`, asserts the `until` and `release` refusal in both listings and the `eventually` outcome and `result_identity` across both listings. Both mutations of `require_order` (identity tie-break restored; refusal extended to `eventually`) kill it.
- **FR-501-AC-20.** Still traced by `qsl-eval` `finite_window.rs:1089` and `qsl-replay` `temporal.rs:2397`, both using distinct keys.
- **FR-501-AC-7.** Still traced at `finite_window.rs:532` and `temporal.rs:1841`.

No code in the diff lacks an owning requirement. The test does not replace the unit under test.

## Verdict

**APPROVE.** Coverage of AC-26 is real: the test fails under both mutations. One low gap remains. The refusal bullet covers `since` and `triggered`, but AC-26 and the test exercise only the future operators.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-501 refusal bullet applies the repeated-key refusal to all four order-sensitive operators, but AC-26 and its test cover only `until` and `release` (`on origin`). The past scan visits candidates in reverse canonical order, so `require_order` takes a different path there, and no test checks that `since` or `triggered` (`on each`, anchored at position 2) refuses at position 2 over a repeated key. AC-7 already covers the keyless case for both directions. Adding `since` and `triggered` to AC-26 and its test would close the gap. | spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md:367; qsl-eval/tests/it/finite_window.rs:1389-1401 |
