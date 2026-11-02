---
id: TC-1310
title: "Read the checked predicate and the temporal subject from a checked package"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: verifies
---
# TC-1310: Read the checked predicate and the temporal subject from a checked package

## Description

Verify the two checked-package views that succeed the retired
`checked_predicate` and `temporal_subject` modules. Scope: FR-515-AC-1 to
FR-515-AC-9. **R** is the package FR-517 emits from the ported IT-001 recipe.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | emit **R**, read it with `read_checked_package`; `temporal_subject` for each of its three temporal clauses | each accessor equals the value the test reads from the node, lock or `temporal_bindings()` row; `clock()` is the row of that declaration |
| 2 | 2 | clauses `on origin`; `on each (q: T) when (g)`; `on each (q: T)`; `checked_predicate` on each guard | `Origin`; `Each` with trigger and guard nodes; `Each` with no guard; `ActivationGuard` for the guarded one; guard not in `predicate_leaves()` |
| 3 | 3 | `Due`: `always[0,30] holds(Allowed(view.ratio))`; a formula with three `holds` atoms | `always` with `[0, 30]` then `holds`; one leaf; three leaves in pre-order |
| 4 | 4 | `checked_predicate` of the `Due` leaf | kind `HoldsLeaf`; `binders()` = `[view]`; span inside the clause span |
| 5 | 5 | the id of a `protocol_clause`; an id not in the package; a leaf of another clause; an integer-typed node | `UnresolvedClause`; `UnresolvedClause`; `LeafNotInClause`; `LeafNotBoolean`; no partial view |
| 6 | 6 | two packages with a clause of the same name; `belongs_to` | true for the source package, false for the other |
| 7 | 7 | two structurally equal clauses; a changed interval; an added comment | one node, two occurrences with different ordinals; `formula()` and the node id change; the comment changes neither |
| 8 | 8 | compile-fail: construct either view outside the module; call `digest()`, `bytes()`, `identity()`, `package_digest()` and a closure-taking method | none compiles |
| 9 | 9 | `git grep` for the two module paths on the head of the deleting change; run row 1 on that head | no match; row 1 green |

Tag the test `#[trace("FR-515-AC-1", ..., "FR-515-AC-9", "TC-1310")]` for the criteria each row covers.

## Expected Results

Each row's Expected column.
