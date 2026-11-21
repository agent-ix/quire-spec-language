---
id: SR-202611210906543
title: "failure-domain review of FR-1044 composition item through analyze (QSL-2739 PR1)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/filament-spec-language; spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md; PR3287 ae565a5dd5ece5f080d7e6625b7a32b2242a8d82 over f87b2a5c43"
review_set: subset
---

## Summary

Ticket: QSL-2739. Failure-domain review of FR-1044: identity confusion, unstated failure paths and ordering. The PR1 negative path (no provider, `unsupported`, never `proved`) is sound. Four gaps: the premise item's identity contradicts FSpec FR-491's slot key whenever a guarantee has `rely`/`depends` members; the cycle refusal and non-item-backed slots are unaddressed; duplicate selections are unspecified; and the claim that the composition "reads results this request produced" has no mechanism under FSpec FR-331.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Each premise item is defined as "those the same clause settles to when requested alone", but FSpec FR-491 makes a component-guarantee slot's item `G` over `C`'s subject checked under exactly FR-176's assumption set from its `rely`/`depends` members, the assumption set being part of the slot key ("a result for G checked under other assumptions, or under none, is another key"); for any guarantee with a rely or depends the stated item fills no slot. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:96 |
| FND-002 | medium | FR-1044 settles every admitted composition `unsupported`, but FSpec FR-491 "Dependency graph and cycles" refuses the composition item `invalid_model_binding`/`malformed-declaration` naming the cycle's guarantees when no selected profile declares FR-176's lattice, and derives an item-backed `fixed-point` slot when one does; FR-1044 states neither, nor that `rely`, `liveness-dependency`, `fairness` and `environment` slots derive no item. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:109 |
| FND-003 | medium | Duplicate identities are unspecified: the same compose name listed twice, two selected compositions sharing a component guarantee, or a caller item that also names a component guarantee; the spec does not say whether each gets its own derived premise item or one is shared. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:57 |
| FND-004 | medium | "the composition reads results this request produced" has no mechanism: FSpec FR-331 `premises` carries only `{role, key}` per slot, never a result, and a provider may not write `premise-unproved` (FR-331 "planner-only cause"); the composition item also sits at a lower `request_index` than the premises it reads, and the settlement order is unstated. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:60 |

## Verdict

Changes required. Recommended fixes: FND-001, define each premise item as the slot's item, `G` over `C`'s subject under exactly its FR-491 assumption set, carrying its own record, extent, trust basis and certification; "as requested alone" holds only for an empty assumption set (true of AC-1 and AC-6, which have no rely or depends). FND-002, add the cycle refusal and the no-item slot kinds, plus an AC for the cycle refusal. FND-003, state that each selected composition derives its own premise items (or a deduplication rule). FND-004, for PR1, restate as "the composition item's `premises` name the slot keys of these same-request items", and state that FSL settles the premise items before the composition is read; the reading mechanism for a routed provider is a PR2 or arch decision (see the scope-boundary review). Clean and examined: refused selections (unknown name, AC-4, AC-5) derive no premise item; a `refuted` premise not making the composition `refuted` (AC-6), consistent with FSpec FR-492 "never `refuted`".

## Scope and evidence

Spec-only; no code written.

- FR-1044 Decisions bullets 2-3, Outputs, Behavior bullets 1, 2, 7: examined.
- FR-1044-AC-1, FR-1044-AC-4, FR-1044-AC-5, FR-1044-AC-6: examined.
- FSpec FR-491 "Dependency graph and cycles" and "Derived item and slots" lines 106-154: context_only.
- FSpec FR-492 slot key lines 83-91: context_only.
- FSpec FR-331 `premises` lines 586-592, planner-only cause lines 842-849: context_only.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The assume-guarantee item is said to write 'the same record' as a routed one, that is 'settled at negotiation (FR-621 S6b)', with empty candidates. But it writes no FR-331 request item and is not routed, so no negotiation runs. FR-621's item echoes the request item that FR-620's request_item_wire writes, and for this item there is none. Its request member, stage, candidates and settled_by are therefore unspecified. Fix: state them for the planner-local item, e.g. no FR-331 request member, settled by FSL before negotiation, with the same diagnostic. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:128 |
| FND-006 | low | The request-stage premises refusal applies only when kind is composition, so `{"claim": "Sys", "premises": []}` (no kind) or premises on another kind fall outside it. A caller never writes premises, so the condition should not depend on kind. Fix: refuse a premises member on any analyze item at the request stage, as for request_index. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:162 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | An assumption-bound premise item is a temporal-satisfaction item, and EN-1 advertises that kind. Computed normally, its candidates therefore hold en-1, and FSpec FR-331-AC-4 says 'a single candidate settles through its negotiation arm'. FR-1044 states only 'unsupported ... without running an engine' and never says whether FSL writes empty candidates and settles it at negotiation (S6b) or disposes of it before negotiation (S4, as for the assume-guarantee item). Its request member, candidates and stage are unstated. Bounded fix: settle it before negotiation by a disposition at S4, with its request member as declaration_items writes it, no candidates, and settled_by disposition. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:168 |
| FND-008 | medium | LEAD DECISION. AC-5 says Capped's 'slot key and request identity differ' from the item alone. But Decisions lines 86-88 say the written item is 'a temporal-satisfaction item with no member that carries an assumption set'. AC-5's comparison is against a request over another program (test/part-b), so it passes for reasons unrelated to the assumption. Within one request (AC-8: Sys plus SysRely), the empty-set Capped and the assume-Bounded Capped have identical request members by that shape but different records. Meanwhile FSpec FR-492 lines 83-91 define a request-backed slot key as the FR-331 request identity without limits, and FSpec FR-491 lines 140-142 put the assumption set in the slot key. So no written member carries the distinction the standard requires. Options: (a) arch adds an FR-331 member carrying the assumption set; (b) FR-1044 states that the two request members are equal and only FSL's slot key, request identity plus the FR-491 assumption set, separates them, recording the FR-492 definition gap as remaining work, and AC-5 compares the two Capped items in one request. Recommend (b) for PR1. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:86 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | The premises equality and mismatch rules apply to every `kind` `composition` item, assume-guarantee included, since AC-2 admits SysAG. But 'the derived slot list' is defined only for the three routed rules (Behavior lines 178-182: 'the declaration's slots in order but for its fixed-point slot'). For `{"claim": "SysAG", "kind": "composition", "premises": P}` the comparison target is therefore undefined. It could be FR-491's slots without fixed-point, or FSpec FR-492's settlement `slots`, which are 'exactly the declaration's derived premise slots' and include fixed-point. Mechanical fix: define the derived slot list for every rule as FR-491's slots in order but for the fixed-point slot, and add the SysAG case to AC-4(a)/(b). | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:178 |

## Dispositions

Round 1, fix commit a0d6cd85503d2d32235981d82d262ac51bfb166e over ae565a5dd5ece5f080d7e6625b7a32b2242a8d82; verified against the committed text, not the author's report.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | round 2, fix commit 9884f67c9d |
| FND-006 | fixed | round 2, fix commit 9884f67c9d |
| FND-007 | fixed | round 3, fix commit 1598638ee1 |
| FND-008 | fixed | round 3, fix commit 1598638ee1 |
| FND-009 | fixed | round 4, fix commit 1513bee894 |
