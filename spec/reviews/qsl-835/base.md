---
id: SR-60050
title: "Base requirements review for QSL-835 mapped-surface retirement"
type: SpecReview
analysis: base
scope: "FR-490 and TC-484"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-490
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-484
    type: reviews
---

## Summary

The retirement requirement names the exact root modules, files, Cargo feature,
and direct dependency removed by QSL-835. Its test case also protects the
separate FR-030/FR-031 `qsl-replay` path and the root feature-gated integration
tests that must move with it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-490-AC-1 to AC-3 each have a concrete compile, metadata, or retained integration-test check; no independent product choice or unowned result is introduced. | FR-490, TC-484 |
