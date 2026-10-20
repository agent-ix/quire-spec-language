---
id: SR-13379
title: 'Integrity analysis of PR #1075: traceability of FR-161-AC-2 and FR-391-AC-7'
type: SpecReview
analysis: integrity
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

Ticket: QSL-541. Integrity lane: relationships, test-case pointers and TC scope for the edited ACs. make check-spec-validation exits 0 (no warning on the changed files) and tools/check-index-completeness.sh passes.

Measured on main 9567ed601 (which holds merged #1066) with a throwaway probe test in qsl-analyze/tests/it over the `Token` domain of flag_world.rs (patch kept at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1075/probe-patch.diff, output at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1075/probe.log and probe2.log; the clone was reverted). (a) One initial state, `a.tok` false and `b.tok` true, full class `[a, b]`: `check_model` returns `Reduction{SymmetryBroken{population: holders, cause: InitialStatesNotClosed{initial: 0, generator: (a b)}}}` with `product_states: 0`. With both initial states (#1066's fixture) the quotient has one product state and the lasso starts at initial state 0 (quotient.rs, a_loop_that_needs_two_passes_closes_at_the_concrete_entry). (b) Every population form in a clause refuses today with `PopulationFormOutsideRefinement`, which maps to `unsupported_construct`/`expression-form` (FR-139): the rewritten post `forall(o in allInstances<M::Holder>(M::holders) : o != self implies o.tok)`, the bare `forall(o in M::holders : ...)`, a pre with `allInstances<M::Holder>(M::holders)`, `M::holders = M::holders`, a function body, and a temporal `holds` atom with `M::holders` in either spelling. FR-391 is not implemented yet (footprint.rs says so), so AC-7's first two clauses are future behaviour. Only an unqualified `holders` refuses `missing_declaration`/`missing-name` (MissingName("holders")). (c) The claim as FR-161-AC-2 writes it, `eventually holds(forall x in holders: x.tok)`, fails S1 (`expected (`), and a temporal clause with no `over` fails S1 (`expected over`).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-161-relationships | spec/functional/FR-161-concretise-a-reduced-counterexample.md | examined |
| FR-161-AC-2 | spec/functional/FR-161-concretise-a-reduced-counterexample.md | examined |
| TC-585-step-3 | spec/test-cases/TC-585-symmetry-counterexamples-concretise-and-close.md | examined |
| TC-1076-description | spec/test-cases/TC-1076-a-state-clause-names-a-population.md | examined |
| TC-1076-row-9 | spec/test-cases/TC-1076-a-state-clause-names-a-population.md | examined |
| FR-151-AC-3 | spec/functional/FR-151-admit-a-request-s-symmetry-declarations.md | context_only |
| TC-568-step-1 | spec/test-cases/TC-568-initial-states-must-be-closed-under-the-generators.md | context_only |
| FR-161-Behavior-Start | spec/functional/FR-161-concretise-a-reduced-counterexample.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-161-AC-2 now writes its postcondition with `allInstances<M::Holder>(M::holders)`, which only FR-391 admits (main refuses it today), and asserts the `SymmetryBroken` pre-check that FR-151 owns. FR-161's relationships list US-019, ADR-021, FR-126, FR-128, FR-152 and FR-153 only. Add `depends_on` FR-391 and FR-151, so TC-585 step 2 shows as blocked on FR-391's implementation and the blast radius of a FR-391 or FR-151 change reaches FR-161. | spec/functional/FR-161-concretise-a-reduced-counterexample.md:1-18 |
| FND-002 | medium | The added sentence ('With only the first initial state ... settles `inconclusive`, `SymmetryBroken{...}` with no state expanded') never reaches concretisation, which is all FR-161's Behavior covers; it is FR-151's admission (FR-151-AC-3, verified by TC-568). The AC cites a test case (TC-568) in criterion text instead of the requirement, and TC-585 step 3 carries no pointer at all. Either keep the case as a fixture guard and cite FR-151-AC-3 (and FR-151 in relationships, FND-001), or move it to FR-151 as an AC over the Token fixture verified by TC-568. | spec/functional/FR-161-concretise-a-reduced-counterexample.md:93 |
| FND-003 | low | TC-1076's scope now reaches FR-391-AC-7, but its 'Catches' sentence still lists only the AC-1 to AC-6 defects. Row 9 catches a postcondition read at `pre` instead of `post`, a bare population accepted as a quantifier domain, and a population accepted in a temporal atom; name them. | spec/test-cases/TC-1076-a-state-clause-names-a-population.md:13-15 |

## Verdict

Two medium findings and one low. TC-585 verifies FR-161 and its scope 'FR-161-AC-1 to FR-161-AC-2' still covers the edited AC; TC-1076's scope was widened to AC-7 and its row 9 matches AC-7 clause for clause. The TC-568 pointer is to the right test of the rule, but the owning requirement (FR-151) is neither cited nor related.
