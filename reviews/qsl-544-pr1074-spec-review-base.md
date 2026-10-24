---
id: SR-13383
title: 'Spec review of PR #1074: FR-141 cites QSpec FR-377-AC-12/13, base'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@673da86ac; git diff refs/remotes/origin/main...673da86ac (PR #1074, base main 9567ed601): spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md, spec/test-cases/TC-555-a-refused-or-incomplete-mapping-row-leaves-the-refinement-undetermined.md; read against merged QSpec main 0ad376803 FR-377 and TC-315; ticket QSL-544'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-141
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-555
  type: reviews
---

## Summary

Ticket: QSL-544. PR: quire-spec-language#1074, head 673da86ac, scoped to `git diff refs/remotes/origin/main...673da86ac` (one commit, two files, +6/-7). Spec-only: FR-141 drops 'QSL's extension of FR-140's precedence' and cites QSpec FR-377-AC-12 and FR-377-AC-13; FR-141-AC-10 and TC-555 step 6 gain four cases. Checked against the merged QSpec (quire-specification main 0ad376803, STD-211 #292): FR-377 'Failure precedence across a step', 'Abstract budget stop', AC-12, AC-13, and TC-315 step 9 with its expected results. The four new AC-10 cases match TC-315 (k) to (n) exactly: (k) R and A1 undefined reports R; (l) A1 and P undefined reports A1; (m) A1 incomplete and H out of type is mapping-undetermined naming A1; (n) H and P undefined reports H. Together with the existing five cases they fix the order R < A1 < A2 < H < P, as AC-12 states it. The AC-12 citation is accurate: the read order, fault first and outside the order, undefined over undetermined, and the four between-group pairs are AC-12's. No 'QSL's extension' wording remains anywhere under spec/ (grep for extension wording finds only unrelated uses). EvaluationMeter stays as QSL's FR-120/FR-126 limit name inside LimitReached{EvaluationMeter, B, setting}; no QSL text attributes it to QSpec, and it instantiates QSpec's limit-reached{limit, value, setting} for the limit bounding one clause evaluation. FR-141-AC-9/10/11, FR-142, FR-144 and FR-145 otherwise agree with FR-377/FR-379 (fault failed via QSpec FR-383; budget stop settles FR-379's incomplete/limit-reached row; replay reuses check_step, so the reported read is the same in verdict and replay). Base review: wording, behaviour-not-changelog, and whether the cited QSpec rules are carried. The replaced sentence now describes the system ('This is QSpec FR-377's failure precedence across a step (FR-377-AC-12), which check_step implements') rather than the change. Methods: spec-review/base (this file), spec-review/ears (SR-13384), spec-review/integrity (SR-13385); code-review and gap-analysis were not run because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 17 warnings, the same 17 as on main 9567ed601 and none in FR-141 or TC-555; tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2); the rollout note names quire 0.36.1/engine 0.50.1, so the binary is newer, not older.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-141-read-order-bullet | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-stopped-bullet | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-AC-8 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-AC-9 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-AC-10 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| TC-555-step-6-expected | spec/test-cases/TC-555-a-refused-or-incomplete-mapping-row-leaves-the-refinement-undetermined.md | examined |
| QSpec-FR-377-precedence-across-a-step | quire-specification:spec/functional/analysis/FR-377-decide-model-refinement.md | context_only |
| QSpec-FR-377-abstract-budget-stop | quire-specification:spec/functional/analysis/FR-377-decide-model-refinement.md | context_only |
| QSpec-FR-377-AC-12 | quire-specification:spec/functional/analysis/FR-377-decide-model-refinement.md | context_only |
| QSpec-FR-377-AC-13 | quire-specification:spec/functional/analysis/FR-377-decide-model-refinement.md | context_only |
| QSpec-TC-315-step-9-expected | quire-specification:spec/test-cases/TC-315-refinement-undetermined-and-replay.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-141 now cites FR-377-AC-13 but does not state its precedence clause: 'an undefined read, an undetermined read or a fault in the same step settles by AC-12 instead of the stop', which FR-377 grounds in 'the step rules are decided only when every read returned a value'. The Stopped bullet says a budget stop is 'never Fault, Fails or Undetermined' and the read-order bullet says a failed read returns Fault/Undefined/Undetermined, with nothing ranking the two. The abstract precondition at `a` needs only R and the arguments, so an implementation may evaluate it before reading H and P; if it exhausts the meter while P is undefined, the two bullets give different verdicts (QSpec: refuted, reporting P). No QSL AC or TC case mirrors TC-315 (h), (i) or (j), so either order passes TC-555. State in FR-141 that check_step decides the step rules only when every read returned a value, so a fault, undefined or undetermined read outranks a budget stop, and add (h)-(j)-style cases (HeavyEnq with an undefined, an incomplete and a faulted read) to FR-141-AC-9 or AC-10 and to TC-555. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:202-209 |
| FND-002 | low | The fixture defines each read's choices as 'cheap, over-budget (costing more than the meter B, so incomplete), undefined, out of type or of an unmapped type', and the other cases use 'over-budget'. The new case 'A1 incomplete and H out of type' names a state, not one of the listed choices. Write 'A1 over-budget and H out of type' so every case picks from the fixture's list. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:243 |

## Verdict

Not mergeable until FND-001 is fixed or explicitly deferred: one medium (AC-13's read-over-stop precedence is cited but not stated or tested) and one low. The four new AC-10 cases and TC-555 step 6 match QSpec TC-315 (k)-(n) exactly.

## New findings (disposition pass 1)

Round 1, head bd198c950 (fix commit bd198c950 over 673da86ac). Gates: make check-spec-validation exits 0 with the same 17 warnings as main, none in FR-141 or TC-555; tools/check-index-completeness.sh passes; quire validate on FR-141 reports no new diagnostic. The fix commit also adds this review's round-0 SR files under reviews/; this file supersedes them.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The new FR-141-AC-9 cases and TC-555 step 5 make `A1` undefined or over-budget over `HeavyEnq`, but `HeavyEnq` is `RingIsQueue`, whose row is `put -> enq(self.ring, _)` (FR-141-AC-3): it has no argument expression called `A1`, and `_` ranges over the parameter domain and cannot be undefined or over-budget. `A1` is a name from the `Reads` fixture (AC-10). As written the cases cannot be built. Say which read is made to fail, for example FR-141-AC-7's argument `g(self.index)` (undefined at index 0) and that argument made over-budget, on the `HeavyEnq` postcondition. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:257 |
| FND-004 | low | The new cases cannot tell a correct engine from the one the new rule warns about. An abstract precondition or postcondition needs the arguments, so no engine can exhaust the abstract meter before it has read a failed argument; and a swapped system faults on `WrongSystem` before anything is evaluated, so it never reaches the meter (TC-315 (j) puts the fault in a post-state read instead). The rule's own example, the abstract meter exhausted before the post-state row `P` is found undefined, is not tested. To pin the rule, add a case whose abstract precondition is the heavy part (it needs only the receiver and arguments) with `P` undefined, expecting `Undefined` at `P`, and one with a fault injected into `P` (TC-555 step 4's mapping fault), expecting `Fault`. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:257 |

## New findings (disposition pass 2)

Round 2, head 105996b7a (fix commit 105996b7a over bd198c950). Gates: make check-spec-validation exits 0 with the same 17 warnings as main, none in FR-141 or TC-555; tools/check-index-completeness.sh passes. Renumbered: this review's ids were SR-13374 (base), SR-13375 (ears) and SR-13376 (integrity), which collided with PR #1073's SR-13374/SR-13375; they are now SR-13383, SR-13384 and SR-13385. Earlier comments on this ticket are edited to match.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The order-sensitive cases name a post-state row `P`, but `HeavyEnq` (`RingIsQueue`) has no row of that name; `P` is a `Reads` (AC-10) name. 'A post-state row `P` that divides by zero after `put`' leaves the row unnamed: an implementer must add a field and row to `RingIsQueue` (for example a `Queue` field mapped by `1 div (1 - self.size)`, defined at the empty queue and undefined after `put(1)`). It also leaves open whether the injected-fault case keeps the heavy precondition; without it that case cannot tell a correct engine from one that stops first. Name the row and say the fault case keeps the heavy precondition. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:257 |

## Dispositions

Round 1, head bd198c950 (fix commit bd198c950 over 673da86ac). Gates: make check-spec-validation exits 0 with the same 17 warnings as main, none in FR-141 or TC-555; tools/check-index-completeness.sh passes; quire validate on FR-141 reports no new diagnostic. The fix commit also adds this review's round-0 SR files under reviews/; this file supersedes them.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |

Round 2, head 105996b7a (fix commit 105996b7a over bd198c950). Gates: make check-spec-validation exits 0 with the same 17 warnings as main, none in FR-141 or TC-555; tools/check-index-completeness.sh passes. Renumbered: this review's ids were SR-13374 (base), SR-13375 (ears) and SR-13376 (integrity), which collided with PR #1073's SR-13374/SR-13375; they are now SR-13383, SR-13384 and SR-13385. Earlier comments on this ticket are edited to match.

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
