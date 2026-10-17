---
id: TC-1055
title: "Snapshots and invocations carry set, bag and ordered-set values"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
---
# TC-1055: Snapshots and invocations carry set, bag and ordered-set values

## Description

Verify FR-106's `set`, `bag` and `ordered-set` value forms beside `sequence`:
the admitted value and its element order for each kind, raw comparison in the
frame check, duplicate refusal, the bound checked before any element, the
refusal of a form of another kind, an element type with no value form and the
producer's canonical element order.

Scope: FR-106-AC-50 to FR-106-AC-59.

## Preconditions

- The ConfigVersion model of TC-466 with an operation `probe` whose parameter
  `arg` and result carry the collection type each row names, and, for the rows
  that say object field, an object type whose field `tags` has that type.
  `Int9` is `Int[0, 9]`; `Set<Int9>[0, 2]` is a set with that inclusive bound.
- Rows 1 to 4, 6 to 15 and 21 are run two ways: as the `arg` of a `PreCall` selection, and as
  the `arg` and the `result` of an invocation. Rows 19 and 20 are run over the
  TC-465 package variant whose `ConfigVersion` has the field `tags` typed
  `Set<Reference<ConfigVersion>>[0, 5]`, with `root` and `child` of the healthy
  parent snapshot.

## Test Procedure

| # | Type | Input value | Expected |
| --- | --- | --- | --- |
| 1 | `Set<Int9>`, then again | `{"set":[{"integer":"2"},{"integer":"1"}]}`, then `{"set":[{"integer":"1"},{"integer":"2"}]}` | both admit; the elements of each are `1, 2` |
| 2 | `Bag<Int9>`, then again | `{"bag":[{"integer":"1"},{"integer":"2"},{"integer":"1"}]}`, then `{"bag":[{"integer":"1"},{"integer":"1"},{"integer":"2"}]}` | both admit; the elements of each are `1, 1, 2` |
| 3 | `OrderedSet<Int9>`, then again | `{"ordered-set":[{"integer":"2"},{"integer":"1"}]}`, then its reverse | admit with elements `2, 1`, then `1, 2` |
| 4 | `Sequence<Int9>` | `{"sequence":[{"integer":"1"},{"integer":"2"},{"integer":"1"}]}` | admits with elements `1, 2, 1` |
| 5 | raw values, in the frame check | `set` `1, 2` against `set` `2, 1`; `bag` `1, 1, 2` against `bag` `1, 2, 1`; `bag` `1, 1, 2` against `bag` `1, 2, 2`; `ordered-set` `1, 2` against `ordered-set` `2, 1`; `sequence` `1, 2` against `sequence` `2, 1`; `set` `1, 2` against `ordered-set` `1, 2` | equal; equal; unequal; unequal; unequal; unequal |
| 6 | `Set<Int9>`; `OrderedSet<Int9>`; `Bag<Int9>`; `Sequence<Int9>` | `{"set":[1,2,1]}`; `{"ordered-set":[2,1,2]}`; `{"bag":[1,2,1]}`; `{"sequence":[1,2,1]}` (each element spelled as an integer object) | `invalid_runtime_input`/`duplicate-member` at index 2 naming `earlier-index` `0`; the same at index 2 naming `0`; admits three elements; admits three elements |
| 7 | `Set<Int9>[0, 3]` | `{"set":[{"integer":"1"},{"integer":"1"},{"integer":"x"}]}` | `invalid_runtime_input`/`invalid-value` at index 2, not `duplicate-member` |
| 8 | `Set<Int9>[0, 2]`; `Bag<Int9>[0, 2]`; `OrderedSet<Int9>[0, 2]`; `Sequence<Int9>[0, 2]` | each holding three entries `1, 2, 3` (the bag `1, 1, 1`) | `cardinality_out_of_bound`/`above-maximum`, count 3, each time |
| 9 | `Sequence<Int9>[1, 3]` | `{"sequence":[]}` | `cardinality_out_of_bound`/`below-minimum`, count 0 |
| 10 | `Set<Int9>[0, 2]` | `{"set":[1,1,1]}`; `{"set":[{"integer":"1"},{"integer":"2"},{"boolean":true}]}` (integer objects) | `cardinality_out_of_bound`/`above-maximum`, count 3, not `duplicate-member` and not `wrong-value-kind`; the bound comes before any element |
| 11 | `Set<Int9>[0, 2]` | `{"set":[{"integer":"1"},{"integer":"2"}]}` | admits at the bound |
| 12 | `Set<Int9>`; `Set<Int9>`; `Sequence<Int9>` | `{"bag":[{"integer":"1"}]}`; `{"sequence":[{"integer":"1"}]}`; `{"set":[{"integer":"1"}]}` | `invalid_runtime_input`/`wrong-value-kind` at the value, each |
| 13 | `Set<Int9>`; `Set<Int9>`; `Bag<Int9>` | `{"set":{}}`; `{"set":[{"integer":"1"}],"count":"1"}`; `{"bag":[{"element":{"integer":"1"},"count":"2"}]}` | `invalid_runtime_input`/`wrong-value-kind` naming `arg`, each |
| 14 | `Set<Decimal[0, 100; 0, 2; exact]>` | `{"set":[]}` | `unknown_required_feature`/`unsupported-feature` naming `Decimal` as `feature`, before any element |
| 15 | `Set<UUID>` | two equal UUID objects; then two different UUID objects | `duplicate-member` at index 1 naming `0`; admits |
| 16 | the writer, `Set<Integer>` | the value `{9, 10}` | `{"set":[{"integer":"10"},{"integer":"9"}]}`: `10` before `9`, by bytes |
| 17 | the encoder, `Bag<Text[0, 8; nfc]>`; then `Sequence<Text[0, 8; nfc]>` | a bag of `U+0065 U+0301` twice; a sequence of `U+00E9` then `U+0065 U+0301` | two identical Text objects of `U+00E9`; the sequence keeps both payloads, `U+00E9` then `U+0065 U+0301` |
| 18 | the writer, `OrderedSet<Integer>` and `Sequence<Integer>`; `Bag<Integer>` | the values `2, 1`; `9, 10, 9` | written in the order `2, 1`; the bag as `10, 9, 9` |
| 19 | the `tags` field, in the object fixture | `child.tags` `{"set":[root, child]}` then `{"set":[child, root]}`, `root.tags` `{"set":[]}` | both admit as one value |
| 20 | the `tags` field | `child.tags` `{"set":[root, root]}`; then six entries; then `{"bag":[root]}`; then `{"sequence":[root]}` | `duplicate-member` at index 1 naming `earlier-index` `0`; `cardinality_out_of_bound`/`above-maximum` count 6; `wrong-value-kind`; `wrong-value-kind`, each naming `child` and `tags` |
| 21 | `Set<Text[0, 8; nfc]>`; `Bag<Text[0, 8; nfc]>` | `U+00E9` and `U+0065 U+0301` | `duplicate-member` at index 1 naming `0`; admits, both elements retaining `U+00E9` |

## Expected Results

- Rows 1 to 4 admit in every way a value is supplied, and the admitted
  collection holds its elements in the order the row states: key order for a
  set and a bag, array order for an ordered set and a sequence (FR-106-AC-50).
- Row 5 compares the raw values as the frame check does (FR-106-AC-51).
- Rows 6 and 7 refuse or admit as stated (FR-106-AC-52).
- Rows 8 to 11 refuse by bound before any element is read, and admit at the
  bound (FR-106-AC-53).
- Rows 12 and 13 refuse as `wrong-value-kind` (FR-106-AC-54).
- Rows 14 and 15 (FR-106-AC-55).
- Row 16 and the integer part of row 18 (FR-106-AC-56); row 17 (FR-106-AC-59).
- Rows 19 and 20 (FR-106-AC-57) and row 21 (FR-106-AC-58).
