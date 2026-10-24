---
id: SR-2446
title: "QSL-656 manual AC-to-test mapping disposition round 11"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d2f1ad69a71d014169e93c054d1c372f6f4a6bff; PR #665 against origin/main fe7d883ade8ed3d12c1152fa49dad8a48b325680; FR-100, TC-921, tests/it/spine_run.rs"
review_set: subset
relationships:
  - {target: ix://agent-ix/quire-spec-language/FR-100, type: reviews}
---
## Summary

Round 11 manual trace disposition. FND-006 remains fixed by the native-run `integer_bits=0` oracle. FND-007 is fixed: the test is now named `native_run_integer_bits_limit_reaches_evaluation` and traces to TC-921 / FR-100-AC-13, matching the any-counter behavior.

## Verdict

**PASS** — no open or missing gap finding remains in the reviewed accounting slice. Prior code-review FND-005 retains accepted-no-change status under IR-684; the driver API mismatch remains downstream and unresolved.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | Round-10 trace gap: native-run integer_bits test was tagged to AC-6 rather than AC-13; fixed by the exact-head trace correction. | tests/it/spine_run.rs:794-798 |

## Coverage

Confirmed the test's behavior: it sends `call.accounting.integer_bits=0` through native-run, then asserts exit 22 and `incomplete/integer_bits`. Confirmed the updated `#[trace("TC-921", "FR-100-AC-13")]` binding; AC-6 remains bound to the separate work_units-zero test. Prior gap findings FND-001 through FND-005 remain fixed; code-review FND-005 is accepted-no-change. No builds were run.

## Dispositions (round 11)

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-006 | fixed | 3f1777851bbd0d2d0b10d616f7b8ce33be33e4a4 |
| FND-007 | fixed | d2f1ad69a71d014169e93c054d1c372f6f4a6bff |

## New findings (disposition pass 11)

No new findings.

---

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2446 round=11 reviewed=d2f1ad69a71d014169e93c054d1c372f6f4a6bff model=gpt-5 run=4e54da62-8eb1-43e2-85a9-42bbf58a9303 -->

Exact-head gap-analysis/manual trace disposition against origin/main fe7d883ade8ed3d12c1152fa49dad8a48b325680. FND-006 remains fixed: the native-run integer_bits=0 test drives the value through evaluation and asserts incomplete/integer_bits. FND-007 is fixed at d2f1ad6: the test is renamed and now traces to TC-921 / FR-100-AC-13, matching its any-counter behavior. Rechecked all earlier findings: FND-001 through FND-005 retain their recorded fixed outcomes; code-review FND-005 remains accepted-no-change under IR-684's downstream/non-blocking decision.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-006 | fixed | 3f1777851bbd0d2d0b10d616f7b8ce33be33e4a4 |
| FND-007 | fixed | d2f1ad69a71d014169e93c054d1c372f6f4a6bff |

+++ [reviewer data]

```yaml
reviewed_head: d2f1ad69a71d014169e93c054d1c372f6f4a6bff
base: fe7d883ade8ed3d12c1152fa49dad8a48b325680
method: gap-analysis/manual-ac-to-test-mapping
prior_outcomes:
  - {fnd: FND-001, outcome: fixed, fix_sha: 8c7fce235}
  - {fnd: FND-002, outcome: fixed, fix_sha: 8c7fce235}
  - {fnd: FND-003, outcome: fixed, fix_sha: 0d81c68d7}
  - {fnd: FND-004, outcome: fixed, fix_sha: 4d41432191cc795ad36fb4826e58630607eb14da}
  - {fnd: FND-005, outcome: fixed, fix_sha: babba1fed643d2ed08c299ffc2532b4e998c6bba}
prior_code_review_outcome:
  fnd: FND-005
  outcome: accepted-no-change
  reason: "IR-684 explicitly records that the API mismatch is downstream/non-blocking for #665; it remains unresolved in the driver."
dispositions:
  - fnd: FND-006
    outcome: fixed
    fix_sha: 3f1777851bbd0d2d0b10d616f7b8ce33be33e4a4
    after_excerpt: |-
      assert_eq!(
          document["outcome"],
          json!({"kind": "incomplete", "limit": "integer_bits"})
      );
  - fnd: FND-007
    outcome: fixed
    fix_sha: d2f1ad69a71d014169e93c054d1c372f6f4a6bff
    after_excerpt: |-
      /// FR-100-AC-13 (TC-921 step 5): `run_complete` forwards the supplied
      /// `integer_bits` limit through evaluation and reports the exhausted counter.
      #[test]
      #[trace("TC-921", "FR-100-AC-13")]
      fn native_run_integer_bits_limit_reaches_evaluation() {
new_findings: []
bindings:
  - {test_id: "tests/it/spine_run.rs::native_run_integer_bits_limit_reaches_evaluation", ac_id: FR-100-AC-13, trace: correct}
  - {test_id: "tests/it/spine_run.rs::native_run_integer_bits_limit_reaches_evaluation", ac_id: FR-100-AC-6, trace: removed}
coverage:
  matrix: "Manual focused trace review; no new matrix run."
  builds: "Not run, as requested."
```

+++
