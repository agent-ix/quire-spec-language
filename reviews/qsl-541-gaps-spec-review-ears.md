---
id: SR-13378
title: 'EARS analysis of PR #1075: FR-161-AC-2 and FR-391-AC-7'
type: SpecReview
analysis: ears-conformance
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

Ticket: QSL-541. EARS lane over the edited acceptance criteria and the FR-391 statements they verify. quire validate reports no EARS or AC-shape warning on the four changed files.

Measured on main 9567ed601 (which holds merged #1066) with a throwaway probe test in qsl-analyze/tests/it over the `Token` domain of flag_world.rs (patch kept at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1075/probe-patch.diff, output at /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1075/probe.log and probe2.log; the clone was reverted). (a) One initial state, `a.tok` false and `b.tok` true, full class `[a, b]`: `check_model` returns `Reduction{SymmetryBroken{population: holders, cause: InitialStatesNotClosed{initial: 0, generator: (a b)}}}` with `product_states: 0`. With both initial states (#1066's fixture) the quotient has one product state and the lasso starts at initial state 0 (quotient.rs, a_loop_that_needs_two_passes_closes_at_the_concrete_entry). (b) Every population form in a clause refuses today with `PopulationFormOutsideRefinement`, which maps to `unsupported_construct`/`expression-form` (FR-139): the rewritten post `forall(o in allInstances<M::Holder>(M::holders) : o != self implies o.tok)`, the bare `forall(o in M::holders : ...)`, a pre with `allInstances<M::Holder>(M::holders)`, `M::holders = M::holders`, a function body, and a temporal `holds` atom with `M::holders` in either spelling. FR-391 is not implemented yet (footprint.rs says so), so AC-7's first two clauses are future behaviour. Only an unqualified `holders` refuses `missing_declaration`/`missing-name` (MissingName("holders")). (c) The claim as FR-161-AC-2 writes it, `eventually holds(forall x in holders: x.tok)`, fails S1 (`expected (`), and a temporal clause with no `over` fails S1 (`expected over`).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-161-AC-2 | spec/functional/FR-161-concretise-a-reduced-counterexample.md | examined |
| FR-391-AC-7 | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-161-Behavior-Start | spec/functional/FR-161-concretise-a-reduced-counterexample.md | context_only |
| FR-391-Resolution-outside | spec/functional/FR-391-name-a-population-in-a-state-clause.md | examined |
| FR-391-Typing | spec/functional/FR-391-name-a-population-in-a-state-clause.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-391-AC-7 verifies a refusal with a catalog code for a temporal `holds` atom naming a population, and calls it 'outside this requirement'. The only FR-391 text about that case is the Resolution bullet 'Outside a state clause body a `M::P` name resolves as it did before this requirement: it names no population', which has no SHALL, no subject and no response. An AC should verify a statement of its own FR; either add an unwanted-behaviour statement ('If a temporal clause atom names `M::P`, then the checker SHALL refuse ... at the name', with the code FR-139 fixes) or drop the clause from AC-7 and verify it under FR-139. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:93-95 |

## Verdict

One medium finding. FR-161-AC-2's new one-initial-state clause is owned by ADR-021 SYM-5 and FR-151, not by a statement of FR-161 (see SR-13379 FND-002). AC-7's first two clauses are backed by FR-391's Typing SHALLs. Both ACs are direct assertions of outcome, not given-when-then.

## Dispositions

Round 1 verified at head 261c99f4d against main 9567ed601 with the same throwaway probe (patch /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1075/probe-patch-r1.diff, output /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1075/probe-r1.log; clone reverted). The Token unit with `temporal C using t0 over (w: M::Watch) ... { eventually holds(w = w) }` checks; the Toggle unit with `fair weak each M::Toggle::flip; fair strong each M::Toggle::fire; eventually holds(w = w)` checks; one initial state still settles `SymmetryBroken{holders, InitialStatesNotClosed{0, (a b)}}` with 0 product states. #1066's tests use the same form: quotient.rs's EVENTUALLY_FIRED is `eventually holds(w = w)`, and flag_world.rs passes its AllTrue letters to both `check_model` and `replay_unit`. FR-161-AC-2's postcondition as written, `not self.tok and (forall o in holders: o != self implies o.tok)`, fails S1 (`expected (`); `forall(o in holders : ...)` refuses MissingName("holders") and `forall(o in M::holders : ...)` refuses PopulationFormOutsideRefinement at S3. FR-391 and TC-1076 are byte-equal to main.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after: FR-391-AC-7 removed; FR-391 is byte-equal to main 9567ed601, so no AC verifies the unowned temporal-atom refusal. |
