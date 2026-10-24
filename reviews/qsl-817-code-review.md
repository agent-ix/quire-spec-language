---
id: SR-12040
title: "QSL-817 PR #920 code review: the public state-model seam"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d459e9ddf224ef9c70858516199c52dcc6d659b8; qsl-semantics/src/model/mod.rs, qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/document.rs, qsl-semantics/src/model/observation/initial.rs, qsl-semantics/src/model/population.rs, qsl-semantics/src/model/state.rs, qsl-semantics/tests/it/main.rs, qsl-semantics/tests/it/model_state.rs"
review_set: subset
---

## Summary

Ticket: QSL-817. PR quire-spec-language#920, diff `origin/main...HEAD` (8 files). Code review with the rust-review lane. Read-only: no build was run, because a gate was using the worktree's target directory.

Checked: `admit_initial` semantics, `observation()` against `admit_current_snapshot`, ModelState and StateObject key derivation, absent optional fields, the newly public `decide_frame` and `FrameDecision` family, and the `population_index` and `ancestor_steps` accessors.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `admit_initial` admits a snapshot's `relationships` extents through checks 6 to 8 and then drops them. `ModelState` has nowhere to keep them, and `observe_state` builds its extents from `&[]`. So `observation()` of an admitted state reports every relationship as unlisted, and navigation gives `incomplete_population`/`incomplete-scope`, where `admit_current_snapshot` on the same document gives the admitted extents. The equality test compares objects and slots only, so it cannot see this. | qsl-semantics/src/model/observation/initial.rs:85-90, qsl-semantics/src/model/observation/initial.rs:168-169, qsl-semantics/src/model/state.rs:28-33, qsl-semantics/tests/it/model_state.rs:347-413 |

## Verdict

FIX, because of FND-001. Either refuse a snapshot that carries `relationships` entries with a named cause until ModelState can hold extents, or carry the admitted extents in ModelState and pass them to `build_extents`. Add a test that admits a snapshot with a complete extent and compares navigation against `admit_current_snapshot`.

The rest is sound:

- `admit_initial` runs check 1, check 3, check 4 and checks 6 to 8, in FR-106 order. Check 3 requires `current` and an `initialization` anchor. Its cause strings match `admit_observations` (a missing anchor gives `wrong-observation`, another kind gives `wrong-anchor`). Check 7 requires every listed population.
- The population key comes from `find_population`, the same node match check 6 uses, so `population.node` agrees with the wire name `Observation.populations` is keyed by.
- `field_key` derives field keys the same way `observation::frame` builds FrameObject fields. An absent optional field has no entry and comes back as `Absent`. A missing required field is a Fault. Admission never produces `Null`, so faulting on it is correct.
- `decide_frame` compares the union of pre and post field keys, so omitting absent fields is safe.
- Making the frame family public exposes only types that were already public (`ModelIndex`, `OperationEffect`, `ModelRefusal`). `StateModel` keeps its fields `pub(crate)`.
- `population_index` and `ancestor_steps` supply `FrameDecision.index` and `.ancestor_steps`, which QSL-493's ModelSystem needs for the FR-120 frame decision. They are justified.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 985bfb519c877d26f91d6296b4093fbd24524c81: `ModelState.extents` holds the admitted extents (`supplied_extents`) and `observe_state` admits them through `admit_supplied_extents`. The new test `a_states_observation_navigates_the_relationships_its_snapshot_listed` asserts `ExtentState::Complete` and equal `targets` against `admit_current_snapshot`, and it fails without the stored extents, because the relationship would be unlisted. |
