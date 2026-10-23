---
id: SR-16258
title: "Round-2 spec-review of the hybrid reader and encoding fix-forward (quire-spec-language PR 1364)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b65ec862a2b4d0023788aacb2b6f958ee03cb459 (fix commit 9d52f017 against SR-16256; origin/main...HEAD); spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md; spec/test-cases/TC-1187-a-hybrid-declaration-checks-to-a-classified-model-and-routes-its-claim.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1187
    type: references
---

# Round-2 spec-review of the hybrid reader and encoding fix-forward (quire-spec-language PR 1364)

## Summary

Ticket: STD-385. This round re-reviews #1364 at head b65ec862: the fix
commit 9d52f017 against SR-16256, plus a merge of main. Main is an
ancestor of the head. The companion QSpec round is SR-16257 (#414 at
39d4bb9f).

Gates run by the reviewer with `QSPEC_DIR` at QSpec 39d4bb9f:

- `make check-spec-validation`: exit 0.
- `cargo test -p qsl-package --lib checked_v2`: 41 passed, 6 failed. Every
  failure is `UnsupportedNodeTag` at `/semantic_graph/nodes/39/node_tag`,
  the missing reader code and nothing else.

Dispositions of SR-16256:

- FND-001 is partly fixed. The S3 profile refusal is gone and S4 emits the
  law the `using` alias selects. Two parts are not fixed: `HybridInterval`
  still holds `u64` seconds, and S4 still emits every interval in the
  integer form (FND-001 below).
- FND-002 is fixed: the all-families fixture's `Tank` terms are Boolean at
  QSpec 39d4bb9f, so AC-6 can pass once the reader code lands.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | [soundness] S3 now admits a `timed/v1` hybrid claim, but S4 still emits it in a form the reader refuses. FR-447 S4 still emits "each `HybridInterval` in the integer form". `HybridInterval` is `{lower: u64, upper: Option<u64>}` and the horizon is `Option<u64>`. QSpec FR-481 at 39d4bb9f requires the timed form in exact rational seconds under `quire.temporal.timed/v1`. FR-370 refuses an integer-form interval under `timed/v1` `operation-member-mismatch`. So `SlowEnough` `using` a `timed/v1` alias checks at S3, emits the `timed/v1` law with an integer `[0, 10]`, and is refused. That contradicts QSpec FR-481-AC-8/HY-13, and it is the SR-16261 FND-003 hole reopened. A rational bound such as `always [0, 1/2]` also still cannot be written. Fix: give `HybridInterval` exact rational bounds with open or closed ends where `timed/v1` admits them, and make the horizon a rational. Emit each interval in the form FR-370 admits under the claim's profile (the integer form under the integer profiles, the timed form under `timed/v1`). Add an AC that `SlowEnough` under a `timed/v1` alias emits the timed `[0, 10]` and is admitted. Also state, with its S3 refusal and span, an interval the selected profile does not admit at source (for example `[0, *]` under `timed/v1`, or `null` under a bounded profile), unless the profile's own clause FR already covers it (FR-123 covers only infinite-trace). | spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:146; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:157; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:351; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md:380 | wrong-requirement |

## Verdict

Not mergeable. There is one high finding (FND-001): QSL's interval type
and S4 emission do not follow the profile-dependent interval form that
QSpec now requires, so a `timed/v1` hybrid claim is admitted at S3 and then
refused by the reader. The read-back rules, the true/false encoding and the
law emission are sound. The conformance failures are only the missing
reader code.
