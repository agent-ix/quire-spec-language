---
id: SR-400
title: "Risk-complexity review of LC04 native backend parity completion"
type: SpecReview
analysis: risk-complexity
scope: "FR-009-AC-5; IT-008; TC-094; TM-006; Plan-008 Task-020"
review_set: all
---

## Summary

The volatile edge is confined to an exact cargo-llvm-cov 0.9.0 / LLVM JSON 3.1.0
fixture reader. Exact version, manifest, file and probe assertions make producer
drift fail closed; production lowering and the reusable reader are unchanged.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Future LLVM format drift requires an explicit fixture revision and cannot silently satisfy activation evidence. | IT-008; TC-094 |
