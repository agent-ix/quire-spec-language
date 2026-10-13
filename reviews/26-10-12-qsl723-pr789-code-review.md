---
id: SR-7856
title: "QSL-723 PR 789 code review: state machine and native scalar nodes"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0d84e0cea7b32600caa6e2f2d95b18ea13d2dd0d; qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/check/lowering/model/tests.rs, qsl-eval/tests/it/tc235_business_meaning_runtime.rs"
review_set: subset
---

Redacted: QSpec content removed (public repo); the unredacted record is in the private tracker.

## Summary

Ticket: QSL-723. Code review with the rust-review lane over PR 789. The Rust is small and idiomatic: no catch-all arm, no panic surface, typed assertions in the new unit tests, and both new unit tests fail before the fix (a `None` from `record_form` and an `Err` from `type_node` each hit their `expect`). The defects are in what the new nodes mean on the wire. QSpec checked-package-v2, FR-322 and FR-340 do not admit the forms this PR now emits (details in the private record). Reviewer ran no build.

## Verdict

FAIL. FND-001 and FND-002 are high. Each new node form breaks a QSpec wire rule. The `record_form` arm and the scalar arms are mechanically correct, and the run 1-5 un-ignore and run 6 split are sound.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | String/Uuid/Timestamp now key as `scalar_type` forms `string`, `uuid` and `timestamp`. The forms conflict with QSpec checked-package-v2 `ScalarTypeNode` and QSpec FR-322 (details in the private record). No QSL spec (FR-056, FR-092) defines these forms either. | qsl-semantics/src/check/lowering.rs:1917 |
| FND-002 | high | `StateMachineRecord` keys as `model`/`state_machine` with a `ModelOwner`. This conflicts with QSpec checked-package-v2 `OwnerPresenceRule`, QSpec FR-322, and QSpec FR-340-AC-2/AC-3/AC-12, so a conforming reader refuses a package with a state-machine anchor or frame. QSpec FR-322-AC-25 appears to contradict those rules, a QSpec defect no ticket records. Details in the private record. | qsl-semantics/src/check/lowering/model.rs:309 |
| FND-003 | medium | The same latent `UnnamedRecordKind` internal fault remains for `PersistenceInterface`. assemble.rs binds it as a checked object type with operations, and it has an EffectiveId through `normalizable_types`, so an invariant on one, an anchor on its operation, or a `Reference` to it reaches `record_form` and gets `None`. QSpec FR-322-AC-25 and FR-340 bear on `persistence_interface` and `RecordValueType`, which also returns `None`. | qsl-semantics/src/check/lowering/model.rs:311 |
| FND-004 | medium | The trace tags on `state_machine_declaration_keys_as_a_state_machine_model_node` are wrong. Unprefixed `FR-322-AC-25` resolves to QSL's own FR-322 (union construction), which has no AC-25; the intended criterion is `QSpec-FR-322-AC-25`. `FR-094-AC-1` covers `Reference<M::Order>`/`Invoice`/`billedTo`, not a state machine. | qsl-semantics/src/check/lowering/model/tests.rs:1360 |
| FND-005 | medium | The trace tag `FR-092-AC-12` on `native_string_uuid_and_timestamp_key_as_distinct_scalar_nodes` covers record `Point` key independence, not native scalar nodes. No AC states the new forms, so the test inflates FR-092-AC-12 coverage. | qsl-semantics/src/check/lowering/model/tests.rs:1414 |
| FND-006 | low | Run 6 asserts `format!("{:?}", refusals[0]).contains(...)` for the operation id and `Operation.pre` rather than typed location fields. This code moved without change into an ignored test, so it does not run yet. It should become typed assertions when the test is un-ignored. | qsl-eval/tests/it/tc235_business_meaning_runtime.rs:1041 |

## Coverage

Examined: `record_form` (every `DomainPackageRecord` variant), its callers `model_node`, `model_node_key`, `object_node`, `frame_field`, `frame_objects` and `relationship_nodes`, the state-clause and operation-anchor lowering in state.rs, the `type_node` native arms, the two new unit tests (red before the fix by inspection), and the M15 run 1-5 un-ignore and run 6 split.

Context only: QSpec FR-322, FR-340 and checked-package-v2, QSL FR-056/FR-092/FR-094, and assemble.rs model binding.
