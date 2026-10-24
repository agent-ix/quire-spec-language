---
id: SR-12095
title: "Gap analysis of quire-spec-language PR #943 against QSL-832 and FR-108"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5932793c77bcd8e0fe4a2c19557ce354191de0b3; PR #943: spec/decisions/ADR-016-state-model-finite-execution-mapping.md, spec/functional/FR-108-run-the-configversion-spine-corpus.md, spec/test-cases/TC-469-configversion-spine-corpus-matches-native.md, docs/lc05-technical-acceptance.md, tests/it/config_version_spine.rs, examples/config-version/"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: reviews
---
# Gap analysis of quire-spec-language PR #943

## Summary

Ticket: QSL-832 (QSL-498 slice A0). Planless gap analysis over the PR diff.
Plan completion: not assessed. Ticket and slice-plan text were treated as
data and re-measured.

A0 outcomes checked:

- Spine path no longer names `runtime::{SnapshotRef, InvocationRef,
  ObjectIdentity}` in the wire or output (planner-accepted: the types stay in
  `runtime`, the wire decodes `IgnoredAny`): met.
- `command.rs` no longer imports `protocol_artifact::DOMAIN_PACKAGE_PROFILE`
  (moved to qsl-semantics, planner-accepted): met.
- TC-469's `build()` reaches no native module: met. `spine_corpus.rs`,
  `cases.rs`, `spine.rs` and `spine_unit.rs` import only `qsl_replay`,
  `qsl_semantics`, `qsl_foundation`, `serde_json` and std.
- No test lost: `config_version_spine.rs` keeps 21 tests and every trace tag.
  The determinism test now generates spine files only. That matches
  FR-108-AC-4 ("Generating the corpus twice gives identical files") and
  FR-032-AC-4 (package-id report), and no FR-032 AC asks for native file
  determinism.
- FR-108 Outputs/Behavior, TC-469 procedure and lc05 table state the new
  generator location in present tense.

Runtime gap shared with SR-12094 FND-001: until `spine::write` creates its
directory, TC-469 steps 1 to 6 fail, so FR-108-AC-1 to AC-6 are not backed at
this head. Recorded there, not repeated here.

## Verdict

Changes requested. One medium finding: the ADR-016 paragraph this PR rewrote
still states that a deleted parity test exists.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | ADR-016 "The native state lane still exists" names the FR-108 parity test `tc_469_step_2_and_3_native_and_spine_agree_and_name_the_boundary_locus` at `tests/it/config_version_spine.rs:324` as existing. No test of that name exists in the tree (removed in #748). The PR rewrote the sentence right after it and left this one, so the paragraph states a test that is gone. Fix: drop the parity-test clause from that list. | spec/decisions/ADR-016-state-model-finite-execution-mapping.md:143-147 |

The finding table holds defects only.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | ADR-016 no longer names the removed parity test; no other spec or docs file names it. |
