---
id: SR-773
title: "QSL-309 gap re-analysis of PR 510 after the SR-770/SR-771 fix round"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@fbb63d61c382c527c957f029b94dc14902f70e90; spec/functional/FR-113-resolve-scoped-anchors-in-nested-control-scopes.md; spec/functional/FR-114-bind-a-protocol-attempt-to-its-operation-frame.md; spec/test-cases/TC-513-s3-binds-a-protocol-attempt-to-its-operation-frame.md; spec/decisions/ADR-012-semantic-family-extension-contracts.md (read only, §12.2); qsl-forms/tests/it/protocol_clause_forms.rs; qsl-package/src/emit/tests.rs; qsl-semantics/src/check/protocol_clause.rs; qsl-semantics/src/check/assemble/tests.rs; qsl-replay/src/spine.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-513
    type: reviews
---
## Summary

Ticket: QSL-309. PR: quire-spec-language#510 at fbb63d6. This is the
gap-analysis re-review after the SR-771 fix round. SR-771's own dispositions
are appended to SR-771.

What holds:

- **AC coverage.** FR-114-AC-1 to AC-4 are each backed by an emission test
  (`an_attempt_and_its_clause_share_one_frame_node_through_emission`,
  `an_operation_named_only_by_an_attempt_emits_its_anchor_frame_and_record`,
  `an_inherited_operation_binds_its_declaring_types_anchor_and_frame`) or by
  check-stage tests (AC-3: `missing`, `Absent`, `ParentOrder`, `ProbePre`).
  The step-5 ConfigVersion emission test uses only covered content: a role
  `on Config::ConfigVersion`, `run sequence`, one `attempt`, a `finish`, and
  bodies that are all `{ true }`. So it passes through the checked group; it
  is not exempted.
- **TC-513's `{ true }` body.** TC-513's scope is FR-114-AC-1 to AC-4, which
  cover anchor/frame identity, frame records and the `contracts` list. No AC
  or expected result depends on the body reading the binder, so `{ updated }`
  to `{ true }` narrows nothing TC-513 claims to prove.
- **SR-771 FND-006, left undone, is correctly not needed.** An attempt under
  `case`/`branch`/`await` sits inside a refused node kind, so the protocol
  cannot compile whatever S2 captured. The capture uses the one shared
  `event_node_anchors` path at every depth.
- **The garbage regression guard is restored.** The guard is qsl-replay
  `a_protocol_with_unchecked_garbage_content_does_not_compile_silently`, plus
  two emission-level probes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-113 and FR-114 Status sections leave out that a protocol which compiles is not emitted. Probe: the wire of a compiled covered protocol holds only `text`, `operation_anchor`, `frame`, `boolean` and `object_type` nodes, and names neither `Flow` nor `End`. The old FR-113 Status held the refusal "until those tickets complete protocol checking and emission". SR-770 FND-001 asked for Status to say so if the lift was intended. The new text drops the emission condition silently, which would mislead QSL-300/301/303. ADR-012 §12.2's Package row (anchor and frame only) makes the behaviour legitimate, so this is a text fix. FR-113 Status also does not list the duplicate-role-name refusal that `content` enforces. | spec/functional/FR-113-resolve-scoped-anchors-in-nested-control-scopes.md:153-172; spec/functional/FR-114-bind-a-protocol-attempt-to-its-operation-frame.md:112-117 |
| FND-002 | low | Some content the checker covers is untested. No committed test exercises `MissingProfile` (`using` naming a model alias), `MissingRole` (`by` naming no role), the duplicate-role `Ambiguous` refusal, or the `Relationship` construct (the one refused construct that no test writes). The code handles all four (reviewer probes), but if any one regressed, a protocol would again compile with that content unchecked. | qsl-package/src/emit/tests.rs:3112-3150; qsl-forms/tests/it/protocol_clause_forms.rs:384-470 |

## Verdict

Approve after FND-001, which the reviewer fixed in-PR together with FND-002.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | f976743e: FR-113 and FR-114 Status say that the protocol itself is not emitted, and FR-113 lists the duplicate-role refusal |
| FND-002 | fixed | f976743e: `a_valid_attempt_does_not_let_garbage_content_through` covers `using Config`, `by Q`, a duplicate role and a `relationship`. A mutation that removes the `Relationship` capture turns it red |
