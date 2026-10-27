---
id: SR-822
title: "Scope-boundary review of QSL conformance corpus bindings"
type: SpecReview
analysis: scope-boundary
scope: "QSL FR-350/351/353 and QSpec FR-336"
review_set: subset
---
# Scope-boundary review of QSL conformance corpus bindings

## Summary

PASS. QSL execution is restricted to public subject entry points; QSpec owns
wire shape, oracle, applicability and recorded results.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved scope allocation issue found: the QSL requirements do not rewrite QSpec-owned vectors or treat the stored result member as settlement input. | FR-350; FR-351 |
