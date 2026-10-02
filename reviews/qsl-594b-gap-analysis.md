---
id: SR-1234
title: "Gap analysis of quire-spec-language PR #609: FR-284-AC-1 and AC-4 with the FCD exemptions"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3f607547271412ef60ff235ea429d454edd028cd; PR #609: FR-284-AC-1, FR-284-AC-4, FR-280-AC-3 against tools/arch-lint/qualified_core.rs tests (tc_767_*, tc_768_*); FR-284 Status, spec.md and tests.md rows"
review_set: subset
---
# Gap analysis of quire-spec-language PR #609

## Summary

The two new tests are traced correctly and use literal oracles:
- `tc_767_only_the_fcd_frontend_clap_edge_is_skipped` (FR-284-AC-1) lists the exact pairs and the `via` chain.
- `tc_768_only_the_fcd_lift_scratch_directory_is_skipped` (FR-284-AC-4) lists the exact (file, line, category) findings.

The spec.md FR-284 row and the TC-767/TC-768 rows match: the target runs in `make ci`, and TC-768 steps 1 and 2 still wait on FR-275. Removing the OnceLock caches changes no AC, and the existing lock and catalog tests (`complete_value_lock`, `ieee_profiles`, `integer_division`) pass.

## Verdict

Changes requested: two low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No AC or Behavior paragraph of FR-284 owns the two exemptions. AC-1 says the direction check "passes over the QSL workspace", and AC-4 says the scan fails "naming the file and line of each ... filesystem or search-path access", with no exception. Only the Status prose describes the skipped edge and the skipped site, yet both new tests trace to AC-1 and AC-4 and assert the exemption behaviour. Fix: state both exemptions, with their exit conditions (SR-1233 FND-001), in FR-284's Behavior or in AC-1 and AC-4, so the tests trace to stated behaviour. | spec/functional/FR-284-keep-the-qualified-core-separable-by-crate.md:81-84; tools/arch-lint/qualified_core.rs:1143; tools/arch-lint/qualified_core.rs:1303 |
| FND-002 | low | No test covers a second route to `agent-ix-extraction-frontend` alongside the sanctioned one. A fixture such as `qsl-semantics -> Y -> agent-ix-extraction-frontend` added to `with_fcd_frontend()` passes today (SR-1233 FND-002). Add that case to `tc_767_only_the_fcd_frontend_clap_edge_is_skipped`, expecting a finding. | tools/arch-lint/qualified_core.rs:1143-1173 |
