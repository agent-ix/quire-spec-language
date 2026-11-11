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
