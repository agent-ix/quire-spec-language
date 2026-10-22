---
id: SR-12299
title: "Code review of PR #1178: select the IEEE profile of a package that declares or evaluates a float (QSL-948), code-review with rust-review lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@4303b9935336fb691a1f9a0a156f4d8f31515fe1; git diff origin/main...4303b9935 (PR #1178, branch qsl948-ieee-route): qsl-semantics/src/check/check.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/check/typing.rs, qsl-package/src/emit.rs, qsl-eval/src/value/expression/evaluate/invariants.rs, qsl-replay/tests/it/compile_ieee_profile.rs, qsl-replay/tests/it/main.rs, qsl-replay/tests/fixtures/ieee_float/*.native; ticket QSL-948"
review_set: subset
date: "2026-10-22"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-397
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: references
---

## Summary

Ticket: QSL-948. PR: quire-spec-language#1178, head 4303b9935. Method: code-review with the rust-review lane folded in. Single reviewer pass over the whole PR diff.

The change has three parts.

- `Scope::select_declared_ieee_profile` (called from `PackageDeclarations` after every signature resolves) scans the resolved declarations: type aliases, function parameters and results, record/tuple/union positions and object-type attributes, descending through `Option` and collection elements. When a float is present it records the trigger (`ieee_operation`) and, if no profile was injected, admits the catalog `ieee_profile` row through `DefinitionLock::admit_ieee_profile`.
- `emit_package_inner` adds the selected profile's definition to the lock's laws, so a package that declares a float but applies no IEEE operation still lists the row (FR-397-AC-2).
- `Typer::equality_typed` routes a float operand of `convert<T>(e)` in an equality through `Typer::convert` instead of refusing it, so `convert<Rational[...]>(f) = r` lowers to `quire.op.ieee.to_rational`.

Measured at the head, in a dedicated worktree:

- The 11 tests in `compile_ieee_profile` pass.
- Mutation M1 (main's source with the PR's tests): 6 of 11 fail. These are AC-1, AC-2, AC-3, AC-4, the equality-conversion test and the evaluation test. The 5 that still pass are the refusal tests (AC-5 to AC-8 and the S2 constructor test), whose behaviour already held on main.
- Mutation M2 (drop the `emit.rs` push): the AC-2 declared-float test fails.
- Mutations M4 and M5 (no `Option`/collection descent; main's `typing.rs`): the AC-2 test, the equality-conversion test and the evaluation test fail.
- Scratch probes, deleted afterwards:
  - `convert<Float64>(a) = b` and `convert<Float64>(a) != b` refuse `type-mismatch`.
  - `convert<Int>(a) = i` refuses `type-mismatch`.
  - `convert<Rational>(a) = convert<Rational>(b)` emits two `to_rational` nodes and one selection.
  - `convert<Rational>(a) < r` lowers.
  - A unit with an integer function and a float function selects the row once.
  - `plain` selects none.
- `cargo clippy --all-features --all-targets -D warnings` is clean on qsl-semantics, qsl-package, qsl-eval and qsl-replay.
- `xtask string-edge` passes ("no unmarked, unlisted string comparison").
- `make arch-lint-api-surface-qsl` reports 0 FAIL.
- The FR-110-AC-3 tests pass: `spine::tests::a_header_profile_resolves_by_identity_against_the_root_row` and the qsl-semantics `profile` tests.

Rust-review lane:

- No `unwrap`, `expect` or panic is added in production code.
- The float scan is an exhaustive `match` over `ValueType` with no wildcard, so a new variant fails to compile instead of being missed.
- There is no compat layer or shim.
- No new pin, digest or tracking record is added.
- The new `pub fn Scope::ieee_profile_selection` passes the API-surface lint.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| `Scope::ieee_profile_selection`, `Scope::select_declared_ieee_profile` | qsl-semantics/src/check/check.rs:1430-1505 | examined |
| trigger call site after signature resolution | qsl-semantics/src/check/mod.rs:1065-1070 | examined |
| `Typer::equality_typed` float branch | qsl-semantics/src/check/check/typing.rs:1642-1668 | examined |
| `Typer::convert` float arms (unchanged, reached by the new branch) | qsl-semantics/src/check/check.rs:3448-3523 | context_only |
| `emit_package_inner` selected-profile law push | qsl-package/src/emit.rs:1289-1295 | examined |
| `split_laws` | qsl-package/src/emit.rs:791-809 | context_only |
| TC-917 trace addition (FR-397-AC-9) | qsl-eval/src/value/expression/evaluate/invariants.rs:966 | examined |
| TC-1091 tests and fixtures | qsl-replay/tests/it/compile_ieee_profile.rs; qsl-replay/tests/fixtures/ieee_float/*.native | examined |
| model-intake float refusal (bounds the declaration-only trigger) | qsl-semantics/src/check/assemble.rs:1078-1108 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `bit_level_float_constructors_remain_unrepresented_source_forms` is tagged `FR-397-AC-8`, but it asserts the S2 `UnrepresentedConstruct{FloatValue}` refusal of `float32(bits:)`/`float64(bits:)` (FR-397 Inputs, FR-091). FR-397-AC-8 is about `convert` into a float, and `conversion_into_a_float_is_never_admitted` already covers it. The trace over-claims AC-8 coverage. | qsl-replay/tests/it/compile_ieee_profile.rs:253-255 |
| FND-002 | low | `admitted_float_arithmetic_and_rational_conversion_evaluate_through_the_source_spine` is tagged `FR-397-AC-1` and `FR-397-AC-3`, but it asserts evaluated results. AC-1 and AC-3 state lock, law, mode and lowering facts, and those facts are asserted by other tests. The test is useful, but it is not evidence for those ACs, so the matrix counts it twice. | qsl-replay/tests/it/compile_ieee_profile.rs:273-275 |
| FND-003 | low | `select_declared_ieee_profile` discards the `admit_ieee_profile` error (`map_err(\|_\| ...)`), so the internal fault `catalog-ieee-profile-admitted` loses its cause. It also spells long inline paths (`quire_semantic_value::location::Origin::Expression`, `super::refusal::KeyFault::StageFault`, `qsl_foundation::diagnostic::InternalFault::new`) and uses a function-local `use`. Module-level imports would make it shorter. | qsl-semantics/src/check/check.rs:1486-1503 |
| FND-004 | low | The `ieee_operation` field is documented as "the IEEE package-selection trigger", but it is computed from declarations only. FR-397's trigger also covers holding an IEEE application. The two agree today only because no float enters a body without a declared float: there are no float constants and no conversion into a float, and model float fields and operations are refused at intake (`AssemblyCause::FloatingType`). That invariant is stated nowhere at the field or the function, so a later float source (a constant, a float-returning intrinsic) would silently fall back to `IeeeProfileNotAdmitted`. | qsl-semantics/src/check/check.rs:792-793,1436-1437 |

## Verdict

Mergeable. There is no major and no medium finding. All four findings are low.

- The selection rule matches FR-397 for every case measured.
  - Float32 and Float64 `+ - * /` select the row once, and the law's `DefinitionRef` equals the selection.
  - A declared float with no application selects the row. This was checked for a parameter, a result, record/tuple/union fields, an alias, and `Option`/`Sequence` nesting.
  - Float-to-rational, standalone and inside an equality, selects the row and lowers to `to_rational`.
  - An integer-only unit selects nothing.
- The strict V2 reader admits the AC-4 packages. Before this change those packages refused `IeeeProfileNotAdmitted` at checking. The emitted identity is `quire.value.ieee754-2019-default/v1` under authority `agent-ix`, the row that quire-contract-ir's `checked_package_v2_ieee_admission` tests admit.
- FR-110-AC-3 still holds.
- The test oracles fail without the change (mutations M1, M2, M4 and M5).
- The spec (FR-397/TC-1091) landed in #1169, and the code matches it. No spec text is needed in this PR.

## Dispositions

Round 1, reviewed at 8e935d42e (PR head 7bd96771a after a merge of main). The reviewer checked each row against the fix commits 94e5e489e and 8e935d42e. The `compile_ieee_profile` tests pass (12 of 12), clippy with all features is clean on qsl-semantics and qsl-replay, and quire validate passes on FR-397 and TC-1091.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 94e5e489e: `bit_level_float_constructors_remain_unrepresented_source_forms` is now traced `TC-1091` only, with a comment that it does not claim AC-8. |
| FND-002 | fixed | 94e5e489e: the evaluation test now also asserts that the emitted arithmetic identity, its law, `rounding nearest-even` and the `to_rational` law match the selected row before it evaluates, so its AC-1/AC-3 tags are backed by assertions. |
| FND-003 | fixed | 94e5e489e: `KeyFault::IeeeProfileAdmission(PackageRefusal)` keeps the typed cause under the same invariant id `catalog-ieee-profile-admitted`. The function-local `use` and long inline paths are now module imports. |
| FND-004 | fixed | 94e5e489e: the `ieee_operation` doc and FR-397 Inputs state that every body float comes from a declaration, and that any new float source must extend the trigger. |
