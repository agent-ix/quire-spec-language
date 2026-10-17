---
id: SR-12086
title: "Gap analysis of quire-spec-language PR #939 against QSL-828, FR-003-AC-9 and NFR-013"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@484d683bf38a918b50e5b6b360e9c8a5e32f1403; PR #939: Makefile (test-deep-format), tests/it/formatter_identity.rs, xtask/src/ci_feature_lane_tests.rs; FR-003-AC-9, TC-885, NFR-013-AC-1, NFR-013-AC-3, QSL-828 acceptance"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-013
    type: reviews
---
# Gap analysis of quire-spec-language PR #939

## Summary

Ticket: QSL-828 (ticket text read as data). Planless, scoped to the PR diff.
Plan completion: not assessed. Read-only: no build or test was run. A full
`quire matrix` run timed out after 120 s, so trace bindings were checked
against the tags in the source and the TC-885 / FR-003-AC-9 text.

## Verdict

Pass. No major gap found.

- QSL-828 asks to cut the time without lowering what the test proves: "the
  formatter keeps format identity at the full size the ACs name". FR-003-AC-9
  and TC-885 name no numeric size. They quantify over "each complete-V1
  fixture unit in the repository's tests that checks". The full size comes
  from the owning fixture's 100,000-level generators, and all three are kept
  at 100,000. Every assertion of the old combined test is in the helper (see
  SR-12085). The full-size cases stay, and they still run in `make ci`
  through `test-deep-format` in CI_CHECKS.
- Trace tags. Each split test keeps `#[trace("TC-885", "FR-003-AC-9")]`, and
  that is honest. The AC is per unit ("for each ... fixture unit"). Each split
  test checks the whole AC (check, format, second-pass byte identity, equal
  package identity) on its own fixture unit through `emitted_with_authority`.
  No test claims a part of the AC that only all three together would check.
  This matches how the other per-unit TC-885 tests are tagged
  (`lifecycle_large_declaration_unit_keeps_format_identity`,
  `replay_large_source_generator_keeps_format_identity`,
  `dependency_chain_and_order_variants_keep_format_identity`). The old single
  test was the odd one out, covering three units under one tag.
- NFR-013. The stage still runs `test --locked --workspace --tests` in the
  default lane's directory, using the caller root's `ci-default-features`
  child or `CI_DEFAULT_TARGET_DIR`. The NFR-013 sentence about
  `test-deep-format` names no thread count or case list, so it is still true.
  No feature lane's flags changed.
- NFR-013-AC-1 / AC-3 tests. In TC-915,
  `deep_formatter_target_runs_each_full_size_case_and_is_in_ci` pins the exact
  five-case argv with `--test-threads=5` in all three directory modes, keeps
  the CI_CHECKS membership check, and keeps the 0/2 listing rejections.
- Not assessed: QSL-828's before/after seconds report. The leader's final
  measured run is still in progress. The PR body states the profile figures
  (17 s parse, 60 s check, 83 s package per 1.79 MB source). This review did
  not re-measure them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
