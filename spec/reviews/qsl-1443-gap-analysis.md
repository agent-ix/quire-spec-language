---
id: SR-202611010114
title: "Focused gap analysis of QSL-1443 PR #2045"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ee32e014f525ca0eaab96df7abeeadd8e8d5c70f; PR #2045 diff: Cargo.lock, qsl-package/src/emit.rs; QSL-1443; FR-050; FR-230-AC-50"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---

## Summary

Ticket: QSL-1443. Plan completion: not assessed. The manual check confirms the relock and call-site adaptation, C14 admission, and CL-57..62 reader/emitter vectors. The package suite exposes six timed producer/read-back failures assigned to QSL-1384, plus a PR-specific adverse-mutation failure in the QSpec fixture test that must be corrected in #1443.

## Verdict

**FAIL** — C14 and CL-57..62 pass, but `conformance_i2_read_over_qspec_checked_package_v2_fixtures` fails on its adverse unknown-profile mutation: the test changes the clause profile but leaves `temporal_bindings` inconsistent, yielding `TimedNodeReference` instead of the expected `UnknownProfile`. Correct this #1443 API/test fallout. The six producer-side temporal-binding failures are specifically QSL-1384 scope and remain sequenced after the relock. The full repository matrix could not be computed with the installed Quoin commands.

## Scope

- QSL-1443 relock outcome: examined branch-main/no-rev dependency policy, lockfile scope, C14 requirement, and CL-57..62 replay requirement.
- `FR-050`: examined the temporal binding and typed clock-configuration contract as context for the next QSL-1384 slice.
- `FR-230-AC-50`: examined timed-model emission boundary as context.
- `qsl-package/src/emit.rs`: examined both writer call sites.
- `qsl-package/src/checked_v2/tests.rs`: examined C14 and temporal-model reader/writer coverage.
- `qsl-package/src/emit/tests/{hyper_clause,temporal_clause}.rs`: examined temporal clause read-back tests.

## Coverage

- The tested C14 fixture is admitted; its prior `UnknownMember` at `/temporal_bindings` is gone.
- CL-57..62 reader and emitter vectors pass; equality sidecar preservation passes.
- `a_no_clock_dense_timed_model_needs_no_integer_literal_type`, `changed_timed_inputs_rekey_and_read_back_the_mutated_model`, `conformance_typed_timed_model_writer_emits_an_admitted_package`, `emit::tests::hyper_clause::bounded_and_open_intervals_emit_and_are_admitted`, `emit::tests::temporal_clause::each_interval_form_emits_its_temporal_interval_member`, and `emit::tests::temporal_clause::the_i2_read_refuses_each_fr370_defect_at_its_locus` fail with `TimedNodeReference` after the lock advances to IR #788. These six failures can be addressed in QSL-1384 by emitting the required per-declaration temporal bindings; no test weakening or skip is acceptable.
- `checked_v2::tests::conformance_i2_read_over_qspec_checked_package_v2_fixtures` passes its positive fixtures. Its adverse unknown-profile mutation is PR-specific: it alters the temporal clause profile identity but not the temporal sidecar, so IR #788 refuses at `/temporal_bindings/0/clock/kind` with `TimedNodeReference`, instead of the expected `UnknownProfile`. Update the mutation so the sidecar remains consistent (or revise the expected refusal with spec backing); this belongs in #1443. On clean main, the test fails earlier on the positive fixture with `UnknownMember /temporal_bindings`, which is the old reader baseline.
- `emit::tests::union_nodes::a_function_naming_an_omitted_node_is_omitted_and_the_other_is_written` reproduces on clean main with the same `MissingEqualityIdentityContext`; it is not caused by this PR.
- The manual acceptance-to-test check found no missing test for C14 or the CL-57..62 vectors. No reverse-gap claim is made outside these changed paths.
- The `quoin gap-analysis` command is not installed in the resolved Quoin 0.29.1 CLI. `quoin matrix --repo /private/tmp/qsl1443-ir1230-relock --json` produced no output after 90 seconds and was interrupted. The full repository matrix and reverse-gap inventory are therefore unavailable.
- Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The QSpec adverse unknown-profile mutation in `conformance_i2_read_over_qspec_checked_package_v2_fixtures` leaves the new temporal sidecar inconsistent with the altered clause, so it now refuses with `TimedNodeReference` instead of exercising the expected `UnknownProfile`. Correct the mutation or justify an expectation change against the spec in #1443. | qsl-package/src/checked_v2/tests.rs:2956 |
| FND-002 | medium | Six timed writer/read-back tests fail because `temporal_bindings` is still `None`. The typed per-declaration binding producer belongs to explicitly sequenced QSL-1384; these tests are genuine remaining successor scope and must not be skipped or weakened. | qsl-package/src/emit.rs:4678 |


## Follow-up verification at 0f2c41443

Reviewed PR #2045 head `0f2c41443a75fb63eef365c89dfe7b58737dca1c`. The root integration helper now constructs `CheckedPackageSidecarsRef { checked_term_data: None, temporal_bindings: None }` at `tests/it/emit_checked_v2.rs:427`; all 11 tests in the `emit_checked_v2` integration module pass. No new finding was identified. The prior unknown-profile mutation finding remains fixed at `5114fb983`; its focused fixture test passes, as do C14 and both CL-57..62 vector tests.

Prior FND-002 remains deferred to linked QSL-1384: all six temporal producer/read-back failures stay enabled, not waived. The equality-identity failure remains the clean-main baseline established in round 1. The full package suite and aggregate gate were not rerun in this follow-up; the lead will run the aggregate gate after disk cleanup.
