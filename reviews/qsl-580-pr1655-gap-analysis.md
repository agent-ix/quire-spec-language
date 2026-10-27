---
id: SR-1655002
title: "Gap analysis of quire-spec-language PR #1655: record clock collections (580j)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2f321e12ed2e001ac34909a5edd91208f78903ef; PR #1655 against FR-230 Clock collections, Element identity, AC-19, AC-21 (identity parts), AC-23 (record parts) and TC-685; QSpec FR-154 (package map and sequence types, AC-27), FR-322-AC-109 and FR-415 (context); FCD semantic-ir d8cb065 schema (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Gap analysis of quire-spec-language PR #1655

## Summary

Ticket: QSL-580, slice 580j. Plan completion: not assessed.

The tests cover the record parts of AC-19 and AC-23: the four collection
shapes, the path, the bound, the dimension and the unbounded `None`. They
cover the identity part of AC-21: `size`, `e.n`, and the refusals of `e.x`,
`e`, `=` and `contains`. At qsl-eval they cover seven places, slot liveness,
the digest by slot, stepping and `UnboundedCollection`.

These deferrals are correctly routed:
- the `Clock`-element collections, to IR-1036;
- `collection_bounds` typing and package identity, to STD-493 item 9;
- selection, quantifiers and resets by selection, to 580k;
- collection-valued writes, to 580l.

**Map.** The claim that "the domain package has no map type kind" is wrong at
the FCD and QSpec level; see FND-001.

## Verdict

**CONDITIONAL.** FND-001 and FND-002 are medium: one is buildable work
reported as blocked, the other an untracked deferral of specified behaviour.
FND-003 and FND-004 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `Map<Text, Timer>` is a QSL intake gap, so it can be built now. It is not an FCD or a QSpec gap. The FCD IR schema at the locked d8cb065 lists `map` as a TypeDefinition `kind` and requires `values` for it (with `sequence` requiring `items`). QSpec FR-154 reads a package map as `Map<Text, …>` ("a package map is keyed by `Text`"), and FR-154-AC-27 and FR-322-AC-109 (type `CM`) exercise map `values`. QSL's `DomainPackageRecord` has no sequence or map type record, and no QSL code reads either kind (nothing implements FR-154-AC-27), so intake has nothing to build `m` from. A map whose `values` is the record `Timer` is not a Clock position, so IR-1036 does not block it. Only `Clock` as map `values` does. Fix: read package `sequence`/`map` type definitions into records, either here or in a dedicated slice owning AC-19's `m`, AC-23's sequence and map types and AC-17. Correct the PR body and the test doc, which say the package cannot declare a map. | FR-230-AC-19, FR-230-AC-23; quire-specification FR-154 (lines 193-195), FR-154-AC-27, FR-322-AC-109; filament-core-data schema/semantic/v1/semantic-ir.schema.json (TypeDefinition kind `map`, `values`) |
| FND-002 | medium | The unbounded collection is refused, though FR-230 admits it. FR-230 says a collection with no declared maximum "SHALL record `max: None` and add no static bound, and its live places are bounded in analysis by the model checker's `max_clocks` limit (QSpec FR-415 'Cardinality bound')". qsl-eval refuses the whole timed subject (`UnboundedCollection`) instead. QSL has no `max_clocks` setting yet (no code names it). The refusal is honest, but no ticket owns the specified behaviour. Track it, with the FR-255 limit registration, or bound the live slots by a `max_clocks` limit. | FR-230 "Clock collections"; qsl-eval/src/simulation/model/timed.rs:404-412 |
| FND-003 | low | Spec gap. AC-19's `m: Map<Timer, Int>` refusal (`ill_typed`/`operator-ineligible` at the key type) cannot be reached. Package maps are keyed by `Text` (FR-154), and QSpec's shared `type-ref` grammar has no `Map`, so neither a package nor a unit can declare a clock-bearing key. File on STD-493: drop the clause or say where such a key can be written. | FR-230-AC-19, FR-230 "Clock collections" |
| FND-004 | low | Edge coverage. `!=` over a clock collection and `convert` are named in FR-230 "Element identity" but untested; the `!=` probe refuses as expected. The `filter(…) = filter(…)` case is SR-1655001 FND-001. At qsl-eval only a `Sequence` is exercised, not the slot listing and liveness of a set or bag, nor a nested collection (a record in a collection inside a record) whose `bound` is a product. | qsl-semantics/tests/it/clock_places.rs:533-632; qsl-eval/tests/it/timed_subject.rs |

## Dispositions

Round 1, head f5f9dfedf (fix commit fcbfb29ef).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | moved-to-580m | The coordinator moved this to 580m. QSL intake of package `map` and `sequence` type definitions (FCD `kind: map` with `values`, QSpec FR-154-AC-27) is built there. It is recorded as moved, not still open. |
| FND-002 | moved-to-580m | The coordinator moved this to 580m. Bounding the live places of an unbounded clock collection by `max_clocks` (FR-230 "Clock collections", QSpec FR-415) replaces `UnboundedCollection` there. |
| FND-003 | deferred | Filed as STD-493 item 26 (2026-10-27 03:17): no input can declare a clock-bearing `Map` key, because package maps are keyed by `Text` and the `type-ref` grammar has no `Map`. |
| FND-004 | fixed | `a_record_collection_reads_its_data_and_refuses_value_comparison` now covers `!=`, `filter(..) = filter(..)` and `convert`. `convert` refuses at the target type form `Set<Config::Timer>[0, 3]`, which is a reasonable place, because the conversion's element comparison is named there. `a_collection_in_a_collection_bounds_its_place_by_the_product` asserts that `hs.[slot].ts.[slot].x` has two slots and a bound of 6. `a_set_or_bag_of_records_holds_a_clock_per_held_slot` covers a set and a bag with 2 of 3 slots live at qsl-eval. |
