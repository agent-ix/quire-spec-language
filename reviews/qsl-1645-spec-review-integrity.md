---
id: SR-20261104153720
title: 'Spec review of PR #2300: ADR-032 CK-2 (c) and FR-270 addressing-value carve-out'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@6c7714f17; PR #2300: spec/decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md, spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md; ticket QSL-1645'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1645. PR: quire-spec-language#2300. This review covers ADR-032 CK-2 (c) and FR-270, which are refined so that five `quire-contract-model` addressing values fall outside the pre-check representation set. FR-516, FR-522 and FR-514 were read as context.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-032 CK-2 | spec/decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md:91 | examined |
| FR-270 statement, contract-model bullet and addressing-value bullet | spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md:53-63 | examined |
| FR-270-AC-1 | spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md:91 | examined |
| FR-270-AC-3 | spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md:93 | examined |
| FR-516 request/result table and clock-row selection | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:89-93,303-310 | context_only |
| FR-522 clock authentication | spec/functional/FR-522-bind-a-temporal-trace-to-the-checked-clause-s-profile-and-clock-and-report-the-authenticated-clock.md:69,89-101 | context_only |
| FR-514 qsl_inventory formal identity | spec/functional/FR-514-specify-the-spine-type-report-the-inventory-on-exhaustion-and-the-multi-unit-cycle-refusals.md:42,164 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The cited requirements do not name the types the way FR-270 and CK-2 (c) say they do. FR-516 names the occurrence by its shape `{ role, ordinal }`, not `CheckedOccurrence`. FR-522 names the `temporal_bindings` row, not `TemporalBinding`. FR-514 names "the selected formal identity", not `SourceIdentity`. Either reword to "the identity key of the occurrence (FR-516), the `temporal_bindings` row and its `ClockConfiguration` (FR-522), the formal source identity (FR-514)", or cite the code item that carries each. | spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md:58-63 |
| FND-002 | low | FR-270-AC-1 does not cover the carve-out's boundary. No AC criterion states that a `quire-contract-model` type outside the five names still fails when it appears beside an exempt one, for example in the same signature or `use` group. AC-1's only contract-model plant is a package by full path. Add that case to AC-1 so that the exact-name carve-out has a failing counterpart. | spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md:91 |

## Verdict

Mergeable. The carve-out is a real spec refinement, not a lowered gate:

- **CK-2 (c) keeps its purpose.** It exists so that executable IR content (packages, checked nodes, terms) reaches a stage entry only through the I2 reader. None of the five types holds such content (verified at quire-contract-model rev 64de9ec). The new text bounds the exemption by a stated property ("holds no checked node or term") and by a closed list of names. AC-1's package plant is unchanged and keeps the gate's failing case.
- **The change resolves a real conflict.** FR-516, FR-522 and FR-514 already require these keys in `qsl-eval` and `qsl-replay` stage-entry signatures. Before this PR, ADR-032 CK-2 (c) and those requirements contradicted each other.
- **AC-3 gains two no-finding plants.** They cover each crate kind: an eval entry and a replay entry outside `spine`.
