---
id: SR-1160
title: "QSL-370 spec review of ADR-023, FR-171 to FR-184 and their test cases"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1db46c41518fbe2be55b8dd64d2443ecd52f2710; diff 404a5a88...1db46c41; spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md; spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md; spec/usecase/US-021-compare-runs-over-every-behaviour-of-a-model.md; spec/functional/FR-171..FR-184; spec/test-cases/TC-596..TC-611, TC-615, TC-616; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-171
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-177
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-178
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-181
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: reviews
---

## Summary

Ticket: QSL-370, PR quire-spec-language#571 (draft), head 1db46c41,
reviewed as `git diff 404a5a88...HEAD` (the PR is stacked on a stale #562
base). One file for the review set: base checklist, EARS, integrity and
scope-boundary checks, the owner-ruling check, consistency with the QSpec
counterpart (QSpec FR-395 to FR-403 on `spec/wave-b-q3-possible-hyper`) and
technical soundness.

`quire validate --scope .` over the 38 changed files exits 0 with no error;
`tools/check-index-completeness.sh` passes. Every AC has an index row and a
behaviour TC. No ticket id appears outside References. No ID collides with
the sibling PRs (#564 uses TC-540 to TC-554, #569 TC-590 to TC-595 and
TC-612 to TC-614). FR-181 cites FR-166 to FR-170 in prose; those exist only
on #569's branch, so #569 must merge first.

**Rulings (read as data from the QSL-370 brief).** "Exhaustive" is the
method and the basis is `closed-scope`: HV-1, SE-5, RU-3 and the FR-182 V-1
row all say so, and nothing calls `exhaustive` a basis. Global
well-definedness for HP-5: SE-4, FR-181 and TC-615/TC-616 make `proved` wait
for a completed exploration, and a run stopped after a witness settles
`WellDefinednessUnchecked`. Undefined settles `refuted` with
`UndefinedEvaluation` and no new result kind (HV-8, RU-5, FR-179, FR-182,
FR-183); `MatchUndetermined` is kept for refused or incomplete `μ`. No fixed
caps: `max_witness_set` and `max_relation_tuples` are caller budgets with
published defaults that name the limit and how to raise it, and `max_depth`
stays a method parameter settling V-5. Exceptions are FND-007 (a depth bound
re-asserted) and FND-003 (echoed digests).

**Soundness checked and found correct.** §8.1: 14 leaky product states over
10 model pairs, two accepting SCCs, the `Lockstep` counterexample table, 8
secure states, and copy-swap classes 9 and 6. §8.2: `X_0 = {((1,0), g)}`,
`X_1` empty, 6 leaky states, 4 secure states. FR-179 tuple counts (64; 16 at
`max_depth` 1) and the `Cell` vectors of FR-179-AC-4/AC-5. FR-176-AC-2's
vacuous match, FR-176-AC-3 and FR-181-AC-3's weak-fairness verdicts on the
`reset` vault. HC-2's König argument and the HX-4 basis argument. The HP-1
model/code claim split (XC-1 to XC-5) stays inside QSL's boundary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | QSL's `HyperCounterexample` disagrees with the QSpec wire it cites. HX-1 and FR-183 carry an undefined evaluation as an optional `undefined` member on the four kinds, with traces for universal variables only. QSpec FR-400 defines a fifth `undefined` kind with one trace per variable, universal and existential, and an `UndefinedEvaluation` carrying variables, position, state keys and locus. With QSL's shape, an undefined evaluation on an HP-3 existential tuple (QSpec FR-399 covers "a universal or an existential variable") cannot be carried or replayed. Align HX-1, FR-183's types, FR-176 and FR-179 with QSpec FR-400, citing it rather than restating it. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:301; spec/functional/FR-183-replay-a-hyper-counterexample.md:66-79 |
| FND-002 | high | FR-181-AC-5's body `v.l @ b = 0 and 1 / (1 - v.l @ b) = 1` never evaluates undefined. Definedness follows left-to-right short-circuiting (FR-016), so at `l = 1` the left conjunct is false and the division is not reached. The AC's expected `Undefined` at the first `l = 1` state is wrong, and TC-615 inherits it. Put the division first, or drop the guard. | spec/functional/FR-181-check-a-single-existential-claim-through-the-possible-family.md:139; spec/test-cases/TC-615-a-single-existential-witness-proves-only-after-exploration.md:24 |
| FND-003 | medium | FR-183 Outputs retain "the source identity and digest, the `package_id`, the clause identities, each trace's initial state identity and digest, each replayed post-state digest". FR-128 at the base deleted exactly these echoes after SR-961 FND-009: they repeat the envelope and no requirement reads them from the result. They fail the value test and the no-provenance rule. Keep the evaluated value and `trace_position`, as FR-128 does. | spec/functional/FR-183-replay-a-hyper-counterexample.md:95-99 |
| FND-004 | medium | FR-177 (HP-3) and FR-178 (HP-6) give no undefined-evaluation behaviour, although HV-8 applies to both. The missing cases are a body letter, `μ_U` or `μ_E` undefined (refuted, first in canonical order), and `μ` refused or incomplete (`MatchUndetermined`). No AC or TC covers HV-8 for HP-3 or HP-6, or `MatchUndetermined` from any engine. QSpec FR-399-AC-5, AC-8 and AC-9 require these. | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:56-99; spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:58-97 |
| FND-005 | medium | FR-172 refuses a `match` conjunct that reads state as `ill_typed`, and FR-172-AC-2 tests that code. QSpec FR-395 Admission, which QSL cites as owner, refuses it `unsupported_construct`/`expression-form`. The refusal code QSL emits must match QSpec's. | spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:108-109,157 |
| FND-006 | low | QSpec FR-395 refuses an object parameter naming a population of an alias the clause does not bind (`ill_typed` at the parameter). FR-172 has no such refusal and no AC for it. | spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:90-93 |
| FND-007 | medium | FR-171 says "The S2 depth and node bounds of FR-091 SHALL apply to these forms". That re-asserts nesting depth as a limit kind in new text, which contradicts ADR-023 HC-10 ("no form, formula size, arity or depth is capped") and the owner rule that depth is never a limit kind. Delete the depth clause. | spec/functional/FR-171-build-forms-for-hyper-clauses-over-behaviours.md:86-87 |
| FND-008 | medium | The order of FR-176's pre-check ("every hyper form") and FR-181's phase 0 is unspecified for HP-5, and ADR-023 HC-8 only says "HP-5 is checked as §15 states". FR-181-AC-6 runs the 4-state secure vault with `max_states` 2. If the pre-check runs first, the subject alone reaches the limit and the item stops V-7 before phase 0, not `WellDefinednessUnchecked`. State whether the pre-check applies to HP-5 and where phase 0 sits relative to it. | spec/functional/FR-181-check-a-single-existential-claim-through-the-possible-family.md:140; spec/functional/FR-176-check-a-universal-hyperproperty-by-self-composition.md:62-68; spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:242 |
| FND-009 | low | The HP-6 row says "as HP-2", and HP-2 admits "any fairness sets". PA-1 and PA-3 refuse every fairness set on an `align skip` clause, and FR-172 follows PA-3. State that HP-6 has no fairness set. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:197 |
| FND-010 | low | A single-`exists` clause with `align skip` matches both the HP-5 row and the HP-4 row (PA-1: `align skip` with an existential variable). FR-173's classification order resolves it as HP-4; the ADR form table does not. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:195-196 |
| FND-011 | low | Status lists "TC-596 to TC-609". The record's test cases also include TC-610, TC-611, TC-615 and TC-616. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:44 |
| FND-012 | low | References say "Its QSpec half, QS-1 to QS-9, goes to a paired STD ticket". Name it: STD-136, QSpec FR-395 to FR-403. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:641 |
| FND-013 | low | TC-610's file slug `step-relation-undefined-tuples-settle-undefined` states the reading RU-5 rejected; its title and body say `refuted`. Rename the file. | spec/test-cases/TC-610-step-relation-undefined-tuples-settle-undefined.md:1 |
| FND-014 | low | FR-176-AC-4's undefined `match` fixture is "an integer division by an argument that is 0 there". Step arguments are `Option<T>` (HS-5), so that needs an unwrap, and the AC gives no `μ` expression. Two implementers would build different fixtures. Write out the conjunct. | spec/functional/FR-176-check-a-universal-hyperproperty-by-self-composition.md:127 |

## Verdict

Not mergeable as it stands. FND-001 (counterexample shape diverges from
QSpec FR-400) and FND-002 (an AC whose expected result contradicts the
evaluator's short-circuit rule) are high. FND-003 to FND-005, FND-007 and
FND-008 are medium. The rulings are carried correctly, the worked examples
check out, and every AC is traced to a behaviour TC.
