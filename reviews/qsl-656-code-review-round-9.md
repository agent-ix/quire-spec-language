---
id: SR-2445
title: "QSL-656 code and Rust review disposition round 9"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b9aac134252b69003043b3f3be75cebabe32241f; PR #665 against origin/main fe7d883ade8ed3d12c1152fa49dad8a48b325680; see reviews/qsl-656-code-review.md for original findings"
review_set: subset
relationships:
  - {target: ix://agent-ix/quire-spec-language/FR-255, type: reviews}
---
## Summary

Round 9 exact-head code-review and rust-review disposition for QSL-656 / PR #665. No new code or Rust findings. No builds were run; the lead reported the separate v2 gate green.

## Verdict

**PASS** — prior source findings are fixed. FND-005 remains accepted-no-change under the explicit sequencing decision that IR-684 is downstream and non-blocking for #665; the driver API mismatch is not claimed fixed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `LimitExceeded::new` panic / partial `Setting::kind` finding; fixed in the separate accounting-setting type redesign. | qsl-foundation/src/diagnostic/stage.rs; qsl-foundation/src/setting.rs |
| FND-002 | low | `CallerLimits::for_request` used a default-based, unused accounting field; fixed to use effective request limits. | qsl-replay/src/limits.rs |
| FND-003 | low | Stale clause-meter documentation named `work_units`; fixed to `accounting`. | qsl-replay/src/spine/clause.rs |
| FND-004 | low | PR title used `accounting.<counter>` despite bare-name contract; title corrected. | PR #665 |
| FND-005 | medium | quire-driver consumes `parse_operands` as `Setting` while QSL now returns `SettingName`; downstream mismatch remains accepted-no-change under IR-684 sequencing. | qsl-foundation/src/setting.rs; quire-driver src/limits.rs |
| FND-006 | high | Rebase dropped `IntakeInputBytes`; restored. | qsl-foundation/src/setting.rs |
| FND-007 | high | Rebase restored removed `ClauseRunRequest.model_limits`; corrected. | qsl-replay/src/spine/clause.rs |
| FND-008 | high | Test omitted the new `Cancel` argument; corrected. | qsl-replay/src/outcome.rs |
| FND-009 | medium | Rebase reverted main's FR-255 test changes; corrected. | qsl-replay/src/limits.rs |
| FND-010 | high | FR-312 accounting field did not match the clause wire reader; fixed with closed clause accounting input. | src/command/wire.rs; FR-312 |
| FND-011 | high | FR-100 still specified `call.work_units` while implementation accepted `call.accounting`; aligned in the spec and tests. | src/command/wire.rs; FR-100 |

## Coverage

Static code-review and rust-review; exact head confirmed as b9aac134252b69003043b3f3be75cebabe32241f and base as fe7d883ade8ed3d12c1152fa49dad8a48b325680. The only code delta since round 8 was integration-test code in `tests/it/config_version_spine.rs`; no new implementation defect was found. Existing review source: `reviews/qsl-656-code-review.md` (SR-2445). Linear round-9 disposition comment: `08cc19ad-3363-4e21-a727-f317545c5d43`.

## Dispositions (round 9)

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8c7fce235 |
| FND-002 | fixed | 8c7fce235 |
| FND-003 | fixed | 8c7fce235 |
| FND-004 | fixed | b76b7a9c2 |
| FND-005 | accepted-no-change | IR-684 is explicitly downstream/non-blocking for #665; the driver API mismatch is not claimed fixed |
| FND-006 | fixed | 0d81c68d7 |
| FND-007 | fixed | 0d81c68d7 |
| FND-008 | fixed | 0d81c68d7 |
| FND-009 | fixed | 0d81c68d7 |
| FND-010 | fixed | babba1fed643d2ed08c299ffc2532b4e998c6bba |
| FND-011 | fixed | babba1fed643d2ed08c299ffc2532b4e998c6bba |

---

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2445 round=9 reviewed=b9aac134252b69003043b3f3be75cebabe32241f model=gpt-5 run=c2317ad3-9f69-40da-a3be-59f8e487e9c2 -->

Exact-head code/Rust disposition for PR #665, compared with origin/main fe7d883ade8ed3d12c1152fa49dad8a48b325680. I reviewed the changes since the prior round (the clause accounting integration-test fixture and integer_bits case) plus the current source/test path. No new code or Rust finding. The separate v2 gate is reported green by the lead; this review ran no builds.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8c7fce235 |
| FND-002 | fixed | 8c7fce235 |
| FND-003 | fixed | 8c7fce235 |
| FND-004 | fixed | b76b7a9c2 |
| FND-005 | accepted-no-change | IR-684 remains explicitly downstream/non-blocking for #665; the driver API mismatch is not claimed fixed |
| FND-006 | fixed | 0d81c68d7 |
| FND-007 | fixed | 0d81c68d7 |
| FND-008 | fixed | 0d81c68d7 |
| FND-009 | fixed | 0d81c68d7 |
| FND-010 | fixed | babba1fed643d2ed08c299ffc2532b4e998c6bba |
| FND-011 | fixed | babba1fed643d2ed08c299ffc2532b4e998c6bba |

+++ [reviewer data]

```yaml
reviewed_head: b9aac134252b69003043b3f3be75cebabe32241f
base: fe7d883ade8ed3d12c1152fa49dad8a48b325680
methods: [code-review, rust-review]
verdict: PASS
new_findings: []
prior_outcomes:
  - {fnd: FND-001, outcome: fixed, fix_sha: 8c7fce235}
  - {fnd: FND-002, outcome: fixed, fix_sha: 8c7fce235}
  - {fnd: FND-003, outcome: fixed, fix_sha: 8c7fce235}
  - {fnd: FND-004, outcome: fixed, fix_sha: b76b7a9c2}
  - {fnd: FND-005, outcome: accepted-no-change, reason: "The current quire-driver API mismatch remains explicitly downstream under IR-684 and non-blocking for #665 per the planner's recorded decision; this is not a claim of compatibility."}
  - {fnd: FND-006, outcome: fixed, fix_sha: 0d81c68d7}
  - {fnd: FND-007, outcome: fixed, fix_sha: 0d81c68d7}
  - {fnd: FND-008, outcome: fixed, fix_sha: 0d81c68d7}
  - {fnd: FND-009, outcome: fixed, fix_sha: 0d81c68d7}
  - {fnd: FND-010, outcome: fixed, fix_sha: babba1fed643d2ed08c299ffc2532b4e998c6bba}
  - {fnd: FND-011, outcome: fixed, fix_sha: babba1fed643d2ed08c299ffc2532b4e998c6bba}
notes:
  - "The latest code delta after 4d414321 is tests/it/config_version_spine.rs only. The integer_bits clause CLI test asserts incomplete/evaluate and limit=integer_bits, so it exercises actual evaluation, not only decoding."
  - "The updated accounting type is shared by call and clause wire inputs; both command paths use call_accounting. The #665 native-run call site and its all-counter conversion test remain as previously reviewed."
```

+++
