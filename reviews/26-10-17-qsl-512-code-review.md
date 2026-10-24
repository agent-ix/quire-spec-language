---
id: SR-12083
title: "Code review of quire-spec-language PR #938: abstraction relation at S3, frame binding and per-item export"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@40d04ade61ce19e6e316dd35e7aacf1f9cd969d9; PR #938 (git diff origin/main...HEAD, 13 files): qsl-semantics/src/check/abstraction.rs, qsl-semantics/src/check/references.rs, qsl-semantics/src/check/claims.rs, qsl-semantics/src/check/claims/tests.rs, qsl-semantics/src/check/state_clause.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/tests/support/abstraction.rs, qsl-package/src/checked.rs, qsl-package/src/export.rs, qsl-package/src/export/tests.rs, qsl-package/src/lib.rs, spec/functional/FR-307-export-the-bindings-each-item-references.md"
review_set: subset
---
# Code review of quire-spec-language PR #938

## Summary

Ticket: QSL-512. Code review with the rust-review lane folded in. Read-only
review, no builds; the author's gate run is taken as reported.

The S3 checker (`CheckedGraph::check_abstraction`) applies FR-450 rules 1, 2,
4 (partly) and 5 in source order with typed refusal variants
(`AbstractionRefusal`, `MalformedEntry`); causes come from the variant, not
from prose. A key naming a declaration of another kind refuses
`malformed-declaration` through `declaration_keys`. Inherited operations
refuse at the subtype (`declaring != context`). `CheckedAbstractionRelation`
is constructible only inside `abstraction.rs`. `AbstractionKey`'s derived
`Ord` gives object type, population, operation frame, then key, which matches
the FR-307 order the PR adds; the export collects into a `BTreeSet`, so its
output is deterministic. Every `CheckedPackage` and `ValueClaim` constructor
was updated. No new `CheckCause` or `AssemblyCause` variant is added, so the
qsl-replay `refinement.rs` classifier needs no new arm. `read_object` is an
exhaustive match under `deny(clippy::wildcard_enum_match_arm)`. No `NodeKey`
or `EffectiveId` is minted (frame nodes are copied from
`CheckedOperationFrame`). No new panics or indexing on production paths.

## Verdict

Changes requested: one medium finding. A value claim does not reference the
object types read by a `let`-bound value it uses, so equivalent bodies
export differently and a bound item can omit a binding it needs.

Not recorded as findings (style only): in `lowering/model.rs`,
`record_keys` was inserted between `population_domains` and its doc comment,
so that doc now attaches to `record_keys`. In `abstraction.rs:469`, a failed
`Identifier::new` of an omitted parameter is reported as a user
`missing-name` refusal when it is an internal invariant.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Model-object references do not flow through `let`. `leave` adds a `Local`'s let-bound reads (`roots.slots`) to the claim's `reads`, but `objects` only gathers `read_object` over the claim node's own subtree. `Let` stores only `first_reads` for the slot, not `first_objects`. So `function f using v(c: M::ConfigVersion): Integer pure { let x = deref(c).version in x + 1 }` keys a value-validity record whose `references.objects` is empty. `export_bindings` then returns `Ok` with no bindings even when `ConfigVersion` is unbound, while the semantically identical `deref(c).version + 1` refuses naming `ConfigVersion`. That breaks FR-307 "a bound item SHALL return every binding it references", and CG would receive an item marked fully bound that reads an unbound object. Fix: store the bound value's `first_objects` with the slot and extend `objects` on `Local`, the same way `reads` is extended, and add a let-through test. | qsl-semantics/src/check/claims.rs:605; qsl-semantics/src/check/claims.rs:663-674 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | A `let` slot now stores its bound value's object types and a `Local` read extends the claim's objects with them, as `reads` is extended. Test `a_let_bound_read_references_the_same_object_types_as_the_inline_read` shows the `let` and inline forms refuse alike. |
| Style note 1 | fixed | `record_keys` moved above `population_domains`' doc comment. |
| Style note 2 | fixed | A failed `Identifier::new` of a declared parameter name is now `AbstractionRefusal::Internal` (`runtime_invariant`/`established-invariant-broken`), not `missing-name`. |
