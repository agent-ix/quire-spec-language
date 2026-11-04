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
FR-515-AC-17. **R** is the package FR-517 emits from the ported IT-001 recipe.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | emit each of the three packages of **R**, read each with `read_checked_package`; `temporal_subject` for each of `Due`, `DueLate`, `DueSample`, `DueTimestamp` | each accessor equals the value the test reads from the node, lock or `temporal_bindings()` row; `clock()` is the row of that declaration |
| 2 | 2 | clauses `on origin`; `on each (q: T) when (g)`; `on each (q: T)` | `Origin`; `Each` with trigger and guard nodes; `Each` with no guard |
| 3 | 3 | `Due`: `always[0,30] holds(Allowed(view.ratio))`; a formula with three `holds` atoms | `always` with `[0, 30]` then `holds`; one leaf; three leaves in pre-order |
| 4 | 4 | `checked_predicate` of the `Due` leaf | kind `HoldsLeaf`; `binders()` = `[view]`; span inside the formula root's source regions |
| 5 | 5 | the id of a `protocol_clause`; an id not in the package; a leaf of another clause; an integer-typed node | `UnresolvedClause`; `UnresolvedClause`; `LeafNotInClause`; `LeafNotBoolean`; no partial view |
| 6 | 6 | two packages emitted from the same source; `belongs_to` | true for the source package, false for the other |
| 7 | 7 | two structurally equal clauses under an infinite-trace profile; a changed interval; an added comment | one node, two occurrences with different ordinals; `formula()` and the node id change; the comment changes neither |
| 8 | 8 | compile-fail: construct either view outside the module; call `digest()`, `bytes()`, `identity()`, `package_digest()` and a closure-taking method | none compiles |
| 9 | 9 | compile-fail `use` of each of the two module paths | neither resolves |
| 10 | 10 | a guard `g` that is no `holds` operand; `checked_predicate` of `g` | `ActivationGuard`; `g` absent from `predicate_leaves()` |
| 11 | 11 | a guard equal to a `holds` operand (one node); `checked_predicate` of it | `ActivationGuard`; the node present in `predicate_leaves()` once per reading `holds` atom |
| 12 | 12 | a leaf the clause reads whose source map entry is removed | `LeafHasNoSource`; no partial view |
| 13 | 13 | two structurally equal clauses under a clock-configured profile | one node, two occurrences, one clock row per occurrence, admitted |
| 14 | 14 | a reader that implements IR-1278; `name()` | the declared name, equal to the sidecar's; gated on IR-1278 |
| 15 | 15 | a clause named by no protocol `requirements` entry; a clause named by one | empty; the protocol clause's node id |
| 16 | 16 | `checked_predicate` of an activation guard | `source_span` is an entry of the guard node |
| 17 | 17 | `checked_predicate` of a `ClauseLeaf` that is neither a `holds` operand nor the guard | `source_span` inside the formula root's source regions |
| 18 | 18 | the in-memory package before emit vs the read-back package, each of **R**'s clauses | equal `profile()` and `clock()` |
| 19 | 19 | two clauses of different nodes and clock rows | each node's `clock()` is its own row, in both packages |
| 20 | 20 | twins of one node, ordinals 7 and 3 listed in that order, different clock configurations | `clock_of` returns the row keyed by each ordinal; `clock()` `None`; same in both packages |

Tag the test `#[trace("FR-515-AC-1", ..., "FR-515-AC-20", "TC-1310")]` for the criteria each row covers.

## Expected Results

Each row's Expected column.
