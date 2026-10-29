---
id: SR-071
title: "Native model and checker risk review"
type: SpecReview
analysis: risk-complexity
scope: "FR-015/016, FR-006 judgments, docs/native-model-checking.md, IT-005, TC-025–029/040–053 and TM-003"
review_set: all
evaluated_revision: "ceccabb564b61742597ad356eb1019ab0c8d1544"
review_date: "2026-09-08"
---

## Summary

The main risk is accepting an unsound guard proof, followed by identity drift and expansion cost. Each has a specific bounded qualification control before handoff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Implementation risk remains: the first native proof abstraction must overapproximate actual values and preserve evaluation order. Mitigated in the specification by independent TC-053 and real IR negative controls; this is not an open specification defect or an executed proof. | FR-016; TC-025–029; TC-047; TC-050; TC-053 |

## Risk register

| Requirement | Technical risk | Volatility | Driver | Mitigation |
| --- | --- | --- | --- | --- |
| StR-001 | High | Medium | Multiple unfinished workflow stages | Preserve full runtime/backend/Quire acceptance |
| NFR-005 | Low | Low | Established owner language policy | Rust producer, assertions and local commands |
| FR-013 | Low | Low | Landed compatibility boundary | Retain old linker tests and profile |
| FR-014 | Low | Low | Landed coordinate bridge | Reuse actual FormalSource checks |
| FR-015 | High | Medium | New nominal/reference model interpretation | Independent source producer; role and artifact mutation tests |
| FR-006 | High | Low | Safety judgments must not become layout-only examples | Preserve all five actual reference/operation cases |
| FR-016 | High | Medium | New proof abstraction and shared graph expansion | TC-053 independent oracle; IR discharge; resource accounting |
| FR-007 | Medium | Medium | Downstream input assumptions must stay explicit | Retain population/context/frame requirements; no early truth |

Top hazards: unsound guard joins or alias identity; model/provenance drift; proof expansion before accounting; resource contention on the shared desktop. The first three are addressed by failure-domain.md and TC-041–053. The last uses serial low-priority execution, bounded generated families and existing caches; an idle snapshot is not permission for build fanout. No provisional shared service or speculative parallel worker is needed.

## Verdict and provenance

PASS for implementation of this specified scope. Agent A applied the actual
QUOIN base and all seven analysis skills serially, following the owner's
selected all review set. No subagents or builds were started for this review.
No applicable required AssuranceProfile was found. Catalog schema and existing
public interfaces were inspected. Implementation/test completion is not claimed.
The owner-declined optional gap-analysis semantic comparison remains excluded.

