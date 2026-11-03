---
id: SR-20261103150648
title: "spec-review of QSL-1524 FR-507/FR-057 declaration-family applicability"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #2201 at 8a2223ad2; spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md, spec/functional/FR-057-admit-shared-capability-kinds.md, spec/test-cases/TC-1302-apply-each-capability-kind-to-its-declaration-family-only.md, spec/test-cases/TC-971-capability-family-projection-matches-fr-290.md, spec/test-cases/TC-115-preserve-composed-admission-stages.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-507
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: references
---

## Summary

Ticket: QSL-1524. Spec review (base, integrity and EARS on the added SHALLs) of quire-spec-language PR #2201 at 8a2223ad2 over `git diff origin/main...HEAD` (5 files). Checked against QSpec FR-290 "Declaration-family applicability" (family table, claim-form assignment rows with the "Declaration family" column, `item_kind` table, node-to-family table, refusal order and record), QSpec FR-375-AC-8, and the QSL FRs FR-507 cites (FR-062, 104, 124, 144, 146, 165, 173, 180, 186, 211, 224, 231, 232, 246, 447) plus FR-147, FR-281 and FR-499.

Re-measured, not taken from ticket or PR text:

- FR-507's seven-family table equals QSpec FR-290's family table row for row (labels, kinds, `finite-replay` on `protocol` only, `state-model` = `operation-contract` only).
- FR-507's claim-form to family table agrees with every QSpec claim-form row whose family is not none, and each cited QSL FR governs the form it is cited for (FR-104 state clauses; FR-124/FR-211 deadlock-freedom over model/protocol; FR-224 race-freedom; FR-232 time-lock-freedom; FR-165 state-graph; FR-173 hyper and step-relation classification; FR-180 relation code claim; FR-144 whole refinement, `temporal-satisfaction`; FR-146 per-step and initial-state `operation-contract` records anchored on the refinement node; FR-186 probabilistic; FR-246 task sets; FR-231 timed; FR-447 hybrid).
- AC-1 count: per-family applicable kinds 1+1+1+2+7+1+1 = 14; 12 x 7 = 84; 84 - 14 = 70. Correct.
- (3a) is already specified: FR-180 "Hand-over" requires S4 to emit one QSpec FR-477 `claim`/`step_relation` node for every checked relation clause over model operations, whether or not the code claim is recorded (FR-180-AC-4, TC-1143; FR-362 point 8, FR-362-AC-7). No new emission requirement is needed.
- (3c) Family-level applicability matches QSpec FR-290's wording ("the kinds that apply are exactly the kinds ... assigned to the rows whose Declaration family cell names that family"; refusal keyed on the claim's family, not its form).
- (3d) QSpec maps a "relation-family refinement gate between two operation contracts" to `relation-claim`; FR-507's mapping of FR-057's ownerless refinement row to `relation-claim` agrees.
- (4) The placement agrees with QSpec's order and record (inapplicable-kind precedes absent-extent and the candidate table, so re-settling in `analyze` yields what a conforming `negotiate_*` gives) but contradicts FR-281 as written (FND-001).
- (5) No residual "exactly one family"/"every other family"/72-pair text in spec/, docs/ or plan/; TC-115 and TC-971 edits agree with FR-507. FR-499 retains a stale successor cell (FND-004).
- `make check-spec-validation` exit 0; no diagnostic names a touched file.

## Examined scope

- FR-507 Description, Inputs, Outputs (examined).
- FR-507 "Families and their kinds" table and the sentence at line 78 (examined): "Each family's kinds are exactly the kinds FR-057's claim-form table assigns to the claim forms that family holds."
- FR-507 "QSL claim forms and their families" table (examined), each row against QSpec FR-290 and the cited QSL FR.
- FR-507 "Where the family is read" (examined): `item_kind` precedence, node-to-family rule, no-family rule.
- FR-507 "The refusal", Order, Record, Scope bullets (examined) against QSpec FR-290 and QSL FR-281 Behavior lines 213-230.
- FR-507-AC-1 .. FR-507-AC-10 (examined).
- FR-057 "Kind applicability" section, claim-form table, FR-057-AC-10, FR-057-AC-11, FR-057-AC-13, Dependencies note on #210 (examined).
- TC-115 Expected Results, TC-971 Description/Procedure/Expected Results (examined).
- FR-147 "Requirement record" (context only, line 278): "The `requirements` hook SHALL write one requirement record of kind `refinement` for the declaration, in place of FR-144's `temporal-satisfaction` record".
- FR-499 QSL-841 table row "Capability-to-family applicability" (context only).

## Verdict

CHANGES REQUESTED. The family vocabulary, the claim-form mapping and the 84/14/70 count are correct and faithful to QSpec FR-290; (3a) is already specified by FR-180 and (3c)/(3d) are consistent. Two high findings: FR-507's override of caller-supplied dispositions contradicts FR-281's unamended Behavior, and QSL FR-147's model-to-protocol refinement requests kind `refinement` on a `relation`/`refinement` node, which FR-507 now refuses as inapplicable (and which QSpec FR-375-AC-8 already forbids). One medium: FR-507 derives family kinds and its "never refuse" guard from FR-057's claim-form table, which has no rows for the probabilistic, task-set, relation-clause, whole-refinement, state-graph, hyper or derived-item forms.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-507 "Order" (and AC-10) has `analyze` re-settle an item `inapplicable-kind` "whatever disposition the caller supplied", including `RequiresBound`, `Unsupported` and `InvalidRequest`, and before FR-281's `unknown-backend`/`inconsistent-candidates` checks on a `Supported` item. FR-281 is not amended and still says "If an item's disposition is `RequiresBound`, `Unsupported` or `InvalidRequest`, then `analyze` shall return ... a record settled by that disposition with its diagnostic" and "`analyze` ... settles no disposition". The two SHALLs contradict for every inapplicable item handed a non-`Supported` disposition, and the precedence against FR-281's own `unknown-backend`/`inconsistent-candidates` rules is unstated in FR-281. Amend FR-281's Behavior and routing prose to apply FR-507's check first. | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:139-148; spec/functional/FR-281-analyze-claims-with-in-process-engines.md:78-81,218-230 |
| FND-002 | high | QSL FR-147 writes a requirement record of kind `refinement` for a refinement whose abstract side is a protocol, anchored on the refinement node. That node is `relation`/`refinement` (QSpec FR-375), which FR-507 reads as `relation-claim`; `refinement` does not apply to `relation-claim`, so every FR-147 item settles `inapplicable-kind` under this PR and FR-147's ACs cannot pass. FR-507's "refinement between two protocols -> protocol" row does not cover it (the concrete side is a model). QSpec FR-375-AC-8 already says a whole refinement declaration requires `temporal-satisfaction` and never `refinement`. Bring FR-147 to `temporal-satisfaction` (FR-144's record) or state FR-147's claim form in FR-507 with QSpec agreement. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:78-81,276-289; spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:94-95 |
| FND-003 | medium | FR-507 line 78 ("Each family's kinds are exactly the kinds FR-057's claim-form table assigns to the claim forms that family holds") and the guard at line 136 ("`analyze` SHALL NOT refuse `inapplicable-kind` an item whose kind is the kind FR-057's claim-form table assigns to its claim form") both rest on FR-057's claim-form table, which has no row for probabilistic claims, task-set claims, relation-clause model/code claims, whole model refinement, per-step obligations, state-graph, `hyper`, timed/hybrid or derived deadlock/time-lock/race items. The derivation is false for `probabilistic`, `real-time` and `relation-claim`'s `temporal-satisfaction`, and the guard protects none of those forms. Point both sentences at FR-507's own claim-form table (with QSpec FR-290's kinds) or add the missing rows to FR-057 (and FR-057-AC-10). | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:78-79,136-137; spec/functional/FR-057-admit-shared-capability-kinds.md:171-186 |
| FND-004 | low | FR-499's QSL-841 table still gives the successor of "Capability-to-family applicability, `InapplicableCapability`, twelve-kind table (FR-057-AC-11)" as "none / PORT QSL-1507", while FR-507 states itself the spine successor and FR-057-AC-11 now defers to FR-507. Name FR-507/TC-1302 there. | spec/functional/FR-499-delete-each-retired-family-only-behind-its-named-successor.md:150 |
| FND-005 | low | EARS: the rewritten FR-057 statement "then the pair SHALL be recorded as an inapplicable capability" drops the subject the old text had ("the QSL composed linker SHALL record"), so it no longer says which stage records the refusal (composed linker here, spine `analyze` in FR-507). Restore an explicit subject. | spec/functional/FR-057-admit-shared-capability-kinds.md:205-209 |
