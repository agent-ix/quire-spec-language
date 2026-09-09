---
id: SR-094
title: "Native runtime scope and ownership review"
type: SpecReview
analysis: scope-boundary
scope: "FR-007/008/018, NFR-006, native-runtime input/evaluation contracts, IT-006, TC-055–077 and TM-004"
review_set: all
evaluated_revision: "045025f843346001a91919b8d0816a519e2df337"
review_date: "2026-09-09"
---

## Summary

Agent A owns the native payload, population validator and source-AST evaluator. Existing IR/model and B/C boundaries are retained with explicit assumptions and contract tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved: native emitted domain bytes and local source labels are explicitly not B's portable ArtifactRef/TechniqueResult authority. No shared-reference reader or result envelope is recreated. | FR-018; FR-008; docs/native-runtime-inputs.md |
| FND-002 | low | A native-rule-model milestone does not establish compiled ConfigVersion semantics or backend qualification. IT-006 explicitly retains IT-002 as the original end-state integration. | IT-006; IT-002 |

## Responsibility allocation

| Requirement | Owning component | Class |
| --- | --- | --- |
| StR-001 (existing context) | Agent A compiler pipeline | cross-cutting |
| FR-018 | Compiler runtime input module | core |
| FR-007 | Compiler runtime validator | core |
| FR-008 | Compiler reference interpreter | core |
| NFR-006 | Compiler runtime stage budgets | cross-cutting |

## External dependencies and trust

| Dependency | Assumed or guaranteed | Contract and evidence boundary |
| --- | --- | --- |
| Caller draft data and completeness | Assumed assertions; validated internal consistency | FR-007/018, no deployment-completeness proof |
| Caller cancellation poll | Assumed prompt termination; guaranteed true/panic handling | NFR-006, TC-065/075 |
| Contract IR at 690bde7 | Guaranteed native consumption through contract tests | Landed IT-005; IT-006 reuses actual declarations/proof API |
| Native source-derived model | Guaranteed reviewed source correspondence | Landed FR-015/016; runtime mutation setup retains it |
| serde_json and SHA-256 encoding | Guaranteed selected emitted representation via tests | FR-018, TC-056; existing dependency grants retained |
| Adopted state profile at e897f81 | Assumed normative meaning, implemented behavior tested | FR-008 and independent reference vectors |
| B's portable references/results | External owner; not consumed by this implementation packet | Inspected shared_reference.rs at d577486; future adapter handoff |
| Existing backend binder and Quire adapter | External/downstream; not qualified here | LC04/LC05 and IT-002/003 remain required |

## Context

Caller + checked native source/model → input constructor → validator →
source-AST evaluator → native source-bound report → later B-owned result adapter.
The existing IR enters through model/checker setup and retains later backend
binding. There is no database, persistence service, Markdown expression parser,
new model authority, networking or process runner.

All mutations stay in Agent A's language worktree. B/C/TL/Filament work is neither
claimed nor edited. Hosted CI remains workflow_dispatch only, without dispatch.
Standard historical profile bytes and licensing decisions remain unchanged.

## Verdict and provenance

PASS for implementation of this specified LC03 API scope. Agent A applied the
actual installed QUOIN base and all seven analysis skills serially, under the
owner's existing all-review selection. No additional agents or Cargo builds
ran. No applicable required AssuranceProfile was found. The declined optional
semantic gap comparison remains excluded; this specification review still
checks the actual adopted meaning and existing interface boundaries.

All runtime test rows remain planned. Review approval does not qualify native
execution, finish LC02/FS03 acceptance or complete the original backend/Quire
workflow. Implementation changes to this contract reopen specify/review.

## Constructor setup correction — 2026-09-09

Reviewed dc63f337fcdaee1320d221383eca38599239f62c. TC-055–057 now explicitly
exercise A's constructor boundary with existing IR identities. They confer no
model validity, population closure, runtime Boolean or portable result authority.
The model/checker owns static admission and later validation consumes it; no
reader or dependency on B's workspace is added. All component allocations and
external assumptions above remain unchanged. PASS for this correction.
