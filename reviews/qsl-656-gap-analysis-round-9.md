---
id: SR-2446
title: "QSL-656 manual AC-to-test mapping disposition round 9"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b9aac134252b69003043b3f3be75cebabe32241f; PR #665 requirements FR-255, FR-100, FR-312, TC-921 and linked tests against origin/main fe7d883ade8ed3d12c1152fa49dad8a48b325680"
review_set: subset
relationships:
  - {target: ix://agent-ix/quire-spec-language/FR-255, type: reviews}
  - {target: ix://agent-ix/quire-spec-language/FR-100, type: reviews}
---
## Summary

Round 9 manual acceptance-criteria-to-test mapping. The added integer_bits CLI test exercised the clause-run path (FR-312-AC-6), not FR-100's native-run `call.accounting` path, so FND-006 remained open at b9aac134.

## Verdict

**CONDITIONAL** — one medium coverage gap remained at the reviewed head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | AC-13's all-counter conversion assertion had no end-to-end oracle for a non-work_units counter through FR-100 native-run evaluation. The new integer_bits test exercised the distinct clause command path and could not catch a regression at `run_complete`. | FR-100-AC-13; src/command.rs:1088; tests/it/config_version_spine.rs |

## Coverage

Manually mapped FR-255-AC-7 through AC-11, FR-100-AC-3/6/13, and FR-312-AC-6 to their tagged tests and assertions. `quoin matrix --repo . --json` produced no output and was stopped; no matrix pass is claimed. Existing review source: `reviews/qsl-656-gap-analysis.md` (SR-2446). Linear round-9 disposition comment: `3a22d40d-a167-4104-8842-e10f0c3365de`.

## Dispositions (round 9)

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8c7fce235 |
| FND-002 | fixed | 8c7fce235 |
| FND-003 | fixed | 0d81c68d7 |
| FND-004 | fixed | 4d41432191cc795ad36fb4826e58630607eb14da |
| FND-005 | fixed | babba1fed643d2ed08c299ffc2532b4e998c6bba |
| FND-006 | still-open | The integer_bits oracle covered clause-run, not FR-100's native-run handoff. |

---

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2446 round=9 reviewed=b9aac134252b69003043b3f3be75cebabe32241f model=gpt-5 run=c2317ad3-9f69-40da-a3be-59f8e487e9c2 -->

Exact-head manual AC-to-test mapping disposition against origin/main fe7d883ade8ed3d12c1152fa49dad8a48b325680. TC-921 and the changed FR-100/FR-255 contract were read at b9aac134. The additional integer_bits test is an end-to-end clause-run CLI oracle (FR-312-AC-6), but it does not exercise FR-100's native-run/1 `call.accounting` path required by AC-13. Thus FND-006 remains open; the gate being green does not close a behavioral coverage gap.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8c7fce235 |
| FND-002 | fixed | 8c7fce235 |
| FND-003 | fixed | 0d81c68d7 |
| FND-004 | fixed | 4d41432191cc795ad36fb4826e58630607eb14da |
| FND-005 | fixed | babba1fed643d2ed08c299ffc2532b4e998c6bba |
| FND-006 | still-open | b9aac134 adds a real integer_bits exhaustion oracle through the clause CLI, but it tests FR-312-AC-6 / src/command/clause.rs. It does not traverse FR-100's run_complete call-accounting handoff at src/command.rs:1088, so a regression at that native-run call site could still leave AC-13's end-to-end requirement untested. |

+++ [reviewer data]

```yaml
reviewed_head: b9aac134252b69003043b3f3be75cebabe32241f
base: fe7d883ade8ed3d12c1152fa49dad8a48b325680
method: gap-analysis/manual-ac-to-test-mapping
matrix: "Not available: quoin matrix --repo . --json produced no output and was stopped; no matrix pass is claimed."
scope:
  - {id: FR-255-AC-7, role: examined, excerpt: "work_units=0 and 1000000 settle Incomplete/Completed with setting and value assertions"}
  - {id: FR-255-AC-8, role: examined, excerpt: "each of ten accounting operands sets only its named counter and preserves defaults"}
  - {id: FR-255-AC-9, role: examined, excerpt: "unknown, malformed, overflow and duplicate operands produce named usage refusals"}
  - {id: FR-255-AC-10, role: examined, excerpt: "stage_limits override one accounting counter and the replay result reflects it"}
  - {id: FR-255-AC-11, role: examined, excerpt: "equivalent effective limits yield equal sent/claimed identities, settlements and charges"}
  - {id: FR-100-AC-3, role: examined, excerpt: "invalid accounting members and legacy call.work_units are refused through the native-run CLI"}
  - {id: FR-100-AC-6, role: examined, excerpt: "call.accounting.work_units=0 produces an incomplete native-run result"}
  - {id: FR-100-AC-13, role: examined, excerpt: "any subset of ten counters is accepted, defaults apply, supplied counters reach evaluation unchanged, direct call.work_units is refused"}
  - {id: FR-312-AC-6, role: examined, excerpt: "clause accounting accepts ten counters and rejects direct clause.work_units"}
bindings:
  - {test_id: "qsl-replay/src/outcome.rs::a_reached_budget_set_by_operand_names_its_setting", ac_id: FR-255-AC-7, trace: correct}
  - {test_id: "qsl-replay/src/limits.rs::each_accounting_setting_sets_its_counter_alone", ac_id: FR-255-AC-8, trace: correct}
  - {test_id: "qsl-replay/src/limits.rs::a_malformed_accounting_operand_is_a_usage_refusal_naming_it", ac_id: FR-255-AC-9, trace: correct}
  - {test_id: "qsl-replay/src/limits.rs::a_stage_limits_counter_replaces_only_that_accounting_counter", ac_id: FR-255-AC-10, trace: correct}
  - {test_id: "qsl-replay/src/execute/tests.rs::tc_921_a_split_accounting_limit_gives_the_same_claim_settlement_and_charges", ac_id: FR-255-AC-11, trace: correct}
  - {test_id: "tests/it/spine_run.rs::tc_451_step_7_zero_work_units_is_incomplete", ac_id: FR-100-AC-6, trace: correct}
  - {test_id: "src/command.rs::call_accounting_reaches_the_call_as_its_limits", ac_id: FR-100-AC-13, trace: correct, verification: "all ten values and defaults asserted after wire::Call conversion; not end-to-end into run_complete"}
  - {test_id: "tests/it/config_version_spine.rs::clause_run_command_reports_select_admit_and_incomplete_classes", ac_id: FR-312-AC-6, trace: correct, verification: "integer_bits=1 reaches actual evaluation and renders incomplete/integer_bits"}
dispositions:
  - {fnd: FND-001, outcome: fixed, fix_sha: 8c7fce235, after_excerpt: "The AC-11 test checks ValueIdentity::sent for whole and split limits and claim identity equality."}
  - {fnd: FND-002, outcome: fixed, fix_sha: 8c7fce235, after_excerpt: "When <name> is in neither table..."}
  - {fnd: FND-003, outcome: fixed, fix_sha: 0d81c68d7, after_excerpt: "The FR-100-AC-13 accounting conversion is tagged and associated with TC-921."}
  - {fnd: FND-004, outcome: fixed, fix_sha: 4d41432191cc795ad36fb4826e58630607eb14da, after_excerpt: "FR-100-AC-13 is the QSL-656 accounting-object criterion."}
  - {fnd: FND-005, outcome: fixed, fix_sha: babba1fed643d2ed08c299ffc2532b4e998c6bba, after_excerpt: "FR-100 Inputs and AC-3/AC-6 specify call.accounting and refuse the legacy direct member."}
  - {fnd: FND-006, outcome: still-open, reason: "The b9 integer_bits test covers the clause command path, not FR-100's run_complete call-site. AC-13's native-run path can regress independently without failing this oracle."}
new_findings: []
```

+++
