---
id: SR-11090
title: "Gap analysis (round 2) of quire-spec-language PR #845: supplied-value phase budgets and magnitude bound B"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0dc1e01879383b0727bef72a6ec9d3837ec82ae1; PR #845 tests vs FR-098-AC-8/9/12, FR-255-AC-3/4, FR-277-AC-4/5/7, TC-906 step 10; qsl-eval/tests/it/supplied_phases.rs, qsl-replay/src/execute/tests/{supplied.rs,composite.rs,tests.rs,child_composite.rs,variant_fields.rs}, qsl-replay/src/limits.rs, src/command.rs tests"
review_set: subset
---
# Gap analysis (round 2) of quire-spec-language PR #845

## Summary

Ticket: QSL-772. A planless gap analysis of the PR's tests and trace tags
against the merged QSL specs and QSpec FR-323 AC-10..16 / TC-278 (cited by
id only). Plan completion: not assessed. Nothing was built. Gate evidence
comes from the coder's logs: clippy -D warnings exit 0 on qsl-semantics,
qsl-eval and qsl-replay; qsl-replay lib 527 passed, 1 ignored; qsl-eval 62
and 342 passed. The trace-to-AC check was done by hand from the diff; no
matrix was run.

Examined:
- SR-9127 FND-001 (overclaiming tags): fixed. No `FR-321-AC-5` or
  `FR-277-AC-6` tag remains in code (clean).
- FR-277-AC-7 / TC-906 step 10: 4097 bits stops with operand `value`, limit
  4096, next_charge/counter 4097, value-visit, consumed 0, pre-call.
  Omitted == stated 4096; 4095 and 4097 differ. The boundary at exactly B
  holds for the value (4096) and for an endpoint (B=4, upper 9). Zero is
  one bit; B=0 admits nothing (clean; one medium recorded).
- FR-277-AC-4/5, FR-098-AC-12: H-1/A-1 deny the enumerated next event, the
  exact ceiling admits, both budgets are cumulative across arguments, and
  Cancel keeps its cause and the spend (clean).
- A/H/B independence: `a_h_and_b_are_accounted_independently` (clean).
- Existing tests: after the strict commit 872752303 the only change to a
  pre-existing test file is the one `#[ignore]` on
  `tc_906_a_set_holding_numerically_equal_decimals_refuses`, with an
  untouched body. The earlier edits move the carrier from a kernel
  `Incomplete` to the FR-277 pre-call record, and their assertions are
  equal or stricter (phase, event, no kernel limit) (clean).
- The PR body states the reservation-denial AC and the TC-830 full Tree are
  not done (clean).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS for gap analysis.** No AC this PR claims lacks a live typed test. The
composite-parity numeric-root gap is code-review SR-11089 FND-001.

Recorded, not blocking:
- medium: the TC-906 step 10 fixture is not used verbatim. It specifies
  `[0, 2^4096 - 1]` with `2^4096 - 1` admitting; the test uses `Int[0, 9]`,
  where the 4096-bit value is refused by domain. The B boundary is still
  bound by the B=4 test.
- low: 3 of the 58 listed tests are root-crate `spine_run` tests. They were
  not re-run at the final head; the lead's final `make ci` covers them.
