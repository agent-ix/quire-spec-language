---
id: SR-281
title: "Risk review of the runtime input schema"
type: SpecReview
analysis: risk-complexity
scope: "FR-024-AC-5 at f4679ef"
review_set: all
---
## Summary

The main risks are schema drift and callers treating structural success as
admission. The narrow amendment and executable boundary controls address both.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Risk register

| Requirement | Technical risk | Volatility | Drivers and mitigation |
| --- | --- | --- | --- |
| FR-024-AC-5 | Medium | Medium | Companion schema must follow private wire types; actual producer and every-record adverse tests detect shape drift. |

The schema is authored data, not compiler-generated or a second runtime decoder.
Its integer constraints cannot restore parser precision, and its graph-shaped
payload cannot prove referential validity. Both limits are explicit in the
contract; raw-token and cyclic-arena controls retain the reader's authority.
The failure-domain review is SR-277. This slice does not resolve the native
admission questions in #30 or claim the carried-forward ruling is discharged.
