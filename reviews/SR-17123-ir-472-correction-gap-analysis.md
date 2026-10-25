---
id: SR-17123
title: "Gap analysis of IR-472 QSL enum admission correction"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; IR-472; PR #1510; qsl-semantics/tests/it/enum_type_admission.rs, xtask/src/test_suites.rs; RT TC-024, QSL FR-091-AC-28, TC-481, NFR-014-AC-5/7, TC-950"
review_set: subset
---
# Gap analysis of IR-472 QSL enum admission correction

## Summary

Reviewed the corrected enum test against RT TC-024 procedure 3 and the QSL requirement and test case previously tagged by mistake. At final candidate e365525fdbb2f78e8821643e5e936188b02f9514, the additional xtask helper change supports the existing real Cargo test for NFR-014-AC-5/7 and TC-950 by making each fixture rewrite newer than the completed prior build.

## Verdict

**PASS** — the direct checker control is present, the false QSL acceptance-criterion binding has been removed, and the xtask change preserves the real Cargo empty-suite control through addition and restoration. The test does not claim to complete FR-091-AC-28's broader scenarios.

## Coverage

Plan completion: not assessed.

- RT TC-024 procedure 3: examined direct missing-enum type-form refusal at the QSL checker boundary.
- QSL FR-091-AC-28 and TC-481: examined the distinct package-assembler and enum-member scenarios to verify removal of the former false trace tag.
- `qsl-semantics/tests/it/enum_type_admission.rs`: examined positive and negative checker behavior and test provenance.
- `qsl-semantics/src/check/check.rs`, `type_form.rs` and `mod.rs`: examined actual enum indexing, name lookup and signature checking path.
- No production code was added, so no new unowned source or stub was introduced.
- `xtask/src/test_suites.rs`: examined the existing `#[trace("TC-950", "NFR-014-AC-5", "NFR-014-AC-7")]` binding and its baseline, ignored-test and restore assertions. The changed timestamp helper supports that test without changing the suite policy or adding an unowned production path.

## Checks

- The prior SR-17083 FND-001 disposition matches the current test: `#[trace("TC-481", "FR-091-AC-28")]` is absent and RT TC-024 appears only as provenance.
- No new Test Matrix row was hand-written. Matrix-wide coverage of FR-091-AC-28 was outside this corrective PR.
- Both carried SpecReview files passed targeted `quire validate`.
- The focused real Cargo test passed at e365525fdbb2f78e8821643e5e936188b02f9514; all three fixture builds completed and the test returned `ok`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
