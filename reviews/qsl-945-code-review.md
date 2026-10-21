---
id: SR-12288
title: 'Code review of PR #1170: admitted union keys and member ids through the replay facade (FR-396), code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@f3eb22fe0; git diff origin/main...f3eb22fe0 (PR #1170, branch qsl-945-impl): qsl-replay/src/admitted.rs, qsl-replay/src/lib.rs, qsl-replay/src/execute/tests/composite.rs, qsl-replay/tests/admitted_composite_facade.rs, qsl-replay/tests/admitted_composite_root_only.rs; ticket QSL-945'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-396
  type: reviews
---

## Summary

Ticket: QSL-945. PR: quire-spec-language#1170, head f3eb22fe0. Method: code-review with the rust-review lane folded in. Verified by measurement with a throwaway probe test (deleted).

The probe compiled `record P { a: Boolean; } union Shape { Empty, Circle(Integer), Rect(P, Boolean) } function f using v(a: Shape, b: Shape): Boolean pure { a = b }` through `compile_admitted` and through the spine directly, and measured:

- `Shape.key()` (5c029c72…) equals the checked parameter type of `f.a` (`ValueType::Composite(5c029c72…)`), the `scope().types()` handle. Correct.
- `types.union_key(key())` equals `node()` (40a2ac98…), and `types.union_handle(node())` equals `key()`. So the composite_type node id is the union's settled key.
- Every sealed binding has `declaration() == node()` and `variant()` equal to the facade's `variant()`. Replay's witness decode (`execute/argument.rs`) checks the wire declaration against `union_key(slot_key)` and takes the member from this same binding, so a replayed union value carries `node()` and these `VariantId`s.
- Payloads: `Rect` is `[Composite(P.key()), Boolean]`, in declared order.
- A root-only consumer that declares the union with `UnionMemberDeclaration::resolved` (as the AC-6 test does) gets an environment that admits no union value: `TypeEnvironment::union(key(), variant, [3])` refuses `TypeMismatch` and `admits` returns false. It only works after `seal_union_verified(key(), node(), bindings)`, and each binding needs `UnionMember::from_admitted(node(), NodeKey::from_digest(*variant.as_bytes()), identifier)`. After sealing, `union()` succeeds and `admits` is true.

Gates run at f3eb22fe0 via locked-build: `admitted_composite_facade` (4 pass), `admitted_composite_root_only` (2 pass), the in-crate `admitted` tests including `admitted_union_node_is_what_replay_converts_a_union_witness_against` (15 pass); `cargo clippy -p qsl-replay --all-targets --all-features -D warnings` clean.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| composite_table Union arm | qsl-replay/src/admitted.rs | examined |
| union_members (sealed binding read) | qsl-replay/src/admitted.rs | examined |
| AdmittedUnionMember / AdmittedComposite docs | qsl-replay/src/admitted.rs | examined |
| AdmittedCompositeForm::Union | qsl-replay/src/admitted.rs | examined |
| WrongKind / Foreign refusals | qsl-replay/src/admitted.rs | examined |
| witness union decode | qsl-replay/src/execute/argument.rs | context_only |
| TypeEnvironment::union_key / seal_union_verified / union | quire-semantic-value declaration.rs | context_only |
| UnionMember::from_admitted | quire-exact value.rs | context_only |
| CG composite equality renderer | quire-contract-codegen src/composite_equality.rs | context_only |
| tests | qsl-replay/tests/admitted_composite_*.rs, src/execute/tests/composite.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A consumer cannot use an admitted union without building a key. CG's generated oracle builds a runtime `TypeEnvironment` and evaluates values in it. Measured: an environment with the union declared from the facade (`UnionMemberDeclaration::resolved`) refuses `union()` with `TypeMismatch` and admits no union value. Sealing it needs `UnionMember` bindings, and `UnionMember::from_admitted` takes the member key as a `NodeKey`, so the consumer has to retype `NodeKey::from_digest(*variant.as_bytes())`. QSL-945 check 1 says "no decoding or hashing", and ADR-011 T12 forbids a consumer to build a `NodeKey`. `union_members` already holds the sealed `&UnionMember` and throws it away. The AC-6 root-only test passes only because it declares the environment and never seals it or admits a value. Fix: expose the binding, e.g. `AdmittedUnionMember::member() -> &UnionMember`, or a ready `UnionMemberDeclaration::from_verified`. Then extend the root-only AC-6 test to `seal_union_verified(key(), node(), …)` and admit a value of each member. | qsl-replay/src/admitted.rs:98-128,582-611; qsl-replay/tests/admitted_composite_root_only.rs:110-153 |
| FND-002 | medium | The docs state the key contract wrongly for unions. The `AdmittedComposite` doc says `key` is what "every … replayed value of it carries". For a union, the replayed value's `UnionMember::declaration()` is `node()` (measured), while `ValueType::Composite` and `CompositeDeclaration` use `key()`. `union_members` only says "a replay witness names a union by `node()`". Nothing says that `node()` is the settled union key, the `final_key` of `seal_union_verified` and the declaration every union value carries. A CG author who follows the struct doc will build bindings over `key()`. Those seal without error, but the values do not match replay or the IR. This is the same wrong-key trap as #1165. Fix: state the asymmetry on `node()`, `key()` and `union_members`. | qsl-replay/src/admitted.rs:131-146,170-177 |
| FND-003 | low | `members: Option<Vec<AdmittedUnionMember>>` is documented as "`Some` exactly when `form` is `Union`", but nothing enforces that. Carrying the members in the form (`Union(Vec<…>)`) or a dedicated union type would make it hold by construction. | qsl-replay/src/admitted.rs:143-145 |
| FND-004 | low | `AdmittedCompositeForm` is not `#[non_exhaustive]`, so the new `Union` variant breaks downstream exhaustive matches. No matcher exists in this repo or in quire-contract-codegen today. Note the break for the CG lead, or mark the enum non-exhaustive. | qsl-replay/src/admitted.rs:90-98 |

## Verdict

Not mergeable on FND-001. The keys themselves are right, measured: `key()` is the `scope().types()` handle that the checked signature carries, `node()` is the settled union key that replay decodes against, and the `VariantId`s and payloads equal the sealed bindings in declared order. The independent FR-441 minting oracle (`mint_union_variant_id(mint_union_member_key(node, name))`) is sound. The refusal changes (union `Foreign`, no union `WrongKind`) are correct and tested. The gap is the last step: a consumer still has to build a `NodeKey` to seal and use the union, and the docs point it at the wrong key.

## Dispositions

Round 1, reviewed at 84af1b5a0 (fix commit f37c88d1b). These rows replace the coder's proposed outcomes; the reviewer checked each against the code. Re-run for this round: `admitted_composite_facade` (4 pass), `admitted_composite_root_only` (2 pass), clippy for qsl-replay with all targets and features (clean), and `quire validate` on FR-396 and TC-1090 (exit 0). Mutation probe on `a_consumer_seals_a_union_and_admits_a_value_of_each_member`: (A) with the seal loop removed, the test fails at the first `union()`, line 199. (B) With another union's member id (`Wrap.Plain`) used for `Shape.Empty`, it fails at line 201. Both mutants were reverted.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | f37c88d1b: `AdmittedUnionMember::member() -> &UnionMember` returns the sealed binding. The root-only test seals `Shape` and `Wrap` with `seal_union_verified(key(), node(), member().clone()…)` and admits a value of every member, including a `Wrap` holding a `Shape`. The consumer builds no `NodeKey` (its only `from_digest` is the pre-existing `WireNodeId` lookup id). Measured by mutation: the test fails if sealing is skipped or another union's id is used. |
| FND-002 | fixed | f37c88d1b: the docs on `AdmittedComposite`, `node()`, `key()` and `union_members()` now say that a union's types use `key()` while its values, members and seal use `node()` (`union_key(key()) == node()`). |
| FND-003 | fixed | f37c88d1b: members live in a private `AdmittedShape::Union(Vec<…>)`, and `form()` and `union_members()` are derived from it, so the invariant holds by construction. |
| FND-004 | rejected | Agreed with the coder. This is prerelease, `AdmittedLookupRefusal` is not `#[non_exhaustive]` either, no exhaustive matcher exists in this repo or in quire-contract-codegen, and the PR body states the break. Marking the enum non-exhaustive would only force wildcard arms. |
