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
FR-515-AC-24. **R** is the package FR-517 emits from the ported IT-001 recipe.

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
| 18 | 18 | the `EvidencePackage` from the `Emission` vs the read-back package, each of **R**'s four clauses over its three packages; the rows held vs the `Emission`'s | equal `profile()`, `clock()`, `occurrences()`; rows byte-equal |
| 19 | 19 | a unit authored over the `config-version` model with two temporal clauses of different nodes and clock rows | each node's `clock()` is its own row, in both packages |
| 20 | 20 | a constructed V2 twin package (node `occurrences` 7 then 3, lock rows ascending 3 then 7, different clock configurations) read back through the reader; no `Emission` | `clock_of` returns the row keyed by each ordinal; `clock()` `None`; a position pairing fails |
| 21 | 21 | each `TemporalClauseRef` vs the read-back `temporal_clause_names` row; the `EvidencePackage`'s ids vs the read-back package's | equal ids; no id derivation in the API |
| 22 | 22 | in a unit authored over the `config-version` model, a clause with a trigger-reading guard and four `holds` atoms at indices 0 to 3 (capture; `over()` only; no binder; capture and `over()`); the first and second swapped | the derived set is `{0, 3}`, none for the guard, equal in both packages; after the swap `{1, 3}`; no caller-supplied set |
| 23 | 23 | the evaluator formula from each route for **R**'s clauses and AC-22's clause, from `formula_graph()`; `formula()` of AC-3's clause; a view with a cyclic or out-of-range `operands` | equal node for node; `formula()` as AC-3; the bad view is unreadable |
| 24 | 24 | in a unit authored over the `config-version` model, a `from_emission` package whose in-memory clause is altered in one operation, one interval and one operand edge, each in its own run; the unaltered clause; the `from_read` package over the same bytes | each altered run `clause-unreadable` at the first call reaching the clause; the unaltered clause evaluated; the `from_read` package evaluated |

Tag the test `#[trace("FR-515-AC-1", ..., "FR-515-AC-24", "TC-1310")]` for the criteria each row covers.

## Expected Results

Each row's Expected column.
