---
id: SR-13387
title: 'EARS analysis of PR #1077: FR-391 and FR-139 statements on population names in clause bodies'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@7d3d73006; git diff refs/remotes/origin/main...7d3d73006 (PR #1077, merge base 9567ed601): spec/decisions/ADR-016-state-model-finite-execution-mapping.md, spec/decisions/ADR-021-state-space-reduction.md, spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md, spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md, spec/functional/FR-391-name-a-population-in-a-state-clause.md, spec/test-cases/TC-1076-a-state-clause-names-a-population.md, spec/test-cases/TC-544-population-valued-expressions-type-and-evaluate-inside-a-refinement.md; measured against main 40f8707 (holds merged #1066) at S1/S3; ticket QSL-541'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-391
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-139
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-135
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1076
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-544
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-016
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-021
  type: reviews
---

## Summary

Ticket: QSL-541. EARS lane over the new and edited SHALL statements of FR-391 and FR-139 and the ACs that verify them. quire validate reports no EARS or AC-shape warning on the changed files.

Measured on main 40f8707 (holds merged #1066; #1077 changes no code) with a throwaway probe test in qsl-analyze/tests/it/counter.rs over the `test/counter` domain (`Counter`, population `counters`, field `value: Int[0, 3]`, operation `inc`), patch kept at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-probe-patch.diff, output at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-probe.log; the clone was reverted. S1 parses every new fixture in the parenthesised spelling: `forall(o in M::counters : e)`, the FR-139 forms `count(o in M::counters where c)`, `exists(...)`, `sum(o in M::counters where c: e)`, `seq(i in lo .. hi: e)`, a function body and a temporal `holds` atom. The shorthand `forall o in M::counters : e` fails S1 (`expected (`) and a temporal clause with no `over` fails S1 (`expected over`). At S3 every population use outside a refinement refuses today with `PopulationFormOutsideRefinement` (= `unsupported_construct`/`expression-form`): the AC-1 precondition, the AC-7 postcondition, the FR-139 count/exists/sum/seq forms in clauses, a function body (both spellings), a temporal `holds` atom, and every AC-3 value use (`M::counters = M::counters`, `let`, `size`, bare body). `M::nothing` refuses `MissingName("M::nothing")` and `M::Counter = M::Counter` refuses `MissingName("M::Counter")`. AC-1's invariant `count<M::Counter>(o in allInstances<M::Counter>(M::counters) : true) < 4` refuses `IllTyped(TypeMismatch)` before reaching the population, because `count<N>` takes the result integer type (FR-091, FR-093), not the element type (pre-existing text, unchanged by this PR). The implied S3 change: the three refusal sites in qsl-semantics/src/check/check/typing.rs (the `ExprNode::Name` arm that matches `known_populations` at :888, the `Seq` arm at :1257, `population_form` at :2590) stop gating on `self.populations.is_none()`; a non-refinement population table resolved from the unit's model aliases (typed `Set<Reference<U>>[0,N]` from FR-380's bound, not a symbolic universe) serves the quantifier source, `count`/`sum` source, FR-139 form and `allInstances`/`lookup` operand positions; every other position of the name must refuse `ill_typed`/`operator-ineligible`, which needs a position flag on the goal because today a refinement population name infers as an ordinary `Set` value anywhere. The refusal mapping in refusal.rs, the S6a arms `ProtocolClauseUnsupported::AllInstances`/`Lookup` (qsl-eval evaluate.rs:2057, :2081), the temporal letter evaluation, and qsl-semantics/tests/it/refinement_populations.rs (asserts FR-139-AC-2's old refusal) change with it.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-391-Resolution-1 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-Resolution-4 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-Typing-1 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-139-clause-bodies | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| FR-391-AC-3 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-AC-7 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The Description defines the scope as 'any clause body: a state clause, a function body or a `holds` atom', but the first Resolution SHALL reads 'inside a clause body (a state clause, FR-102)', which defines 'clause body' as a state clause, and the fourth bullet then adds function bodies and holds atoms 'by the same rule'. FR-139's bullet and FR-391-AC-7 use 'clause body' in the wide sense. Two readers can disagree on whether a later 'clause body' rule (for example the observation and record rules) covers functions. Use one term: 'a state clause body' where only state clauses are meant, and list the three contexts once in one SHALL. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:97-99 |
| FND-002 | low | 'A `M::P` name outside a clause body (a declaration header or an `on` target) SHALL resolve as it did before this requirement' states its response by reference to history, so no test can be written from the spec alone. 'A function may also keep naming a population through a `Population<T>[N]` parameter' sits among SHALLs but has no SHALL and 'keep' is change-log wording. State the response (what the name resolves to, or which refusal) and move the parameter note to the Description or write it as 'The checker SHALL admit ...'. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:106-110 |
| FND-003 | low | 'A population name or one of the six forms in a clause body outside a refinement declaration SHALL be admitted as FR-391 states, wherever an ordinary quantifier domain is admitted' names no system subject (the checker), gives its response only by reference ('as FR-391 states'), and 'wherever an ordinary quantifier domain is admitted' fits the name but not a form, which is an expression, not a domain. The next sentence restates FR-391's `operator-ineligible` refusal, so one behaviour now has two owners, and the closing 'No ... refusal remains' is not an EARS statement. Reduce FR-139's bullet to a pointer in prose ('Outside a refinement declaration, FR-391 governs population names and forms') and keep the SHALLs in FR-391. | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md:73-79 |

## Verdict

One medium and two low findings. The edited Typing SHALL is a compound of an admission and an unwanted-behaviour refusal, a pattern FR-391 already used before this PR, so it is not raised. AC-3 and AC-7 are direct assertions of outcome.

## Dispositions

Disposition pass 1 against fix-round head 95822c0db (merge of main 505186892 plus the fix commit). Each outcome was checked in the spec text at that head, and the fixtures were re-measured with S1/S3 on main 505186892 (probe patch /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-r1-probe-patch.diff, output /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-r1-probe.log; clone reverted): every new AC-7/AC-8/row 9-10 fixture parses; `bound B on M::counters [3]` checks; the domain uses, the temporal `holds` atom, the function-body name and the `seq` in an invariant all refuse today with `PopulationFormOutsideRefinement` (expected until FR-391 is built; AC-8's `missing-name` and `seq` `expression-form` are the implied S3 change); `M::Counter = M::Counter` refuses `MissingName("M::Counter")` as AC-3 now says. make check-spec-validation exits 0 with the same 17 pre-existing warnings, check-index-completeness passes, and the head merges cleanly onto main.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
