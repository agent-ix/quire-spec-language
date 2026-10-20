---
id: SR-13580
title: 'Spec review of PR #1103: history-field spec follow-ups from #1093, base'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@f481e5899; git diff refs/remotes/origin/main...f481e5899 (PR #1103, branch std-545-history-spec): spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md, spec/functional/FR-138-check-and-compute-history-fields.md, spec/test-cases/TC-554-an-undefined-mapping-refutes-a-refinement.md; compared with origin/qsl-545-history 498b438cf (qsl-semantics check/{check,state_model,mod,refinement,refinement/history}.rs, qsl-replay refinement/{history,step,map}.rs, tests/support/refinement_aux.rs), merged FR-135/140/141/142/363, TC-540, QSpec main 44afcf4 FR-375/FR-378, and SR-13460/SR-13461; ticket QSL-545'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-138
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-135
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-141
  type: reviews
---

## Summary

Ticket: QSL-545. PR: quire-spec-language#1103, head f481e5899, scoped to `git diff refs/remotes/origin/main...f481e5899` (one commit on main bdf90d48d; three files, +21/-8). Spec-only follow-ups from the #1093 code review (SR-13460, SR-13461). Code read on origin/qsl-545-history, which is still 498b438cf (the coder's fix round is not pushed yet). Verified against the code: (1) the fixture INV is `(1 mod x) + 1` (tests/support/refinement_aux.rs:585): 1 mod 1 = 0, +1 = 1, a member of Int[0, 1]; 1 mod 0 is undefined with `division-by-zero` (asserted at qsl-replay/tests/it/refinement_history.rs:358, same cause FR-141-AC-7 uses for `1 mod i`); integer `/` with no Rational expected type is ill_typed (FR-100-AC-5). TC-554 and FR-138-AC-5 agree; FR-141-AC-6 and FR-142-AC-8 cite the update by name. (2) `advance` evaluates updates over a post observation built from `carried` values, the pre-state history (qsl-replay/src/refinement/history.rs:203-249), so an outside-`pre` read gives the pre value; S3 admits it. (3) `Scope::history_names` is every history row name of the unit's refinements (qsl-semantics/src/check/refinement.rs:618, check/mod.rs:969), and missing-name fires only when the receiver's type has no such field and no navigable relationship end (check/state_model.rs:286-290); otherwise type-mismatch (FR-363-AC-2). (4) the code's `UnsupportedFeature` holds only `ProtocolAbstractSide`; `history-row` is gone. (5) `HistoryValues` is a BTreeMap on (ObjectKey{population, key}, field) and `key` encodes population, key, field, typed value in that order (history.rs:33-80, map.rs:33-40). (6) FR-141 already gives `NoAbstractMatch` for any row whose post candidate set is empty. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in FR-135, FR-138 or TC-554; tools/check-index-completeness.sh passes. No status wording in the diff. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). Methods: spec-review/base (SR-13580), spec-review/ears (SR-13581), spec-review/integrity (SR-13582); code-review and gap-analysis not run (no code or test file changed). This review: each follow-up against the code, and whether FR-138/FR-141 state the correct behaviour for SR-13460's two soundness bugs.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-138-AC-5 | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| TC-554-fixture | spec/test-cases/TC-554-an-undefined-mapping-refutes-a-refinement.md | examined |
| FR-138-S3-outside-pre | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138-S3-missing-name | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-135-support-set | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135-AC-5 | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-138-L5-key-order | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138-L5-candidate-rule | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138-L5-failures | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-141-read-order | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-candidate-sets | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-failure-kinds | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | SR-13460 FND-002 is not ruled out: neither FR-138 nor FR-141 says what the post-state mapping reads when a history update fails. FR-138 gives `step_history` an Err and no post history; FR-141 still maps the post-state with FR-140 and ranks a later undefined row above the update's undetermined outcome. Evaluating the rows over a stand-in value (the code keeps the pre value, history.rs:156-163, step.rs:333-366) is therefore allowed, and it refutes a step the update alone leaves Undetermined. Fix: state in FR-141 (read order) or FR-138 Layer 5 that a post-state row which reads a history field whose update failed is not evaluated at any stand-in and reads as undetermined, so the step settles `Undetermined(MappingUndetermined)` naming the update unless a read that does not depend on it is undefined; add the SR-13460 probe (`writes` out of type, `prev` reading `1 mod (1 - self.value + self.writes)`) as an AC. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:159 |
| FND-002 | medium | SR-13460 FND-001 is left to two rules that pull apart: 'An undecided contract conjunction of either model SHALL return Undetermined(UndecidedSuccessor)' and the Passing rule, which lists every accepted abstract transition identity. The Candidate sets bullet does not say what happens to a completion or successor that is undecided while another is accepted, so an engine may pass with the accepted members only and later refute on a set that lost a member (the code does: step.rs:786-811). Fix: state in the Candidate sets bullet that, with hidden fields, an undecided completion or successor of any member makes the step `Undetermined(UndecidedSuccessor)` (it is never dropped from the post set), and add SR-13460's `toss`/`reveal` probe as an AC. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:194 |
| FND-003 | medium | 'History values SHALL play no part in the candidate rule' is false: the candidate rule keeps the successors whose visible part equals map(s', h') (QSpec FR-378 Hidden fields), and h' is the history; the code compares against the key of the state mapped with the history (step.rs:342-370). The second clause restates FR-141 without settling FR-141's own overlap: its Failure kinds give `AbstractStepRejected` for an abstract-operation row and `NoAbstractMatch` for any row whose post set is empty, and a rejected hidden-field abstract-operation row is both (SR-13460 FND-003). Fix: drop the first clause, and state in FR-141's Failure kinds that a hidden-field row with an empty post set fails `NoAbstractMatch`, which takes precedence over `AbstractStepRejected`, with an AC (for example `Coin` with the concrete `reveal` setting a face no candidate allows). | spec/functional/FR-138-check-and-compute-history-fields.md:120 |
| FND-004 | medium | The support set paragraph now 'names three features', but QSpec FR-375 (Unbuilt features) fixes a closed set of four feature names, `history-row` among them, and defines the support set as the features a producer has built. Removing `history-row` from the list leaves FR-135 with three names that QSpec calls a closed four, and says nothing about history rows at all. Fix: keep the four names as QSpec's closed set, and state that this build's support set holds `history-row` (checked by FR-138) and omits the other three. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:173 |
| FND-005 | low | The new sentence 'a read of `x.f` where the receiver's type declares no field `f`, and `f` is a history field name of the unit, refuses `missing-name`' reads as unconditional, so it also covers the declaring refinement's own `self.last`, which is legal. It also omits the code's second condition: a relationship end named `f` navigates and is not refused (state_model.rs:286-290, FR-363). Fix: 'In any clause other than the declaring refinement's rows, a read of `x.f` where the receiver's type has no field and no relationship end `f`, and `f` is ...'. | spec/functional/FR-138-check-and-compute-history-fields.md:82 |
| FND-006 | low | The bullet sits under S3 but states what layer 6 evaluates. 'The value the pre-state holds' does not cover an object the step creates, which has no pre-state value; the code reads the initial literal there (`carried`, history.rs:118-140). Fix: keep 'S3 SHALL NOT refuse it' under S3, and move the evaluation rule to Layer 5 as 'reads the value the step started from: the pre-state's history value, or the initial literal for an object the step creates'. | spec/functional/FR-138-check-and-compute-history-fields.md:90 |

## Verdict

Not mergeable. Follow-ups (1), (2), (3), (5) match the code, and (1)'s expected values hold. (4) contradicts QSpec FR-375's closed feature set, and (6) is stated in a form that is false and does not settle FR-141. FR-138 and FR-141 are still silent on SR-13460 FND-002 and ambiguous on FND-001, so the spec does not yet rule out either soundness bug: FND-001 and FND-002 here ask for the Undetermined rule to be written down.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | The new 'Failed history update' rule says that if `step_history` fails, `check_step` returns `Undetermined(MappingUndetermined)` naming the update unless an independent read is undefined. It covers an update that evaluates undefined, so FR-141-AC-6 (the `inv` update undefined, `Undefined` at position 1) and FR-141-AC-10 ('`Undefined` at `H` for `H` and `P` both undefined') now contradict it. It also overrides the read order: FR-141-AC-10 names `A1`, not the update, for `A1` over-budget and `H` out of type. Fix: 'If an update is undetermined (refused, incomplete or out of type), then a post-state row that reads its history field SHALL count as an undetermined read at that row's place in the read order and SHALL NOT be evaluated at any stand-in value; the read-order rule above then decides.' | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:176 |
| FND-008 | low | 'If a completion or successor of any member is undecided' also covers an `any` row's undecided successor whose visible part differs from the mapped one, which could never join the set. The step then settles `Undetermined` where it is decidable. Fix: 'a completion or successor whose visible part equals the mapped one'. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:212 |
| FND-009 | low | 'The build holds a closed support set' followed by 'It names four feature names' still makes the support set the subject, and the new bullet then says the support set holds `history-row` only. Fix: 'QSpec FR-375's closed set has four feature names: ...', with the support set described by the new bullet alone. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:172 |

## Dispositions

Round 1, head e19819b95 (one fix commit over f481e5899; main is still bdf90d48d). The three round-0 SR files in reviews/ are byte-equal to the reviewer's; this file supersedes them. The coder's code branch origin/qsl-545-history is still 498b438cf, so the new FR-141 rules (failed history update, undecided completion, NoAbstractMatch precedence) state behaviour the code does not have yet; that is the coder's fix round, not a spec defect. Checked against 498b438cf: the outside-`pre` read of a created object gives the initial literal (`carried`, history.rs:118-140); missing-name needs no field and no navigable end (state_model.rs:286-290); the key order of FR-138-AC-7 (r.last, r.writes, s.last, s.writes) is the BTreeMap order on (ObjectKey, field). FR-138-AC-8's probe: at `write(r, 1)` the stand-in `writes` 0 makes `1 mod (1 - 1 + 0)` undefined, while the computed 2 makes it `1 mod 2` = 1, defined, so `Undetermined` is the honest verdict. Gates: make check-spec-validation exits 0 with the same 17 warnings, none in FR-135, FR-138, FR-141, TC-540, TC-543, TC-546 or TC-554; tools/check-index-completeness.sh passes.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
