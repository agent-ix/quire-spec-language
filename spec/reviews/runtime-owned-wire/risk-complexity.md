---
id: SR-270
title: "Risk review of owned runtime decoding"
type: SpecReview
analysis: risk-complexity
scope: "FR-024 amendment; TC-099/100; Task-035; implementation 3c6a0e6"
review_set: all
---
## Summary

The main risk is accepting a malformed wire shape or changing existing bytes while relocating decoder ownership. The change is confined to runtime types and their shared digest codec.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | - |

## Risk register

| Requirement | Technical risk | Volatility | Driver | Mitigation |
| --- | --- | --- | --- | --- |
| FR-024 amendment | High | Low | Untrusted input boundary and byte identity | Direct-record negative controls, all value variants, required-null result controls, existing fixed artifacts and actual execution regression tests |

The hazards are positional arrays, silently missing optional fields, and altered digest encoding. Closed private wire types, exhaustive conversions and the explicit required-result representation address them without a new parser. See [failure-domain](failure-domain.md). The pending native-profile interpretation is isolated in #30 and is not silently decided by this refactor.

