---
id: SR-936
title: "Gap analysis of PR #551 (delete ToolPin and the toolchain pin from qsl-replay)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@cfef8e790f508ebbe8ffbdba796072805841ef1f; qsl-replay/src/**, spec/functional/FR-069-implement-typed-proof-result-envelope.md, spec/functional/FR-072-implement-typed-replay-result.md, spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-122-replay-a-state-clause-counterexample.md, spec/test-cases/TC-179-proof-result-round-trip.md, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-351 (ToolPin part only). Manual AC-to-test-to-code check for the
ACs the PR edits.

- FR-069-AC-3 (round trip preserves `backend` and every disposition) is
  traced by `#[trace("TC-179", "FR-069-AC-3")]` on
  `tc_179_round_trip_preserves_backend_and_dispositions`
  (proof_result.rs:514), which still asserts backend identity, manifest
  digest, the per-item records and the mutated-digest case. The tests.md row
  and the TC-179 locator line name the renamed function.
- FR-098-AC-2 is traced by `tc_444_an_input_counterexample_replays_and_agrees`
  (execute/tests.rs:201); the AC no longer claims a pin and the test no
  longer asserts one; the charges assertion it keeps matches the AC's
  remaining "carrying the call's charges".
- FR-072 Description/Inputs/Outputs and FR-122 Behavior no longer list a
  pin; no FR-072 or FR-122 AC ever named one, so no AC was deleted or
  renumbered.
- No spec text now promises a member the code lacks, and no code member
  survives without a spec owner.
- Held items (`TerminalValue::Inconclusive`, typed `request_index`,
  QSL-352) were not assessed, per the brief.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. Every edited AC keeps a tagged test that exercises its remaining
text, and the tests.md locators match the code.
