---
id: SR-2026111015000000000003
title: "integrity review of QSL-1400 source-declared object types (PR 2674)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; PR 2674 origin/main...b029e1e03; spec/functional/FR-569, FR-104, FR-105, FR-106, FR-390, FR-517, FR-531; spec/test-cases/TC-1407, TC-1408; spec/decisions/ADR-013 QC-30"
review_set: subset
---

## Summary

Ticket: QSL-1400. Cross-requirement consistency of the new FR-569 and its amendments. The identities/4 member count (seven: format, declarations, fields, source_records, source_unions, source_operations, source_objects) agrees across FR-531, FR-517 and FR-390, and FR-517-AC-23 now requires all four source lists. The source-operation and source-object row keys and the refusal order (Package before Node, then Missing, Unknown, Foreign) also agree. Two existing requirements contradict the new source forms that TC-1407 E-S depends on, and one witness-refusal case does not have a single defined answer.

## Verdict

FAIL. FND-001 is high: fixture E-S's protocol cannot be emitted or read as written, so FR-390-AC-70 to AC-73 cannot pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A protocol `over (view: Node)` with a source-declared `Node` contradicts FR-383 and FR-218. Both write `owner_model` only for an object type of a selected model, and refuse any other `over` as `NoOwnerModel`. IR's reader also refuses an `owner_model` that `owners.recover` cannot resolve to a selected row. Neither FR-383, FR-218 nor QC-30 is amended, so TC-1407 E-S never yields a readable package. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:135-141; spec/test-cases/TC-1407-evaluate-a-source-declared-operation-call-from-the-reader-view.md:36-46; spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md:59-75; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:274-278 |
| FND-002 | medium | FR-569 says a one-segment type name resolves only to a source object type and is otherwise refused `missing-name`, and it lists `role R on Node` as a source form. FR-218 admits `role w on M`, where the one-segment `M` is a model alias. FR-569 makes no exception and FR-218 is not amended. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:210-218; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:263-271 |
| FND-003 | medium | FR-569-AC-8's foreign case, the `Order` row of S emitted under another source authority, carries the same `structural_node`, because node ids key on the package name and not the authority. Its only distinguishing trait is a disagreeing authority, which FR-569 defines as `UnknownIdentityWitness`. If the row replaces S's own `Order` row, `MissingIdentityWitness` also arises and ranks before Foreign in FR-390's reason order. "ForeignIdentityWitness only" is then not the defined outcome. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:315-321; spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:360; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:456-462 |

## Scope and evidence

Direct review. No AssuranceProfile requires an enforced spec-review run for this repository. IR evidence was read on quire-contract-ir origin/main a156efda: `protocol_body.rs` around line 6512 checks `owner_model` and calls `owners.recover`, and `recover` requires `owner_by_key`, which only selected rows satisfy (`model_members.rs:2997`).

- FR-569 (examined): the object, population and operation declaration, the resolution rules and the identity rows.
- FR-104 (examined): the one-segment context amendment, which is consistent with FR-569.
- FR-105-AC-18 (examined): a source anchor at `Order`. See SR failure-domain FND-001.
- FR-106 (examined): snapshot source identity and the conditional `model` member, which are consistent with FR-569.
- FR-390 view-creation paragraph and AC-70 (examined): consistent with the FR-569 rows. AC-67 order (examined): consistent.
- FR-531 identity-file table (examined): seven members, consistent. FR-517 row and AC-23 (examined): consistent.
- TC-1407 E-S (examined): FND-001. TC-1408 (examined).
- ADR-013 QC-30 (examined): see SR scope-boundary.

## Dispositions

Disposition pass 1, checked against fix commit ba4ea0fcd.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-383 and FR-218 are amended. S4 now refuses a protocol over a source type typed, naming `SourceOwnerModel` (FR-383-AC-67, FR-569 interim table, QC-30 item 3). FR-390-AC-70 to AC-73 are split into "now" and "waits" parts, and the "now" parts run over fixture S. |
| FND-002 | fixed | FR-569:152-156 and 234-237 add the `role w on M` exception. FR-218 is amended, and FR-218-AC-39 tests both kinds of role in S-M. |
| FND-003 | fixed | FR-569:375-391 and FR-390 now classify rows in order: Foreign by an authority of no `SourceOwner`, then Unknown, then Missing for a type no admitted row names. AC-8's foreign case supplies S's complete rows plus the foreign row, so it gives Foreign only. |

Disposition pass 2, checked against fix commit b3e72440c: no finding of this SR is outstanding, so this round adds no rows. The round-2 re-split checks (honest "now" cases, no remaining claim that IR admits anything now, the three old refusals removed everywhere, and the cross-AC consistency of FR-390-AC-70 to AC-73, FR-105-AC-18, FR-383-AC-67, FR-218-AC-39, TC-1407 and TC-1408) are clean.

Disposition pass 3, checked against fix commit 3ef786bab: no finding of this SR is outstanding, so this round adds no rows.

Disposition pass 4, checked against fix commit e9821eeed: no finding of this SR is outstanding, so this round adds no rows.
