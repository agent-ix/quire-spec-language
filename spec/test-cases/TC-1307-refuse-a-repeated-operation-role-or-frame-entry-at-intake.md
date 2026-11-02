---
id: TC-1307
title: "Refuse a repeated operation role or frame entry at intake"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-512
    type: verifies
---
# TC-1307: Refuse a repeated operation role or frame entry at intake

## Description

Verify the operation and frame checks of FR-512. Scope: FR-512-AC-1 to
FR-512-AC-6.

## Test Procedure

Fixture `O0` is the TC-458 ConfigVersion package extended with object type
`Gauge` (fields `g`, `h`), object types `A` and `B`, event type `Ev`, and
object `Counter` with operation `bump` (identity `.../Counter/bump`).

| Row | Mutation of O0 | Expected |
| --- | --- | --- |
| 1 | an `operations` member on the population `config_history` | the reader's refusal of the member, retained; no declaration admitted |
| 2 | an operation on event type `Ev` | `invalid_model_binding`/`malformed-declaration` at the operation |
| 3 | a second `Counter` operation with identity `.../Counter/bump` | `invalid_model_binding`/`conflicting-binding` at `.../Counter/bump`, path `TypeDefinition.operations[1]` |
| 4 | a second `Counter` operation named `bump` with identity `.../Counter/bump2` | `ambiguous_declaration`/`ambiguous-name` at the second operation's `name`, naming both |
| 5 | `Sub.bump` with `redefines Counter/bump` instead | admitted |
| 6 | parameters `a`, `a` (one identity) | `invalid_model_binding`/`conflicting-binding` at the shared identity, path `TypeDefinition.operations[k].params[1]`, and no other refusal |
| 7 | parameters named `a` with identities `.../a` and `.../b` | `invalid_model_binding`/`malformed-declaration` |
| 8 | parameter `result` on an operation with `returns`; then on one without | `ambiguous_declaration`/`ambiguous-name` at the parameter's `name`; then admitted |
| 9 | `modifies [Gauge/g, Gauge/g]`; `modifies [Gauge/g, Gauge/h]` | admitted with effect `[g]`; admitted with effect `[g, h]` |
| 10 | `creates [A, A]`; `deletes [B, B]` | admitted, effects `[A]` and `[B]` |
| 11 | `creates [A]` with `deletes [A]` | admitted |
| 12 | `modifies [Gauge/g]` with `modifies_receiver [Gauge/g]` | `invalid_model_binding`/`conflicting-binding` (FR-154-AC-4) |
| 13 | repeated parameter `a` plus an unresolved `modifies` entry | the `conflicting-binding` refusal only |
| 14 | parameter `result` with `returns` plus an unresolved `modifies` entry | the `ambiguous-name` refusal only |
| 15 | an event-type operation list with a repeated name | the owner refusal only |
| 16 | rows 3, 4, 6 and 8 with unrelated records reordered in the document | the same refusals naming the same entries; the package of rows 9 and 10 equal under reordering |

Tag the test `#[trace("FR-512-AC-1", "FR-512-AC-2", "FR-512-AC-3", "FR-512-AC-4", "FR-512-AC-5", "FR-512-AC-6", "TC-1307")]`.

## Expected Results

Each row's Expected column; a refusal admits no declaration.
