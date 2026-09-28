---
id: SR-769
title: "QSL-314 gap analysis of PR 509 (TC-469 ConfigVersion spine corpus)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@01356698adcabce3ed91138221b7424b12097bed; spec/functional/FR-108-run-the-configversion-spine-corpus.md; spec/test-cases/TC-469-configversion-spine-corpus-matches-native.md; tests/it/config_version_spine.rs; examples/config-version/spine.rs; examples/config-version/model.semantic-ir.json; examples/config-version/fixtures.rs; examples/config-version/cases.rs (unchanged); examples/config-version/README.md (unchanged); tests/it/config_version.rs (unchanged)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-469
    type: reviews
---
## Summary

Ticket: QSL-314. PR: quire-spec-language#509 at 01356698. There is no plan
bundle. The scope is FR-108-AC-1 to AC-6, verified by TC-469 steps 1 to 6.
QSL-315 is the bug split out of AC-6.

AC-to-test coverage, measured against the code and my own `make ci` run on
the merge with main (exit 0, 93 ok, 0 FAILED):

| AC | Test | Status |
| --- | --- | --- |
| FR-108-AC-1 | `tc_469_step_1_every_case_matches_the_independent_expected_table` | Covered. The 17-arm exhaustive match agrees with the Corpus table. |
| FR-108-AC-2 | `tc_469_step_2_and_3_...` | Partial. Exit code only (SR-768 FND-001). |
| FR-108-AC-3 | `tc_469_step_2_and_3_...` | Partial. The spine field is exact, but the object may be either, and the native object check is vacuous (SR-768 FND-002). Boundary true-in-both holds through step 1 (spine) and native exit 0. |
| FR-108-AC-4 | `tc_469_step_4_...` | Partial. 3 of 17 cases re-run, a report subset compared, selection and limits provenance not asserted (SR-768 FND-003). |
| FR-108-AC-5 | `extraction::tc_469_step_5_...` | Covered under `--all-features`, which `make ci`'s `ci-all-features` runs. |
| FR-108-AC-6 | `tc_469_step_6_package_id_is_pinned_across_every_case` plus the ignored `tc_469_step_6_the_emitted_package_admits_via_i04` | Half. Every case's `package_id` agrees across cases and with a direct `spine::compile`. I04 `read` is deferred to QSL-315. |

The QSL-315 deferral is genuine: un-ignored, the I04 test fails with
`InvalidSemanticGraph` at `/semantic_graph/nodes/2/body/modifies/0`, and
nothing else fails first. The package_id half is real coverage. It would have
caught the per-case `SourceIdentity` defect the PR describes, and it checks
`run_clause`'s compile against an independent `spine::compile` call. It does
not pin a value, though (FND-003).

Underspecified code: none. `spine.rs` writes a `clause-run-request.json` that
nothing reads. Its module doc says so, and it serves FR-108's "ClauseRunRequest
written beside the native files" output.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The spec status is stale after this PR. FR-108-AC-6's Verification says "pending STD-111", and FR-108's Status still says AC-6 is pending STD-111. The PR itself establishes that STD-111 is not the blocker and QSL-315 is. TC-469's Status still reads "Planned (QSL-273). Step 6 is pending STD-111". Update both to say steps 1 to 5 and step 6's package_id half are implemented, and the I04 read is pending QSL-315. | spec/functional/FR-108-run-the-configversion-spine-corpus.md:131; spec/functional/FR-108-run-the-configversion-spine-corpus.md:145-149; spec/test-cases/TC-469-configversion-spine-corpus-matches-native.md:72-76 |
| FND-002 | low | FR-108 Inputs names the domain package "version `1`", and its unit text says `version "1"`. The committed package declares `1.0.0`, and the generated unit says `version "1.0.0"`. I1 admission requires the model statement to equal `package.version`, and the in-crate fixture also uses `1.0.0`. The spec text should say `1.0.0`, so the spec states what is built. | spec/functional/FR-108-run-the-configversion-spine-corpus.md:40-41; spec/functional/FR-108-run-the-configversion-spine-corpus.md:52; examples/config-version/spine.rs:45-53 |
| FND-003 | low | FR-108-AC-6 and TC-469 step 6 say "the expected table pins the unit's `package_id`". The test checks agreement (across cases, and with a direct compile) but pins no value, so a change to PackageId derivation would pass unnoticed. QSL-315 changes this unit's PackageId anyway, so either add the literal when QSL-315 lands, or amend AC-6 to the agreement wording that is actually tested. Do not add a literal now just to churn it. | tests/it/config_version_spine.rs:552-598; spec/functional/FR-108-run-the-configversion-spine-corpus.md:131 |
| FND-004 | low | `examples/config-version/README.md` does not mention the spine files the generator now writes into every case directory (`spine-unit.native`, `spine-model.semantic-ir.json`, the `spine-*` documents and `clause-run-request.json`). Its expected-outcome table also still lists only 13 cases (the four boundary cases predate this PR). | examples/config-version/README.md:18-37 |

## Verdict

PARTIAL. AC-1 and AC-5 are fully covered. AC-6's runnable half is real, and
the I04 half is honestly deferred to QSL-315. AC-2, AC-3 and AC-4 are only
partly tested; the code-level fixes are SR-768 FND-001 to FND-003. The four
low findings here are spec, status and doc text that can ride in the same fix
round.
