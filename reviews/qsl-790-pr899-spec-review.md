---
id: SR-11705
title: "Spec review of quire-spec-language PR #899: clause-set outcome over verdicts and the UpdateRaises reword"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@2cd406d93b36e2bfb970ebc4cc37df15c29dae15; spec/functional/FR-107-evaluate-state-clauses-at-s6a.md, spec/test-cases/TC-1039-a-clause-set-reports-its-results-together.md"
review_set: subset
---
# Spec review of quire-spec-language PR #899

## Summary

Ticket: QSL-790. One pass of spec-review over the PR diff, with the
contradiction, completeness, vector-soundness and EARS/AC-consistency checks.
Spec-only PR, so no code review. Only major issues block. Mediums and lows
are recorded in the Verdict.

Oracles, read-only on the reviewed head and on main: FR-090-AC-1, FR-100-AC-11,
FR-104 Definedness facts, FR-109 Behavior and FR-109-AC-7, FR-120 conjunction
and FR-120-AC-9, FR-301-AC-2, TC-468 step 7, TC-472, TC-452 step 4;
qsl-eval `value/expression/evaluate.rs` (dispatch guard, sum fold,
IEEE-to-rational), `tests/it/state_model_family.rs` (TC-792),
qsl-semantics `check/refusal.rs` obligations, quire-exact `outcome.rs`; the
clause-set code in PR #897 (`clause_set.rs`: private `Verdict`, `verdict`,
`combine`).

Examined:
- FR-107 Clause sets: the new Behavior bullet (combined outcome over verdicts,
  "no linked clause evaluates to `Undefined`") (examined)
- FR-107-AC-18, rewritten (examined)
- TC-1039 clause block, invocation table, step 6 and its expected result,
  steps 1 to 5 expected results against the `UpdateRaises` reword (examined)
- FR-107 outcome and postcondition-skip bullets; FR-301-AC-2; FR-109-AC-7;
  FR-120-AC-9 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new FR-107 bullet's claim "no linked clause evaluates to `Undefined`" is false against merged FR-301-AC-2. That AC, and its passing test `source_state_clause_retains_false_selected_precondition_as_model_undefined` (TC-792), evaluate the linked state clause `pre F ... { self.ready() }` through `evaluate_clause` to `FamilyResult::Undefined` with reason `precondition-false`. FR-109's Behavior counts that as a clause that "evaluates undefined". So the family-result X row, and a postcondition skipped after a not-completed-Boolean precondition, can be reached through the production `evaluate_clause_set`, not only "by giving that function the verdicts directly". The bullet's rationale (divisor, presence, reduction) omits dispatched calls, which FR-302 compiles in clauses. Fix: drop or narrow the universal claim (at most: no kernel `Undefined` from the ConfigVersion clauses), and back the family-Undefined X of AC-18 through `evaluate_clause_set` with a dispatched-call clause whose selected precondition is false (the FR-301 pattern). Keep the verdict-level unit test only for kernel `Undefined` and kernel `Refused`. | spec/functional/FR-107-evaluate-state-clauses-at-s6a.md:225-231 |
| FND-002 | high | The same bullet contradicts two merged ACs that still require a linked clause to evaluate to kernel `Undefined`. FR-109-AC-7 (TC-468 step 7) expects `Ratio`, `6 / (self.versionNumber - self.versionNumber) >= 0`, to report category `undefined`, `division-by-zero`, exit 10. FR-120-AC-9 (TC-472) expects `pre Low { sum<Tiny>(x in self.items: x) = 0 }`, evaluated through FR-107's `evaluate_clause_set`, to record `ContractUndetermined { .., Low, Undefined(SumOutOfDomain) }`. The repository's own test notes say a linked `sum` whose prefixes are not proved members is refused at checking. The PR says FR-109-AC-7 is left for a follow-up and does not mention FR-120-AC-9. After this merge the spec states both "no linked clause evaluates to `Undefined`" and two ACs whose only passing result is such a clause. Fix in this PR: move FR-109-AC-7 and FR-120-AC-9 to a reachable source (FR-301's `precondition-false`) or to the verdict level, or remove them. | spec/functional/FR-107-evaluate-state-clauses-at-s6a.md:228-229 |

## Verdict

FAIL, two blocking findings (FND-001, FND-002). Both have the same root
cause: the universal "no linked clause is Undefined" sentence. Narrowing it,
and using FR-301's dispatched `precondition-false` clause as the production
source of a not-completed-Boolean verdict, resolves both.

Answers to the brief:
(a) For kernel `Undefined`, the claim holds as far as traced. Division needs
a `Nonzero` obligation, `value(..)` needs `Presence`, `EmptyReduction` and
`NoneValue` come only from unlinked kernel evaluation, and a linked `sum` out
of domain is refused at check. `IeeeNotFinite` has no obligation (FR-090-AC-1),
but no float value reaches a clause: snapshots have no float form and float
literals are unrepresented (FR-091-OQ-5). For family `Undefined` the claim
is false (FND-001). It contradicts FR-109-AC-7 and FR-120-AC-9 (FND-002). It
does not contradict FR-146, FR-091 '/' or FR-100-AC-6.
(b) In PR #897 `combine`, `verdict` and `Verdict` are private to
`clause_set.rs`. The kernel `Undefined` and `Refused` X can be observed only
by a crate unit test that feeds constructed evaluations through `verdict`
into `combine`. That test is not vacuous: it fails on a wrong precedence, and
the TC-452 step 4 constructed-evaluation test is precedent. The family
`precondition-false` X can be observed through the public
`evaluate_clause_set` / `ClauseSetOutcome` (FND-001).
(c) Against Incomplete > False > Undecided > True, with the empty set True,
every listed vector is correct. Gaps are recorded below.
(d) Removing `up01` and `UpdateDivides` strands no spec reference. The only
other mention is PR #897's test module comment, which the code PR adapts.
The `UpdateRaises` reword keeps steps 1 to 5: up12 gives 2 - 1 = 1, true,
and up34 skips it after `UpdateAllowed` false. PR #897 records that the old
`pre(..) + 1` form was a type mismatch on this unit.

Recorded, not blocking:
- medium: AC-18 has no {not a completed Boolean, incomplete} vector. Every
  listed vector passes an implementation that ranks Undecided above
  Incomplete when no False is present.
- medium: the postcondition skip after a not-completed-Boolean precondition
  lost its only AC row (the old AC-18 first half). The skip is reachable
  (FND-001) and AC-13 and AC-14 cover only the skip after `false`.
- low: the bullet cites FR-104 "Definedness facts" for nonzero divisors and
  non-empty reductions. That section states only presence and interval
  facts, and the obligations come from `CheckMode::Linked`.

Clean: no roadmap language, no ticket ids, and no QSpec text quoted in the
diff. EARS form of the new bullet is acceptable apart from the false
rationale.
