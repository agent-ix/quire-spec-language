---
id: SR-407
title: "Risk-complexity review of protocol-role refusal loci"
type: SpecReview
analysis: risk-complexity
scope: "FR-042-AC-8; TC-121; issue #68"
review_set: subset
---

## Summary

Technical risk and volatility are low: this is a diagnostic-state correction within an existing
single-threaded admission path, with no protocol, dependency, security, concurrency, or performance
contract change. The principal hazard is regression through future fallible work before locus setup.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Future role-admission refactors could reintroduce stale diagnostic state; the exact two-source regression test is the named mitigation. | FR-042-AC-8; TC-121 |

The owning requirement is low technical risk and low volatility. Its mitigation runs through the
public native admission path and asserts the semantic source/span pair, not an internal helper call.
No high-risk or high-volatility item requires a spike, staged rollout, or additional dependency.
