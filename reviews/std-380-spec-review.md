---
id: SR-16212
title: "Spec review of quire-spec-language PR #1318: STD-380, state constraint over protocol subjects and bound-limited counting"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@d64dbf8a6451dcf3f6548b8bd23641a6e32a3274; PR #1318 (git diff origin/main...d64dbf8a): spec/functional/FR-158-cut-the-search-with-a-state-constraint.md, spec/functional/FR-162-offer-reductions-through-transition-system-hooks.md, spec/functional/FR-225-bound-the-memory-component-by-modelchecklimits-budgets.md, spec/test-cases/TC-581-state-constraint-edges-identity-and-determinism.md, spec/test-cases/TC-670-memory-bounds.md; read for context: ADR-025 sections 2-6 and 10 (MA-5, MS-2, MS-4, MB-2..MB-4, TSO-1..TSO-7, RA-1..RA-3, the SB worked example), ADR-021 SC-2/RV-4"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-158
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-162
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-225
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-581
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-670
    type: reviews
---
# SR-16212: Spec review of quire-spec-language PR #1318: STD-380

## Summary

Ticket: STD-380. Sub-analyses folded in: integrity, EARS, criterion strength (manual, no Jev), cross-record fidelity with ADR-025, hand recount of every stated count, id uniqueness, scope-deferral check. Examined: FR-158 Inputs, Outputs, Resolution, the new "Protocol subjects" rule and FR-158-AC-5; FR-162 Offering; the FR-225 counting rule, FR-225-AC-4 (amended) and FR-225-AC-5; TC-581 step 3 and TC-670 steps 4-5. Main is an ancestor of the head. FR-158-AC-5 and FR-225-AC-5 are unique across all open QSL PR heads, and no other open PR touches these files. The decisions are flagged as lead defaults. The `constraint` clause grammar (QSL-913 item 1) is outside STD-380's stated scope, has its own ticket and is not split from this ticket, so it is not a deferral.

Counts recomputed by hand by ADR-025 §10's method, with `Zero`/`One` holding when every `Cell` satisfies the body:

- FR-158-AC-5, `sc`: the initial state (both at `A`) satisfies `Zero` and is expanded. Its only successors are `Sx` and `Sy`, and each sets a cell to 1, so both are boundary states. That gives 3 states and `ConstraintReached{2}`. Correct.
- FR-158-AC-5, `tso`: stores stay buffered and memory stays 0, so every state with no flush satisfies `Zero` and is expanded. The unconstrained depth-4 path `Sx`, `Sy`, `Lx`, `Ly` has no flush, so it survives, and removing successors keeps canonical order among the survivors. `Violated` with that counterexample is correct.
- FR-225-AC-4: `left` with no flush has 3 local states (0, 1 or 2 buffered stores; a third is cut at bound 2), and `right` has 4 (before and after each of its three loads, always reading 0). That is 12 expanded states. The 4 with two buffered stores are bound-limited, since `flush` stays enabled. A first flush from the 4 states with one buffered store and from the 4 with two gives 8 boundary states. Total 20, `ConstraintReached{8}`, bound reached. Correct.
- FR-225-AC-5: `One` is false at the initial state, so that state is a boundary state, never expanded and never bound-limited. That gives 1 state, `ConstraintReached{1}`, `max_messages` 1 not reached. Correct.

## Verdict

**MERGEABLE** once FND-001 is resolved or accepted. Every hand count checks out, the counting rule matches ADR-025 MB-3/MB-4 and ADR-021 RV-4's precedence, and no high finding was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | "Protocol subjects" states that under `ra` the constraint reads each shared location's mo-last message, but no vector tests it. FR-225-AC-5's `ra` constraint is false at the initial state, so it never reads a state after a store. FR-158-AC-5 has no `ra` run. An implementation that reads the reading thread's view (MA-5) under `ra` passes every AC. Add an `ra` vector where the mo-last value and a thread's view differ at a reached state, for example `SB` under `ra` with `Zero`: after `Sx`, `x`'s mo-last message is 1 while `right`'s view still reads `x0`. | spec/functional/FR-158-cut-the-search-with-a-state-constraint.md:97-106,139 |
