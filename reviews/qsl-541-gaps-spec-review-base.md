---
id: SR-13377
title: 'Spec review of PR #1075: FR-161-AC-2 closed initial states and FR-391-AC-7 population forms'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@68ce9ac19; git diff refs/remotes/origin/main...68ce9ac19 (PR #1075, base main 9567ed601): spec/functional/FR-161-concretise-a-reduced-counterexample.md, spec/functional/FR-391-name-a-population-in-a-state-clause.md, spec/test-cases/TC-1076-a-state-clause-names-a-population.md, spec/test-cases/TC-585-symmetry-counterexamples-concretise-and-close.md; measured against main''s S1/S3 and qsl-analyze model_check (merged #1066); ticket QSL-541'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-161
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-391
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-585
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1076
  type: reviews
---

## Summary

Ticket: QSL-541. The diff edits FR-161-AC-2 (two initial states forming one orbit, the postcondition in `allInstances` form, and a one-initial-state case settling `SymmetryBroken`), adds FR-391-AC-7 and TC-1076 row 9, and adds TC-585 step 3.

Measured on main 9567ed601 (which holds merged #1066) with a throwaway probe test in qsl-analyze/tests/it over the `Token` domain of flag_world.rs (patch kept at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1075/probe-patch.diff, output at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1075/probe.log and probe2.log; the clone was reverted). (a) One initial state, `a.tok` false and `b.tok` true, full class `[a, b]`: `check_model` returns `Reduction{SymmetryBroken{population: holders, cause: InitialStatesNotClosed{initial: 0, generator: (a b)}}}` with `product_states: 0`. With both initial states (#1066's fixture) the quotient has one product state and the lasso starts at initial state 0 (quotient.rs, a_loop_that_needs_two_passes_closes_at_the_concrete_entry). (b) Every population form in a clause refuses today with `PopulationFormOutsideRefinement`, which maps to `unsupported_construct`/`expression-form` (FR-139): the rewritten post `forall(o in allInstances<M::Holder>(M::holders) : o != self implies o.tok)`, the bare `forall(o in M::holders : ...)`, a pre with `allInstances<M::Holder>(M::holders)`, `M::holders = M::holders`, a function body, and a temporal `holds` atom with `M::holders` in either spelling. FR-391 is not implemented yet (footprint.rs says so), so AC-7's first two clauses are future behaviour. Only an unqualified `holders` refuses `missing_declaration`/`missing-name` (MissingName("holders")). (c) The claim as FR-161-AC-2 writes it, `eventually holds(forall x in holders: x.tok)`, fails S1 (`expected (`), and a temporal clause with no `over` fails S1 (`expected over`).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-161-AC-2 | spec/functional/FR-161-concretise-a-reduced-counterexample.md | examined |
| FR-391-AC-7 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| TC-1076-row-9 | spec/test-cases/TC-1076-a-state-clause-names-a-population.md | examined |
| TC-585-step-3 | spec/test-cases/TC-585-symmetry-counterexamples-concretise-and-close.md | examined |
| ADR-021-SYM-4 | spec/decisions/ADR-021-state-space-reduction.md | examined |
| ADR-021-SYM-5 | spec/decisions/ADR-021-state-space-reduction.md | examined |
| ADR-021-RV-3 | spec/decisions/ADR-021-state-space-reduction.md | examined |
| FR-391-Typing | spec/functional/FR-391-name-a-population-in-a-state-clause.md | context_only |
| FR-391-Resolution-outside | spec/functional/FR-391-name-a-population-in-a-state-clause.md | context_only |
| FR-139-other-clause-bodies | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | context_only |
| FR-150-AC-4 | spec/functional/FR-150-check-the-symmetric-population-annotation.md | context_only |
| FR-153-AC-3 | spec/functional/FR-153-decide-each-fairness-on-the-annotated-quotient.md | context_only |
| TC-568-step-1 | spec/test-cases/TC-568-initial-states-must-be-closed-under-the-generators.md | context_only |
| FR-151-AC-3 | spec/functional/FR-151-admit-a-request-s-symmetry-declarations.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-161-AC-2 rewrites the postcondition into the admitted `allInstances<M::Holder>(M::holders)` form but keeps the claim `eventually holds(forall x in holders: x.tok)`. That atom names the population inside a temporal `holds`, which FR-391-AC-7, added in the same PR, says refuses (and FR-139 refuses as `unsupported_construct`/`expression-form`). Measured: the literal claim fails S1 (`expected (`, old quantifier spelling); respelled `forall(x in M::holders : x.tok)` or with `allInstances` it refuses at S3; and a temporal clause with no `over` fails S1 (`expected over`). So no unit can carry the AC's claim and TC-585 step 2 cannot be run as written. #1066's test works around this with `holds(w = w)` over a watcher and test-side letters. The AC must say how the claim is written (the bound-watcher form with the atom's reading stated, or an `over` binding), or the owner must admit populations in temporal atoms. FR-153-AC-3 has the same claim shape. | spec/functional/FR-161-concretise-a-reduced-counterexample.md:93 |
| FND-002 | high | FR-391-AC-7 says a temporal `holds` atom naming `M::counters` refuses `missing_declaration`/`missing-name`. FR-139 ("A population name or one of the six forms in any other clause body SHALL refuse `unsupported_construct`/`expression-form` at its span") prescribes a different code for the same input, and main refuses `M::holders` in a temporal atom with `PopulationFormOutsideRefinement` = `unsupported_construct`/`expression-form`, at the name. Only an unqualified `holders` gives `missing-name`; #1066's comment that S3 refuses `M::toggles` there with `missing-name` does not reproduce. No SHALL in FR-391 states a code for a name outside a state clause (its Resolution bullet says only that it 'names no population'). Either AC-7 and TC-1076 row 9 expect `unsupported_construct`/`expression-form` per FR-139 with a locus, or FR-391 adds a SHALL that overrides FR-139 for temporal atoms. The same tension holds for the bare quantifier domain in a state clause (FR-391 Typing: `ill_typed`/`operator-ineligible`; FR-139: `expression-form`); FR-391 should state that it narrows FR-139 for state clause bodies and add the FR-139 edge. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:172 |
| FND-003 | medium | FR-161-AC-2 and TC-585 step 3 write `SymmetryBroken{InitialStatesNotClosed{0, (a b)}}`. ADR-021 RV-3 defines `SymmetryBroken{population, cause}`, and FR-151-AC-3 and TC-568 write `SymmetryBroken{config_history, InitialStatesNotClosed{0, (a b)}}`. The measured cause carries `population: holders`. Write `SymmetryBroken{holders, InitialStatesNotClosed{0, (a b)}}` so a test asserts the population too and the spelling matches its owner. | spec/functional/FR-161-concretise-a-reduced-counterexample.md:93 |
| FND-004 | medium | The owner decision keeps the rule that `M::P` is only an `allInstances`/`lookup` operand and that a temporal atom cannot name a population, and AC-7 now pins it with codes. Elsewhere the spec still says the opposite: FR-150-AC-4 (and TC-566 step 2) require the invariant `forall p in config_history: p.parent = self implies p != self` to check, a bare population as a quantifier domain in a state clause, which AC-7 says refuses `ill_typed`; FR-153-AC-3, FR-337's ACs and TC-1079 use the temporal atom `holds(forall x in toggles: x.fired)`, which AC-7 says refuses. These rows are outside the diff, but the decision this PR records makes them unsatisfiable; they need the same rewrite as FR-161-AC-2, in this PR or a named follow-up. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:172 |

## Verdict

Not mergeable as is: two high findings. The SYM-4 half is right: ADR-021 SYM-4 and SYM-5 say a single state with `b` holding the token is not closed under `(a b)` and settles `inconclusive` `SymmetryBroken` in the pre-check, and the engine on main does exactly that with no product state; with both initial states the quotient is one state and the lasso starts at the first, as the AC says. The refuse-via-SymmetryBroken decision agrees with RV-3, RV-9 and FR-151-AC-3. The population half is not consistent: FR-161-AC-2's claim stays inexpressible under the rule the PR adds (FND-001), AC-7's temporal-atom code contradicts FR-139 and the engine (FND-002), and other ACs still contradict the kept rule (FND-004). AC-7's first two clauses agree with FR-391's Typing and observation rules and are future behaviour; FR-391 is not implemented on main.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-153-AC-3 (and FR-161-AC-2) now state, as a rule, '(every temporal clause has an `over` parameter)'. FR-337 (line 70 and AC-5) and TC-1079 step 3 specify a clause with no `over` parameter and name it as 'FR-153-AC-3's `eventually holds(forall x in toggles: x.fired)` shape'. That clause no longer exists in FR-153-AC-3, and the parenthetical contradicts FR-337-AC-5. TC-1079 says the no-`over` form waits on QSpec (STD-186), so S1 requiring `over` is today's state, not a rule. Say 'today S1 requires an `over` parameter (STD-186)' or drop the parenthetical, and point FR-337 and TC-1079 at their own no-`over` example. | spec/functional/FR-153-decide-each-fairness-on-the-annotated-quotient.md:99 |
| FND-006 | medium | The postcondition `not self.tok and (forall o in holders: o != self implies o.tok)` fails S1 with `expected (`. The shared grammar spells the quantifier `forall(x in D: P)`. The planner's population ruling (option a) decides whether `holders` may be the domain, but it does not make this spelling parse. Write `forall(o in holders: o != self implies o.tok)` (or the qualified name the population PR settles on), and say that the fixture's postcondition waits on that PR, as the claim's wording does for the watcher. | spec/functional/FR-161-concretise-a-reduced-counterexample.md:95 |
| FND-007 | low | FR-161-AC-2 says the run supplies the atom's letter, then 'it replays'. The atom `w = w` is true everywhere, so an FR-128 replay that evaluated the atom would satisfy `eventually` and not reproduce the counterexample. #1066's test passes the same letters to replay. Say 'and it replays under the same letter'. | spec/functional/FR-161-concretise-a-reduced-counterexample.md:95 |

## Dispositions

Round 1 verified at head 261c99f4d against main 9567ed601 with the same throwaway probe (patch /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1075/probe-patch-r1.diff, output /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1075/probe-r1.log; clone reverted). The Token unit with `temporal C using t0 over (w: M::Watch) ... { eventually holds(w = w) }` checks; the Toggle unit with `fair weak each M::Toggle::flip; fair strong each M::Toggle::fire; eventually holds(w = w)` checks; one initial state still settles `SymmetryBroken{holders, InitialStatesNotClosed{0, (a b)}}` with 0 product states. #1066's tests use the same form: quotient.rs's EVENTUALLY_FIRED is `eventually holds(w = w)`, and flag_world.rs passes its AllTrue letters to both `check_model` and `replay_unit`. FR-161-AC-2's postcondition as written, `not self.tok and (forall o in holders: o != self implies o.tok)`, fails S1 (`expected (`); `forall(o in holders : ...)` refuses MissingName("holders") and `forall(o in M::holders : ...)` refuses PopulationFormOutsideRefinement at S3. FR-391 and TC-1076 are byte-equal to main.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after: The claim is `eventually holds(w = w)` over a watcher `w` of a second, unreduced population `watchers` (every temporal clause has an `over` parameter), so it runs under the full group of `holders`; the run supplies the atom's letter, true exactly in the states where every `holders` object has `tok` true. / FR-153-AC-3: The claim is `eventually holds(w = w)` over a watcher `w` of a second, unreduced population `watchers` (every temporal clause has an `over` parameter), so it runs under the full group of `toggles`; the run supplies the atom's letter, true exactly in the states where every `to... |
| FND-002 | fixed | after: FR-391-AC-7 and TC-1076 row 9 removed: spec/functional/FR-391-name-a-population-in-a-state-clause.md and spec/test-cases/TC-1076-a-state-clause-names-a-population.md are byte-equal to main 9567ed601. |
| FND-003 | still-open | FR-161-AC-2 now writes `SymmetryBroken{holders, InitialStatesNotClosed{0, (a b)}}`, but TC-585's expected result for step 3 still reads `- Step 3: `inconclusive`, `SymmetryBroken{InitialStatesNotClosed{0, (a b)}}`, no state expanded.` without the population. |
| FND-004 | deferred | The planner ruled the population question (option a: population forms admitted wherever an ordinary quantifier domain is); that spec change, which reconciles FR-150-AC-4, FR-153, FR-337 and TC-1079 with FR-391, comes in a separate PR. This PR no longer adds the conflicting AC-7. |
| FND-005 | still-open | New in disposition pass 1; routed to the coder. |
| FND-006 | still-open | New in disposition pass 1; routed to the coder. |
| FND-007 | still-open | New in disposition pass 1; routed to the coder. |
