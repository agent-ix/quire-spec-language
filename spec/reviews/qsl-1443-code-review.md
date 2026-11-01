---
id: SR-202611010113
title: "Code and Rust review of QSL-1443 PR #2045 IR-1230 relock"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ee32e014f525ca0eaab96df7abeeadd8e8d5c70f; PR #2045 diff: Cargo.lock, qsl-package/src/emit.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---

## Summary

Ticket: QSL-1443. Reviewed PR #2045 at `ee32e014f525ca0eaab96df7abeeadd8e8d5c70f`. The manifest correctly remains on IR `branch = "main"`; the lock advances only `quire-contract-model` to `9a4156f5`, and both writer call sites adapt to `CheckedPackageSidecarsRef`. The requested C14 test and CL-57..62 vector tests pass. The touched crate's full lib suite has six timed-sidecar failures assigned to the immediately following QSL-1384 slice, one PR-specific adverse-mutation failure requiring API fallout in this relock, and one unrelated baseline failure.

## Verdict

**FAIL** — the required QSL-1443 adverse unknown-profile mutation is inconsistent with IR #788’s newly read temporal sidecar and now returns `TimedNodeReference` before reaching the expected `UnknownProfile`. Fix that test/API fallout in this PR. Separately, six timed producer/read-back failures are explicitly sequenced to QSL-1384; they are remaining scope for that successor and should not be implemented or hidden in #2045.

## Assurance Context

No applicable AssuranceProfile was found under `spec/` for the changed lock and writer call sites.

## Scope

- `Cargo.lock`: examined the `quire-contract-model` revision; no unrelated lock entries changed.
- `Cargo.toml`: examined the existing branch dependency; no manifest pin was added.
- `qsl-package/src/emit.rs`: examined both `encode_checked_package_v2` call sites, including retained equality data and absent temporal sidecars.
- `FR-050`: examined the temporal-binding and typed clock-configuration requirement that QSL-1384 is implementing.
- `FR-230-AC-50`: examined as context for the current timed-model producer boundary.
- `checked_v2::tests::conformance_c14_source_map_lookup_over_qspec_positive_fixtures`: examined and ran against QSpec main `65d14b984`; passes.
- CL-57..62 reader and emitter vector tests: examined and ran; pass.
- `qsl-package --lib`: ran all 489 tests; 479 passed, 8 failed, 2 ignored.

## Checks

- C14 positive fixture test: passed (1/1); `/temporal_bindings` no longer returns `UnknownMember`.
- CL-57..62 reader test and emitter test: passed (1/1 each).
- Equality checked-term sidecar producer test: passed (1/1).
- `cargo clippy --locked --offline -p qsl-package --lib --tests -- -D warnings`: passed.
- `cargo fmt --all --check`: passed.
- `git diff --check`: passed.
- `cargo test --locked --offline -p qsl-package --lib`: failed (479 passed, 8 failed, 2 ignored); details are in FND-001 and the baseline notes below.
- `cargo deny check`: failed because the existing dependency graph includes AGPL-3.0-or-later packages rejected by the current `deny.toml` allowlist; this PR changes neither local manifests nor license policy.
- `quoin gap-analysis` is unavailable (`command gap-analysis not found`). `quoin matrix --repo /private/tmp/qsl1443-ir1230-relock --json` remained active without output for 90 seconds and was interrupted; full repository matrix coverage is unavailable.
- No CI workflow files changed.

Failure classification for `cargo test --locked --offline -p qsl-package --lib` (8 failed): six are the QSL-1384 temporal producer follow-up; one is a PR-specific mutation/API fallout finding; one is a clean-main baseline. `checked_v2::tests::conformance_i2_read_over_qspec_checked_package_v2_fixtures` successfully reads the positive fixtures, then its adverse unknown-profile mutation changes the temporal clause profile identity without making the corresponding `temporal_bindings` sidecar consistent. IR returns `InvalidPackage/TimedNodeReference` at `/temporal_bindings/0/clock/kind` instead of the test’s expected `UnknownProfile`. This mutation needs to preserve sidecar consistency (or the expected refusal must be deliberately revised with spec backing) as part of #1443. `emit::tests::union_nodes::a_function_naming_an_omitted_node_is_omitted_and_the_other_is_written` fails with `MissingEqualityIdentityContext` and reproduces on clean origin/main `e52e95709`, so it is an unrelated baseline failure.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `conformance_i2_read_over_qspec_checked_package_v2_fixtures` mutates an unknown profile in the temporal clause but leaves the new `temporal_bindings` sidecar inconsistent. IR #788 rejects the mutated package at `/temporal_bindings/0/clock/kind` with `TimedNodeReference` before the expected `UnknownProfile`. Keep the sidecar and mutated clause semantically aligned (or update the expectation only with spec backing). This is required #1443 test/API fallout. | qsl-package/src/checked_v2/tests.rs:2956 |
| FND-002 | medium | Six timed producer/read-back tests fail with `TimedNodeReference` because both writer paths intentionally pass `temporal_bindings: None`. The per-declaration clock name and `ClockConfiguration` writer is explicitly sequenced to QSL-1384; record this as remaining successor scope, without adding it or weakening/skipping these tests in #2045. | qsl-package/src/emit.rs:4678 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | At 5114fb983, the fixture mutation removes the temporal binding for the mutated declaration and omits the optional member when empty. Its focused test passes and reaches UnknownProfile. |
| FND-002 | deferred | Linked QSL-1384 owns the six still-failing producer/read-back cases. They remain enabled; the seventh suite failure reproduces on clean main as an equality-identity baseline. |
| FND-003 | still-open | Keep the binding row after changing its declaration to the mutated NodeRef; the current retain removes that row and the test masks missing-binding coverage behind UnknownProfile precedence. |
| FND-003 | rejected | False positive: FR-322 limits rows to clock-configured selected profiles; IR #788 `configured_profile` returns `None` for this unknown profile, so the row must be absent. |

Round 1 verification at `5114fb98342baf1b51cd1284c5964396a53cac8f`: the adverse-mutation fixture test, C14 test, and both CL-57..62 reader/emitter vector tests pass. Full `qsl-package --lib`: 480 passed, 7 failed, 2 ignored; six temporal binding failures are tracked by QSL-1384 and `union_nodes::a_function_naming_an_omitted_node_is_omitted_and_the_other_is_written` reproduces on clean main. The QSL-1443 → QSL-1384 blocker relation exists, and QSL-1384 records the six tests as retained successor scope.


## Follow-up verification at 0f2c41443

Reviewed PR #2045 head `0f2c41443a75fb63eef365c89dfe7b58737dca1c`. The root integration helper now constructs `CheckedPackageSidecarsRef { checked_term_data: None, temporal_bindings: None }` at `tests/it/emit_checked_v2.rs:427`; all 11 tests in the `emit_checked_v2` integration module pass. No new finding was identified. The prior unknown-profile mutation finding remains fixed at `5114fb983`; its focused fixture test passes, as do C14 and both CL-57..62 vector tests.

Prior FND-002 remains deferred to linked QSL-1384: all six temporal producer/read-back failures stay enabled, not waived. The equality-identity failure remains the clean-main baseline established in round 1. The full package suite and aggregate gate were not rerun in this follow-up; the lead will run the aggregate gate after disk cleanup.


## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The unknown-profile fixture changes the temporal clause identity and briefly updates its binding declaration to match, then `retain`s that same changed declaration out of `temporal_bindings`. The test therefore has a second defect (missing required binding) and only observes `UnknownProfile` because that profile check precedes binding coverage. Keep the updated binding row so the mutation isolates the unknown profile. | qsl-package/src/checked_v2/tests.rs:2981-2987 |

Disposition round 3 at `2c253e21a9dfa72e13de3a52a136852312402320`: the focused fixture test passes, and the profile/node identity and binding declaration are updated. However the row is then deleted. The package reaches the expected profile refusal before checking the missing binding, so the earlier UnknownProfile symptom is gone while the test still carries an unrelated malformed-wire condition. This remains open pending removal of the `retain` that deletes the changed row.


## Disposition round 4: FND-003

FND-003 is rejected as a false positive. QSpec main `6fee29e352dfacfe403fe86ba1d0eb0e3b18c56a`, FR-322 §Temporal clock bindings (lines 3018-3024) and FR-322-AC-294, require rows only for temporal declarations whose selected profile requires clock configuration; the member is absent when none selects a clock-configured profile. IR #788 `configured_profile` returns `None` when the profile identity does not map to one of the three clock-configured profiles. The missing-row coverage check filters on `configured_profile(...).is_some()`, so it does not require a row for this unknown profile. Removing the changed clause's row is contract-correct and isolates the intended `UnknownProfile` refusal. The focused fixture test passes at PR commit `2c253e21a9dfa72e13de3a52a136852312402320`.
