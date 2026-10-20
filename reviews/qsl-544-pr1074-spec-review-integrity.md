---
id: SR-13376
title: 'Spec review of PR #1074: FR-141 cites QSpec FR-377-AC-12/13, integrity'
type: SpecReview
analysis: integrity
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

Ticket: QSL-544. PR: quire-spec-language#1074, head 673da86ac, scoped to `git diff refs/remotes/origin/main...673da86ac` (one commit, two files, +6/-7). Spec-only: FR-141 drops 'QSL's extension of FR-140's precedence' and cites QSpec FR-377-AC-12 and FR-377-AC-13; FR-141-AC-10 and TC-555 step 6 gain four cases. Checked against the merged QSpec (quire-specification main 0ad376803, STD-211 #292): FR-377 'Failure precedence across a step', 'Abstract budget stop', AC-12, AC-13, and TC-315 step 9 with its expected results. The four new AC-10 cases match TC-315 (k) to (n) exactly: (k) R and A1 undefined reports R; (l) A1 and P undefined reports A1; (m) A1 incomplete and H out of type is mapping-undetermined naming A1; (n) H and P undefined reports H. Together with the existing five cases they fix the order R < A1 < A2 < H < P, as AC-12 states it. The AC-12 citation is accurate: the read order, fault first and outside the order, undefined over undetermined, and the four between-group pairs are AC-12's. No 'QSL's extension' wording remains anywhere under spec/ (grep for extension wording finds only unrelated uses). EvaluationMeter stays as QSL's FR-120/FR-126 limit name inside LimitReached{EvaluationMeter, B, setting}; no QSL text attributes it to QSpec, and it instantiates QSpec's limit-reached{limit, value, setting} for the limit bounding one clause evaluation. FR-141-AC-9/10/11, FR-142, FR-144 and FR-145 otherwise agree with FR-377/FR-379 (fault failed via QSpec FR-383; budget stop settles FR-379's incomplete/limit-reached row; replay reuses check_step, so the reported read is the same in verdict and replay). Integrity: citation accuracy (AC numbers and wording), cross-repo edges, and consistency of FR-141-AC-9/10/11 and FR-142/144/145 with QSpec FR-377/FR-379. Gates on the head: make check-spec-validation exits 0 with 17 warnings, the same 17 as on main 9567ed601 and none in FR-141 or TC-555; tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2); the rollout note names quire 0.36.1/engine 0.50.1, so the binary is newer, not older.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-141-read-order-bullet | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-stopped-bullet | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-relationships | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-AC-10 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| TC-555-step-6-expected | spec/test-cases/TC-555-a-refused-or-incomplete-mapping-row-leaves-the-refinement-undetermined.md | examined |
| QSpec-FR-377-AC-12 | quire-specification:spec/functional/analysis/FR-377-decide-model-refinement.md | examined |
| QSpec-FR-377-AC-13 | quire-specification:spec/functional/analysis/FR-377-decide-model-refinement.md | examined |
| QSpec-FR-377-abstract-budget-stop | quire-specification:spec/functional/analysis/FR-377-decide-model-refinement.md | examined |
| QSpec-TC-315-step-9-expected | quire-specification:spec/test-cases/TC-315-refinement-undetermined-and-replay.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The parenthetical quotes as AC-13 a rule that AC-13 does not contain. 'Every clause evaluation of either subject gets a fresh meter' is FR-377's 'Abstract budget stop' paragraph; AC-13 states the settlement (incomplete, resource-incomplete, limit-reached{limit, value, setting}, never failed/refuted/mapping-undetermined/undecided-successor) and the AC-12 precedence. The citation also follows a sentence that covers max_candidates, which AC-13 does not address (QSpec FR-378's budgets do). Cite 'QSpec FR-377 "Abstract budget stop" and FR-377-AC-13' (with FR-378 for max_candidates), and rewrap the 240-column line to the file's width. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:202-209 |
| FND-002 | low | '(a fault takes no part in the order, QSpec FR-377-AC-12 and FR-383)' leaves FR-383's repository to inference, and QSL has its own unrelated FR-383 (emit a checked protocol as v2 protocol nodes). Elsewhere FR-141 writes '(QSpec FR-383)'. Write 'QSpec FR-377-AC-12 and QSpec FR-383'. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:153-168 |
| FND-003 | low | FR-141 states that check_step implements QSpec FR-377's precedence and budget stop, but its relationships carry no quire-specification edge. QSL's convention is an FR-level `ix://agent-ix/quire-specification/FR-NNN` depends_on edge for each QSpec requirement an FR implements (148 of 356 FRs, 290 depends_on edges; siblings FR-120 and FR-125 carry them); no QSL file targets a QSpec AC id. The gap predates this PR and is shared by FR-136 to FR-145, but this PR makes the dependency normative. Add depends_on edges to QSpec FR-377, FR-378 and FR-383. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:5-25 |

## Verdict

Mergeable: three low findings (one citation quotes the body rather than AC-13; a bare FR-383; no cross-repo depends_on edge). The AC-12 citation, the (k)-(n) cases and the EvaluationMeter naming are accurate.
