---
id: SR-12325
title: "Code review of quire-spec-language PR #1215: OperationSite carries frame_operation (QSL-972)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a59c5848f; qsl-replay/src/call_site.rs; qsl-replay/src/spine/clause/tests/call_site.rs; qsl-semantics/src/check/mod.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1215

## Summary

Ticket: QSL-972. Covers `OperationSite::frame_operation`, the new
`CheckedGraph::declaring_object` accessor, the Fault refusal for an
unresolvable declaring object, and the AC-23/AC-24 tests. Includes the
rust-review lane.

Measured at a59c5848f:

- `cargo test -p qsl-replay --lib call_site`: 25/25 pass.
- `make string-edge`: no unmarked string comparison.
- `make arch-lint-api-surface-qsl`: T12-B to T12-F and FR-100-AC-8 pass;
  T12-A skipped under `--qsl-only`.
- `cargo clippy -p qsl-replay -p qsl-semantics --all-targets --all-features
  -- -D warnings`: clean.
- Mutation 1 (throwaway, restored): build `frame_operation.object` from the
  selected type instead of the declaring one. AC-23's test fails.
- Mutation 2 (throwaway, restored): make `declaring_object` always answer
  `ConfigVersion`, the redefined base. AC-24's test fails.
- `OperationSite` has one constructor, `operation_site` in call_site.rs.
  No other crate or test builds one.
- The declaring object is right in every reachable case. `operation.declaring`
  is the AD-006 declaring type that `operation_frame` already resolves:
  the supertype for an inherited operation, the redefining type for a
  redefined one. A multi-level chain resolves the same way, because the
  value comes from the effective view, not one parent step.
- Supertypes are intra-package. Intake refuses a supertype that "names no
  type of this domain package" (qsl-semantics/src/model/intake.rs). So the
  declaring type is always in the selected alias's table, and `Fault` is a
  true invariant break, not reachable from valid input.
- The object is `<model alias>::<artifact id>`, the form FR-121 and the
  `FrameOperation` rustdoc state (`M::T`). Two aliases that bind one package
  each name the type under the alias selected. That matches "under the
  unit's model alias".
- `anchor`, `frame` and `frame_occurrence` are unchanged. AC-23 asserts the
  inherited selection gets the same ones as the declaring selection.

## Verdict

Mergeable. No major findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `declaring_object` rustdoc says "the object type, process or state machine". `model_objects` holds only `DomainPackageRecord::ObjectType` records (`AdmittedModel::object_types`, qsl-semantics/src/check/lowering/model.rs:549), so a process or state machine never resolves. Clause resolution and `resolve_operation` use the same table, so no frame of a process or state-machine operation exists to hit this. The doc overclaims; it should say "object type". | qsl-semantics/src/check/mod.rs:3843 |
| FND-002 | low | No test covers an operation inherited through two levels (A declares, B extends A, C extends B; select C). The value comes from the effective view and mutation 1 shows the selected-type mistake is caught, so this is coverage depth only. | qsl-replay/src/spine/clause/tests/call_site.rs |
| FND-003 | low | `declaring_object` scans every type of the alias to match one `EffectiveId`. This is O(types) per call; it is fine at call-site rates. `field_domain` does the same reverse scan. | qsl-semantics/src/check/mod.rs:3848-3859 |

## Dispositions

| FND | outcome | sha |
| --- | --- | --- |
| FND-001 | fixed | f13d0e0f4 |
| FND-002 | fixed | f13d0e0f4 |
| FND-003 | accepted-no-change | f13d0e0f4 |
