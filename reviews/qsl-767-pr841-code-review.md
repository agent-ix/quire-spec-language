---
id: SR-9115
title: "Code review of quire-spec-language PR #841: QSL source inventory envelope"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@78a09cb4470660f0a4becac0e921e993be45745a; src/checking/source_inventory.rs, src/checking.rs, src/linking/composed.rs, qsl-package/src/checked_v2.rs, qsl-package tests, tests/it/source_inventory.rs, tests/it/config_version_spine.rs, Cargo.lock"
review_set: subset
---
# Code review of quire-spec-language PR #841

## Summary

Ticket: QSL-767. Code review with the rust-review lane folded in, scoped to
`git diff origin/main...HEAD` (4 commits, head 78a09cb4). The contract is
Contract IR `crates/quire-contract-model/src/source_obligation.rs` at
1227928d (the rev Cargo.lock now resolves).

Examined:
- `qsl_inventory` builds one `RequirementRecord` per `graph.requirements()`
  key. `node` is `key.node().as_bytes()`, `role` is
  `key.origin().role().as_str()`, `ordinal` is `key.origin().ordinal()`. All
  three come from the real `OccurrenceKey`; nothing is synthesized (examined)
- One `PendingObligation` per retained `Obligation` of every
  `DeclarationTypes`. `site` is projected field by field from the native
  `Site` (declaration, unit, `ExprId.0`, span start/end). `kind` is an
  exhaustive in-crate match over `ObligationKind` (`#[non_exhaustive]` does
  not bind in-crate). `PredicateTotal` keeps its `target` handle (examined)
- Identity collision: `ExprId.0` is an index into the unit's expression
  arena (`solver.rs` `site(at)` uses `unit.expressions()[at.0]`). Paired with
  the unit handle it is unique per expression, so two obligations at
  different sites never project to equal ids. Two kinds at one site
  (`Nonzero` + `ArithmeticRange`, `SumProjection` + `SumPrefixes`) differ by
  `kind`. Every obligation is created through `Solver::obligation`, which
  always carries `Some(expression)` (examined)
- Stop cause: `types.exhaustion()` is checked first and maps to
  `TypeWorkExhausted` with dimension, limit, prior, requested and site.
  Otherwise the first id of `declaration_ids()` (0..len, deterministic) with
  `disposition == Unfinished` is named. Rows of unfinished declarations stay
  in `items`. IR's envelope says `stop_reason: None` "does not prove that
  enumeration exhausted the owner scope", and the envelope has no count or
  Complete field, so the builder claims no census (examined)
- Mutations, by reading (no builds):
  - Drop requirement rows: test 2 asserts the reported (node, role, ordinal)
    list equals `graph.requirements()` keys, with at least 2 (caught)
  - Swap Requirement/Pending kinds: test 2 filters `RequirementRecord` rows
    and counts `PendingObligation` rows against the native obligation total;
    either swap empties one set (caught)
  - Lose the PredicateTotal target: test 1 requires exactly one row with
    `target == Positive` (caught for any target other than Positive's handle)
  - Exhaustion mapped to Unfinished, or Unfinished checked before
    exhaustion: test 3 (normalization 0) also leaves Fraction unfinished and
    requires `TypeWorkExhausted` (caught)
  - Unfinished mapped to exhaustion: test 4 requires
    `TypeAdmissionUnfinished { Pending }` with no native exhaustion (caught)
  - Complete claim: the IR type has no field for one. Dropping the stop
    cause is caught by tests 3 and 4
  - Collapse the sets: test 2 checks `items.len() == requirements + pending`
    and each set separately (caught)
- `SyntaxNamespace::declaration_ids()` is `pub`, like every other accessor
  on that public type. It borrows nothing (examined)
- Lock bump: `insert_domain_package_document` / `insert_dependency_package`
  now return `Result`. `git grep` finds the two production callers in
  `qsl-package/src/checked_v2.rs` (both mapped to the new
  `ImportViewRefusal::Evidence`) and test callers only. IR returns `Ok` for an
  equal repeat and refuses only different bytes under an occupied key, which
  the reader would refuse later anyway. `Evidence` maps to
  `Code::RuntimeInvariant`. The only other match on `ImportViewRefusal`
  (`qsl-replay/src/spine/lifecycle.rs`) picks `Read { Limit }` with `_ =>
  None`, which is correct for `Evidence` (examined)
- No IR type is copied into QSL. Cargo.toml files are unchanged (examined)
- Gate: `/home/peter/dev/worktrees/logs/qsl767-ci.log` ends `EXIT 0`, with
  all four `source_inventory::` tests `ok`. The log was written at 11:11.
  The final commit 78a09cb4 is from 09:39 and the worktree is clean
  (examined)
- rust-review lane: no `unwrap`, `expect` or panics in the builder. `usize`
  handles are passed through without conversion, and the iterators are
  bounded by retained records (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS.** Every row identity is projected from QSL's real records. Every
mutation the brief asked about is caught by a typed assertion.

Recorded, not reported:
- medium: the builder pairs any `CheckedGraph`, `TypeReport` and IR
  `SourceIdentity` without checking that they describe one source. The
  "complete source" test does this: its requirement rows come from a
  `function f` unit (source a/u), its obligations from a different composed
  source, all under a third identity "Inventory". Every row's `source` is
  what the caller asserts.
- medium: test 4 has one declaration, so it cannot tell "first unfinished"
  from "last" or "any".
- medium: pending-row `site` fields (unit, expression handle) are not
  asserted. Only the count and the PredicateTotal target are checked, so a
  mutation that drops `expression` would pass.
- low: no test reaches `ImportViewRefusal::Evidence`.
- low: a declaration refused at binding is never solved. It adds no
  obligations and no stop cause, by design.
