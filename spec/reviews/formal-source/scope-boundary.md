---
id: SR-062
title: "scope-boundary review of native formal source correspondence"
type: SpecReview
analysis: scope-boundary
scope: "FR-014, docs/formal-source-binding.md, TC-035–039 and TM-003 addition"
review_set: all
evaluated_revision: "4eb4ef6"
review_date: "2026-09-08"
---

## Summary

Agent A owns local source correspondence; existing native intake and IR type constructors retain their responsibilities.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No overlap with B/C/TL or Filament producer ownership is required. | FR-014; FR-001; FR-010 |

## Analysis

| Requirement | Owning component | Class |
| --- | --- | --- |
| FR-001 | Native source intake | core |
| FR-010 | Native diagnostics | cross-cutting |
| FR-014 | Native formal_source module | core |
| NFR-005 | Native production/qualification paths | cross-cutting |

System context: caller → native FormalSource → existing Source coordinate API and pinned Contract IR source constructors → caller. Caller authority to assign the appropriate formal identity is assumed and exposed explicitly. Source byte/coordinate correspondence is guaranteed by TC-035–039; actual pinned IR constructor consumption is guaranteed by the integration cases. There are no external services.
    
The bridge does not create a model source decoder, authored requirement registry, shared evidence store or executable projection. Local path equality guards requests without creating a portable path identity. Package-wide identity uniqueness and preserving original extraction segments belong to their future owning integration contracts. Public release, hosted CI and changes to another owner's repositories are outside this task.

## Verdict and provenance

PASS for this specified bridge. Agent A applied the actual QUOIN base and all
seven selected analysis skills serially, with no subagents. Catalog contracts,
source/IR implementations and the scoped acceptance cases were inspected.
There is no applicable required AssuranceProfile. No implementation or full
state-workflow completion is claimed. The declined optional gap-analysis
semantic comparison remains excluded.

