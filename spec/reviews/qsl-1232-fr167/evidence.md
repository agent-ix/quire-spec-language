---
id: SR-117433
title: "QSL-1232 evidence analysis"
type: SpecReview
analysis: evidence
scope: "FR-167-AC-6, FR-168-AC-14/15 and FR-169-AC-17"
review_set: all
---

## Summary

Each new criterion names a focused test case. The cases exercise stored quotient states and edge permutations, both reduced claim outcomes plus pre-check refusals, and the terminal mapping without a certificate check.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No evidence-shape mismatch found; new criteria remain untagged until the code stage implements their tests. | FR-167-AC-6; FR-168-AC-14/15; FR-169-AC-17; TC-592; TC-593; TC-594 |

## Evidence map

| Criterion | Test case | Evidence required |
| --- | --- | --- |
| FR-167-AC-6 | TC-592 | 27 concrete versus 18 quotient states, canonical targets, retained permutations and binding-specific labels |
| FR-168-AC-14 | TC-593 | Symmetry and POR `HoldsReduced` results with binding stabilizer, concrete paths/replay, and positive POR state/edge reduction |
| FR-168-AC-15 | TC-593 | Symmetry and POR `Trapped` counterexamples replayed over the unreduced closure |
| FR-169-AC-17 | TC-594 | Exact reduced proof basis and `Uncertified` label; replay for SG-1 paths, no replay for path-free SG-2 |

The computed matrix over the changed FR files reports the new criteria as `untagged`, with no source binders. This is expected for the spec-first stage: the acceptance criteria identify concrete TCs, and the subsequent implementation stage must add source tests tagged to these ACs. The full-repository matrix did not finish within three minutes and was stopped; the focused matrix completed in under a second.
