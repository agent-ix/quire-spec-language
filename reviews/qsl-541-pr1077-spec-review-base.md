---
id: SR-13386
title: 'Spec review of PR #1077: population names and forms in clause bodies (FR-391, FR-139)'
type: SpecReview
analysis: base
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

Ticket: QSL-541. The diff makes FR-391 the home of population names in clause bodies (title, description, a FR-139 edge, function bodies and holds atoms, typing as a quantifier or FR-139 form domain, AC-3 narrowed, new AC-7, TC-1076 rows 4 and 9), replaces FR-139's blanket `expression-form` refusal, flips FR-139-AC-2/TC-544 step 2 to 'checks', and updates ADR-016 FE-4, ADR-021 and FR-135. make check-spec-validation exits 0 with the same 17 pre-existing warnings as main and none on the changed files; tools/check-index-completeness.sh passes.

Measured on main 40f8707 (holds merged #1066; #1077 changes no code) with a throwaway probe test in qsl-analyze/tests/it/counter.rs over the `test/counter` domain (`Counter`, population `counters`, field `value: Int[0, 3]`, operation `inc`), patch kept at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-probe-patch.diff, output at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-probe.log; the clone was reverted. S1 parses every new fixture in the parenthesised spelling: `forall(o in M::counters : e)`, the FR-139 forms `count(o in M::counters where c)`, `exists(...)`, `sum(o in M::counters where c: e)`, `seq(i in lo .. hi: e)`, a function body and a temporal `holds` atom. The shorthand `forall o in M::counters : e` fails S1 (`expected (`) and a temporal clause with no `over` fails S1 (`expected over`). At S3 every population use outside a refinement refuses today with `PopulationFormOutsideRefinement` (= `unsupported_construct`/`expression-form`): the AC-1 precondition, the AC-7 postcondition, the FR-139 count/exists/sum/seq forms in clauses, a function body (both spellings), a temporal `holds` atom, and every AC-3 value use (`M::counters = M::counters`, `let`, `size`, bare body). `M::nothing` refuses `MissingName("M::nothing")` and `M::Counter = M::Counter` refuses `MissingName("M::Counter")`. AC-1's invariant `count<M::Counter>(o in allInstances<M::Counter>(M::counters) : true) < 4` refuses `IllTyped(TypeMismatch)` before reaching the population, because `count<N>` takes the result integer type (FR-091, FR-093), not the element type (pre-existing text, unchanged by this PR). The implied S3 change: the three refusal sites in qsl-semantics/src/check/check/typing.rs (the `ExprNode::Name` arm that matches `known_populations` at :888, the `Seq` arm at :1257, `population_form` at :2590) stop gating on `self.populations.is_none()`; a non-refinement population table resolved from the unit's model aliases (typed `Set<Reference<U>>[0,N]` from FR-380's bound, not a symbolic universe) serves the quantifier source, `count`/`sum` source, FR-139 form and `allInstances`/`lookup` operand positions; every other position of the name must refuse `ill_typed`/`operator-ineligible`, which needs a position flag on the goal because today a refinement population name infers as an ordinary `Set` value anywhere. The refusal mapping in refusal.rs, the S6a arms `ProtocolClauseUnsupported::AllInstances`/`Lookup` (qsl-eval evaluate.rs:2057, :2081), the temporal letter evaluation, and qsl-semantics/tests/it/refinement_populations.rs (asserts FR-139-AC-2's old refusal) change with it.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-391 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-Resolution-1 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-Resolution-2 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-Resolution-4 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-Typing-1 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-Observation | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-Lowering | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-Evaluation | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-Requirements | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-AC-3 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-AC-7 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| TC-1076-row-4 | spec/test-cases/TC-1076-a-state-clause-names-a-population.md | examined |
| TC-1076-row-9 | spec/test-cases/TC-1076-a-state-clause-names-a-population.md | examined |
| FR-139 | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| FR-139-clause-bodies | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| FR-139-AC-2 | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| TC-544-step-2 | spec/test-cases/TC-544-population-valued-expressions-type-and-evaluate-inside-a-refinement.md | examined |
| FR-139-bounds | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | context_only |
| FR-139-S3 | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | context_only |
| TC-1076-fixture | spec/test-cases/TC-1076-a-state-clause-names-a-population.md | context_only |
| FR-391-AC-1 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | context_only |
| FR-107-allInstances | spec/functional/FR-107-evaluate-state-clauses-at-s6a.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-391-AC-3 (and TC-1076 row 4) now say `M::Counter` used as a value refuses `ill_typed`/`operator-ineligible` 'because a population is no value'. `Counter` is a type, not a population, so the Resolution bullet of the same FR applies: 'If the first segment is a model alias and the second names no population of its domain package, then the assembler SHALL refuse `missing_declaration`/`missing-name` at the second segment'. Main refuses `M::Counter = M::Counter` in a precondition with `MissingName("M::Counter")`, and the replaced row expected `missing-name` too. Restore `missing_declaration`/`missing-name` at `Counter` for this case (in any clause body) and keep `operator-ineligible` for the four population value uses. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:185 |
| FND-002 | high | FR-391-AC-7 cannot be decided by hand from its fixture. (a) `count(o in M::counters where o.v = 0)` 'in an invariant of `R::Ring`': `R::Ring` is ADR-020's ring-buffer model, not part of the `Counter` unit (TC-1076's fixture has alias `M` only), so the clause's `on` fails to resolve before the form is reached. (b) 'typed `Int[0, N]`': no `N` is defined; the base fixture's `counters` has no authored maximum, so by FR-391's Typing the domain is the unbounded `Set<Reference<Counter>>` and the count bound is unstated (FR-139 bounds `count` by the universe size `n`, substituted only at refinement subject admission). (c) 'over a watcher': the fixture declares no `Watch` type or `watchers` population, and the clause needs an `over (w: M::Watch)` binding (S1 requires `over`). (d) The opening sentence lists a precondition and a function body, but no example covers them; TC-1076 row 9 adds a function-body case the AC does not state and drops the AC's `Int[0, N]` typing check. Write each case on the Counter unit (invariant `on M::Counter`, `bound B on M::counters [3]` if a typed bound is asserted, a named `Watch`/`watchers` in the fixture) and make row 9 match the AC case for case. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:189 |
| FND-003 | high | The new bullet admits `M::P` in a function body and in a temporal `holds` atom, but every rule that gives the name meaning is written for a state clause only: reads take 'the clause's observation' (`current`/`pre`/`post`), the `ProtocolClause` evaluator reads the population from 'the snapshot of the clause's observation', and the population becomes 'a domain of the clause's requirement record' and enters 'the footprint (FR-155) of a clause'. A function has no observation or record of its own and is called from clauses at different observations (and from refinement rows); a `holds` atom is read per trace state by the temporal letters (FR-125), not by S6a. QSpec FR-153 also says no population binding is ambient, and a function naming `M::P` makes one ambient. State, for each context, which state the name reads, how a not-`complete` population settles, and whose record and footprint gain the domain (for a function: the calling clause's), or keep FR-391 to state clauses and track the other contexts as a follow-up. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:106-110 |
| FND-004 | medium | FR-391's Typing now admits `M::P` as the domain of a quantifier or FR-139 form, read as `Set<Reference<U>>[0,N]`, but Lowering places the population-reference term only 'in the slot where FR-093's `AllInstances` and `Lookup` rows take `ref(p)`', the S4 `not-yet-implemented` refusal covers only 'a unit whose checked clause contains the term', and Evaluation covers only `allInstances` and `lookup`. Nothing says whether `forall(o in M::counters : e)` lowers as `allInstances<U>(M::counters)` (and so takes FR-084's coverage and incomplete rules) or as FR-139's population-reference node, which term S4 refuses, or that an incomplete population returns `Incomplete` for a domain read. Add one bullet each, for example 'a population domain lowers and evaluates as `allInstances<U>(M::P)`'. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:131-146 |
| FND-005 | medium | FR-139's new bullet admits 'a population name or one of the six forms in a clause body ... as FR-391 states' and says 'No `unsupported_construct`/`expression-form` refusal remains for a population form in a clause body'. FR-391 admits the name only as an operand or as the domain of a quantifier or form; `seq(i in lo .. hi: e)` ranges over integers and has no population domain, so FR-391 says nothing about it, yet FR-139 now implies it is admitted. Main refuses `seq` in an invariant with the same `PopulationFormOutsideRefinement`. State whether `seq` (and its declared-maximum rule) is admitted in clause bodies; and since FR-139's `count`/`sum` bounds use the universe size `n` fixed at refinement subject admission, state the bound that replaces `n` in a clause body (FR-380's maximum `N`, and the result type when there is none). | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md:73-79 |
| FND-006 | low | Requirement text should state the behaviour, not the edit. FR-391's Description says FR-139 'formerly refused them ... that refusal no longer applies' and that FR-391 'narrows FR-139' (it widens what is admitted); its Resolution bullet says a name outside a clause body 'SHALL resolve as it did before this requirement'; FR-139's Description says FR-391 'narrows this requirement's former refusal there' and its bullet ends 'No ... refusal remains'; ADR-016 FE-4 says '(met by FR-391)'. Replace with present-tense rules: 'Inside a refinement FR-139 governs population names; in any other clause body FR-391 does', and state what a `M::P` name in a declaration header or `on` target resolves to. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:61-66 |

## Verdict

Not mergeable as written: three high findings (FND-001 to FND-003) and two medium. The direction is coherent: a population is typed `Population<U>[N]` as an `allInstances`/`lookup` operand and read as `Set<Reference<U>>[0,N]` as a domain, every value use refuses `operator-ineligible`, which agrees with FR-084's operand rules and FR-139's six forms inside a refinement. AC-1, AC-2, AC-4 to AC-6 are unchanged and still consistent with the widened text; FR-139-AC-2's flip and TC-544 step 2 agree with each other. FR-337/TC-1079's 'clause with no over' is out of scope: #1077 does not touch them and merged #1075 already marks it 'today S1 requires over, STD-186'.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | The rewritten AC-7 and TC-1076 row 9 check `invariant Cap ... { count<M::Counter>(o in M::counters : o.v = 0) < 4 }` 'with the `count` result typed `Int[0,3]`'. `count<N>(x in c: p)` is the core collection form (FR-091, FR-093 `Query{visit: Count}`), whose type argument is the result integer type, so `count<M::Counter>` is ill-typed. Measured on main 505186892: that invariant refuses `IllTyped(TypeMismatch)` at the `count` node (depth 1) before the domain is reached, while `count<Tally>(o in M::counters : ...)` with `type Tally = Integer` reaches the population (depth 2, today's `PopulationFormOutsideRefinement`). Its result is then the declared `Tally`, not `Int[0,3]`. FR-391's new bound rule ('the bound of a `count` over a named population is `[0, N]`') fits FR-139's `count(x in P where c)` form. Write the case as `count(o in M::counters where o.v = 0) < 4` (or say which bound a `count<N>` over a population has) in AC-7 and row 9. AC-1's unchanged invariant has the same `count<M::Counter>` defect and should be fixed in the same edit. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:208 |

## Dispositions

Disposition pass 1 against fix-round head 95822c0db (merge of main 505186892 plus the fix commit). Each outcome was checked in the spec text at that head, and the fixtures were re-measured with S1/S3 on main 505186892 (probe patch /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-r1-probe-patch.diff, output /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1077-r1-probe.log; clone reverted): every new AC-7/AC-8/row 9-10 fixture parses; `bound B on M::counters [3]` checks; the domain uses, the temporal `holds` atom, the function-body name and the `seq` in an invariant all refuse today with `PopulationFormOutsideRefinement` (expected until FR-391 is built; AC-8's `missing-name` and `seq` `expression-form` are the implied S3 change); `M::Counter = M::Counter` refuses `MissingName("M::Counter")` as AC-3 now says. make check-spec-validation exits 0 with the same 17 pre-existing warnings, check-index-completeness passes, and the head merges cleanly onto main.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
