---
id: SR-414
title: "Dependency review of ConfigVersion numeric backends"
type: SpecReview
analysis: dependency
scope: "IT-010 and prerequisite FR/codegen/tool identities"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/IT-010
    type: reviews
---

## Summary

IT-010 is a feature integration over already delivered SL and codegen enablement. The exact
dependency graph is acyclic, ordered and pinned; no unresolved implementation prerequisite remains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Closed: the draft now selects codegen `5e2a6a9`, IR `04eb6f8`, runtime `8a4d02b` and cargo-kani 0.67.0 by exact identity, eliminating the former two-IR executable graph. | IT-010-SC-01 |
| FND-002 | low | No dependency cycle found; the object/graph IR design is an explicit excluded future capability rather than a hidden prerequisite for ConfigVersion scalar parity. | IT-010-SC-06 |

## Classification

| Requirement or dependency | Class | Rationale |
| --- | --- | --- |
| FR-032 | Enablement | Supplies the real ConfigVersion model, source and native runtime fixtures. |
| FR-033 | Enablement | Supplies bounded integer projection and command selection. |
| FR-034 | Enablement | Supplies pre/post scalar projection and validated primitive inputs. |
| codegen PRs #29/#30/#31 | Enablement | Supply numeric oracles, constructive strategies and bounded Kani contracts. |
| IT-010 | Feature | Demonstrates the user-visible cross-backend formal-spec path. |

## Dependency Graph

```text
FR-032 ---> FR-034 -----------+
FR-033 ---> FR-034            |
                              v
codegen #29 -> #30/#31 ----> IT-010
          IR/runtime/Kani ----^
```

## Topological Order

1. FR-032, FR-033 and FR-034 (delivered SL enablement).
2. Codegen #29, #30 and #31 (delivered external enablement).
3. One SL pin resolution, then IT-010 compilation/execution and evidence.
