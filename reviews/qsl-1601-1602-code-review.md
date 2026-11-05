---
id: SR-20261105001151
title: "Code review — FR-091 unit refusal retention and package units"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2357; FR-091-AC-35/46/47/50/55/71; qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-semantics/src/check/assemble/units.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/tests/it/dimension_equivalence.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/lifecycle.rs, qsl-replay/src/spine/refinement.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
---

## Summary

The Rust review covered typed FR-091 topology refusal retention, unresolved dimension ownership, imported package dimensions and units in source unit graphs, and the replay consumers. The changes follow the repository's documented Rust conventions, and the focused tests assert retained values and assembled scales directly.

## Verdict

**PASS.** No actionable defects were found in the reviewed diff.

## Scope

- **FR-091 — context_only** (`spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md`): “With both in place, complete-V1 source reaches `PackageDeclarations::check` through S1, S2 and E3.”
- **FR-091-AC-35 — examined** (`spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md`): “Each of these units refuses with exactly one topology error carrying its QSpec FR-142 row's code and cause and naming what the row retains, and returns no `PackageDeclarations`: `dimension L; unit r : L = rational(1, 1); unit z : L = rational(0, 1) * r;` (row 1, `invalid_package`/`invalid-value`, `z` and its scale 0); `dimension L; unit r : L = rational(1, 1); unit u : L = rational(1, 1) * ghost;` (row 2, `missing_declaration`/`missing-name`, `u` and `ghost`); `dimension L; dimension T; unit r : L = rational(1, 1); unit s : T = rational(1, 1) * r;` (row 3, `invalid_package`/`incompatible-defin”
- **FR-091-AC-46 — examined** (same path): “First defect (QSpec FR-142-AC-15). `dimension L; unit r : L = rational(1, 1); unit z : L = rational(0, 1) * ghost;` refuses row 1 at `z`, not row 2; `dimension L; unit r : L = rational(1, 1); unit u : L = rational(1, 1) * ghost; unit z : L = rational(0, 1) * r;` refuses row 2 at `u`, the earlier unit; `dimension L; unit r : L = rational(1, 1); unit q : L = rational(1, 1); unit s : L = rational(1, 1) * s;` refuses row 4 (`r`, `q`), not row 6 (`s`'s self-cycle); `dimension L; dimension T; unit t : T = rational(0, 1) * tr; unit tr : T = rational(1, 1); unit l : L = rational(2, 1);` refuses row 5 ”
- **FR-091-AC-47 — examined** (same path): “Least-unit cycle (QSpec FR-142-AC-15). `dimension L; unit r : L = rational(1, 1); unit y : L = rational(1, 1) * x; unit x : L = rational(1, 1) * y;` refuses row 6, `invalid_package`/`definition-cycle`, with edges `y -> x` then `x -> y`, from `y`, the cycle's least unit by source position, not from `x`, the least by name.”
- **FR-091-AC-50 — examined** (same path): “In a unit declaring `model R = "acme/rpc" …`, `dimension Time;`, `unit s : Time = rational(1, 1);`, `unit tenth : R::Time = rational(1, 10) * R::s;` and `dimension Speed = R::Time ^ -1;`, where `acme/rpc` declares a base dimension `Time` and a root unit `s`, the assembler admits every form with no unresolved-name and no duplicate error: the source `Time` and `R::Time` are two dimensions and the source `s` and `R::s` two units, and the forms keep the qualified names as written for S3. `unit u : Time2 = rational(1, 1);` with no `Time2` declared in the source refuses with an unresolved-name error”
- **FR-091-AC-55 — examined** (same path): “Package dimensions and units (QSpec FR-142-AC-16, AC-18, AC-19). With admitted domain packages `P` and `Q` selected by the unit in that order, `P` first and `Q` second, `P` declaring the TC-187 `P` dimensions and units and `Q` declaring U11z2's `T` with its unit `s`, each of QSpec TC-187's U11r, U11s, U11t, U11u, U11y, U11z1 and U11z2, its `P` declarations supplied by `P` and its local declarations in the unit, gives TC-187's result through the assembler. U11r admits a local `cm` that targets `P`'s root `m`. U11s refuses row 6 at `P`'s `L`, retaining `a -> b` and `b -> a`, and `P`'s `m` is not”
- **FR-091-AC-71 — examined** (same path): “Topology error fields (QSL-1601; QSpec FR-142-AC-15 to AC-17, TC-187 U06, U11a to U11f, U11n, U11o, U11p, U11w, U11x and U11z3). Each refusal of FR-091-AC-35, AC-46 and AC-47 carries the fields of the "Topology error fields" table as values: row 1's `z` with its scale `0`; row 2's `u` with the target name `ghost`; row 3's `s`, its target `r` and the dimension nodes `T` (of `s`) and `L` (of `r`); row 4's `L` with `r` and `q` in source order; row 5's root `r` with scale `2` and offset `0`; and row 6's edges `y -> x`, `x -> y`. `dimension L; dimension D = L * K; unit m : L = rational(1, 1);` refu”
- **FR-091-AC-72 — context_only** (same path): “Package dimension and package unit forms (QSL-1602; QSpec FR-142-AC-18, AC-19, AC-20). With `model M = "acme/rpc"`, where `acme/rpc` declares base dimension `Rate` with root unit `us`, and a unit of compilation declaring `dimension Time; unit s : Time = rational(1, 1);`: `unit tick : M::Rate = rational(1, 2) * M::us;` resolves its `:` name to the package dimension `Rate` (FCD identity `ix://acme/rpc/dimension/Rate`) and its target to the package unit `us`, is checked at `M::Rate`'s turn with `us` as an admitted member, and admits `tick` with scale `1/2` to `us`; `function k1 using v(d: Quantit”
- `qsl-semantics/src/check/assemble.rs`, `qsl-semantics/src/check/assemble/units.rs`, `qsl-semantics/src/check/assemble/tests.rs`, `qsl-semantics/src/check/mod.rs`, `qsl-semantics/tests/it/dimension_equivalence.rs`, `qsl-replay/src/spine.rs`, `qsl-replay/src/spine/lifecycle.rs`, and `qsl-replay/src/spine/refinement.rs` — examined in full diff.

## Coverage

- No duplication, vendoring, weakened workflow gate, unsafe change, or applicable AssuranceProfile was found.
- Public enum additions are documented and all in-repository consumers compile exhaustively. Unknown package targets retain their exact spelling; imported dimensions retain their nominal key; dimension-term failures carry both owner and term.
- The new and updated tests use real assembler entry points and assert values that fail if the production mapping is wrong. The tagged tests cover AC-35/46/47/50/55/71 in the changed scope.
- `cargo fmt --all -- --check`, 111 focused `qsl-semantics` assembler tests, 4 scan tests, the exact package-unit integration test, both `qsl-semantics` Clippy configurations, `xtask string-edge`, the combined `cargo check --locked --workspace --all-targets`, `git diff --check`, and `cargo deny check` passed. Cargo commands used the isolated `codex3-target` through `locked-build.sh`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
