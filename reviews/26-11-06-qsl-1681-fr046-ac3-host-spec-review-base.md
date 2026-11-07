---
id: SR-202611070800001
title: "base spec review of FR-046-AC-3 admitted-artifact host (QSL-1681, 89aef287f)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; FR-046-AC-3 and TC-127 Admitted-artifact host as changed by 89aef287f; fixture examples/checked-handoff/unit-duplicate-queries.native; test duplicate_queries_over_the_receipts_evaluate_from_the_handoff_files"
review_set: base
---

## Summary

The amended FR-046-AC-3 and the TC-127 "Admitted-artifact host" row make no false claim against the fixture or the test. Every conjunct of `Full` is an equality, so `Full` true over [2,2,3] asserts each stated value (filter size 2 and sum 4, map size 3 with two true, size 3, count 2, sum 7). `Partial` false over [2,2,3] and the inversion over [2,3] match the four asserts. The model's `Recovery.receipts` is `Amount`, ordered, non-unique, upper 5, as stated. The non-assertion of map/filter element order and of unit U is stated precisely, and the reason given matches the FR-533 section "Types the bytes cannot project". No new bytes-route call is introduced. There are two quality findings: the AC is not atomic, and one phrase claims a little more than the leg checks.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-046-AC-3 is not atomic. One row carries the compilation leg, the admitted leg, the fixture path, the Rust API name `BarePackage::evaluate_protocol_slot`, the per-slot conjunct inventory and the non-asserted scope. Fix: keep in the AC only the observable claim (admitted-artifact evaluation over [2,2,3] and [2,3] yields the Boolean results that require duplicate retention and size 3, count 2, sum 7; order and unit U are compilation-route only), and move the fixture path, API name and conjunct list to the TC-127 "Admitted-artifact host" section. Alternatively, split the admitted leg into its own appended AC. | FR-046-AC-3, TC-127 |
| FND-002 | low | "against the independently expected values above" overclaims. The compilation-route map value above is [2,2,3] (an identity map), but the fixture's map is the Boolean map `x = 2`, which yields three results with two true, and the filter sum 4 is not one of the "values above". Fix: say that the admitted leg checks size 3, count 2 and sum 7 from the values above, plus fixture-specific filter/map witnesses (filter size 2 and sum 4, Boolean map size 3 with count 2). | FR-046-AC-3, unit-duplicate-queries.native |

## Scope and evidence

No applicable enforced AssuranceProfile was found, so this is a direct base review of the single commit diff only. The fixture, the test helper `recover_receipts` / `recovery`, `examples/checked-handoff/model.semantic-ir.json` (`Recovery/receipts` multiplicity 0..5, ordered, not unique) and FR-533 "Types the bytes cannot project" (`MissingTypeRepresentation`) were read. No build or cargo run was done; the caller reports that the test passes. `Partial` false over [2,2,3] follows logically from `Full` true. Its value is as a non-constant control over [2,3], and that is consistent with the text. The commit adds no hash, digest or pin. Public text cites QSpec by ID only.
