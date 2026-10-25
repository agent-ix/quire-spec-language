---
id: SR-17046
title: "IR-878 scope-boundary review"
type: SpecReview
analysis: scope-boundary
scope: "spec/functional/FR-486-supplied-admission-and-conversion-owner-allocation.md"
review_set: all
---

## Summary

Checked the QSL, QSV, and quire-exact responsibility boundaries. FR-486 assigns only the QSL evaluator and caller portions and leaves helper semantics and direct QSV omission behavior with their named owners.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No scope leakage found; the 4096-bit default is a QSL caller responsibility, while required-bound semantics for a direct QSV caller remain external. | FR-486 |
