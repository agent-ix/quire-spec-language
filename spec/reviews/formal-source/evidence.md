---
id: SR-060
title: "evidence review of native formal source correspondence"
type: SpecReview
analysis: evidence
scope: "FR-014, docs/formal-source-binding.md, TC-035–039 and TM-003 addition"
review_set: all
evaluated_revision: "4eb4ef6"
review_date: "2026-09-08"
---

## Summary

Actual QUOIN advice agrees with the Test class for all five criteria; the concrete suite uses real IR constructors and bounded generation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Advisor granularity requires judgment: example-shaped integration cases and a finite generated oracle suite are selected; no snapshot file or E2E workflow is implied. | FR-014-AC-1..5 |

## Analysis

The recorded advise.json was produced by quoin advise --json. The initial sandboxed run could not detect the installed Quire version; a permitted local retry completed. Quire reports CLI 0.31.0 and engine 0.46.0. All five advice records have mismatch=false, uncatalogued=false and inconclusive=false.
    
AC-1 receives property-based-testing from the universal shape. TC-035 varies selected positive revisions and TC-039 supplies the independently generated correspondence family. AC-2..4 receive unit-testing and bdd-spec-by-example from example shapes. They are integration cases here because failure is measured across actual native and pinned IR source types. AC-5 receives metamorphic/property recommendations from round-trip shape plus golden-approval-testing from a serialization characteristic; the contract defines no wire serialization. By judgment, retain the generated oracle/round-trip test and explicit coordinate examples, not an invented serialized golden file.
    
Planned suite: tests/formal_source.rs, five trace-tagged tests for TC-035–039. This is not proof of all possible documents. Fuzzing is a possible later strengthening; no fuzz campaign is claimed. Loom does not model any concurrent mutation in this immutable API. No benchmark, concolic execution or resource-heavy campaign is needed to establish the stated bounded coordinate behavior. Tests run with one build job and one test thread at nice 10.

## Verdict and provenance

PASS for this specified bridge. Agent A applied the actual QUOIN base and all
seven selected analysis skills serially, with no subagents. Catalog contracts,
source/IR implementations and the scoped acceptance cases were inspected.
There is no applicable required AssuranceProfile. No implementation or full
state-workflow completion is claimed. The declined optional gap-analysis
semantic comparison remains excluded.

