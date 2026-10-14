---
id: SR-9101
title: "base review of the supplied-value phase accounting contract (PR #694)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@86628e9f1; spec/functional/FR-098, FR-255, FR-277, FR-321, FR-322; spec/decisions/ADR-014; spec/test-cases/TC-830, TC-831, TC-906"
review_set: subset
---

## Summary

Ticket: QSL-681. Base checklist review of PR #694 (spec-only), which settles
supplied-value admission and replay-conversion phase accounting under the
ticket's option 1: FR-098's converted-node and occurrence counts keep their
meaning, and two caller-owned cumulative helper budgets
`supplied.admission_work_units` and `supplied.conversion_work_units` are added
(default 16777216 units each). The review was scoped to findings that produce a
wrong or contradictory requirement, an AC that no TC can verify, a broken
validation gate, or a contradiction with merged code. Four blocking findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The two new FR-255 setting-table rows have no code. `qsl-replay/src/limits.rs` embeds FR-255 with `include_str!` and asserts that the code's setting names equal the table's names (TC-720, FR-255-AC-3, plus the published-defaults test). Merging this spec-only PR turns `make ci` red. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:79-80; qsl-replay/src/limits.rs:250,271,324-352,517 |
| FND-002 | high | FR-277 and FR-098 say a replay request carries the helper ceilings in QSpec FR-323's typed `limits.supplied` member. QSpec FR-323's `limits` member names only `ScalarLimitsV1` and `PopulationAdmissionLimitsV1`, so `limits.supplied` does not exist. The requirement also forbids a private member and forbids `stage_limits`. The blocking sentence covers only the result-side phase projection. Nothing says request-side carriage, and so TC-906's replay A/H controls, stay blocked until the shared contract changes. | spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md:113-117,245,393-395; spec/functional/FR-098-execute-a-replay-request.md:109-111 |
| FND-003 | high | Merged FR-255-AC-4 is unchanged. It requires every setting-table row to be raisable through the replay request's `stage_limits` entry of that name. Its only exceptions are `replay.input_bytes`, `format.output_bytes` and rows whose stage a replay does not run. The new `supplied.conversion_work_units` row is a replay stage, yet FR-098 and FR-277 now say it SHALL NOT be passed through `stage_limits`. The two requirements contradict each other. FR-255-AC-1 likewise lists only `explore.*` as a no-code exception. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:233,239,103-106; spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md:113-117 |
| FND-004 | high | TC-906 step 5 cannot pass for the converted-node C bound. Per FR-098, C and evaluation share one effective `work_units`. At `work_units` = C the full Tree converts but cannot complete evaluation: C is at most about 29,999 converted nodes, and evaluation needs TC-831's semantic total (predicted 39,997). The step nevertheless requires "enough evaluation work for a completed replay", and its expected result says the Tree "completes ... at the exact independent bounds". | spec/test-cases/TC-906-composite-arguments-replay-and-refuse-by-kind-domain-and-limit.md:74-80,170; spec/functional/FR-098-execute-a-replay-request.md:186-196 |

## Verdict

Blocking: FND-001 through FND-004.

Checked clean:
- Each new or changed AC has a TC binding that exercises it: FR-098-AC-9 and AC-12 (TC-906), FR-321-AC-5 (TC-830), FR-322-AC-4 (TC-831), FR-277-AC-4/5 (TC-830, TC-906) and FR-277-AC-6 (Analysis, TC-906 steps 8-9).
- New AC ids are unique.
- The 16777216 defaults match the sibling caller-budget defaults and cannot make the default-success ACs false.
- No 69,997 or 129,994 normative totals appear; they occur only as do-not-substitute controls.
- No fabricated FunctionCall, reset, multiplier, bypass or compatibility layer is introduced.
- The ADR-014 B-2 row and the FR-255 Behavior 1 carrying-type row agree with each other and with the rule that the carrying type classifies the limit.
- The Rust mismatch in `qsl-replay/src/execute.rs` and `qsl-eval` `admits()` is target-design work for the implementation ticket, not a finding.
- `quire validate` over the changed files exits clean. No `docs/` or FR-092 vector file is touched; FR-255 is test-parsed (FND-001).

Recorded, not blocking:
- The FR-255 Behavior 1 `explore.*` row lost its three-space list indent, which breaks the rendered table.
- FR-277's phase table names `accounting.work_units` and `accounting.value_occurrences`, but FR-255's accounting settings are bare names.
- TC-830's scope lists FR-277-AC-5, but that AC binds only TC-906.
- FR-277-AC-1 is not amended for the new supplied fields.
- FR-277 still carries source-premise and process prose ("main SHALL route ...", `compare_shifted`).
- QSpec FR-323 needs a `limits.supplied` member; that change is the planner's separate item.
