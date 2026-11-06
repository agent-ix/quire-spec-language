---
id: TC-1308
title: "Name the successor or keep of each retained criterion that rides a retired path"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-513
    type: verifies
---
# TC-1308: Name the successor or keep of each retained criterion that rides a retired path

## Description

Verify the spine successors of FR-513. Scope: FR-513-AC-1 to FR-513-AC-15
(AC-15 is the matrix test of row 25). Vectors are the source fragments below or are
referenced by QSpec case ID and never copied.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | ConfigVersion package; change the `VersionNumber` field bound and an operation frame entry separately; then change only the source package `version` metadata | each semantic mutation changes the checked package bytes; the version-only mutation leaves them equal to the base. `ModelSelection` has no version, digest, or content pin |
| 2 | 1 | same package, operations and populations reordered | bytes equal to the base |
| 3 | 1 | unit `model M` naming identity `other` | `missing_import` |
| 4 | 2 | two documents with identity `p`, equal; then differing | `invalid_package`/`conflicting-definition` naming both, in both cases; no binding |
| 4b | 2 | a unit with two distinct aliases selecting identity `p`, and one supplied document for `p` | admitted; both aliases resolve against that same document, with no per-alias version or content comparison (alias coverage is also retained by the QSL-1530/QSL-1578 cases) |
| 5 | 3 | `Positive` called from a function, an `invariant`, a protocol `check` and a `temporal` clause | one callee node; four call terms with their own argument order, span, receiver |
| 6 | 3 | add a domain package declaring `Positive`; call `Positive(1)` | `ambiguous_declaration`/`ambiguous-name` |
| 7 | 4 | `predicate R using v(x: Boolean): Boolean { R(x) }`; `A` and `B` calling each other | `missing-measure` for each |
| 8 | 4 | the cycles of row 7 over `x: Int[0, 9]`, each with `decreases(x)` and recursive calls at `x - 1`; then `R` calling `R(x)` under `decreases(x)` | admitted; then `unproved-decrease` with obligation `decrease` |
| 9 | 4 | `Positive(1, 2)`; `Positive(true)`; `predicate N using v(x: Int[0, 9]): Integer { x }`; `predicate D using v(x: Rational[-10, 10; 1, 5]): Boolean { rational(10, 1) / x > rational(1, 1) }`; `function F using v(x: Int[-10, 10]): Rational[-10, 10; 1, 10] pure { 10 / x }` called by `predicate DI using v(x: Int[-10, 10]): Boolean { F(x) > rational(1, 1) }` (the integer divisor, promoted exactly under QSpec FR-147-AC-11; the legacy `10 / x > 1` has no expected rational type and is ill-typed) | `ill_typed` at the call; `ill_typed` at the call; assembler refusal at the result form; `undefined_expression`/`unproved-nonzero` at `rational(10, 1) / x`; `undefined_expression`/`unproved-nonzero` at `10 / x`; each caller of a refused predicate refuses too |
| 10 | 4 | `D` with the guard `x != rational(0, 1) and ...` (QSpec FR-146 rational nonzero fact), and its two callers; `F` as `if x != 0 then 10 / x else rational(0, 1)` (QSpec FR-146 integer nonzero fact), and `DI` | admitted; admitted |
| 11 | 5 | queries over `[]` | size 0, contains false, forall true, exists false, filter `[]`, map `[]`, count 0, sum 0 |
| 12 | 5 | `[1, 1, <unavailable>]`: `contains` 1; `exists` x = 1; `forall` x = 2 | completed (decisive occurrence before the unavailable one), true, true, false |
| 13 | 5 | `[<unavailable>, 1]`: `contains` 1; `filter`; `map`; `count`; `sum` | incomplete unavailable each; no partial output |
| 14 | 5 | an invalid request (wrong binder type) with the invalid value last; the maximum-admitted length with duplicates; a nested input | refusal before any result; order and duplicates kept |
| 15 | 6 | a query with predicate expansion, nested traversal and a retained sequence: `accounting.work_units` at usage U, U - 1, 0; `accounting.result_units` at usage R, R - 1, 0 | completed; `Incomplete` at that setting; `Incomplete` at that setting; retry with a fresh meter equals the completed result and inputs are unchanged |
| 16 | 7 | emit row 5's and row 12's packages | call and query records keep handles, binder scopes, type and unit selections, captures; the report names `check` and `replay` evidence separately: a check refusal is `disposition.stage` `compile` with no outcome or truth, an executed replay is `evaluate`; the reader refuses an `evaluate` result relabelled `compile` and a `compile` refusal turned into a success |
| 17 | 8 | the filament-core-data#173 fixture through spine intake and a `1-draft` unit selecting it as `model M` | ports resolve with part, direction, interface type, multiplicity; the connection admits (FR-152); references to field, port, connection bind (TC-148) |
| 18 | 9 | temporal selections: surplus entry; duplicate for one node; out of order; node of another package; out of range; profile and definition mismatch; wrong clock alternative | refusal of FR-050-AC-14's causes, no package |
| 19 | 9 | clock configs: empty name; 4097-byte name; period 0; period -1; period 2/4; period beyond i64; substituted epoch, unit, sequence authority; a selection naming another profile's definition identity (the clock alternative matching the clause) | each `invalid_package`/`invalid-value` naming the field; none defaulted |
| 20 | 10 | a package with 3 temporal bindings; `identity.input_bytes` and `i2.input_bytes` at measured size, one less | admitted; refused naming the setting; retry with enough limits reproduces the bytes |
| 21 | 11 | a protocol with two event-triggered controls; emit, read back; then each mutation of the AC | unchanged round-trip; each mutation refused at read-back before a view is admitted |
| 22 | 12 | a choice guard `visible(x)` where `x` is a binder of another branch, a success-only effect binder, another role's value, a same-spelled name of another owner; each also only in a skipped operand | the FR-218 visible-entry typed cause for each, 8 refusals |
| 23 | 13 | emit a protocol unit; read with Contract IR's reader; then swap source identity; swap package identity; swap result identity; non-canonical re-serialisation | admitted; then four refusals |
| 24 | 14 | two observation occurrences of one observation under distinct occurrence keys; snapshot with a duplicate member, a surplus member, a foreign member, a wrong-type member, a duplicate storage key; deref to an absent target in a complete population; deref in an unavailable closure | the two occurrences admitted; five `invalid_runtime_input` refusals; `undefined`/`absent-key`; incomplete unavailable |
| 25 | 15 | matrix test while the `v2` vocabulary, `v3` and `wire.rs` exist; then delete one module in a fixture tree without a successor test | the seven criteria are each tagged; after the deletion the matrix test fails naming the untagged criterion |

Tag the test `#[trace("FR-513-AC-1", ..., "FR-513-AC-15", "FR-513-AC-15", "TC-1308")]` for the criteria each case covers; the ported tests also keep the legacy tags they succeed (FR-015-AC-1 to AC-4, AC-6, FR-046-AC-1, AC-2, AC-4, AC-7, AC-8, FR-056-AC-8, FR-050-AC-2, AC-3, AC-5, FR-054-AC-1 to AC-4, FR-048-AC-4, AC-9, FR-049-AC-2, AC-5).

## Expected Results

Each row's Expected column.
