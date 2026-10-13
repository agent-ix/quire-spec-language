---
id: SR-2445
title: "QSL-656 code and Rust review disposition round 10"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3f1777851bbd0d2d0b10d616f7b8ce33be33e4a4; PR #665 against origin/main fe7d883ade8ed3d12c1152fa49dad8a48b325680; tests/it/spine_run.rs"
review_set: subset
relationships:
  - {target: ix://agent-ix/quire-spec-language/FR-100, type: reviews}
---
## Summary

Round 10 exact-head code-review and rust-review recheck. The new native-run integration oracle is test-only; no new source-code or Rust finding was found. FND-005 remains accepted-no-change per the recorded IR-684 sequencing decision; the driver API mismatch remains real and downstream.

## Verdict

**PASS** — no new code or Rust defect. This verdict does not supersede the gap-analysis finding about the new test's incorrect AC trace tag.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `LimitExceeded::new` panic / partial `Setting::kind`; fixed. | qsl-foundation/src/diagnostic/stage.rs |
| FND-002 | low | Default-based accounting field in `CallerLimits::for_request`; fixed. | qsl-replay/src/limits.rs |
| FND-003 | low | Stale `work_units` doc comment; fixed. | qsl-replay/src/spine/clause.rs |
| FND-004 | low | Inaccurate PR title; fixed. | PR #665 |
| FND-005 | medium | quire-driver API mismatch; accepted-no-change as downstream/non-blocking under IR-684, not fixed. | qsl-foundation/src/setting.rs; quire-driver src/limits.rs |
| FND-006 | high | Missing `IntakeInputBytes` row after rebase; fixed. | qsl-foundation/src/setting.rs |
| FND-007 | high | Stale `ClauseRunRequest.model_limits`; fixed. | qsl-replay/src/spine/clause.rs |
| FND-008 | high | Missing `Cancel` argument in test; fixed. | qsl-replay/src/outcome.rs |
| FND-009 | medium | Rebase reverted FR-255 test changes; fixed. | qsl-replay/src/limits.rs |
| FND-010 | high | FR-312 accounting contract/reader mismatch; fixed. | src/command/wire.rs; FR-312 |
| FND-011 | high | FR-100 `work_units`/`accounting` contract mismatch; fixed. | src/command/wire.rs; FR-100 |

## Coverage

Confirmed local and GitHub PR head `3f1777851bbd0d2d0b10d616f7b8ce33be33e4a4`, base `fe7d883ade8ed3d12c1152fa49dad8a48b325680`, clean worktree, and `git diff --check`. Inspected the new `tests/it/spine_run.rs::tc_451_step_7_zero_integer_bits_is_incomplete`: the request sets `accounting.integer_bits` to zero and asserts CLI exit 22 with `incomplete/integer_bits`. No build was run. Linear round-10 disposition comment: pending.

## Dispositions (round 10)

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8c7fce235 |
| FND-002 | fixed | 8c7fce235 |
| FND-003 | fixed | 8c7fce235 |
| FND-004 | fixed | b76b7a9c2 |
| FND-005 | accepted-no-change | IR-684 remains explicitly downstream/non-blocking for #665; mismatch not claimed fixed |
| FND-006 | fixed | 0d81c68d7 |
| FND-007 | fixed | 0d81c68d7 |
| FND-008 | fixed | 0d81c68d7 |
| FND-009 | fixed | 0d81c68d7 |
| FND-010 | fixed | babba1fed643d2ed08c299ffc2532b4e998c6bba |
| FND-011 | fixed | babba1fed643d2ed08c299ffc2532b4e998c6bba |

---

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2445 round=10 reviewed=3f1777851bbd0d2d0b10d616f7b8ce33be33e4a4 model=gpt-5 run=b21d98c8-f420-41c9-9601-49c2660633b8 -->

Exact-head code/Rust disposition for PR #665 against origin/main fe7d883ade8ed3d12c1152fa49dad8a48b325680. The change since round 9 is an end-to-end native-run integration test only; static code-review and rust-review found no new implementation issue. No builds were run. FND-005's quire-driver mismatch remains accepted-no-change under the recorded IR-684 sequencing decision: downstream, blocked by QSL-656, and not a #665 merge blocker—not fixed or represented as compatible.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8c7fce235 |
| FND-002 | fixed | 8c7fce235 |
| FND-003 | fixed | 8c7fce235 |
| FND-004 | fixed | b76b7a9c2 |
| FND-005 | accepted-no-change | IR-684 remains explicitly downstream/non-blocking for #665; API mismatch remains |
| FND-006 | fixed | 0d81c68d7 |
| FND-007 | fixed | 0d81c68d7 |
| FND-008 | fixed | 0d81c68d7 |
| FND-009 | fixed | 0d81c68d7 |
| FND-010 | fixed | babba1fed643d2ed08c299ffc2532b4e998c6bba |
| FND-011 | fixed | babba1fed643d2ed08c299ffc2532b4e998c6bba |

+++ [reviewer data]

```yaml
reviewed_head: 3f1777851bbd0d2d0b10d616f7b8ce33be33e4a4
base: fe7d883ade8ed3d12c1152fa49dad8a48b325680
methods: [code-review, rust-review]
verdict: PASS
new_findings: []
prior_outcomes:
  - {fnd: FND-001, outcome: fixed, fix_sha: 8c7fce235}
  - {fnd: FND-002, outcome: fixed, fix_sha: 8c7fce235}
  - {fnd: FND-003, outcome: fixed, fix_sha: 8c7fce235}
  - {fnd: FND-004, outcome: fixed, fix_sha: b76b7a9c2}
  - {fnd: FND-005, outcome: accepted-no-change, reason: "IR-684 records that the API mismatch is downstream, blocked by QSL-656, and non-blocking for #665; no compatibility is claimed."}
  - {fnd: FND-006, outcome: fixed, fix_sha: 0d81c68d7}
  - {fnd: FND-007, outcome: fixed, fix_sha: 0d81c68d7}
  - {fnd: FND-008, outcome: fixed, fix_sha: 0d81c68d7}
  - {fnd: FND-009, outcome: fixed, fix_sha: 0d81c68d7}
  - {fnd: FND-010, outcome: fixed, fix_sha: babba1fed643d2ed08c299ffc2532b4e998c6bba}
  - {fnd: FND-011, outcome: fixed, fix_sha: babba1fed643d2ed08c299ffc2532b4e998c6bba}
coverage:
  - "New integration test drives call.accounting.integer_bits=0 through the native-run CLI and asserts exit 22 plus incomplete/integer_bits."
  - "No implementation code changed at this head; no new source-level Rust finding."
```

+++
