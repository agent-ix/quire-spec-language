---
id: SR-2026111015000000000004
title: "failure-domain review of QSL-1400 source-declared object types (PR 2674)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; PR 2674 origin/main...b029e1e03; spec/functional/FR-569, FR-105, FR-390; spec/test-cases/TC-1407, TC-1408; spec/decisions/ADR-013 QC-30"
review_set: subset
---

## Summary

Ticket: QSL-1400. Failure modes of the strict-reader interim and of the identity and keying of the new nodes. The interim note states that only a package with a source population reference is refused on IR main. In fact IR's reader refuses every field read of a source-declared object type, which answers the author's open item. The keying of source clause functions is undefined.

## Verdict

FAIL. FND-001 is high: FR-105-AC-18 and FR-390-AC-71 to AC-73 assert reader admission that IR main refuses, and the spec defines no outcome for that refusal.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-569 gives a source object-type node the body `aggregate{[]}` and names its fields as FR-322 `field` members on that node. IR main's `check_field_member` (operations.rs:4436) looks for a `Binding` of the field name in that aggregate. It finds none, so it refuses `ill_typed`/`operator-ineligible` for `self.total` in S and for `view.total` in E-S. The strict reader therefore refuses S's and E-S's packages on main. The interim notes (FR-569 Dependencies, QC-30) claim only population references are refused. FR-105-AC-18 and TC-1408 step 5 expect admission, FR-390-AC-71 to AC-73 cannot reach view creation, and no outcome is defined. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:270; spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:279-283; spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:389-397; spec/functional/FR-105-emit-state-nodes.md:308; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1304 |
| FND-002 | medium | FR-569 says a source operation's clause function is built "as for a model operation", but with a `declaration` and a `SourceOwner`. FR-094 keys a clause function's id by the owner's IR node identity (`package` and `path`), which a source operation lacks, so the node id is undefined. No `declaration`-role occurrence is stated for it either. IR's `validate_declaration` (DeclarationOccurrenceRule) refuses a `declaration` that has no such occurrence. As a result, the shape FR-569-AC-6 asserts for `B` is not reproducible and fails the reader. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:283-289; spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:358; spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:267-299 |

## Scope and evidence

Direct review. The IR code read is quire-contract-ir origin/main a156efda:

- `operations.rs:1735-1750` routes a field member whose declaring node is not a model declaration node to `check_field_member`.
- `model_members.rs:2987` defines `is_model_declaration_node` as "no `declaration`, or a selected row".
- `mod.rs:2428` holds `validate_declaration`.
- `state.rs:448` admits a source anchor context via `Ok(None)`, and `frame.rs:790` admits a source frame field entry unchecked. Both agree with FR-569's claim about anchors.

- FR-569 "Lowering and identity" and the Dependencies interim (examined).
- FR-105-AC-18 (examined).
- FR-390-AC-71 to AC-73 (examined): unreachable under FND-001.
- TC-1408 step 5 and TC-1407 step 1 (examined).
- No digest-based identity is introduced. FR-569 states structural ids explicitly (examined, clean).

## New findings (disposition pass 1)

These are found against fix commit ba4ea0fcd. IR evidence is quire-contract-ir origin/main a156efda, by code reading. FR-569:426-427 says S4 SHALL never write a package IR's strict reader refuses. FR-569:441-443 and QC-30 say the following are emitted now, but IR refuses each of them.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | FR-569:320-321 builds a source operation's `operation_effects` row (FR-496) "as for a model operation". IR's `operation_effects::validate` (operation_effects.rs:718-750) requires `document` and `declaring.package` to be selected models and `declaring` to resolve via `owner_by_key`. A source row therefore refuses `missing_declaration`/`missing-selection` or `invalid_package`. Fixture S selects no domain package, and its `bump` has a frame. So S's package is refused, and FR-569-AC-6, AC-11, FR-105-AC-18 and FR-390-AC-70's "passes now" parts fail. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:320-321; spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:441-443; spec/functional/FR-496-emit-model-operation-effects-into-checked-packages.md:53-55 |
| FND-004 | high | Every clause function of one source operation carries the same `declaration` `[<type>, <operation>]` and the same path, and only the ordinal differs (FR-569:327-335). IR's `validate_declaration_names` (mod.rs:3138-3200) keys holders by `qualified_name` alone and refuses a repeat as `ambiguous_declaration`/`ambiguous-name`. So a source query with both a `pre` and a `body`, or a redefinition with an effective precondition, yields a package IR refuses. The interim list says clause functions emit now, and fixture S never exercises the case. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:323-345; spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:441-443 |
| FND-005 | medium | A dispatched call to a source operation (FR-569 equates dispatch with a model's, FR-094 dispatch tables) leaves its `checked_term_data.dispatch_tables` row undefined for a source context. IR's `validate_checked_term_data` (dispatch_projection.rs, around line 200) requires `owner_by_key` for both the context and the declaring family, so such a row is refused. The interim list neither defines the table nor names the construct. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:412-421; spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:441-443 |

## Dispositions

Disposition pass 1, checked against fix commit ba4ea0fcd.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-569:292-308 gives the object node one field binding per effective-view field. IR's `check_field_member` (operations.rs:4436) admits a field member when the aggregate holds that binding. The empty-body check in owner.rs:730-745 applies only to `ModelOwner` nodes. Both were confirmed by code reading. FR-105-AC-18 no longer depends on a population reference, but see FND-003 for a different refusal of S. |
| FND-002 | fixed | FR-569:327-341 now defines the clause function's id (path `[type, operation]` and the FR-094 ordinals) and its `declaration` occurrence. IR's `validate_declaration` and callable.rs `function_body` admit one such node. The duplicate-name refusal across sibling clause functions is new, and is recorded as FND-004. |
| FND-003 | still-open: new this round, routed to the coder | A source operation's `operation_effects` row is refused by IR, and the interim list does not cover it. |
| FND-004 | still-open: new this round, routed to the coder | Sibling source clause functions share one `declaration`, which IR refuses as ambiguous. |
| FND-005 | still-open: new this round, routed to the coder | Dispatch tables for a source operation are undefined, and IR would refuse them. |
| FND-003 | still-open: target row not well defined | Round 2 (b3e72440c). FR-569:324-335 sets `document` to a `SourceOwner` object, but FR-496 and IR's `CheckedOperationEffectRow` type `document` as a string, and FR-496 sorts rows by `document` "as UTF-8 bytes". `provenance` is "exactly as FR-496 states", which means `origin` `model_document_operation_frame` (IR's only variant) and `member` the IR node identity `<declaring owner node>/<operation>`. A source operation has no such identity. So the provenance, the type of `document` and the row order for a source row are undefined, and QC-30 (2e)/(4) do not ask for the wire-type or origin change. |
| FND-004 | still-open: a name segment is not an identifier | Round 2 (b3e72440c). The new names do remove the duplicate. A `pre` label sits only in the fourth segment of a four-segment name, so it can never equal the third segment of `[t, op, "body"]` or `[t, op, "effective-precondition"]`. However, IR's `validate_qualified_name` (identity.rs:2892-2915) requires every segment to match `[A-Za-z_][A-Za-z0-9_]*`. `effective-precondition` has a hyphen and is refused, so "admits sibling clause functions with no change" (FR-569:353-355, QC-30 (2d)) is false for effective preconditions. A segment like `effective_precondition` would pass. |
| FND-005 | fixed | Round 2 (b3e72440c). FR-569:337-347 defines the target FR-494 row: the receiver context and candidate declaring types are source nodes, the `EffectiveId` is the checked source key, the body and effective-precondition ids are the source clause functions, and candidate order follows FR-494. QC-30 (2f) and (4) name the `dispatch_projection.rs` change. |
