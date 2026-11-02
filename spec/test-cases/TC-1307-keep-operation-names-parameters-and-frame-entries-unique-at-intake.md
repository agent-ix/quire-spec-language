---
id: TC-1307
title: "Keep operation names, parameters and frame entries unique at intake"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-512
    type: verifies
---
# TC-1307: Keep operation names, parameters and frame entries unique at intake

## Description

Verify the operation, parameter and frame checks of FR-512. Scope: FR-512-AC-1
to FR-512-AC-17.

## Test Procedure

Fixture `O0` is the TC-458 ConfigVersion package extended with object type
`Gauge` (fields `g`, `h`), object types `A` and `B`, event type `Ev`, record
value type `Rec`, and object `Counter` with operation `bump` (identity
`.../Counter/bump`).

| Row | Mutation of O0 | Expected | AC |
| --- | --- | --- | --- |
| 1 | an `operations` member on the population `config_history` | the reader's refusal of the member, retained; no declaration admitted | 1 |
| 2 | an operation on event type `Ev`; then the same operation on `Counter` | `invalid_model_binding`/`malformed-declaration` at the operation; then admitted | 2 |
| 3 | an operation on record value type `Rec` | `unsupported_construct`/`declaration-form` at the operation | 3 |
| 4 | a second `Counter` operation with identity `.../Counter/bump` | `invalid_model_binding`/`conflicting-binding` at `.../Counter/bump`, path `TypeDefinition.operations[1]` | 4 |
| 5 | a second `Counter` operation named `bump` with identity `.../Counter/bump2` | `ambiguous_declaration`/`ambiguous-name` at the second operation's `name`, naming both | 5 |
| 6 | `Sub.bump` with `redefines Counter/bump` | admitted | 6 |
| 7 | parameters `a`, `a` (one identity) | `conflicting-binding` at the shared identity, path `TypeDefinition.operations[k].params[1]`, reader `DUPLICATE_PARAM` retained, no other refusal | 7 |
| 8 | parameters named `a` with identities `.../a` and `.../b` | reader `DUPLICATE_PARAM` at `/ir/types/i/operations/k/params/1/name`; no intake refusal; no declaration | 8 |
| 9 | one parameter named `x` with identity `.../b` | `invalid_model_binding`/`malformed-declaration` at its `name` | 9 |
| 10 | parameter `result` on an operation with `returns` | `ambiguous_declaration`/`ambiguous-name` at its `name` | 10 |
| 11 | parameter `result` on an operation without `returns` | `ambiguous_declaration`/`ambiguous-name` at its `name` | 11 |
| 12 | `modifies [Gauge/g, Gauge/g]`; `modifies_receiver [Gauge/g, Gauge/g]`; `modifies [Gauge/g, Gauge/h]` | admitted, effects `[g]`, receiver `[g]`, and `[g, h]` | 12 |
| 13 | `creates [A, A]`; `deletes [B, B]`; `deletes_parameters [p, p]` | admitted, effects `[A]`, `[B]`, `[p]` | 13 |
| 14 | `creates [A]` with `deletes [A]`; then `modifies [Gauge/g]` with `modifies_receiver [Gauge/g]` | admitted; then `conflicting-binding` (FR-154-AC-4) | 14 |
| 15 | parameters `a`, `a` plus an unresolved `modifies` entry; a mismatched parameter plus the same entry; a `result` parameter plus the same entry | `conflicting-binding` only; `malformed-declaration` only; `ambiguous-name` only | 15 |
| 16 | `bump` repeated with distinct identities and a `result` parameter on the second; repeated name on `Ev`; repeated name on `Rec` | the operation-name refusal only; the owner refusal only; the owner refusal only | 16 |
| 17 | rows 4, 5, 7-11, 15 and 16 with unrelated records reordered in the document | the same refusals naming the same entries; the package of rows 12 and 13 equal under reordering | 17 |

Tag the test `#[trace("FR-512-AC-1", "FR-512-AC-2", "FR-512-AC-3", "FR-512-AC-4", "FR-512-AC-5", "FR-512-AC-6", "FR-512-AC-7", "FR-512-AC-8", "FR-512-AC-9", "FR-512-AC-10", "FR-512-AC-11", "FR-512-AC-12", "FR-512-AC-13", "FR-512-AC-14", "FR-512-AC-15", "FR-512-AC-16", "FR-512-AC-17", "TC-1307")]`.

## Expected Results

Each row's Expected column; a refusal admits no declaration.
