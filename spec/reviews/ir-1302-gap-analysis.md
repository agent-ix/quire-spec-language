---
id: SR-202611030002
title: "Gap analysis of IR-1302 PR #2221"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@7487fd4ab387d8bb5dc426986f3cc5459af0b86b; PR #2221; Ticket: IR-1302; FR-091-AC-34, AC-35, AC-46, AC-47; QSpec FR-142-AC-15..17, TC-187 U11j, U11m, U16"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
---

## Summary

Ticket: IR-1302. Planless gap analysis of the tagged tests against FR-091 AC-34, 35, 46, 47 and QSpec FR-142 / TC-187. Plan completion: not assessed. The FR-091-AC-34 restatement is faithful: the old test fed input without the unit d and asserted five errors including Width and both cycles, which the AC forbids; the new test uses the AC input, asserts exactly the three pre-topology errors, and checks the second input yields the single row-6 cycle. It fails on old behavior. U11j and U11m are tested and fail on old behavior.

## Verdict

**PASS WITH FINDINGS** — the two ticketed vectors are fixed and verified. U16 is not resolved (FND-004), so IR-1302 must not close on this merge. Retention fields and the package-unit forms are gaps that predate the PR.

## Assurance Context

No applicable AssuranceProfile was found under spec/ for the changed code.

## Scope

- `FR-091-AC-34`: examined; new test matches the AC text, including the reduced second input.
- `FR-091-AC-35`: examined; code and cause assertions added, retained fields only partly asserted.
- `FR-091-AC-46`: examined; five examples covered, dimension-cycle edges not asserted.
- `FR-091-AC-47`: examined; the AC example plus two extras.
- `FR-091-AC-50`, `FR-091-AC-55`: examined; package-unit and package-dimension forms unimplemented, not touched by the PR.
- `QSpec FR-142 turn list`, `TC-187 U11j`, `U11m`: examined; tested.
- `TC-187 U16`: examined; unresolved.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-091-AC-35 and FR-142 rows 1, 2, 3, 5 and step (-1) require refusals to retain more than names: row 1 the scale, row 2 the unit with the target, row 3 both dimension nodes, row 5 scale and offset, step (-1) the owning dimension with the term. `TopologyFault` carries unit names only and `UnresolvedQuantityName` only the missing name, and the tests assert only those. This predates the PR; the PR rewrote the cause mapping and tags AC-35. Follow-up under QSL-936. | qsl-semantics/src/check/assemble.rs:189, qsl-semantics/src/check/assemble/tests.rs:2699 |
| FND-002 | medium | The new step (-1), `mapless`, imported-turn and ordering code has no tests. The AC-46 dimension-cycle example asserts code and span only, not edges A -> B and B -> A that the AC names. U11n, U11o, U11p, U11w and U11x have no tagged test; they pass under a scratch probe, so add them as tests in this PR with the edge assertion. | qsl-semantics/src/check/assemble/tests.rs:2796 |
| FND-003 | medium | FR-091-AC-50 and AC-55 package-unit and package-dimension forms stay unimplemented: a unit typed by `M::Rate` resolves against local dimensions only, so `imported_turn` is reachable only for base dimensions with no unit. Ticketed feature; follow-up under QSL-936, not a PR finding to fix here. | qsl-semantics/src/check/assemble/units.rs:624 |
| FND-004 | medium | IR-1302 lists three mismatches; PR #2221 leaves U16 (`integer_bits` 14 vs the meter's 15) unchanged on the author's reading that 14 is the `unit.target-domain` charge amount and 15 the consumed maximum, with no recorded resolution. The reading is plausible but unproven here, since the meter code is outside this repo. The ticket's outcome requires a fix or a spec correction by the owner. | spec/test-cases/TC-187-quantity-and-unit-conversion.md:84 |
