---
id: SR-12313
title: "Code review (Rust lane) of QSL-559 PR #1202: replicated-role S3 population, send/receive region, universe field reads (STD-243)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c3f550f1dbcd1bb67856715a02b015ee3033829c; qsl-semantics/src/check/{protocol_slots.rs,protocol_slots/tests.rs,protocol_clause/mod.rs,refusal.rs}, qsl-eval/src/simulation/{model.rs,model/footprints.rs}, qsl-replay/src/spine.rs, qsl-analyze/tests/it/{protocol_footprints.rs,protocol_roles.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-208
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
---
# SR-12313: Code review (Rust lane) of QSL-559 PR #1202

## Summary

Ticket: QSL-559. PR quire-spec-language#1202, head c3f550f1d, scoped to `git diff origin/main...HEAD`. The `rust-review` lane is folded into this file. The spec is STD-243 (QSL #1187), FR-208, FR-209, FR-216 and FR-218, plus ADR-027 FT-1 "Locations a predicate reads".

What was measured:

- Baseline: the new tests pass. That is `qsl-semantics --lib protocol_slots` (8 tests) and `qsl-analyze --test it` `protocol_footprints::` and `protocol_roles::` (31 tests).
- Mutations, each restored afterwards, and each killed by its test:
  - M1, the element-type check in `role_population` bypassed: `a_replicated_roles_population_resolves_in_the_package_of_its_type` fails.
  - M2, the `scope` region filter reverted to the written `by` role: `send_and_receive_are_owned_by_their_channel_roles_and_count_in_the_region` fails.
  - M3, the `universe_declaring` expansion in `slot_reads` disabled (back to `AnyField`): `predicate_reads_are_the_fields_of_the_universe_and_a_role_read_orders_spawns` fails.
- `make string-edge` and `make arch-lint-api-surface-qsl` pass.

## Verdict

**PASS**. There is no major finding. Five lows follow.

What holds:

- **POR soundness of `Field(o, f)` over `universe_declaring`.** `universe_declaring(f)` walks every object type whose effective attribute set (inherited fields included, `TypeEnvironment::attributes`) holds `f`. For each such type it lists every instance of every population whose member type conforms in either direction, as a reference typed at that type. A write to `o.f` is made through a receiver or argument whose static type declares `f`. So its typed reference is among those listed. Universes are closed key lists, and nothing is created outside them. An unbounded population, a type with no identity, or a reference fault returns `None`, and the read falls back to `AnyField`, which is the old, wider footprint. The narrowing therefore loses no conflict that `AnyField` caught, except one case that was already lost. That case is a field that a more derived attribute overrides. It has its own key, which `AnyField{f}` did not match either. The PR did not introduce it.
- **The public break is named.** The PR body's changelog names `CheckedReplicatedRole::population` (`String` to `DeclarationKey`) and the two new `ProtocolAnchorCause` variants. `ProtocolAnchorCause` is exhaustive, and the one in-tree exhaustive match (`qsl-replay` `check_message`) is updated.
- **The obligation identity.** The brief said two requests that differ only in `max_live_instances` should differ. The merged spec says the opposite. FR-209 "The bound" says the identity excludes the bound. Its Dependencies section says two such requests "carry equal obligation identities", as QSpec FR-428-AC-3 states. The test that asserts equality is correct against the spec. The latest planner comment on QSL-559 ("... differ") is wrong.
- `written_owner` gives the same owners as the code it replaced, for attempt, event, send and receive.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `scope` region dropped `commit N by r`. Commit declarations carry a written `role` (`qsl-forms` protocol_clause.rs:1000-1006). The old filter counted them. `written_owner` returns `None` for `Commit`, so a commit by a scope role no longer widens its region. FR-218 says "every node `by` that role". This change is neither named nor tested. Either count `Commit` in the region, or state in the PR why a commit is not a node `by` its role. | qsl-semantics/src/check/protocol_slots.rs:1274-1275, qsl-semantics/src/check/protocol_clause/mod.rs:295-316 |
| FND-002 | low | `role_population` finds the package by splitting the written type name at `::` and matching the alias. RR-1 says "the package that declares `T`". The resolved type's `DeclarationKey.package` names that package directly. A type that is unqualified or re-exported is reported as `MissingRolePopulation` against the wrong package. Resolving from the type also needs no string parsing. | qsl-semantics/src/check/protocol_slots.rs:1333-1338 |
| FND-003 | low | `universe_declaring` turns every `InternalFault` and `NotSimulated` into a quiet `None` with `.ok()?`, which silently falls back to `AnyField`. The fallback is sound, but a real fault (for example `universe-key-references`) is hidden. The function is also recomputed in full on every `retire` and `activate` footprint, a walk over types times objects. Propagate the fault, and compute the set once per field. | qsl-eval/src/simulation/model.rs:987-1004 |
| FND-004 | low | `requests_differing_only_in_max_live_instances_share_one_obligation_identity` cannot fail. `author_clause` and `ItemObligation` take no `max_live_instances`, so the three requests' bound never reaches the code under test. The equality holds because of the signatures, and the test only repeats that. It would need to compare an identity that a `check_protocol` or settle path computes from the request. Otherwise drop it. | qsl-analyze/tests/it/protocol_roles.rs:1434-1473 |
| FND-005 | low | `#[allow(dead_code)]` silences the whole included fixture module, where the dead item is one function. The TC-661 test comment calls `M::Level` "an enum". It is an `Int` range: `enumerable` rejects `Enum`, so an enum binder would fail as a record does. | qsl-semantics/src/check/protocol_slots/tests.rs:16-21, qsl-analyze/tests/it/protocol_footprints.rs:1049-1053 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | The `scope` region now counts `commit N by r` again (the FND-001 fix), but no test checks it. With the `declaration.role` branch of the region filter disabled, all 144 `qsl-semantics` protocol tests still pass. Counting a commit in the region is behaviour, so it needs a test: a `scope` role whose last `by r` node is a `commit` after `S` has the region that holds the commit, not `S`. | qsl-semantics/src/check/protocol_slots.rs:1274-1289 |

## Dispositions

Round 1, head 4d558634d (fix commit 4d558634d on top of a merge of main). Focused tests are green: `qsl-semantics --lib protocol_slots` 8 passed, and `qsl-analyze --test it` `protocol_footprints::`/`protocol_roles::` 31 passed. The commit-case mutation was run and restored.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4d558634d. The region filter adds a node whose written `by` role is the role, which covers `commit`. Its test is FND-006. |
| FND-002 | fixed | 4d558634d. The package is found from the resolved type, `AdmittedModel::type_key(element)`. Unresolved types refuse `MissingRolePopulation`. |
| FND-003 | fixed | 4d558634d. The set is cached per field. The `None` to `AnyField` fallback is sound for POR: `AnyField{f}` conflicts with every `Field(_, f)` and `AnyField{f}` write, so it costs precision, never a dependence. A hidden fault is accepted as documented. |
| FND-004 | accepted-no-change | The test is kept, with its doc stating that it cannot fail today. The guarantee is structural, and the test holds the spec in place for when a bound reaches the writer. Low, so it is not worth a round. |
| FND-005 | fixed | 4d558634d |

Round 2, head 2236f590d (fix commit 406c08a38, then a merge of main). Measured by mutation, then restored.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-006 | fixed | 406c08a38. `a_commit_by_a_scope_role_counts_in_its_region` puts a `commit` by the scope role after `S` and expects the region `Main`. With the `declaration.role` branch of the region filter disabled, it fails (1 failed, 8 passed). |
