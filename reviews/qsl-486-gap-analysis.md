---
id: SR-1347
title: "Gap analysis of quire-spec-language PR #647: FR-255, FR-259 and FR-264-AC-4 acceptance criteria against the B5 tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1db5326b01a40a18f17bd5d408736f92fcf7d83d; FR-255-AC-1 to AC-6 (TC-720, TC-721); FR-259-AC-2, AC-3 (TC-728, TC-729); FR-264-AC-4 (TC-739); FR-258-AC-4 (TC-727); tests: qsl-replay/src/limits.rs, qsl-foundation/src/setting.rs, qsl-foundation/src/diagnostic/stage.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-semantics/src/check/node_key/tests.rs, qsl-eval/src/simulation/key.rs, qsl-package/src/checked_v2/tests.rs, qsl-replay/src/spine/lifecycle/tests.rs, qsl-replay/src/spine/dependency_tests.rs, qsl-eval/tests/it/finite_simulation.rs, qsl-semantics/tests/it/state_clauses.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: reviews
---
## Summary

Ticket: QSL-486. PR: quire-spec-language#647. Plan completion: not assessed
(planless). The criteria-to-tests mapping was read from the `#[trace]` tags
at the reviewed sha, and each tagged test was read against its criterion.

- FR-255-AC-2: `a_limit_renders_its_setting_at_every_entry_point` checks the
  FR-461 rendering text without the locus. The locus half is FR-096's, which
  existing tests cover. Accepted as backing the setting text.
- FR-255-AC-5: `a_malformed_operand_is_a_usage_refusal_naming_it` covers all
  four operands the criterion names. `setting.rs` adds `+1`, empty and
  overflow cases. Clean.
- FR-255-AC-1, per row with a real stage-driven test: s1.* and s3.* (except
  decimal_scale), environment.*, most model.* and dependency.* (spine
  lifecycle/dependency tests); s3.decimal_scale (assemble tests); i2.*
  (checked_v2 tests); observation.* (state_clauses.rs); explore.states
  (finite_simulation.rs); replay.input_bytes (bounds.rs); identity.input_bytes
  on a state key (key.rs).
- FR-259-AC-2 (identity byte error to a stage limit), FR-259-AC-3 (state key
  at 100,000 deep on a 512 KiB stack) and FR-264-AC-4 (`i2.nodes` with
  setting and `Locus::Artifact`) each have a test that fails if the
  behaviour is removed.

## Verdict

Request changes. AC-1, AC-3, AC-4 and AC-6 of FR-255 are tagged as backed,
but the tests do not check what those criteria say. The other criteria in
scope are backed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Coverage inflation on FR-255-AC-1. This PR adds `#[trace("TC-720", "FR-255-AC-1")]` to `dependencies_are_keyed_before_their_dependents_whatever_the_source_order` and `dimensions_and_units_are_admitted_with_the_vector_keys`, which assert unit-graph keys and drive no limit. `limit_exceeded_reports_stage_limit_exceeded_per_setting` builds each `LimitExceeded` itself, so it tests catalog plumbing and drives no stage past a limit. Fix: drop those three tags. | qsl-semantics/src/check/assemble/tests.rs:1510; qsl-semantics/src/check/assemble/tests.rs:1530; qsl-foundation/src/diagnostic/stage.rs:277-306 |
| FND-002 | medium | FR-255-AC-1 is "for each row of the setting table". These rows have no test that drives their stage past the limit and checks the setting: admission.population_members, admission.work_units, admission.ancestor_steps, library.definitions, library.dependency_edges, library.artifact_bytes, library.single_artifact_bytes, model.dispatch_candidates, model.family_steps (family-steps tests check the model refusal, not the setting), explore.transitions (only `Limit::Transitions(5).setting()` is constructed), and identity.input_bytes through the check stage (see SR-1346 FND-001). FR-255 has no Status section saying AC-1 is partial. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md |
| FND-003 | medium | The oracle for `each_limits_field_maps_to_one_distinct_setting_and_sets_only_itself` is `Setting::ALL`, not FR-255's table. FR-255-AC-3 requires "the union of those names equals the setting table". The table keeps `intake.input_bytes`, which has no `Setting` variant and no limits type, so the criterion is false at HEAD and the test still passes. Fix: compare against the table's names (or mark the intake row as not yet ported and say so in a Status). | qsl-replay/src/limits.rs:126-157 |
| FND-004 | medium | FR-255-AC-4 asks that "an input that reaches that limit at its default succeeds once the row's setting is raised", for each row and each entry point. `the_settings_operation_and_a_request_raise_every_setting` only checks that the bound value round-trips through `from_operands` and `for_request`, and runs no stage. Only the identity state key (builder raise) and `i2.nodes` (checked_v2 tests) re-run an input after a raise. | qsl-replay/src/limits.rs:233-248 |
| FND-005 | low | `unconfigured_limits_are_at_the_published_defaults` (FR-255-AC-6) leaves out `s1.work_units` and `s3.decimal_scale`, so two table defaults are never compared. It also never checks the criterion's second half, "the effective limits a checked package records equal those defaults". | qsl-replay/src/limits.rs:274-318 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | `each_library_limit_refuses_naming_its_setting_and_admits_once_raised`, the new AC-1 test for the four `library.*` rows, asserts only the setting of each refusal. FR-255-AC-1 also requires the bound `B` and the count the refused charge would have reached, and `PackageError::ResourceLimit` carries both (`limit`, `actual`). Fix: assert `limit` and `actual` for each of the four. | qsl-semantics/src/library/bundle_tests.rs:938-1028 |

## Dispositions

Round 1, reviewed at 0d47d0c1865d0243f4cceca4b0b246af6325e115 (`git diff 1db5326b..0d47d0c1`). FND-002: every listed row now has a stage-driven test (library.* in bundle_tests.rs; admission.population_members and admission.work_units in model_population.rs; model.dispatch_candidates in model_dispatch.rs; explore.transitions in finite_simulation.rs; identity.input_bytes through check in lifecycle/tests.rs), except admission.ancestor_steps and model.family_steps. FR-255's Status marks those two partial, but they fall in B5's scope (SR-1346 FND-006), so the finding stays open until they are fixed in this PR. FND-004: the remaining rows are checked at the entry points only, and FR-255's Status says so. Every row's builder raise is already stage-driven by the FR-277 `assert_field` tests, the operation and the request go through the same `set_bound`, and four rows are driven end to end through both entry points in `a_reached_limit_is_raised_by_the_settings_operation_and_by_a_request`. That composition would be enough, but the test fails at the head on its `identity.input_bytes` leg (SR-1346 FND-007), so the finding stays open until it passes.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 58fc0fd01 |
| FND-002 | still-open | admission.ancestor_steps and model.family_steps still have no refusal that names their setting; FR-255's Status marks them partial, but the fix belongs in B5 (SR-1346 FND-006) |
| FND-003 | fixed | 58fc0fd01 |
| FND-004 | still-open | The stage-driven AC-4 test was added but fails at the head on its identity.input_bytes leg (SR-1346 FND-007) |
| FND-005 | fixed | 58fc0fd01 |

Round 2, reviewed at eab14d4b093a6fc1ebf93def26ee8946380bbd01 (`git diff 0d47d0c1..eab14d4b`). FND-002: `admission.ancestor_steps` and `model.family_steps` now refuse naming their setting and count (model_population.rs, model_dispatch.rs, model_conformance.rs assert `limit_exceeded().setting()` and `actual`). FND-004: the stage-driven AC-4 test passes at this head, `identity.input_bytes` leg included (focused run 138 passed, 0 failed). Its identity leg shares SR-1346 FND-001's weak oracle, which is tracked there. FND-006: the library test asserts setting, bound and count for all four rows.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-002 | fixed | 155aa2e78 |
| FND-004 | fixed | 155aa2e78 |
| FND-006 | fixed | 155aa2e78 |
