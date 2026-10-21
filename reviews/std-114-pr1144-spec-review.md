---
id: SR-13672
title: 'Spec review of QSL PR #1144: FR-120 cites the FR-151 precondition combination (STD-114)'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@8a553371f; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md Contract clauses note; ticket STD-114'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-120
  type: reviews
---

## Summary

Ticket: STD-114. PR quire-spec-language#1144, head 8a553371fbd6213192467da480b5dc722d458518. It merges after quire-specification#318. Review set: base checklist plus the EARS, integrity and failure-domain lenses, in this one document.

The note is right that QSL implements reading (a). `view_clauses` (qsl-semantics/src/check/dispatch_link.rs:309-324) folds a member's `pre` clauses with `BinaryOperator::And`. `effective_terms` (checked_dispatch.rs:460-540) then puts that one own term first and disjoins it with the parent's terms (checked_dispatch.rs:1043-1054). FR-094's E10/C6 `false or true` agrees. The citations to FR-151-AC-17 and TC-196 D09 resolve once #318 merges. Removing the old "QSL's inference" sentence is correct.

Out of scope (code outside this diff), noted for a follow-up: the module doc at qsl-semantics/src/check/checked_dispatch.rs:66-69 still quotes FR-151's old wording, "the disjunction of its own precondition clauses with ...".

Gates at the PR head:

- `make check-spec-validation`: exit 0, warnings only, none in FR-120.
- `tools/check-index-completeness.sh`: exit 0.

Other places in QSL that mention an effective precondition (FR-090, FR-094, FR-096, FR-107, FR-109, FR-146, FR-180, FR-187, FR-215, the ADRs and TC-407/415/418/472/792/1039/1072) use the term or describe the disjunction function. None of them restates the flat reading. No open QSL PR touches FR-120.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-120 Contract clauses effective-precondition note | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md | examined |
| FR-120 clause-set conjunction rule | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md | context_only |
| effective_terms | qsl-semantics/src/check/checked_dispatch.rs | examined |
| view_clauses | qsl-semantics/src/check/dispatch_link.rs | examined |
| FR-094 effective-precondition function bullet | spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The cross-reference "QSpec FR-151:88-90" points to the wrong text. FR-151 lines 88-90 are the field rows of the variance table. The refinement rule is at FR-151:121-122 on QSpec main and at 121-127 after #318. A reader who follows the reference lands on `value type`/`multiplicity`/`refinement`, not on the definition. Fix: cite the rule by name, as QSpec FR-151 `quire.model.conformance.refinement/v1` (FR-151-AC-17), without line numbers. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:366 |
| FND-002 | low | The note credits `effective_terms` (`checked_dispatch.rs:477`) with the whole combination. `effective_terms` starts at line 460, so line 477 is inside it. It only disjoins one own expression per member with the parent's terms. The conjunction of a member's own clauses happens earlier, in `view_clauses` (dispatch_link.rs:309-324). Fix: cite both, for example "`view_clauses` (`dispatch_link.rs`) conjoins the own clauses and `effective_terms` (`checked_dispatch.rs`) disjoins them with the redefined member's". | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:370-371; qsl-semantics/src/check/dispatch_link.rs:309-324; qsl-semantics/src/check/checked_dispatch.rs:460 |
| FND-003 | low | The note writes its own copy of the formula, `(own) or (eff(r1)) or ...`, with `own` where QSpec has `own(m)` and the trailing `eff(rn)` left out. That is a second definition that can drift from the single one in QSpec. Fix: quote QSpec's formula verbatim, or cite it without restating it. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:367-369 |
| FND-004 | low | The note says the dispatch path takes FR-151's conjunction. But FR-120's own conjunction, in the next paragraph, evaluates each clause with a fresh meter, in ascending name order, and depends only on the set of outcomes. The dispatch path folds the clauses with an `and` that skips its right operand, in source-declaration order. With one clause undefined and another false, FR-120 gives `false` (not enabled), and the dispatch fold can give undefined. The note does not say the two differ. This is the QSL side of the conjunct-order gap in QSpec PR #318 (SR-13671 FND-001). Fix: once QSpec fixes the order, say whether the two paths agree, or state the difference. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:365-397; qsl-semantics/src/check/dispatch_link.rs:313-321 |

## Verdict

Not mergeable as is, because of FND-001. It is a one-line fix to a reference that points to the wrong text. FND-002 to FND-004 are low. FND-004 depends on how QSpec resolves SR-13671 FND-001, and may be deferred with that. The substance of the change is correct: QSL's dispatch code implements reading (a), and the note now cites the normative AC and vector instead of calling the rule QSL's own inference.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-120 now has two rules for the precondition conjunction. The `evaluate_clause_set` paragraph still covers "the applicable clauses of one kind" in ascending name order, with an outcome that depends only on the set, and its last bullet mentions preconditions ("none for a precondition"). The new paragraph at line 400 says FR-120 prescribes QSpec's source-order rule for the effective precondition. A reader cannot tell which rule governs preconditions. The implementation gap belongs in QSL-935, not in the spec text. Fix: limit the `evaluate_clause_set` paragraph to postconditions, or say that preconditions are conjoined as QSpec FR-151 defines. Keep the simulator's current behaviour only under Remaining work (QSL-935). | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:378-397; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:400 |

## Dispositions

Round 1, head d689242f8.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The line-number reference is gone. The note now cites QSpec FR-151's rule `quire.model.conformance.refinement/v1` by name (FR-151-AC-17 to AC-19, TC-196 D09 to D11). |
| FND-002 | fixed | The note now says `view_clauses` (`dispatch_link.rs`) conjoins the own clauses and `effective_terms` (`checked_dispatch.rs`) disjoins the result with the redefined member's. That matches dispatch_link.rs:309-324 and checked_dispatch.rs:460-540. |
| FND-003 | fixed | The local formula is gone. The note says "`own(m)` and `eff(m)` are QSpec's" and does not restate them. |
| FND-004 | fixed | FR-120:400 now states the difference: name order and set-based gives `false`, while source order gives `undefined` when the undefined clause comes first. Checked against FR-120's bullets, the statement is correct. Alignment is tracked as QSL-935 (Backlog) under Remaining work, which also covers the stale module doc in checked_dispatch.rs. The resulting contradiction inside FR-120 is FND-005. |

Round 2, head 8b5678943.

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | The set-of-outcomes bullet is now limited to postconditions ("Otherwise, for a postcondition"). A new paragraph (FR-120:404-409) gives preconditions QSpec's ascending-name short-circuit conjunction, and the mentions of source order are gone. The simulator's and dispatch path's current behaviour appears only under Remaining work (QSL-935). What remains is FND-006. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | FR-120 no longer says what an `Undefined` or `Refused` precondition clause does in the simulator. The bullet that made such a conjunction undecided and recorded `ModelFinding::ContractUndetermined` is now limited to postconditions, yet it still says "`candidate` is ... none for a precondition". The new precondition paragraph says only that evaluation stops at the first `false`. The note at FR-120:369-371 says an undefined clause sorting first makes the precondition undefined, but nothing says whether evaluation stops there, whether the application is enabled, or which finding is recorded. Fix: in the precondition paragraph, state that evaluation also stops at the first `Undefined` or `Refused` clause, that the application is then not enabled and the state gains one `ContractUndetermined { candidate: none }` for that clause (or whatever outcome is intended), and move "none for a precondition" there. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:395-402; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:404-409 |
| FND-007 | medium | QSL-935, the ticket that FR-120's Remaining work names, still states the old ruling. Its title is "conjoin own precondition clauses in source order", and its item 1 tells the simulator to move from ascending clause name to source-declaration order "as the dispatch path does". Under the new ruling the simulator already uses the right order, and the dispatch path's `view_clauses` (source order) is what must change. A coder who takes QSL-935 as written would implement the wrong order. Fix: in Linear, retitle and rewrite QSL-935 to match FR-120:603: both paths use ascending clause name with short-circuit; the simulator stops evaluating on the first false or undefined clause instead of deciding by the set of outcomes; `view_clauses` sorts by clause name; then reword the module doc and add a D10 test. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:603 |
