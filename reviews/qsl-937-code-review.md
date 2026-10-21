---
id: SR-12280
title: 'Code review of PR #1165: admitted record and tuple keys through the replay facade (FR-396), code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@efa450952; git diff origin/main...efa450952 (PR #1165, branch qsl-937-impl): qsl-replay/src/admitted.rs, qsl-replay/src/lib.rs, qsl-replay/tests/admitted_composite_facade.rs, qsl-replay/tests/admitted_composite_root_only.rs, qsl-replay/tests/admitted_context_facade.rs, qsl-semantics/src/check/identity.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/lowering/tests.rs; ticket QSL-937'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-396
  type: reviews
---

## Summary

Ticket: QSL-937. PR: quire-spec-language#1165, head efa450952. Method: code-review with the rust-review lane folded in. Compared against FR-394 / PR #1140 (the enum facade of the same shape).

The facade returns the key of each record's and tuple's `composite_type` structural node. QSL's own checker, replay executor, witness decoder and result wire do not use that key for `ValueType::Composite`. They use the declaration handle that `assemble.rs` passes to `CompositeDeclaration::new` (`declared_type_handle(owner, name)`), held in `graph().scope().types()`. A probe compiled `tuple Pair(Boolean, Boolean); record Holder { flag: Boolean; pair: Pair; }` and printed both. Scope `TypeEnvironment`: Pair `37a5ef60…`, Holder `6001269a…`. Admitted facade: Pair `a91057d0…`, Holder `5bbbabb0…`. They differ. A consumer that builds `CompositeDeclaration::new(admitted.key())` and `ValueType::Composite(admitted.key())` therefore builds types and values that QSL replay does not recognise. Replay's witness decoder refuses a composite witness whose `declaration` differs from its `TypeEnvironment` key (`execute/argument.rs`, the `declaration.as_bytes() != key.as_bytes()` check), and replay result wire values report `Composite { declaration: hex(handle) }` (`result/wire/value.rs`). The tests cannot see this. AC-1 builds a self-consistent environment from whatever keys it gets, and AC-2 compares the facade with `checked_type_nodes()`, the same source the implementation reads.

Gates run at efa450952 via locked-build: the three admitted test binaries (9 tests pass); `cargo clippy -p qsl-replay -p qsl-semantics --all-targets --all-features -D warnings` (clean); `make string-edge` (clean); `make arch-lint-api-surface-qsl` (PASS, including T12-F and FR-100-AC-8).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| composite_table / AdmittedComposite::key | qsl-replay/src/admitted.rs | examined |
| composite_declaration refusal order | qsl-replay/src/admitted.rs | examined |
| context_of other_nodes and foreign_composites | qsl-replay/src/admitted.rs | examined |
| AdmittedLookupRefusal::WrongKind | qsl-replay/src/admitted.rs | examined |
| CompositeForm and CheckedTypeNode::Composite form | qsl-semantics/src/check/identity.rs | examined |
| type_nodes insertion | qsl-semantics/src/check/mod.rs | examined |
| close_composite / open_composite | qsl-semantics/src/check/lowering.rs | context_only |
| composite handles and CompositeDeclaration::new | qsl-semantics/src/check/assemble.rs | context_only |
| witness composite decode | qsl-replay/src/execute/argument.rs | context_only |
| result wire ValueType::Composite | qsl-replay/src/result/wire/value.rs | context_only |
| tests | qsl-replay/tests/admitted_composite_*.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `AdmittedComposite::key()` returns the `composite_type` node key, not the key QSL's own checked `TypeEnvironment` gives the declaration in `CompositeDeclaration::new` / `ValueType::Composite`. Replay execution, witness decoding and result wire values all use that handle. Measured: Pair handle 37a5ef60… vs facade a91057d0…; Holder handle 6001269a… vs facade 5bbbabb0…. A CG `CompositeDeclaration` or `ValueType::Composite` built from the facade key will not agree with QSL replay, so QSL-937 check 5 (native composite agreement) cannot pass. Fix: also retain the handle for each composite (for example `CheckedTypeNode::Composite { node, declaration, form }` filled from `composite.key()` in `check/mod.rs`) and return it as `key()`, still looked up by the `composite_type` node's wire id. Or settle in the spec which key is the kernel identity, and make replay agree with it. | qsl-replay/src/admitted.rs:476-505; qsl-semantics/src/check/mod.rs:2724-2735; qsl-semantics/src/check/assemble.rs:2896-2905,3064 |
| FND-002 | high | The test oracles cannot catch FND-001. `a_consumer_builds_composite_declarations_from_admitted_keys` builds a closed `TypeEnvironment` from whatever keys it receives, so any two distinct keys pass. `composite_keys_are_the_spines_distinct_and_stable` compares the table with `checked_type_nodes()`, the same producer path `composite_table` reads, so it restates the implementation. No test compares the key with the producer's `CompositeDeclaration` key (`scope().types().composites()`), or with the `Composite.declaration` of a replay result or witness for the same declaration. | qsl-replay/tests/admitted_composite_root_only.rs:56-110; qsl-replay/tests/admitted_composite_facade.rs:98-170 |
| FND-003 | medium | `Foreign` and `Unknown` still say "enum". The `Foreign` error text reads "enum declaration {declaration} is admitted by dependency package …", and the `Unknown` / `Foreign` docs say "names no enum declaration" and "names an enum". Both variants are now returned by `composite_declaration`, so a refused record lookup reports itself as an enum. | qsl-replay/src/admitted.rs:127-146 |
| FND-004 | low | `other_nodes` keeps a `BTreeSet<[u8;32]>` of every node in the package's checked graph, for the life of every cloned `AdmittedContext`, only to tell `WrongKind` from `Unknown`. That is O(graph) memory, roughly 32 B plus B-tree overhead per node, and it is not charged against `ReplayLimits`. It is acceptable for V1, but the ticket only requires that any wrong-kind refusal be specified, so folding wrong kinds into `Unknown` would remove the set. | qsl-replay/src/admitted.rs:178-180,435-441 |
| FND-005 | low | `AdmittedLookupRefusal` is not `#[non_exhaustive]`, so the new `WrongKind` variant breaks every downstream exhaustive `match`, and the PR does not say so. Mark the enum `#[non_exhaustive]` or note the break for the CG lead. | qsl-replay/src/admitted.rs:126-156 |
| FND-006 | low | Missing transitive case: no test has an own record with a field of an imported record type (`record R { p: d::Point; }`). In that case the dependency's composite node can also appear in this package's graph, so Foreign-before-WrongKind ordering is what decides the result. The ordering in the code is right; it is just not tested. | qsl-replay/tests/admitted_composite_facade.rs:216-266 |

## Verdict

Not mergeable. FND-001 is a correctness defect: the facade returns a key that QSL's own replay does not use for the same declaration, so the published capability does not do what QSL-937 asks. FND-002 is why the tests pass anyway. The refusal ordering (own, then Foreign, then WrongKind, then Unknown), the transitive dependency walk, the `CompositeForm` plumbing, union exclusion, the root-only consumer surface, clippy, string-edge and the api-surface arch lint are all sound. The enum facade (FR-394) regressions still pass.

## Dispositions

| FND | Outcome | SHA |
| --- | --- | --- |
| FND-001 | fixed: key() is the TypeEnvironment handle, node() is the composite_type node id | 1a902624f |
| FND-002 | fixed: oracles compare with the checked TypeEnvironment, a checked signature and a replayed witness (FR-396-AC-5) | 1a902624f |
| FND-003 | fixed: kind-neutral Unknown and Foreign text | 1a902624f |
| FND-004 | fixed: the memory growth is documented on the field and in FR-396 Behavior; the set stays because WrongKind is specified | 1a902624f |
| FND-005 | fixed: the breaking WrongKind variant is stated in the PR body; no #[non_exhaustive], as no other lookup enum in qsl-replay uses it | 1a902624f |
| FND-006 | fixed: FR-396 states a unit cannot name an imported composite (assembler refuses ImportedTypeName), so the case cannot occur | 1a902624f |
