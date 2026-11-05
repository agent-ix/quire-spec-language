---
id: SR-20261105001152
title: "Gap analysis — FR-091 unit refusal retention and package units"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2357 diff; FR-091-AC-35/46/47/50/55/71; qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-semantics/src/check/assemble/units.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/tests/it/dimension_equivalence.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/lifecycle.rs, qsl-replay/src/spine/refinement.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: references
---

## Summary

This planless, PR-diff-scoped audit reconciled the changed FR-091 behavior with the Rust implementation and tagged tests. The computed matrix did not complete within five minutes; a changed-scope trace-tag inspection found coverage for the six changed criteria, but cannot establish repository-wide coverage.

## Verdict

**CONDITIONAL.** The changed-scope inspection found no unimplemented or hollow behavior, but repository-wide computed traceability remains unverified because the matrix command timed out.

## Scope

- **FR-091 — context_only** (`spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md`): “With both in place, complete-V1 source reaches `PackageDeclarations::check` through S1, S2 and E3.”
- **FR-091-AC-35 — examined** (`spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md`): “Each of these units refuses with exactly one topology error carrying its QSpec FR-142 row's code and cause and naming what the row retains, and returns no `PackageDeclarations`: `dimension L; unit r : L = rational(1, 1); unit z : L = rational(0, 1) * r;` (row 1, `invalid_package`/`invalid-value`, `z` and its scale 0); `dimension L; unit r : L = rational(1, 1); unit u : L = rational(1, 1) * ghost;` (row 2, `missing_declaration`/`missing-name`, `u` and `ghost`); `dimension L; dimension T; unit r : L = rational(1, 1); unit s : T = rational(1, 1) * r;` (row 3, `invalid_package`/`incompatible-defin”
- **FR-091-AC-46 — examined** (same path): “First defect (QSpec FR-142-AC-15). `dimension L; unit r : L = rational(1, 1); unit z : L = rational(0, 1) * ghost;` refuses row 1 at `z`, not row 2; `dimension L; unit r : L = rational(1, 1); unit u : L = rational(1, 1) * ghost; unit z : L = rational(0, 1) * r;` refuses row 2 at `u`, the earlier unit; `dimension L; unit r : L = rational(1, 1); unit q : L = rational(1, 1); unit s : L = rational(1, 1) * s;` refuses row 4 (`r`, `q`), not row 6 (`s`'s self-cycle); `dimension L; dimension T; unit t : T = rational(0, 1) * tr; unit tr : T = rational(1, 1); unit l : L = rational(2, 1);` refuses row 5 ”
- **FR-091-AC-47 — examined** (same path): “Least-unit cycle (QSpec FR-142-AC-15). `dimension L; unit r : L = rational(1, 1); unit y : L = rational(1, 1) * x; unit x : L = rational(1, 1) * y;` refuses row 6, `invalid_package`/`definition-cycle`, with edges `y -> x` then `x -> y`, from `y`, the cycle's least unit by source position, not from `x`, the least by name.”
- **FR-091-AC-50 — examined** (same path): “In a unit declaring `model R = "acme/rpc" …`, `dimension Time;`, `unit s : Time = rational(1, 1);`, `unit tenth : R::Time = rational(1, 10) * R::s;` and `dimension Speed = R::Time ^ -1;`, where `acme/rpc` declares a base dimension `Time` and a root unit `s`, the assembler admits every form with no unresolved-name and no duplicate error: the source `Time` and `R::Time` are two dimensions and the source `s` and `R::s` two units, and the forms keep the qualified names as written for S3. `unit u : Time2 = rational(1, 1);` with no `Time2` declared in the source refuses with an unresolved-name error”
- **FR-091-AC-55 — examined** (same path): “Package dimensions and units (QSpec FR-142-AC-16, AC-18, AC-19). With admitted domain packages `P` and `Q` selected by the unit in that order, `P` first and `Q` second, `P` declaring the TC-187 `P` dimensions and units and `Q` declaring U11z2's `T` with its unit `s`, each of QSpec TC-187's U11r, U11s, U11t, U11u, U11y, U11z1 and U11z2, its `P` declarations supplied by `P` and its local declarations in the unit, gives TC-187's result through the assembler. U11r admits a local `cm` that targets `P`'s root `m`. U11s refuses row 6 at `P`'s `L`, retaining `a -> b` and `b -> a`, and `P`'s `m` is not”
- **FR-091-AC-71 — examined** (same path): “Topology error fields (QSL-1601; QSpec FR-142-AC-15 to AC-17, TC-187 U06, U11a to U11f, U11n, U11o, U11p, U11w, U11x and U11z3). Each refusal of FR-091-AC-35, AC-46 and AC-47 carries the fields of the "Topology error fields" table as values: row 1's `z` with its scale `0`; row 2's `u` with the target name `ghost`; row 3's `s`, its target `r` and the dimension nodes `T` (of `s`) and `L` (of `r`); row 4's `L` with `r` and `q` in source order; row 5's root `r` with scale `2` and offset `0`; and row 6's edges `y -> x`, `x -> y`. `dimension L; dimension D = L * K; unit m : L = rational(1, 1);` refu”

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `quoin matrix --repo . --json` produced no result before it was interrupted after five minutes. The changed-scope tag search found tests for the six relevant criteria, but that manual index does not establish repository-wide criterion coverage. | FR-091-AC-35/46/47/50/55/71; changed Rust tests |

## Coverage

- Reconciliation: `quoin matrix --repo . --json` was attempted and timed out with exit 130 after five minutes. No active traceability model declaration was found in project configuration. A manual changed-scope trace-tag index was used and is not represented as matrix output.
- Plan completion: not assessed
- Changed criteria: FR-091-AC-35, AC-46, AC-47, AC-50, AC-55 and AC-71 each have a tagged test in the changed scope (`each_unit_graph_topology_error_refuses_alone`, `the_first_defect_is_the_earliest_row_unit_and_node`, `a_unit_cycle_is_retained_from_its_least_unit_by_source_position`, `step_minus_one_checks_a_dimensions_own_terms_before_its_units`, `a_package_dimension_takes_its_turn_when_a_declaration_reaches_it`, and `package_units_and_dimensions_resolve_in_source_unit_graphs`). This is a grep index, not computed matrix evidence.
- Changed-scope reverse gap: the typed topology refusals, dimension-term ownership, package-unit targets and package-dimension turns all map to FR-091 criteria. No source or test stub was introduced.
- Semantic review: skipped; it is optional and was not requested.
- Evidence: no matrix run evidence was available. Focused tests, both package Clippy configurations, `string-edge`, workspace check, formatting and `cargo deny check` passed.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | The changed-scope review found direct tagged tests for all six affected FR-091 criteria and the focused suite, both Clippy configurations, string-edge, workspace check, and CI passed. The five-minute matrix timeout is a repository-wide tooling limitation, not an uncovered criterion in this PR; the residual repository-wide traceability uncertainty is accepted for this PR. |

## Verdict (disposition pass 1)

**ACCEPTED-NO-CHANGE.** The lead accepts the repository-wide matrix limitation for this PR based on direct changed-scope trace tags and the passing focused and workspace checks. Repository-wide traceability remains unverified.
