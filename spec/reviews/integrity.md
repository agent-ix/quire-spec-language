---
id: SR-003
title: "integrity review of quire-spec-language"
type: SpecReview
analysis: integrity
scope: "spec/spec.md and indexed requirements"
review_set: all
evaluated_revision: "a80a17d1dd303b91712df2023fdba8aba83e89c1"
review_date: "2026-09-07"
---

## Summary

All stories and FRs have a resolved stakeholder/verification lineage, and all NFRs are scoped. The remaining integrity findings concern constraint discoverability, evidence methods and current API assumptions.

## Verdict

**CONDITIONAL** — Findings require disposition before the affected implementation or acceptance gate. This is not owner acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Affected FR artifacts omit references back to the scoped NFR constraints. A reader implementing one FR can miss its budget, rights or identity obligations; preserve the existing constrains relation and make the constraints discoverable from the FR. | NFR-001–NFR-004; FR Dependencies sections |
| FND-002 | medium | NFR verification cells need catalog method identities and evidence planning; all FRs already have a valid Test class, so they are not methodless requirements. | [evidence review](evidence.md) |
| FND-003 | medium | FR-003 allows a selected output budget while the implementation accepts none. Clarify library versus CLI responsibility and exact ceiling behavior before remediation. | FR-003 Inputs/Behavior/AC-4; NFR-001; ../../src/format.rs:8 |
| FND-004 | low | Optional producer checking pins Filament and TypeSpec, but the minimum Node/Python environment and missing-tool diagnostic/timeout runner are not fully specified in NFR-002. This is integration setup debt. | NFR-002; IT-001; ../../tools/check_model_fixture.py:30 |

## Scope and provenance

Reviewed `quire-spec-language@a80a17d1dd303b91712df2023fdba8aba83e89c1`. The owner selected base plus all seven Quoin analyses, and declined the optional intent↔test↔code semantic step in `/gap-analysis`. Ordinary requirements consistency, EARS conformance, and the separate current-code review remain in scope. The assignment prohibits spawning additional agents, so these analyses were performed sequentially by Agent A; no independent reviewer acceptance is implied.

The installed Quoin 0.20.0 `spec-review/SKILL.md` and its analysis skills govern these artifacts. The SpecReview authoring pack was fetched once for this repository and its process skeleton/schema was used. [Provenance](data/provenance.json), [coverage output](data/coverage.json), [advisor output](data/advice.json), and [actual method catalog](data/verification-methods.json) preserve the deterministic inputs. IDs in this report are local to this repository unless qualified.

These are new repositories. Missing formal plans, TC records, suites, and matrices are workflow setup/readiness debt. Unimplemented LC02–LC05 stages and unqualified shared consumers are known remaining work; they are not reported as regressions or hidden stubs. No completion status is fabricated.

## Traceability matrix

| US | FR | StR | Verification |
| --- | --- | --- | --- |
| [US-001](../usecase/US-001-author-native-source.md) | [FR-001](../functional/FR-001-read-exact-source.md) | [StR-001](../stakeholder/StR-001-native-assessment-trust.md) | Test; 4 ACs; no TC/matrix binding yet |
| [US-001](../usecase/US-001-author-native-source.md) | [FR-002](../functional/FR-002-parse-native-units.md) | [StR-001](../stakeholder/StR-001-native-assessment-trust.md) | Test; 5 ACs; no TC/matrix binding yet |
| [US-001](../usecase/US-001-author-native-source.md) | [FR-003](../functional/FR-003-format-native-source.md) | [StR-001](../stakeholder/StR-001-native-assessment-trust.md) | Test; 4 ACs; no TC/matrix binding yet |
| [US-002](../usecase/US-002-link-exact-models.md) | [FR-004](../functional/FR-004-verify-source-maps.md) | [StR-001](../stakeholder/StR-001-native-assessment-trust.md) | Test; 4 ACs; no TC/matrix binding yet |
| [US-002](../usecase/US-002-link-exact-models.md) | [FR-005](../functional/FR-005-link-shared-model.md) | [StR-001](../stakeholder/StR-001-native-assessment-trust.md) | Test; 5 ACs; no TC/matrix binding yet |
| [US-002](../usecase/US-002-link-exact-models.md) | [FR-006](../functional/FR-006-check-defined-expressions.md) | [StR-001](../stakeholder/StR-001-native-assessment-trust.md) | Test; 5 ACs; no TC/matrix binding yet |
| [US-003](../usecase/US-003-evaluate-bounded-state.md) | [FR-007](../functional/FR-007-validate-runtime-inputs.md) | [StR-001](../stakeholder/StR-001-native-assessment-trust.md) | Test; 5 ACs; no TC/matrix binding yet |
| [US-003](../usecase/US-003-evaluate-bounded-state.md) | [FR-008](../functional/FR-008-evaluate-state-reference.md) | [StR-001](../stakeholder/StR-001-native-assessment-trust.md) | Test; 10 ACs; no TC/matrix binding yet |
| [US-004](../usecase/US-004-reuse-existing-toolchain.md) | [FR-009](../functional/FR-009-lower-qualified-projections.md) | [StR-001](../stakeholder/StR-001-native-assessment-trust.md) | Test; 4 ACs; no TC/matrix binding yet |
| [US-001](../usecase/US-001-author-native-source.md) | [FR-010](../functional/FR-010-report-native-outcomes.md) | [StR-001](../stakeholder/StR-001-native-assessment-trust.md) | Test; 5 ACs; no TC/matrix binding yet |
| [US-004](../usecase/US-004-reuse-existing-toolchain.md) | [FR-011](../functional/FR-011-integrate-opaque-extraction.md) | [StR-001](../stakeholder/StR-001-native-assessment-trust.md) | Test; 4 ACs; no TC/matrix binding yet |

## NFR scope and reverse references

| NFR | Affected FRs | Result |
| --- | --- | --- |
| [NFR-001](../non-functional/NFR-001-bound-syntax-work.md) | FR-001, FR-002, FR-003, FR-004 | Explicit NFR scope and constrains edges; affected FR Dependencies sections do not link back |
| [NFR-002](../non-functional/NFR-002-reproduce-native-builds.md) | FR-001, FR-002, FR-010 | Explicit NFR scope and constrains edges; affected FR Dependencies sections do not link back |
| [NFR-003](../non-functional/NFR-003-preserve-uncertain-outcomes.md) | FR-005, FR-006, FR-007, FR-008, FR-009 | Explicit NFR scope and constrains edges; affected FR Dependencies sections do not link back |
| [NFR-004](../non-functional/NFR-004-preserve-implementation-rights.md) | FR-002, FR-009, FR-011 | Explicit NFR scope and constrains edges; affected FR Dependencies sections do not link back |

## Consistency, atomicity and assumption probes

Each FR has one stage-level observable outcome, with refusal/edge cases refining that outcome. No duplicate requirement with a conflicting outcome was found. StR-001 supplies the overall operational validation need rather than a second compiler stage. The user-story and StR lineage resolves locally.

Multi-source lookups must refuse ambiguity or foreign authority; no first-wins registry assumption is introduced. Missing typed adapters and consumers have explicit refusal/unavailable states and a named A/B/C resolution chain. No paginated remote API, authenticated service, retrying network loop or interactive scaffolder is in the implemented native core, so those probe categories are inapplicable here. Native parsing/formatting needs no external runtime; the optional producer process is a separately pinned integration dependency. Wire/profile diagnostics belong to their selected contracts, so differing diagnostic spellings across domains are not silently unified.

Failure/purity/topology checks are in [failure-domain](failure-domain.md). This review checks specification consistency; it does not claim the declined optional semantic test/code comparison.
