---
id: SR-11733
title: "Code review of quire-spec-language PR #903: intake of receiver-scoped frame members"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7df1426cacf99f467739f866c063359847185bbd; qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/process.rs, qsl-semantics/tests/it/main.rs, qsl-semantics/tests/it/model_operations.rs, qsl-semantics/tests/it/model_receiver_frames.rs"
review_set: subset
---
# Code review of quire-spec-language PR #903

## Summary

Ticket: QSL-522 (partial delivery: intake only). One pass of code-review with
the rust-review lane over `git diff origin/main...HEAD` (five files, no
`spec/` or `reviews/` change). Peter's rules apply: only major bugs block;
mediums and lows are recorded in the Verdict.

Examined against FR-154 (Inputs, Behavior "Intake"), QSpec FR-013 (cited by
id: the receiver-scoped members belong to operations of object types;
FR-013-AC-11, AC-12, AC-14) and the pinned FCD semantic-IR frame schema
(filament-core-data `crates/semantic-ir/src/schema.rs`, which already admits
the three optional members).

Examined:
- `read_operation_member` frame block: closed member set, required
  `modifies`/`creates`/`deletes`, member shapes, then the receiver rule
  (examined)
- `ObjectShape` / `OperationReceiver` and every caller of
  `read_operation_member` and `read_object_type` (object type, systems
  interface, process, persistence interface, record value type, and the
  state machine through `read_record_value_type`) (examined)
- `FrameIndex`, `resolve_receiver_modifies`, `resolve_deletes_parameters`,
  `resolve_frame` entry order and the conflicting-binding check (examined)
- existing frame fixtures across the workspace for documents the closed
  frame would newly refuse (examined: every semantic-IR frame fixture carries
  all three arrays; the `fields`/`created`/`deleted` fixtures are the native
  model-source form, not this reader)
- consumers of `receiver_modifies`, `deletes_receiver`, `deletes_arguments`
  (population, conformance, normalize, observation/frame) (context_only)
- S4 frame lowering (`check/lowering/state.rs`) (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS for code-review. The check order matches FR-154 "Intake": member shapes
(unnamed member, required arrays, element types, `deletes_receiver` boolean),
then the receiver rule (`modifies_receiver` first, before any entry
resolves), then `modifies`, `modifies_receiver`, `creates`, `deletes`,
`deletes_parameters`, and `conflicting-binding` last, located at the
`modifies_receiver` entry, which is always the later member in the reader's
key order. Codes and causes are typed (`IntakeMalformedDeclaration`,
`FrameEntryMissing`, `FrameEntryMalformed`, `ConflictingBinding` with a
member path). Inherited fields resolve through `FrameIndex::conforms`. No
unwrap, expect or panic in non-test code; no compat shim (the `is_interface`
flag is replaced outright). The closed frame refuses no document the spec
still wants admitted: FR-154 Inputs and QSpec FR-013-AC-11 both require the
three arrays and name exactly six members, and the FCD validator runs first.

Receiver rule scope: the PR refuses non-empty `modifies_receiver` and
`deletes_receiver: true` on persistence-interface and record-shaped
(including state-machine) operations as well as processes. FR-154 names only
the process; QSpec FR-013 states the members "belong to operations of object
types" and names the state machine explicitly. So: state machine is
spec-stated, persistence interface is spec-silent in QSL but consistent with
QSpec FR-013's general rule. Not contradicted; a one-line FR-154 addition
would settle it.

Emission: the PR body states the scope and `deletes_scoped` are not emitted.
Recorded: it should also say plainly that until the IR reader lands, an
admitted receiver-scoped frame lowers to a frame node without those entries.

Recorded, not blocking:
- low: the `conflicting-binding` detail string carries a run of literal
  spaces (`at                      {every_object_at}`), a lost line
  continuation (intake.rs:4586).
- low: the comment above the frame block still says an absent member reads
  as empty; the three base members are now required (intake.rs:2675-2679).
- low: a wrong-typed optional array member locates at
  `frame.<member>.<member>` (pre-existing `frame_entries`/`array_field`
  doubling, now reached by the new members); the FCD schema refuses it first.
- low: `resolve_deletes_parameters` classifies the parameter's type only
  against this package's records, so a `Reference<T>` to an imported object
  type would refuse `missing-name`.
