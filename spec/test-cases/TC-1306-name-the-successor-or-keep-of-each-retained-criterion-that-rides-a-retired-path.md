---
id: TC-1306
title: "Name the successor or keep of each retained criterion that rides a retired path"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-511
    type: verifies
---
# TC-1306: Name the successor or keep of each retained criterion that rides a retired path

## Description

Verify the spine successors of FR-511. Scope: FR-511-AC-1 to FR-511-AC-14
(AC-15 is by inspection). Vectors are the source fragments below or are
referenced by QSpec case ID and never copied.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | ConfigVersion package; change one field bound / one frame entry / package revision / an unused declaration's bound | four different package byte strings, each different from the base |
| 2 | 1 | same package, operations and populations reordered | bytes equal to the base |
| 3 | 1 | unit `model M` naming identity `other` | `missing_import` |
| 4 | 2 | two documents with identity `p`, equal; then differing | `invalid_package`/`conflicting-definition` naming both, in both cases; no binding |
| 5 | 3 | `Positive` called from a function, an `invariant` and a protocol `check` | one callee node; three call terms with their own argument order, span, receiver |
| 6 | 3 | add a domain package declaring `Positive`; call `Positive(1)` | `ambiguous_declaration`/`ambiguous-name` |
| 7 | 4 | `predicate R using v(x: Boolean): Boolean { R(x) }`; `A` and `B` calling each other | `missing-measure` for each |
| 8 | 4 | the cycles of row 7 with a QSpec FR-146 accepted measure | admitted |
| 9 | 4 | `Positive(1, 2)`; `Positive(true)`; `predicate N using v(x: Int[0, 9]): Integer { x }`; `predicate D using v(x: Int[0, 9]): Boolean { 10 / x > 1 }` | `ill_typed` at the call; `ill_typed` at the call; assembler refusal at the result form; `undefined_expression`/`unproved-nonzero` at `10 / x`; each caller of a refused predicate refuses too |
| 10 | 4 | `D` with a guard form QSpec FR-146 accepts | admitted |
| 11 | 5 | queries over `[]` | size 0, contains false, forall true, exists false, filter `[]`, map `[]`, count 0, sum 0 |
| 12 | 5 | `[1, 1, <unavailable>]`: `contains` 1; `exists` x = 1; `forall` x = 2 | completed (decisive occurrence before the unavailable one), true, true, false |
| 13 | 5 | `[<unavailable>, 1]`: `contains` 1; `filter`; `map`; `count`; `sum` | incomplete unavailable each; no partial output |
| 14 | 5 | an invalid request (wrong binder type) with the invalid value last; the maximum-admitted length with duplicates; a nested input | refusal before any result; order and duplicates kept |
| 15 | 6 | a query with predicate expansion, nested traversal and a retained sequence: `accounting.work_units` at usage U, U - 1, 0; `accounting.result_units` at usage R, R - 1, 0 | completed; `Incomplete` at that setting; `Incomplete` at that setting; retry with a fresh meter equals the completed result and inputs are unchanged |
| 16 | 7 | emit row 5's and row 12's packages | call and query records keep handles, binder scopes, type and unit selections, captures; the report names `check` and `replay` evidence separately |
| 17 | 8 | the filament-core-data#173 fixture through spine intake and a `1-draft` unit selecting it as `model M` | ports resolve with part, direction, interface type, multiplicity; the connection admits (FR-152); references to field, port, connection bind (TC-148) |
| 18 | 9 | temporal selections: surplus entry; duplicate for one node; out of order; node of another package; out of range; profile and definition mismatch; wrong clock alternative | refusal of FR-050-AC-14's causes, no package |
| 19 | 9 | clock configs: empty name; 4097-byte name; period 0; period -1; period 2/4; period beyond i64; substituted epoch, unit, sequence authority | each `invalid_package`/`invalid-value` naming the field; none defaulted |
| 20 | 10 | a package with 3 temporal bindings; `identity.input_bytes` and `i2.input_bytes` at measured size, one less | admitted; refused naming the setting; retry with enough limits reproduces the bytes |
| 21 | 11 | a protocol with two event-triggered controls; emit, read back; then each mutation of the AC | unchanged round-trip; each mutation refused at read-back before a view is admitted |
| 22 | 12 | a choice guard `visible(x)` where `x` is a binder of another branch, a success-only effect binder, another role's value, a same-spelled name of another owner; each also only in a skipped operand | the FR-218 visible-entry typed cause for each, 8 refusals |
| 23 | 13 | emit a protocol unit; read with Contract IR's reader; then swap source identity; swap package identity; swap result identity; non-canonical re-serialisation | admitted; then four refusals |
| 24 | 14 | snapshot with a duplicate member, a surplus member, a foreign member, a wrong-type member, a duplicate storage key; deref to an absent target in a complete population; deref in an unavailable closure | five `invalid_runtime_input` refusals; `undefined`/`absent-key`; incomplete unavailable |

Tag the test `#[trace("FR-511-AC-1", ..., "FR-511-AC-14", "TC-1306")]` for the criteria each case covers; the ported tests also keep the legacy tags they succeed (FR-015-AC-1 to AC-4, AC-6, FR-046-AC-1, AC-2, AC-4, AC-7, AC-8, FR-056-AC-8, FR-050-AC-2, AC-3, AC-5, FR-054-AC-1 to AC-4, FR-048-AC-4, AC-9, FR-049-AC-2, AC-5).

## Expected Results

Each row's Expected column.
