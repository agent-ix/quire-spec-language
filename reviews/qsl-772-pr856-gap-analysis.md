---
id: SR-11180
title: "QSL-772 quire-spec-language PR 856 gap analysis: reservation-denial AC and TC-906 Decimal type link"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1166f187ce558cca83d0e0dcd1c0b05bf42a5c15; qsl-eval/tests/it/supplied_phases.rs, qsl-semantics/src/value/supplied.rs, qsl-replay/src/execute/tests/composite.rs"
review_set: subset
---

## Summary

Ticket: QSL-772. Planless gap analysis of the closing PR against the
ticket's remaining ACs (the reservation-denial AC added 10-15 and the
un-ignore of `tc_906_a_set_holding_numerically_equal_decimals_refuses`),
QSL FR-277-AC-4/AC-5 and FR-098-AC-8 on origin/main, TC-906 step 7, and
QSpec FR-323 item 6 / TC-278 RE-29 (ids only). No build was run.

Examined:

- Reservation-denial AC. `a_denied_reservation_is_a_typed_allocation_stop_and_the_retry_admits`
  goes through the public `CheckedPackage::call_supplied`. An always-deny
  permit gives `SuppliedCause::Storage`, which by type excludes Capacity,
  Input, Limit and Cancelled. It asserts phase Admission, parameter 0, no
  limit, and `assert_no_evaluation` (meter admission count 0 and every
  `LimitKind` consumed 0, so no FunctionCall charge). An always-permit
  retry admits. The test can fail: without the seam the call admits and
  `stop_of` panics, and `seen > 0` proves the permit ran.
- Capacity half of the AC. Not reachable through a public value (see
  SR-11179). It is pinned at the mapping by
  `storage_and_capacity_arms_map_to_their_own_typed_causes`.
- TC-906 Decimal set. The test body is unchanged from main. Only the
  `#[ignore]` line is removed.
- RE-29 Decimal operands. `a_decimal_type_link_over_b_names_the_endpoint_operand`
  has one vector per coefficient endpoint, asserting the typed limit fields,
  event `type-comparison`, consumed 0 and no evaluation, plus B-equal
  admission for the expected endpoints.

## Verdict

PASS. No blocking findings. Recorded (not blocking, medium): the denial test
does not assert `storage.requested` equals the request the permit saw, nor
the stop's event, locus or consumed (TC-906 step 7 asks for the measured
native request), and the retry is not compared with a fresh run beyond
`is_ok`. The mapping unit test is tagged FR-277-AC-5, but what it asserts is
AC-4's storage/capacity distinction. RE-29's exact-B admit is shown only for
same-type calls, not for the actual-endpoint pairings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
