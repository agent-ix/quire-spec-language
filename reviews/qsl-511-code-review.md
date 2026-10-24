---
id: SR-12167
title: "Code review of quire-spec-language PR #1008: S4 lowering and v2 emission of temporal clause bodies"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@8484189f18ba077ecf506179f72d56337f9c1289; PR #1008 diff against merge-base 9a8e7803c: qsl-semantics/src/check/{assemble,check,lowering,mod,region,temporal_clause}.rs; qsl-semantics/src/check/lowering/temporal.rs; qsl-semantics/src/check/temporal_clause/{condition,tests}.rs; qsl-semantics/src/check/node_key/mod.rs; qsl-semantics/src/value/member.rs; qsl-semantics/tests/it/temporal_fairness.rs; qsl-package/src/checked_v2.rs; qsl-package/src/checked_v2/clause_root.rs; qsl-package/src/lib.rs; qsl-package/src/emit/tests/{admission_corpus,temporal_clause}.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1008

## Summary

Ticket: QSL-511. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` (18 files, +1825/-82). Read-only: nothing
was built or run, at the dispatcher's instruction. QSpec FR-370 read at its
current main, cited by id.

Soundness questions from the brief, each traced through the code:

1. **Identity collisions.** The clause node's body carries the profile law,
   a reference to the `over` parameter node (name and type), the clock text,
   the `kind` binding, the captures aggregate, one reference per fairness
   member and a reference to the formula root. Each fairness node's member
   is `{fairness_kind, granularity, declaration, name}` in resolved form,
   and `Member::declaration()` returns the declaring node, so it enters the
   dependencies. `checked_fairness` resolves an unmarked constraint to
   `weak whole` before the dedup, so unmarked and `whole` give one member and
   one node, and `each` gives another (FR-123-AC-1). Every formula node
   carries its own identity and its `temporal_interval` member (`null` for
   unbounded), connectives and constants carry none. The clause name enters
   no key. No path was found where two clauses FR-337 or FR-123-AC-1 say must
   differ get one key, or two that must be equal get two. Fairness order is
   source order of first occurrence, which FR-123 and FR-337 ("in
   fairness-set order") state, so a permuted fairness list giving a
   different key is the specified behaviour.
2. **Unchecked constructs.** `is_emitted` admits only `ActivationForm::Origin`
   with no captures. `when` exists only inside `ActivationForm::Each`, so it
   is covered. A clause with `on each` or a capture is refused `NotEmitted`
   before its conditions are typed or lowered. Tested for both.
3. **Identity drift of non-temporal packages.** Every new path runs only when
   the unit has a temporal clause, and before this PR every temporal clause
   that checked was refused, so no previously emitted package held one.
   `Operator::Temporal` was never used before (its `semantic_form` spelling
   change is unreachable from the generic expression path, and the clause
   node passes `"temporal_clause"` explicitly). The new `Member` variants add
   wire arms only. The `region.rs` change is equivalent for every name not in
   `site_spans`, which is empty without temporal clauses. The admission corpus
   only adds a fixture and moves three rows out of `NOT_EMITTED_BY_QSL`. No
   golden id or vector changes. No drift.
4. **Spaced site names.** `temporal {i}`, `temporal {i} over`,
   `temporal {i} node {n}`, `temporal {i} holds {n}`, `temporal {i} fair {k}`
   are distinct from each other, from declared names and model aliases
   (identifiers, no space) and from the other spaced producers
   (`bound {label}`, `protocol {name}`). No collision today. See FND-002.
5. **Hostile input.** `temporal_formula` keys the arena in one reverse loop,
   not by recursion, so formula depth does not reach the stack. Conditions
   are typed by the shared `Typer` under `CheckingLimits`, with the running
   `nodes_used` threaded through. Fairness members are bounded by the
   written constraints. Lowering inserts go through the existing node and
   work budgets.

FR-326 `temporal-satisfaction` record: FR-326 "Requirement record" requires
it, and ADR-012 §13.5 requires one record per claim site. It is keyed by the
clause node and its `claim` occurrence, its extent is the S3 claim's
(`Unbounded` with the infinite-trace domain keyed by the clause node, plus
the `over` parameter's domains), and the model references are the objects
the conditions read. Correct and necessary, not an unsafe addition. It is
untested at the package level; see SR-12168 FND-001.

`clause_root.rs`: the `over` node is read from the package's own argument 0,
never from the caller (FR-337 "Behavior"); role and ordinal are both checked.
Correct.

Rust lane: no `unsafe`, no `unwrap`/`expect` in production code, no
integer narrowing at a boundary, every invariant break is an `InternalFault`
stage fault, and an emitted clause the lowering could not key is a fault,
never an omission.

## Verdict

PASS. No major findings. Three low findings, recorded, not blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `self` inside a `holds` condition refuses `ill_typed`/`type-mismatch`, not `missing_declaration`/`missing-name`. `check_conditions` enters a state context with `self_slot: NO_SLOT` (`usize::MAX`), so `self_reference` finds a state and `bound_slot` fails with `mismatch`. `self_reference`'s own doc says an unbound `self` refuses `missing-name`. `protocol_obligation.rs` has the same pattern. | qsl-semantics/src/check/temporal_clause/condition.rs:76; qsl-semantics/src/check/check.rs:2557-2562 |
| FND-002 | low | The temporal sites are string-encoded in `Origin::TypeDeclaration` because quire-semantic-value has no temporal-clause origin. They are collision-free only because every other producer happens to use an identifier or a different prefix. No test pins that. They also take part in `enclosing_declarations`' declared-name anchor pass, so a generated occurrence's placement depends on the lexical order of `temporal ...` against declared names. A typed origin variant upstream would remove both. | qsl-semantics/src/check/lowering/temporal.rs:65-80; qsl-semantics/src/check/lowering.rs:4694-4700 |
| FND-003 | low | `resolve_clause_root` scans every graph node and compares hex strings for each lookup. QSL-880's closure checker resolves one triple per certificate key, which makes it O(keys x nodes). An index by node id, built once per package, would make it O(keys log nodes). | qsl-package/src/checked_v2/clause_root.rs:118-131 |
