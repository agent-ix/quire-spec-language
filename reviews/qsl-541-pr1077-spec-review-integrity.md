---
id: SR-13388
title: 'Integrity analysis of PR #1077: cross-artifact consistency of population names in clause bodies'
type: SpecReview
analysis: integrity
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

Ticket: QSL-541. Integrity lane: every spec and ADR that refused or relied on population forms outside a refinement, checked against the new FR-391/FR-139 text. make check-spec-validation exits 0 and tools/check-index-completeness.sh passes; FR-391's new depends_on FR-139 edge resolves. The PR merges cleanly onto current main 505186892 (merged #1074, #1075, #1076).

Measured on main 40f8707 (holds merged #1066; #1077 changes no code) with a throwaway probe test in qsl-analyze/tests/it/counter.rs over the `test/counter` domain (`Counter`, population `counters`, field `value: Int[0, 3]`, operation `inc`), patch kept at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-probe-patch.diff, output at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-probe.log; the clone was reverted. S1 parses every new fixture in the parenthesised spelling: `forall(o in M::counters : e)`, the FR-139 forms `count(o in M::counters where c)`, `exists(...)`, `sum(o in M::counters where c: e)`, `seq(i in lo .. hi: e)`, a function body and a temporal `holds` atom. The shorthand `forall o in M::counters : e` fails S1 (`expected (`) and a temporal clause with no `over` fails S1 (`expected over`). At S3 every population use outside a refinement refuses today with `PopulationFormOutsideRefinement` (= `unsupported_construct`/`expression-form`): the AC-1 precondition, the AC-7 postcondition, the FR-139 count/exists/sum/seq forms in clauses, a function body (both spellings), a temporal `holds` atom, and every AC-3 value use (`M::counters = M::counters`, `let`, `size`, bare body). `M::nothing` refuses `MissingName("M::nothing")` and `M::Counter = M::Counter` refuses `MissingName("M::Counter")`. AC-1's invariant `count<M::Counter>(o in allInstances<M::Counter>(M::counters) : true) < 4` refuses `IllTyped(TypeMismatch)` before reaching the population, because `count<N>` takes the result integer type (FR-091, FR-093), not the element type (pre-existing text, unchanged by this PR). The implied S3 change: the three refusal sites in qsl-semantics/src/check/check/typing.rs (the `ExprNode::Name` arm that matches `known_populations` at :888, the `Seq` arm at :1257, `population_form` at :2590) stop gating on `self.populations.is_none()`; a non-refinement population table resolved from the unit's model aliases (typed `Set<Reference<U>>[0,N]` from FR-380's bound, not a symbolic universe) serves the quantifier source, `count`/`sum` source, FR-139 form and `allInstances`/`lookup` operand positions; every other position of the name must refuse `ill_typed`/`operator-ineligible`, which needs a position flag on the goal because today a refinement population name infers as an ordinary `Set` value anywhere. The refusal mapping in refusal.rs, the S6a arms `ProtocolClauseUnsupported::AllInstances`/`Lookup` (qsl-eval evaluate.rs:2057, :2081), the temporal letter evaluation, and qsl-semantics/tests/it/refinement_populations.rs (asserts FR-139-AC-2's old refusal) change with it.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-016-FE-4 | spec/decisions/ADR-016-state-model-finite-execution-mapping.md | examined |
| ADR-016-OR-6 | spec/decisions/ADR-016-state-model-finite-execution-mapping.md | examined |
| FR-135-refinement-forms | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| ADR-021-observation | spec/decisions/ADR-021-state-space-reduction.md | examined |
| TC-1076-title | spec/test-cases/TC-1076-a-state-clause-names-a-population.md | examined |
| TC-544-title | spec/test-cases/TC-544-population-valued-expressions-type-and-evaluate-inside-a-refinement.md | examined |
| FR-391-Inputs-Outputs | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-139-AC-2 | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| FR-104-population-bullet | spec/functional/FR-104-check-state-clauses.md | examined |
| FR-265-fixture | spec/functional/FR-265-derive-a-state-clause-separating-witness.md | examined |
| FR-107-allInstances | spec/functional/FR-107-evaluate-state-clauses-at-s6a.md | context_only |
| FR-139-S3 | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | context_only |
| FR-391-AC-7 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new FE-4 says the S6a refusal `ProtocolClauseUnsupported` 'is a defensive guard that FR-391's admitted names do not reach', that unreachability is 'verified by analysis', that 'This is not a gap' and that the change is 'a non-goal here', while also marking the reopen condition 'met by FR-391'. In the engine that refusal is the `AllInstances`/`Lookup` arm of the S6a evaluator (qsl-eval evaluate.rs:2057, :2081), and FR-107 ('the clause evaluator no longer refuses them as unsupported in a state clause') and FR-391's Evaluation require that arm to evaluate a named population. OR-6 and the line 'FE-1, FE-4's unreachability ... are verified by inspection or analysis' still rest on the old claim. Rewrite FE-4 as the reopened decision: state clauses read a named population at S6a per FR-391/FR-107, the refusal remains only for a population operand that admission did not supply, and OR-6/§10 drop the unreachability claim. | spec/decisions/ADR-016-state-model-finite-execution-mapping.md:299 |
| FND-002 | medium | The row-checking SHALL now reads '... population-valued forms (FR-139) in scope, admitting the forms a state clause body admits (FR-391)'. FR-391 refuses a population used as a value (`operator-ineligible`), while FR-139 types a population name inside a refinement as a `Set<Reference<T>>[0, n]` value usable in any row (RM-8, FR-139-AC-1). Read literally, the cite imports FR-391's refusal into refinement rows; it also drops the old basis, ADR-016 FE-4's field reads, `deref` and `reaches`. Cite FE-4 for the model reads and FR-139 for population names in a refinement, or say 'the forms FR-104 admits in a state clause body, with population names as FR-139 states'. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:130-133 |
| FND-003 | low | FR-139-AC-2 now reads 'An invariant of `R::Ring` reading `count(s in R::slots where s.ring = self)` checks (FR-391-AC-7)', but FR-391-AC-7's invariant reads `M::counters`, not `R::slots`, and TC-1076's fixture has no `R` alias. The behaviour is FR-391's, so FR-139-AC-2 and TC-544 step 2 now verify another FR's rule under FR-139's id. Either move the R::Ring case into FR-391-AC-7 (with R as a model alias of the unit and `slots` its population) and drop it from FR-139-AC-2, or make the two cases name the same expression. | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md:120 |
| FND-004 | low | Text this PR leaves still states the old rule. TC-544's title 'type and evaluate inside a refinement only' and description 'are refused in any other clause' contradict its own step 2. TC-1076's title 'names a population as the operand of allInstances and lookup' and description 'types as a population operand only' miss row 9. FR-391's Use case, Inputs ('A state clause body (FR-102) holding `allInstances<T>(e)` or `lookup<T>(e, r)`') and Outputs cover only the operand case. FR-104:136 says 'A clause names a population by `M::P` as the operand of `allInstances` or `lookup`'. FR-265:99 says 'A state clause cannot query a population (ADR-016 FE-4)'. ADR-021:76 lists 'a domain or `allInstances` operand' and omits `lookup`. Update each to the domain-or-operand rule. | spec/test-cases/TC-544-population-valued-expressions-type-and-evaluate-inside-a-refinement.md:3,13-16 |

## Verdict

Two medium and two low findings. Leftovers the planner ruled out of this PR, recorded for the follow-up: FR-150-AC-4/TC-566 write `forall p in config_history: ...` (shorthand that fails S1, unqualified name); FR-161-AC-2 on main (merged #1075) says its domain 'waits on population forms in clause bodies, PR #1077', which goes stale when this merges; FR-337/TC-1079 now carry 'today S1 requires over, STD-186' (merged #1075), out of scope here. Code text the implementation must change: typing.rs refusal sites, qsl-replay/src/result/wire/value.rs:1202 and spine/clause/tests/witness.rs:6 comments, flag_world.rs:11 and :21 doc comments, refinement_populations.rs's FR-139-AC-2 test. ADR-024 and QSL FR-087/FR-088 make no population claim. FR-391-AC-1's `count<M::Counter>(...)` is ill-typed (pre-existing, unchanged).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | FR-391 defines a population position as 'the body of a state clause (an invariant, a precondition, a postcondition or a step's clause, FR-102) or a `holds` atom'. FR-366 also checks invariants, declared on a model type's definition, and refuses a population read in a record or event type's invariant with `unsupported_construct`/`expression-form` (FR-366:103-107), while for an object type it binds `self` 'as FR-104 binds `self` for a unit invariant'. A reader cannot tell whether `M::P` resolves in an object type's own invariant, or whether FR-366's refusal or FR-391's `operator-ineligible` wins for a record type. Say that FR-366's object-type invariants are (or are not) population positions and that FR-366's refusal holds for record and event types. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:37-42 |

## Dispositions

Disposition pass 1 against fix-round head 95822c0db (merge of main 505186892 plus the fix commit). Each outcome was checked in the spec text at that head, and the fixtures were re-measured with S1/S3 on main 505186892 (probe patch /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-r1-probe-patch.diff, output /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-r1-probe.log; clone reverted): every new AC-7/AC-8/row 9-10 fixture parses; `bound B on M::counters [3]` checks; the domain uses, the temporal `holds` atom, the function-body name and the `seq` in an invariant all refuse today with `PopulationFormOutsideRefinement` (expected until FR-391 is built; AC-8's `missing-name` and `seq` `expression-form` are the implied S3 change); `M::Counter = M::Counter` refuses `MissingName("M::Counter")` as AC-3 now says. make check-spec-validation exits 0 with the same 17 pre-existing warnings, check-index-completeness passes, and the head merges cleanly onto main.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |

### Disposition pass 2

Disposition pass 2 against fix-round head f394d6817 (main 94020c902 merged in). Re-measured at S1/S3 on main 94020c902 (probe patch /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-r2-probe-patch.diff, output /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-r2-probe.log; clone reverted): AC-7's `count(o in M::counters where o.value = 0) < 4` (with and without `bound B on M::counters [3]`) and AC-1's `forall(o in allInstances<M::Counter>(M::counters) : o.value < 4)` both parse and both reach the population at S3, refusing only with today's `PopulationFormOutsideRefinement` (the FR-391 gate, depth 1 at the form and depth 2 at the operand), with no `IllTyped` before it. The `Int[0,3]` count bound cannot be measured until FR-391 is built. No `count<M::...>` remains under spec/. make check-spec-validation exits 0 (17 pre-existing warnings), check-index-completeness passes, the head merges cleanly onto main.

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | still-open | The new Description sentence settles record and event types: their FR-366 invariant 'is not a population position: it keeps FR-366's refusal'. It does not settle an object type's (or state machine's or process's) own type-level invariant, which FR-366 evaluates over every object of the observation and binds 'as FR-104 binds `self` for a unit invariant'. 'Unit-level' excludes it from population positions, but no rule gives the outcome of `M::P` there: FR-366's refusal covers only record and event types, and FR-391's only explicit refusal outside a population position is for a function body. State the outcome (for example `missing_declaration`/`missing-name` at the second segment, as for a function body). |
