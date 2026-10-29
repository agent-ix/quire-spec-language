---
id: SR-070
title: "Native model and checker evidence review"
type: SpecReview
analysis: evidence
scope: "FR-015/016, FR-006 judgments, docs/native-model-checking.md, IT-005, TC-025–029/040–053 and TM-003"
review_set: all
evaluated_revision: "ceccabb564b61742597ad356eb1019ab0c8d1544"
review_date: "2026-09-08"
---

## Summary

Recorded QUOIN advice covers all 15 new criteria with no mismatch or inconclusive result. Additional bounded mutation and independent truth-table tests are explicit review judgments.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Advice alone does not qualify the new propositional proof layer; resolved by TC-053's independent Boolean oracle and mandatory positive controls. | FR-016-AC-1; FR-016-AC-7; TC-053 |
| FND-002 | low | Catalog characteristic matches require scoped judgment: temporal and precondition-bearing wording here describes static native checking, not a new temporal runtime or Rust compile-time contract. | FR-015-AC-5; FR-016-AC-2; FR-016-AC-3; FR-016-AC-5 |
| FND-003 | low | Fuzzing is recommended for model intake, but no fuzz harness/campaign is claimed; bounded invalid-role mutations provide the planned negative integration evidence. | FR-015-AC-2; TC-041 |

## Deterministic advice and judgment

data/advise.json is the actual completed quoin advise --json output; stderr is empty. It was refreshed for the corrected opaque-ID model contract. The subsequent evaluated revision only corrects matrix bookkeeping; the advised criterion statements are unchanged. All FR-015/016 records have mismatch=false, uncatalogued=false and inconclusive=false.

FR-015-AC-1 receives property-based-testing from a universal shape. Its independently inspected real producer and bounded variations qualify the mapping. AC-2 receives unit/example tests, negative-abuse-testing and fuzzing from untrusted-input/input-validation characteristics. TC-041 selects controlled valid-IR mutations first; a fuzz campaign is deferred and not needed to claim one was run. AC-3 also receives sca-sbom from supply-chain wording; this scope changes no dependency, and TC-042 checks actual artifact mutations/permutations. Existing dependency provenance remains applicable. AC-4/6 receive example/unit advice, strengthened by actual source and budget boundary controls.

FR-015-AC-5 and FR-016-AC-3 also receive model-checking/runtime-monitoring from a temporal characteristic. By judgment these static linking/arithmetic criteria use actual IR integration cases, without claiming a temporal model checker. FR-016-AC-2/5 receive design-by-contract from precondition-bearing wording; runtime compiler acceptance/refusal is measured through integration tests, not Rust type-level contract proof. Other FR-016 criteria receive example/unit advice.

Planned Rust suites are model/link integration tests for TC-040–045 and checker integration/property tests for TC-025–029/046–053. Trace attributes bind exact cases and criteria. Setup must succeed before an adverse checker assertion. The independent guard oracle evaluates its own small Boolean model over every assignment; accepted formulas cannot hide an absent-parent counterexample. Budget and artifact families remain bounded. Loom is inapplicable to this immutable serial API; no concolic, fuzz, mutation-tool or benchmark execution is claimed. Checks use nice 10, one Cargo job and one test thread, sequentially.

## Verdict and provenance

PASS for implementation of this specified scope. Agent A applied the actual
QUOIN base and all seven analysis skills serially, following the owner's
selected all review set. No subagents or builds were started for this review.
No applicable required AssuranceProfile was found. Catalog schema and existing
public interfaces were inspected. Implementation/test completion is not claimed.
The owner-declined optional gap-analysis semantic comparison remains excluded.

