---
id: SR-259
title: "Evidence review of the native sequence ceiling"
type: SpecReview
analysis: evidence
scope: "FR-015 sequence-ceiling amendment; TC-041; TC-065; Task-034"
review_set: all
---
## Summary

Ran quoin advise and read the installed method catalog. Advice failed to detect the installed Quire version; method selection below is explicitly author judgment grounded in that catalog, not an advisor verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Automatic method recommendation is unavailable due to Quoin's CLI-version detection failure. Boundary integration and negative-abuse methods are selected by judgment for this amendment. | FR-015-AC-2; TC-041 |
| FND-002 | medium | Trace binding does not validate the matrix's authored status labels. | TM-003; compiler #28 |

## Selected evidence

Catalog methods integration-testing and negative-abuse-testing fit the two public admission routes and independently constructed adverse IR. Three new Rust tests exercise 21 declaration scenarios: 7 accept the exact 10,000 maximum and 14 refuse 10,001/u32::MAX across field/value/wrapper locations. All three failed on the missing guard before implementation; these are observed red/green controls, not a claimed mutation-testing campaign. TC-065 keeps the 8,388,608 Unicode and 1,000,000 work ceilings. Broader property-based-testing/fuzzing remains later assurance; no new concurrent mechanism calls for Loom. SR-263 records actual checks and retained ignored assurance cases.

