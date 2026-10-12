---
id: SR-7853
title: "QSL-723 PR 765 runtime evidence alignment"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@58a47ac35e160b3772401c928efc56ae1719565a; TC-235 M02/M13/M14/M15/M22 runtime claims and removed semantics controls"
review_set: subset
---

## Summary

Plan completion: not assessed

Scoped manual requirement/test/source review. Runtime vectors carry QSpec trace tags, but several tagged assertions are authored expected artifacts rather than observations of the claimed production behavior. No computed matrix or build run in this dispatch.

## Verdict

FAIL for claimed runtime-vector completion. SR-7852 records production and evidence defects that must be corrected in PR 765.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | M13 manually enumerates values and path strings, M22 compares manually built path JSON, and M14 tests a missing object input rather than absence of event-invariant results. These tests do not substantiate the tagged runtime obligations and can remain green when the required traversal/location behavior is absent. | qsl-eval/tests/it/tc235_business_meaning_runtime.rs:706; qsl-eval/tests/it/tc235_business_meaning_runtime.rs:783; qsl-eval/tests/it/tc235_business_meaning_runtime.rs:1162 |
| FND-002 | high | New state-machine admission lacks coverage for an allowed postcondition pre(self.field), and new invocation path serialization is not exercised by an emitted-path/codec round-trip test. The actual evaluator keeps post receiver identity for pre reads and Object provenance for receiver field reads. | qsl-eval/src/value/expression/mod.rs:481 |
| FND-003 | high | Existing M01/M07 effective-identity, universe, accounting and typed refusal controls are removed or weakened during shared fixture extraction. This reduces previously present verification independently of new runtime selectors. | qsl-semantics/tests/it/tc235_business_meanings.rs:334 |
| FND-004 | medium | PR-body links and full-vector claims do not match the rebased source or open PR 777/781 dependencies. M02 is component equality only; M15 manually selects invariants rather than validating the automatic result inventory. | PR 765 body |

## Coverage

Examined FR-208-AC-4/10/11 and TC-235 M02/M13/M14/M15, FR-207-AC-8 and M22, and prior M01/M07 preservation. Context-only dependency inspection of open PR 777 and PR 781 confirms their assembly/String support is absent from this head. No matrix completeness, runtime acceptance or final execution-gate PASS claimed. No builds run by reviewer.
