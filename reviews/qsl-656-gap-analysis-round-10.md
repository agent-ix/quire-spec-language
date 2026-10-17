---
id: SR-2446
title: "QSL-656 manual AC-to-test mapping disposition round 10"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3f1777851bbd0d2d0b10d616f7b8ce33be33e4a4; PR #665 against origin/main fe7d883ade8ed3d12c1152fa49dad8a48b325680; FR-255, FR-100, FR-312, TC-921, tests/it/spine_run.rs"
review_set: subset
relationships:
  - {target: ix://agent-ix/quire-spec-language/FR-255, type: reviews}
  - {target: ix://agent-ix/quire-spec-language/FR-100, type: reviews}
---
## Summary

Round 10 exact-head manual AC-to-test mapping. The new native-run `integer_bits: 0` CLI oracle closes FND-006 by exercising `run_complete` and observing the actual incomplete outcome. Rechecking the trace exposed new FND-007: this test is tagged to FR-100-AC-6, whose stated case is `work_units: 0`; the any-counter behavior belongs to FR-100-AC-13.

## Verdict

**CONDITIONAL** — FND-006 is fixed, but the new test-to-AC trace is incorrect and remains to be corrected.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | The new integer_bits CLI test asserts a non-work_units counter reaches native-run evaluation, but tags only `FR-100-AC-6`, which specifies the `work_units: 0` case. The test should bind to FR-100-AC-13 (and retain TC-451 if desired); otherwise its evidence is misattributed and AC-13 lacks the end-to-end binding. | tests/it/spine_run.rs:794-796; FR-100-AC-6; FR-100-AC-13 |

## Coverage

FND-006 disposition: the test writes `call.accounting = {"integer_bits": 0}`, launches the actual native-run CLI, and asserts exit 22 and outcome `{"kind":"incomplete","limit":"integer_bits"}`. This exercises the native `run_complete` handoff, unlike the prior clause-run test. FND-005 was rechecked against Linear: IR-684 remains In Progress and its recorded sequencing comment says the QSL API mismatch is downstream, blocked by QSL-656, and non-blocking for #665; it is accepted-no-change, not fixed. No builds were run. Manual mapping only; no matrix result is claimed.

## Dispositions (round 10)

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8c7fce235 |
| FND-002 | fixed | 8c7fce235 |
| FND-003 | fixed | 0d81c68d7 |
| FND-004 | fixed | 4d41432191cc795ad36fb4826e58630607eb14da |
| FND-005 | fixed | babba1fed643d2ed08c299ffc2532b4e998c6bba |
| FND-006 | fixed | 3f1777851bbd0d2d0b10d616f7b8ce33be33e4a4 |

## New findings (disposition pass 10)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | The native-run integer_bits test is tagged only to FR-100-AC-6, whose oracle specifies `work_units: 0`; this non-work counter case belongs to FR-100-AC-13. | tests/it/spine_run.rs:794-796 |

Exact test tag at reviewed head:

```rust
#[trace("TC-451", "FR-100-AC-6")]
fn tc_451_step_7_zero_integer_bits_is_incomplete() {
```

The FR-100-AC-6 criterion specifies the `work_units: 0` case; FR-100-AC-13 requires supplied accounting counters to reach evaluation unchanged.

---

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2446 round=10 reviewed=3f1777851bbd0d2d0b10d616f7b8ce33be33e4a4 model=gpt-5 run=b21d98c8-f420-41c9-9601-49c2660633b8 -->

Exact-head manual AC-to-test mapping disposition against origin/main fe7d883ade8ed3d12c1152fa49dad8a48b325680. The new native-run integer_bits=0 oracle closes FND-006: it traverses `run_complete`, and asserts the CLI's incomplete outcome names `integer_bits`. Rechecking the new test's trace finds FND-007: it tags FR-100-AC-6, which specifies the work_units=0 case, rather than FR-100-AC-13, which owns any-counter behavior. FND-005's earlier code-review accepted-no-change status was rechecked: IR-684 is still In Progress and its recorded comment explicitly marks that API mismatch downstream/non-blocking for #665.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-006 | fixed | 3f1777851bbd0d2d0b10d616f7b8ce33be33e4a4 |

New medium finding FND-007 remains open pending correction of the trace binding.

+++ [reviewer data]

```yaml
reviewed_head: 3f1777851bbd0d2d0b10d616f7b8ce33be33e4a4
base: fe7d883ade8ed3d12c1152fa49dad8a48b325680
method: gap-analysis/manual-ac-to-test-mapping
prior_outcomes:
  - {fnd: FND-001, outcome: fixed, fix_sha: 8c7fce235}
  - {fnd: FND-002, outcome: fixed, fix_sha: 8c7fce235}
  - {fnd: FND-003, outcome: fixed, fix_sha: 0d81c68d7}
  - {fnd: FND-004, outcome: fixed, fix_sha: 4d41432191cc795ad36fb4826e58630607eb14da}
  - {fnd: FND-005, outcome: fixed, fix_sha: babba1fed643d2ed08c299ffc2532b4e998c6bba}
dispositions:
  - fnd: FND-006
    outcome: fixed
    fix_sha: 3f1777851bbd0d2d0b10d616f7b8ce33be33e4a4
    after_excerpt: |-
      assert_eq!(
          document["outcome"],
          json!({"kind": "incomplete", "limit": "integer_bits"})
      );
prior_review_outcome:
  fnd: FND-005
  method: code-review
  outcome: accepted-no-change
  reason: "IR-684 remains explicitly downstream/non-blocking for #665; the API mismatch is not claimed fixed."
new_findings:
  - fnd: FND-007
    severity: medium
    confidence: high
    method: gap-analysis
    check: trace
    artifact_id: FR-100-AC-6
    related_ids: [FR-100-AC-13, TC-451]
    test_id: "tests/it/spine_run.rs::tc_451_step_7_zero_integer_bits_is_incomplete"
    claimed_ac: FR-100-AC-6
    correct_ac: FR-100-AC-13
    path: tests/it/spine_run.rs
    lines: "794-796"
    excerpt: |-
      /// FR-100-AC-6 (TC-451 step 7): the `run_complete` handoff forwards a
      /// non-work counter so `seven`'s integer result exhausts `integer_bits`.
      #[test]
      #[trace("TC-451", "FR-100-AC-6")]
    finding: "This test exercises the non-work_units, any-counter behavior owned by FR-100-AC-13, but its only AC trace is FR-100-AC-6, whose acceptance example is call.accounting.work_units=0. As tagged, the matrix misattributes the test and AC-13 lacks the end-to-end binding; tag this test to FR-100-AC-13 (retaining TC-451 is appropriate)."
bindings:
  - {test_id: "tests/it/spine_run.rs::tc_451_step_7_zero_work_units_is_incomplete", ac_id: FR-100-AC-6, trace: correct}
  - {test_id: "tests/it/spine_run.rs::tc_451_step_7_zero_integer_bits_is_incomplete", ac_id: FR-100-AC-6, trace: wrong}
  - {test_id: "tests/it/spine_run.rs::tc_451_step_7_zero_integer_bits_is_incomplete", ac_id: FR-100-AC-13, trace: missing}
coverage:
  matrix: "Not run; manual mapping was requested. No computed matrix result is claimed."
  verification: "No builds or code edits."
```

+++
