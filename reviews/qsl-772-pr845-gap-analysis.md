---
id: SR-9127
title: "Gap analysis of quire-spec-language PR #845: supplied-value phase budgets"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1d128adad6f111ba0d72d666ed08c860f381aea9; PR #845 tests vs FR-098-AC-8/9/12, FR-255-AC-3/4, FR-277-AC-4/5/6, FR-321-AC-5, TC-830, TC-906; qsl-eval/tests/it/supplied_phases.rs, qsl-replay/src/execute/tests/supplied.rs, qsl-replay/src/spine/call/tests.rs, qsl-replay/src/limits.rs, src/command.rs tests"
review_set: subset
---
# Gap analysis of quire-spec-language PR #845

## Summary

Ticket: QSL-772. Planless gap analysis over the PR's tests and trace tags
against the merged QSL specs, plus QSpec FR-323 AC-10..14 and TC-278 (ids
only). Plan completion: not assessed.

Examined:
- FR-098-AC-12 / FR-277-AC-4: H-1 and A-1 deny the actual next event with
  phase, parameter, event, setting, ceiling, successful prefix, denied amount
  and attempted count, and the exact ceiling admits. Counts 0, 1 and 4, plus a
  20,000-element flat sequence. The expected sequences come from hand-written
  generators of the event tables, not from values read back from the code
  (clean).
- FR-277-AC-5: cumulative A and H across two arguments, where the second is
  denied with consumed = total - 1. A Requested/Deadline cancellation before
  conversion and during the later argument keeps its cause and the first
  argument's spend (clean).
- FR-098-AC-9: C and occurrences are conversion-phase pre-call records with no
  kernel Incomplete (clean).
- FR-098-AC-8: invalid input keeps WrongValueKind with the phase context
  (clean).
- FR-255-AC-3/4, QSpec FR-323 AC-10 (TC-278 RE-12..14): defaults, identity
  equality, the stage_limits refusal, and the closed/complete/u64 wire reader
  in replay and the CLI (clean).
- TC-830 substitution: the PR body says the full Tree is not done (needs
  QSL-501/503 and QSV union membership). The research predictions 69,997 and
  129,994 appear nowhere as expected values (clean as to the body; see
  FND-001 for the tags).
- The result-level zero-charge and no-FunctionCall assertions are in every
  replay limit test (clean).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Trace tags claim coverage the tests do not give. `#[trace("TC-830", "FR-277-AC-4", "FR-321-AC-5")]` sits on a flat Boolean-sequence test, but FR-321-AC-5 is defined only on TC-830's unchanged full Tree, which this PR does not build. `#[trace(.., "FR-277-AC-6")]` sits on runtime tests of the strict refusal and of the lenient unbudgeted admission, but AC-6 is Analysis-only: TC-906 says no runnable trace is claimed for it, and its evidence is not a runtime unsupported result. The matrix would therefore report FR-321-AC-5 and FR-277-AC-6 as covered, which contradicts the PR body's own not-done list | qsl-eval/tests/it/supplied_phases.rs:194, qsl-eval/tests/it/supplied_phases.rs:511, qsl-eval/tests/it/supplied_phases.rs:546, qsl-replay/src/execute/tests/supplied.rs:575 |

## Verdict

**FAIL: 1 blocking finding (FND-001)**, a trace-inflation fix only: drop
`FR-321-AC-5` and `FR-277-AC-6` from those tags. The behavioural coverage of
FR-098-AC-8/9/12, FR-255-AC-3/4 and FR-277-AC-4/5 on supported (Boolean)
fixtures is real and derived independently. The missing numeric-domain A-1
control is code-review SR-9126 FND-001.

Recorded, not blocking:
- The TC-830 full Tree and the 29,998/29,999 controls are not done (QSL-501/503).
- The Storage/Capacity arms and cold/warm cache parity (TC-906 step 6) are
  untested.
- TC-906 steps 8-9 are Analysis that this PR does not carry.
- Decimal is blocked on IR-718/IR-719.
