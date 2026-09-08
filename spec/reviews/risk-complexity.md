---
id: SR-006
title: "risk-complexity review of quire-spec-language"
type: SpecReview
analysis: risk-complexity
scope: "spec/spec.md and indexed requirements"
review_set: all
evaluated_revision: "a80a17d1dd303b91712df2023fdba8aba83e89c1"
review_date: "2026-09-07"
---

## Summary

Every FR/StR/NFR has a technical-risk and volatility assessment with concrete mitigations. The principal risks are identity/qualification boundaries and first implementation of reference behavior, rather than missing infrastructure being mistaken for a regression.

## Verdict

**CONDITIONAL** — Findings require disposition before the affected implementation or acceptance gate. This is not owner acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | High-risk or volatile shared identity, runtime validation and consumer qualification work needs bounded acceptance slices and the named mitigations before task generation. | Risk register and top hazards below; [dependency](dependency.md); [evidence](evidence.md) |

## Scope and provenance

Reviewed `quire-spec-language@a80a17d1dd303b91712df2023fdba8aba83e89c1`. The owner selected base plus all seven Quoin analyses, and declined the optional intent↔test↔code semantic step in `/gap-analysis`. Ordinary requirements consistency, EARS conformance, and the separate current-code review remain in scope. The assignment prohibits spawning additional agents, so these analyses were performed sequentially by Agent A; no independent reviewer acceptance is implied.

The installed Quoin 0.20.0 `spec-review/SKILL.md` and its analysis skills govern these artifacts. The SpecReview authoring pack was fetched once for this repository and its process skeleton/schema was used. [Provenance](data/provenance.json), [coverage output](data/coverage.json), [advisor output](data/advice.json), and [actual method catalog](data/verification-methods.json) preserve the deterministic inputs. IDs in this report are local to this repository unless qualified.

These are new repositories. Missing formal plans, TC records, suites, and matrices are workflow setup/readiness debt. Unimplemented LC02–LC05 stages and unqualified shared consumers are known remaining work; they are not reported as regressions or hidden stubs. No completion status is fabricated.

## Risk register

| Requirement | Technical risk | Volatility | Drivers | Mitigation |
| --- | --- | --- | --- | --- |
| [FR-001](../functional/FR-001-read-exact-source.md) | medium | low | Untrusted bytes and bounded source identity intake. | Property/fuzz tests of UTF-8, exact digest and size boundaries. |
| [FR-002](../functional/FR-002-parse-native-units.md) | medium | medium | Grammar/Pratt changes can drift precedence or consume resources. | Generated valid/invalid cases and independent precedence expectations. |
| [FR-003](../functional/FR-003-format-native-source.md) | medium | medium | Formatter can expand source and its budget API conflicts with the draft. | Reconcile selected ceiling; token/idempotence properties and boundary tests. |
| [FR-004](../functional/FR-004-verify-source-maps.md) | high | medium | Source maps control original evidence locations. | Generated exact correspondence, layout and foreign-binding controls. |
| [FR-005](../functional/FR-005-link-shared-model.md) | high | high | External typed-model adapter and declaration IDs remain unqualified. | Require selected shared model contract and independent binding fixtures. |
| [FR-006](../functional/FR-006-check-defined-expressions.md) | high | high | First definedness/type implementation and changing semantic examples. | Pin reviewed rules; generated and independent adverse type cases. |
| [FR-007](../functional/FR-007-validate-runtime-inputs.md) | high | high | Runtime closure and invocation/frame integrity are trust boundaries. | Validate complete populations first; inject partial observations/failures. |
| [FR-008](../functional/FR-008-evaluate-state-reference.md) | high | high | New evaluator must preserve observations and incomplete outcomes. | Independent reference cases, resource injection and exact activation records. |
| [FR-009](../functional/FR-009-lower-qualified-projections.md) | high | high | Existing IR cannot silently encode unsupported native identity semantics. | Capability-qualified lowering; obligation census and independent parity limits. |
| [FR-010](../functional/FR-010-report-native-outcomes.md) | medium | medium | CLI OS boundary currently has a reproducible panic. | Specify args_os encoding policy; test usage/I/O/resource exit codes. |
| [FR-011](../functional/FR-011-integrate-opaque-extraction.md) | high | high | Existing extractor integration is C-owned and not yet qualified. | Opaque-body contract and real original-source map integration test. |
| [NFR-001](../non-functional/NFR-001-bound-syntax-work.md) | medium | medium | Allocation/depth ceilings must be checked before growth. | Boundary/property tests; long flat chain on a constrained stack. |
| [NFR-002](../non-functional/NFR-002-reproduce-native-builds.md) | medium | medium | Pinned native runtime and optional producer environments differ. | Locked minimal-feature CI plus explicit external-tool detection contract. |
| [NFR-003](../non-functional/NFR-003-preserve-uncertain-outcomes.md) | high | high | Future stage failures can be erased in portable result translation. | Fail each stage; assert no Boolean or dropped obligation. |
| [NFR-004](../non-functional/NFR-004-preserve-implementation-rights.md) | high | medium | New and inherited artifacts have different rights. | Included-file inventory, preserved notices and selected dependency review. |
| [StR-001](../stakeholder/StR-001-native-assessment-trust.md) | high | high | Full workflow needs multiple not-yet-qualified boundaries. | Small real ConfigVersion acceptance slice with independent exact bindings. |

## Top hazards

1. FR-004/FR-005: source or model identity attribution across adapters.
2. FR-006/FR-008: first evaluator/type implementation over unsettled profile refinements.
3. FR-007/NFR-003: partial validation or cancellation becoming a Boolean.
4. FR-009/FR-011: backend/extractor qualification across A/B/C ownership.
5. FR-003/FR-010: current formatter-budget discrepancy and CLI argument panic.

## Failure-domain cross-check

The register uses the trust, identity, purity and topology boundaries in [failure-domain](failure-domain.md). A high technical-risk score is a planning priority, not a high-severity defect or a claim of a failed test. Stage new technology behind a reviewed bounded slice; keep volatile external contracts isolated in their owning adapters. No concurrency implementation currently warrants Loom adoption.
