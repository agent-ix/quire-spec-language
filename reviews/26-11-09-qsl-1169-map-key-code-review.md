---
id: SR-20261109020843
title: "code-review and rust-review of QSL-1169 package map key types, bounds and key-selected clock places"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; git diff origin/main...qsl-1169-qsl-intake (434869e86..1038e4e6c): qsl-semantics intake/assemble/lowering/timed_clause/refusal, qsl-eval domain/symbolic"
review_set: subset
---

## Summary

Ticket: QSL-1169 (QSL half). The branch reads a package map type's `keys`, `values` and `bound` (QSpec FR-154 "Map types"), refuses ineligible key types and malformed or unpreservable bounds, lists key-selected clock places under a map (FR-230 "Clock collections", AC-19, AC-62) and counts bounded map domains (FR-120, TC-1270). Intake, refusal and place-listing logic is sound and its unit tests pass (33 map-filtered lib tests green with `test-support`). Two spec conformance defects and one resource-bound defect remain.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A map keyed by native `Text` (the FR-154/FR-230-AC-19 canonical key) no longer assembles: `element(&map.keys)` goes through `model_value_type`, which returns `Unmapped::Untyped` for bare `Text`, so a field typed `Map<Text, V>` becomes an untyped field | qsl-semantics/src/check/assemble.rs:2401, qsl-semantics/src/check/assemble.rs:2294 |
| FND-002 | medium | A map slot's selector falls back to the `Integer` node for admitted key types with no builtin scalar form (unit, enumeration, Float32/Float64), and for a package scalar or alias key references the builtin node rather than the key type node; FR-230 requires the selector to reference the key type node, never `Integer` | qsl-semantics/src/check/lowering/timed_clause.rs:633-636, qsl-semantics/src/check/lowering/model.rs:1677-1692 |
| FND-003 | medium | `entries_size` iterates every entry count from `min` to `max` with an O(count) `power` per step, although every term above the key count is 0; a declared bound such as `[0, 10000000]` or `upper: u64::MAX` (accepted by intake) hangs model-system construction | qsl-eval/src/simulation/model/domain.rs:706-712 |
| FND-004 | low | `key_eligibility` admits every package `ScalarValueDomain::Rational` (including `domain: None`) and a scalar of `Native(Rational)`; FR-154 admits only a bounded `Rational[n1, n2; d1, d2]` and refuses the native `Rational` | qsl-semantics/src/model/intake.rs:6540-6551 |
| FND-005 | low | Refusal order differs from FR-154: `values` presence is checked before `keys` form, and key eligibility runs in a post pass only when no other refusal exists, so a map with an ineligible key and a malformed `bound` reports only the bound | qsl-semantics/src/model/intake.rs:5340-5352, qsl-semantics/src/model/intake.rs:6491 |

## Verdict

Changes requested. FND-001 breaks the main case of the feature at assembly and is masked by the known-red `clock_places::a_clock_collection_field_parameter_and_result_check_with_their_bounds` (the semantic-IR validator refuses first). FND-002 contradicts FR-230-AC-62's selector rule for non-Text keys. FND-003 is a bounded fix (clamp `max` to `min(max, keys)`). Clean: `read_map_bound` (PM-21..PM-28 incl. lower == upper edge), extra-member checks, `UnpreservedModelMeaning` wiring in code/cause/refusal_node and fixtures, map place listing and `[key]` naming, `CollectionShape.map` threading, removal of the `.expect("a leaf key")` panic in assemble. Rust style: no new panics on input, closed-enum matches exhaustive, docs present. quoin gap-analysis was skipped (not cheap for this diff); the AC-to-test check was done manually on FR-154-AC-48/49, FR-230-AC-19/62 and FR-120-AC-28.

## Scope and evidence

- QSpec FR-154-AC-48 (quire-specification spec/functional/type-model/FR-154-admit-domain-package-model.md): map type keys/values/bound admitted, key types Integer, Int[0,9], payload-free variant, unit, alias of Text admitted. Examined; tests at intake.rs `a_map_type_reads_its_keys_values_and_bound`, `only_a_scalar_key_type_keys_a_map`.
- QSpec FR-154-AC-49: each single defect refuses once at `M`. Examined; tests `a_malformed_map_type_refuses_at_the_member`, `a_map_bound_that_cannot_be_preserved_refuses_unpreserved_model_meaning`, `a_non_map_type_declaring_bound_or_keys_refuses`.
- FR-230-AC-19 (spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md): places including `Call.m.[key].x` with Map shape [0, 2]. Examined; `a_package_map_of_clocks_lists_a_key_selected_place`.
- FR-230-AC-62: selector references the `Text` key type node, not `Integer`. Examined (FND-001, FND-002); ref/deref/place-id parts out of scope for this PR.
- FR-120-AC-28 / TC-1270: map size sum_k C(|K|,k)|V|^k. Examined (FND-003); `a_bounded_map_counts_its_entries_by_key_subsets_and_values`.
