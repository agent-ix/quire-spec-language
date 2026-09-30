---
id: SR-816
title: "QSL-330 code review (with rust-review lane) of PR 535"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@62a311d3a84b4264764964283846aac1ba2c7ddb; qsl-replay/src/execute/frame.rs; qsl-replay/src/spine/clause.rs; qsl-replay/src/spine/clause/tests/frame.rs; qsl-replay/src/spine/clause/tests/frame_replay.rs; qsl-semantics/src/check/assemble.rs; qsl-semantics/src/check/check.rs; qsl-semantics/src/check/lowering/model.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/state_clause.rs; qsl-semantics/src/model/intake.rs (unchanged, context); qsl-semantics/src/model/observation.rs (unchanged, context); xtask/src/string_edge.rs (unchanged, context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
## Summary

Ticket: QSL-330 (ADR-017 G-1 / TK-1). PR: quire-spec-language#535 at 62a311d3,
one commit over origin/main. Methods: code-review with the rust-review lane
folded in.

The PR retypes `qsl_replay::spine::OperationName` to three `Identifier`s,
adds `CheckedGraph::resolve_operation` (alias to `DomainPackageRef`, object to
its `DeclarationKey`, operation kept as an `Identifier`), and makes
`CheckedGraph::operation_frame` take the resulting `OperationSelection`, whose
fields are `pub(super)`. It adds `PackageDeclarations::model_aliases`,
`CheckedGraph::model_aliases`, `CheckedGraph::object_types` and
`AdmittedModel::object_types`. `resolve_frame` now calls the resolver, and
both entry paths (the FR-115 run at clause.rs:874 and the FR-116 replay at
execute/frame.rs:187) go through it. No string selection path remains: the
only `operation_frame` callers are `resolve_frame` and the tests.

Checks run, with results:

1. Oracle strength, by mutation (each reverted with `git checkout`, tree clean
   afterwards):
   - M1: `OperationSelection` fields made `pub`. The E0451 compile_fail
     doctest goes red with "Test compiled successfully, but it's marked
     `compile_fail`". The doctest fails for the intended reason.
   - M3: resolver ignores the alias (`model_aliases.values().next()`).
     `an_unresolved_model_alias_or_object_type_refuses_at_select` goes red.
   - M5: `operation_frame` returns a frame only when `declaring == context`.
     `an_inherited_operation_selects_its_declaring_frame_through_the_resolver`
     goes red.
   - M2: the resolver's package filter (mod.rs:1826) replaced by a
     tautology. All 26 frame tests stay green. See FND-001.
2. The E0061 doctest: E0061 is the arity error, and rustdoc requires that code
   in the output, so it cannot pass on an unrelated error such as a bad path.
   It proves the old two-argument call is gone. It does not prove that a
   string cannot act as a selection. See FND-002.
3. `model_aliases` / `object_types`: `CheckedGraph` keeps no `AdmittedModel`,
   and `model_correspondence` is NodeKey to DeclarationKey, so there was no
   existing DeclarationKey to EffectiveId map. The new maps are not duplicates
   of existing lookups. Two layout issues remain: FND-003 and FND-004.
4. The `EffectiveId` that `object_types` yields comes from the same
   `view.type_identities()` that `model_object_types` uses to build the
   `M::T` scope names (assemble.rs:606-623). The artifact segment is the same
   `type_identity_segment` value. The old name match and the new resolver
   therefore pick the same type.
5. Refusal: `resolve_frame` returning `None` still maps to
   `ClauseDisposition::MissingName` (FR-115) and to
   `ReplayRefusal::UnknownOperation` (FR-116).
6. Rust idioms: no new panics on a production path, no `unwrap`, no casts.
   `Identifier` has no `Display`, so the `as_str()` calls in
   `OperationName`'s `Display` are needed. Public items have docs.

## Verdict

Correct, and the exit criteria are met in code. The PF-3 resolver is the only
selection path, and inherited operations resolve to the declaring frame
(M5 shows a test guards this). One untested branch matters: FND-001, the
package filter that separates two models declaring the same type name. The
LOW items are layout and oracle nits. Mergeable once FND-001 has a test, or
once the leader accepts it as a follow-up.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers the resolver's package filter `key.package == package.identity`. Mutation M2 replaced it with `!package.identity.is_empty()` and all 26 `spine::clause::tests::frame*` tests stayed green. Every fixture has one model, so the alias lookup is only checked for existence, never for choosing between packages. Failure scenario: a unit with `model A = ...; model B = ...;` whose packages both declare `T`. If the filter regresses, `A::T::op` resolves to whichever `T` key sorts first in the `BTreeMap`, which may be B's. The run then admits and evaluates against the wrong package's frame, or reports `missing-name` for a declared operation. Fix: add a two-model resolver test (both packages declare `T`) asserting `resolve_operation(A, T, op).object().package` is A's identity and B's for `B`. | qsl-semantics/src/check/mod.rs:1824-1829 |
| FND-002 | low | The E0061 compile_fail doctest checks arity, not the criterion "a formatted string no longer type-checks as a selection". Scenario: a later change makes `operation_frame` accept `impl Into<Selector>` with a `From<&str>` impl. `graph.operation_frame(&format!("Config::ConfigVersion::attemptUpdate"))` then compiles, and this doctest stays green because the two-argument call is still E0061. Fix: make the doctest a one-argument string call, `graph.operation_frame(&format!("Config::ConfigVersion::attemptUpdate"))`, marked `compile_fail,E0308`. | qsl-semantics/src/check/mod.rs:1845-1851 |
| FND-003 | low | `resolve_operation` scans every admitted object type linearly and re-parses each `DeclarationKey.node` with `type_identity_segment` to compare strings. That comparison sits outside a `#[string_edge]` function (FR-064 idiom; the lint cannot see a two-binding comparison). `model_object_types` already derives the same artifact segment once per type (assemble.rs:606-623). So one fact is derived in two places, and each selection costs O(types) string work. No wrong result today. Fix: key `CheckedGraph::object_types` by (package identity, artifact `Identifier`) to (`DeclarationKey`, `EffectiveId`), built where the segment is first derived. The resolver becomes a map lookup with no post-check string parse. | qsl-semantics/src/check/mod.rs:1825-1829; qsl-semantics/src/check/mod.rs:366-369 |
| FND-004 | low | `PackageDeclarations::model_aliases` is a new `pub` field, filled in the same loop as `models` (assemble.rs:1183-1187). Nothing ties the two together. Constructors that set `models` directly, such as qsl-package/src/checked.rs:566 and the lowering-model test fixtures, leave `model_aliases` empty. On a graph built that way, `resolve_operation` returns `None` for every alias, so a `Frame` run reports `missing-name` for an operation that exists and has a frame. Fix: store the alias on `AdmittedModel`, which is built in the same loop, and derive the alias table from `models`. Then there is one source for alias to selection. | qsl-semantics/src/check/check.rs:319-322; qsl-semantics/src/check/assemble.rs:1183-1187 |
| FND-005 | low | No production code reads `OperationSelection::package` or its `package()` accessor. `operation_frame` reads only `object` and `operation`, and frame admission finds the model through the context's view (observation.rs:1256). The `DeclarationKey` already carries the package identity, so `DomainPackageRef` adds only the version, and only a test reads it. ADR-017 TK-1 names the triple, so this is a judgement call. Either give it a reader or drop it and amend TK-1's wording. | qsl-semantics/src/check/state_clause.rs:71-82 |
