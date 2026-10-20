---
id: SR-13375
title: 'Spec review of PR #1074: FR-141 cites QSpec FR-377-AC-12/13, EARS'
type: SpecReview
analysis: ears-conformance
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

Ticket: QSL-544. PR: quire-spec-language#1074, head 673da86ac, scoped to `git diff refs/remotes/origin/main...673da86ac` (one commit, two files, +6/-7). Spec-only: FR-141 drops 'QSL's extension of FR-140's precedence' and cites QSpec FR-377-AC-12 and FR-377-AC-13; FR-141-AC-10 and TC-555 step 6 gain four cases. Checked against the merged QSpec (quire-specification main 0ad376803, STD-211 #292): FR-377 'Failure precedence across a step', 'Abstract budget stop', AC-12, AC-13, and TC-315 step 9 with its expected results. The four new AC-10 cases match TC-315 (k) to (n) exactly: (k) R and A1 undefined reports R; (l) A1 and P undefined reports A1; (m) A1 incomplete and H out of type is mapping-undetermined naming A1; (n) H and P undefined reports H. Together with the existing five cases they fix the order R < A1 < A2 < H < P, as AC-12 states it. The AC-12 citation is accurate: the read order, fault first and outside the order, undefined over undetermined, and the four between-group pairs are AC-12's. No 'QSL's extension' wording remains anywhere under spec/ (grep for extension wording finds only unrelated uses). EvaluationMeter stays as QSL's FR-120/FR-126 limit name inside LimitReached{EvaluationMeter, B, setting}; no QSL text attributes it to QSpec, and it instantiates QSpec's limit-reached{limit, value, setting} for the limit bounding one clause evaluation. FR-141-AC-9/10/11, FR-142, FR-144 and FR-145 otherwise agree with FR-377/FR-379 (fault failed via QSpec FR-383; budget stop settles FR-379's incomplete/limit-reached row; replay reuses check_step, so the reported read is the same in verdict and replay). EARS: the one edited SHALL statement is the Stopped bullet, which stays unwanted-behaviour ('If the abstract system's evaluation or expansion exhausts ... then it SHALL return StepVerdict::Stopped ...'); the added parenthetical carries no SHALL. The read-order bullet's SHALL is unchanged, and the replacement sentence after it is descriptive prose, not a requirement statement. The quire EARS lint run by make check-spec-validation reports nothing for FR-141 or TC-555. Gates on the head: make check-spec-validation exits 0 with 17 warnings, the same 17 as on main 9567ed601 and none in FR-141 or TC-555; tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2); the rollout note names quire 0.36.1/engine 0.50.1, so the binary is newer, not older.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-141-stopped-bullet | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-read-order-bullet | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean: no EARS defect in the edited statements.
