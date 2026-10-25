---
id: SR-17035
title: "Gap analysis of quire-spec-language PR 1478 (QSL-814): set, bag and ordered-set value forms"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1393f5efaeb5e9dccb9e5c8b8fe650ca2c2f2dba; PR #1478 diff against origin/main; FR-106 'Set, bag and ordered-set values', FR-106-AC-3, FR-106-AC-50..52 (AC-53..61 noted out of PR scope), TC-1171, TC-465 row 17; QSpec FR-144 'Supplied collection values' and FR-181 as cross-spec context; trace tags at the reviewed sha"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/quire-spec-language/FR-106
---
# Gap analysis of quire-spec-language PR #1478

## Summary

Ticket: QSL-814. This is a planless audit of the requirement surface the PR
touches. The spec was not changed by the PR.

Examined:
- FR-106-AC-50 (examined). Tagged by `native_snapshot_values::set_bag_and_ordered_set_fields_admit_with_their_element_order` (native_snapshot_values.rs:2238), which passes at the head. Every clause is asserted. The one deviation: the "reversed" `counts` document uses `1, 2, 2` because `2, 1, 2` reversed is itself (FND-003). The deviation is justified.
- FR-106-AC-51 (examined). Tagged by `defective_collections_refuse_in_a_field_and_in_a_parameter` (native_snapshot_values.rs:2404). It asserts every case for the field root and the parameter root. The state-machine-context root is asserted only by an untagged test in another crate, and only partly (FND-001).
- FR-106-AC-52 (examined). Tagged by `the_frame_and_the_limits_see_collections_by_value` (native_snapshot_values.rs:2472). It covers the no-change reorders, the dropped `2`, the `1, 3` order, the Witness `OutOfRange` at index 1, and the exact and one-less `observation.values`, where `22 + 15 + 15` pins element counting. The Witness clause uses a new `restock` operation, because admission runs frame check 11 and `retag` grants nothing (FND-004). The deviation is justified.
- FR-106-AC-3 (examined), for its "a field typed a set of `String`" clause, from which the PR removed a tag (FND-002).
- FR-106 "Set, bag and ordered-set values" statement (examined)
- TC-1171 fixture and steps 1-3 (examined)
- TC-465 row 17 (examined)
- QSpec FR-144 "Supplied collection values" and FR-144-AC-22 at quire-specification main 0a11c663 (context_only)
- QSpec FR-181 typed canonical form (context_only)

Wire form (brief item g): the code keeps the externally tagged `{"set": [...]}`
form with `{"kind": ...}` payload objects. Main's FR-106 defines this as "the
native spelling of QSpec FR-181's set, bag and ordered-set forms". QSpec FR-144
"Supplied collection values" says instead that the wire form "is the FR-181
typed canonical form ... `{"type": <tag>, "elements": [...]}` ... with no
second grammar". The code follows its governing QSL requirement, so this is
not a code finding. It is a cross-spec follow-up, recorded as FND-006.

QSpec FR-144-AC-22's old wording ("A supplied `Set<UUID>` ... is
`unknown_required_feature`/`unsupported-feature` ... lifted by QSL-831",
from quire-specification #269) is no longer current. quire-specification
#372 (c1033008) rewrote it, and it now admits `Set<UUID>` and `Set<Timestamp>`
and refuses an uppercase UUID and the timestamp `01` with `invalid-value`.
That agrees with this PR and with FR-106-AC-51, so no tension remains on that
point.

Reverse gap: FND-005 records the sequence-bound behaviour change, and FND-007
records the typed-order statement. Every other new production path belongs to
the FR-106 set/bag/ordered-set statement.

Stubs: none. No test is tautological. The AC tests read real admitted values,
and the reviewer's mutation of `widen_value` fails AC-52's test.

## Verdict

CONDITIONAL. FR-106-AC-50, AC-51 and AC-52 are tagged and their tests
discriminate. The findings are one partial trace (AC-51's context root), one
stale AC (AC-3), and spec follow-ups. FR-106-AC-53..61 are outside this PR and
untagged. They are listed as a follow-up gap (FND-008), not a blocker, as the
dispatching brief directs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-106-AC-51's last clause ("The same cases in a parameter or a state-machine context field use their own path roots") is not earned by the tag for the context root. The tagged test asserts the field and parameter roots only. The context root is asserted by `a_context_set_field_follows_the_collection_rules_of_an_object_field`, which has no `#[trace]` even though its file already uses `#[trace]` 9 times, so the matrix does not see it. That test also covers only 4 of the `ids` cases: duplicate, bound, element wrong-kind and the `sequence` tag. It omits the uppercase-hex UUID `invalid-value`. Fix: tag it `#[trace("TC-1171", "FR-106-AC-51")]` and add the uppercase-UUID case. | qsl-eval/tests/it/machine_contexts.rs:1620-1690; qsl-semantics/tests/it/native_snapshot_values.rs:2404 |
| FND-002 | medium | FR-106-AC-3 still lists "a field typed a set of `String`" as a check-6.2 `unsupported-feature` case, but the code now admits it. The PR removed the `#[trace("TC-465", "FR-106-AC-3")]` tag from row 17 and the AC-3 `excluded_collection_kinds_remain_explicit_refusals` test. TC-465 row 17 now reads "a set of a population", which no integration test exercises: there is only the untagged predicate unit test `a_collection_of_a_population_is_refused_by_kind_and_depth`, because no model document declares one. Spec follow-up: amend AC-3's clause to the population case, or drop it. | spec/functional/FR-106-admit-snapshots-and-invocations.md:1073; qsl-semantics/tests/it/state_clauses.rs:4797-4808; spec/test-cases/TC-465-admission-refuses-each-input-defect.md:44 |
| FND-003 | low | FR-106-AC-50's "same document with `ids`, `counts` and `peers` elements in reversed order" is vacuous for `counts`, because `2, 1, 2` reversed is itself. The test reorders `counts` to `1, 2, 2`, which is the AC's intent. Spec follow-up: say "in another order", or give the reordered array. | spec/functional/FR-106-admit-snapshots-and-invocations.md:1119; qsl-semantics/tests/it/native_snapshot_values.rs:2296-2301 |
| FND-004 | low | FR-106-AC-52's Witness clause ("a post `counts` `[1, 10, 2]` admits") cannot hold over TC-1171's fixture as written. Its only operation, `retag`, grants no field, and admission runs frame check 11 (observation.rs:2966-2981), so the change refuses `frame_violation`. The test adds a `restock` operation that grants `counts`. Spec follow-up: TC-1171's fixture should declare that operation. | spec/test-cases/TC-1171-snapshots-carry-set-bag-and-ordered-set-values.md; qsl-semantics/tests/support/native_values.rs:169-187; qsl-semantics/tests/it/native_snapshot_values.rs:2526 |
| FND-005 | low | Reverse gap: a bound violation on a `Sequence` now refuses `cardinality_out_of_bound` before its elements are read (it used to refuse `invalid_runtime_input`/`invalid-value` after them). FR-106 states the cardinality rule only inside the set/bag/ordered-set paragraph. The change agrees with QSpec FR-144, whose "Supplied collection values" covers `Sequence<T>`, but no QSL requirement text owns it. Spec follow-up. | qsl-semantics/src/model/observation/document.rs:1807-1816; qsl-semantics/tests/it/observation_composites.rs:753-796 |
| FND-006 | low | Cross-spec tension, a spec follow-up and not a code defect. FR-106 defines a set/bag/ordered-set wire form `{"set": [...]}` with `{"kind": ...}` elements as a "native spelling" of FR-181. QSpec FR-144 "Supplied collection values" requires the FR-181 typed canonical form `{"type": "set", "elements": [...]}` "with no second grammar". The owners of FR-106 and FR-144 need to reconcile: either QSpec admits a host spelling, or FR-106 adopts the FR-181 form. | spec/functional/FR-106-admit-snapshots-and-invocations.md:235-238 |
| FND-007 | low | FR-106 says "When admission writes a set or bag value into the typed observation, it SHALL order its elements ascending by the JCS bytes". The typed `Value` keeps quire-exact's key order (numeric for integers, so `9` before `10`), and the JCS byte order (`10` before `9`) is applied only by `SnapshotValue::to_json`. AC-50's single-digit vectors cannot tell the two apart. Spec follow-up: say the written form, or require the typed order. | spec/functional/FR-106-admit-snapshots-and-invocations.md:263-268; qsl-semantics/src/model/observation/encode.rs:225-237 |
| FND-008 | low | Out of PR scope; needs a follow-up ticket. FR-106-AC-53..61 (Decimal, Rational, Float, Quantity, String elements, and Bytes/JsonObject/Duration) carry no trace tag. Meanwhile a set/bag/ordered-set of Rational, Decimal, Float or Quantity refuses `unsupported-feature` (`missing_wire_form`), which contradicts FR-106's SHALL that such sets are "read and ordered exactly as any other element type". | qsl-semantics/src/model/observation/document.rs:1893-1956; spec/functional/FR-106-admit-snapshots-and-invocations.md:1122-1130 |

## Coverage

- `quire matrix --scope . --format json` at the head gives:
  - FR-106-AC-50, AC-51 and AC-52: `tagged`, one binder each.
  - FR-106-AC-3: `tagged`, 48 binders.
  - FR-106-AC-53..61: `untagged`, 0 binders.
- Criteria in scope: 3 (FR-106-AC-50, AC-51, AC-52), plus FR-106-AC-3 for the clause the PR touched.
  - Tagged: 3 of 3. Untagged: 0. Ignored-only: 0.
  - AC-51 is tagged, but one of its three roots is unbound (FND-001).
- Out of PR scope, untagged: FR-106-AC-53..61 (FND-008).
- Tagged tests pass at the head.
- Mutation evidence:
  - Disabling `widen_value` kills AC-52's test.
  - The duplicate earliest-later rule survives (code-review SR-17034 FND-002).
- Reverse gap: 2 behaviours with no QSL requirement text (FND-005, FND-007).
- Stubs / coverage inflation: none.
- Semantic review: run, for AC-50..52 intent against test against code, at the dispatching brief's request.
- Plan completion: not assessed
