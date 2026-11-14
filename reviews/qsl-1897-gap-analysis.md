---
id: SR-2026111405325202
title: "gap analysis of PR2702 serving local-projection and realizability"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-573, FR-574, FR-057-AC-15, TC-155, TC-1413 against qsl-analyze/src/protocol_analysis/{item,manifest,realize}.rs, qsl-replay/src/protocol_analysis/{realizability_replay,settle}.rs, qsl-analyze/tests/it/protocol_realizability.rs; PR2702 5d32b7e0f4cc3d33ce7d17af10881b37d8a62c14"
review_set: subset
---

## Summary

Ticket: QSL-1897. This is a planless, diff-scoped gap analysis. Plan completion: not assessed. The reviewer read the tests' `#[trace]` tags against the AC text rather than running `quire matrix`. The brief allows no builds, and the tags are directly readable. Every new test uses `ix_trace_rs::trace`, and none has a `/// Trace:` line. Each test asserts typed values: labels, axes, `TerminalValue`, `NegotiationSettlement`, `RealizabilityDisagreement`.

Bindings checked: `a_driver_run_settles_each_kind_under_each_label` covers FR-573-AC-1, AC-2, AC-6 and AC-7, with 11 items in one `analyze` run, each asserted on all four axes and its value. `the_protocol_kinds_are_served_by_the_linked_manifest` covers the `Single` and `UnsupportedRequestedCapability` clauses of FR-057-AC-15 for both kinds. `the_realizability_replay_arm_reproduces_both_forms_and_refuses_edits` covers AC-2's replay half for both realizability forms. Its AC-3 binding is partial; see code-review FND-003 (SR-2026111405325201), which is not repeated here. Code with no owning requirement: none. The manifest, the item adapters, the settlement map, the replay arm and `Execution::Unsupported` each trace to FR-573 or ADR-036 PA-4/PA-20.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The driver test is tagged FR-573-AC-4 and FR-573-AC-5 but does not run the cut and meter cases those ACs name. AC-4 covers "a result that depended on a cut", and no cut-count item is settled. AC-5 covers `max_transitions` and the evaluation meter, and only `max_states` is settled. The new meter arm in `realizability_item` (`ExpansionLimit::EvaluationMeter` to `ModelCheckLimit::EvaluationMeter`, with its value from `meter_bound`) has no test. TC-1413 still lists the cut-count run (step 4) and the `max_transitions` and meter stops (step 5). One meter-stopped realizability item in the driver run would cover the new code. | qsl-analyze/src/protocol_analysis/item.rs:129-137; qsl-analyze/tests/it/protocol_realizability.rs:732-742 |

## Scope and evidence

- FR-573-AC-1 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): sound projection and realizable protocol settle proved, closed-scope, completed, satisfied, Uncertified. (examined)
- FR-573-AC-2 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): counterexamples settle refuted only after replay. (examined)
- FR-573-AC-3 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): a disagreeing replay settles inconclusive. (examined)
- FR-573-AC-4 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): horizon, absent premise, cut settle inconclusive. (examined)
- FR-573-AC-5 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): max_states, max_transitions, meter settle failed resource-incomplete. (examined)
- FR-573-AC-6 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): FR-575 construct settles unsupported. (examined)
- FR-573-AC-7 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): no proved is Certified or Trusted. (examined)
- FR-057-AC-15 (spec/functional/FR-057): negotiation cause codes and settle_candidates settlements. (examined)

## Verdict

Coverage is adequate for the brief, which does not ask for exhaustive tests. FND-001 is low, and adding a meter-stopped item would close it. The AC-3 gap is carried by code-review FND-003.
