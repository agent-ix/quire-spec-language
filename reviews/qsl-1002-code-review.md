---
id: SR-16623
title: "Code and Rust review of QSL-1002 hybrid solver budget"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #1428 (branch qsl1002-solver-budget, head 2595f5f8c); qsl-replay/src/hybrid.rs, qsl-replay/src/lib.rs"
review_set: subset
---

## Summary

Ticket: QSL-1002. Review pass (round 0) at frozen head 2595f5f8c against the
origin/main merge base. Method: code-review with the rust-review lane folded
in. A separate gap-analysis artifact (SR-16624) covers FR-250-AC-3 traceability.
spec-review was skipped because no `spec/**` or `plan/**` file changed.

Gates were run in an isolated worktree with its own target dir.
`cargo fmt --all -- --check` is clean.
`cargo clippy -p qsl-replay --all-targets -- -D warnings` is clean.
`cargo test -p qsl-replay --lib hybrid::` passes all 15 tests.
The qsl-only `arch-lint` checks (api-surface, canonical-encoder,
qualified-core) pass.
`cargo run --locked -p xtask -- string-edge` FAILS with one row,
qsl-replay/src/hybrid.rs:558 in `SolverBudget::read_hybrid`. `string-edge`
is in the Makefile's `CI_CHECKS`, so the aggregate gate will fail.
cargo-deny was not run: the local 0.20.2 rejects `--config`, which is known
environment noise.

Red checks:
- Making rounding floor (dropping the `+ 1`) fails 2 tests.
- Always adding 1, which breaks exact integers (2 s becomes 3), fails 1 test.
The rounding tests are therefore not tautological.

Rounding is correct. `Rational` is reduced with a positive denominator, and
`div_mod_floor` plus a non-zero remainder gives the ceiling for positive
values. 1.5 gives 2, 2 gives 2 and 0.2 gives 1. Zero and negative values are
refused NotPositive. NaN and infinity cannot be represented in a `Rational`.
A value past u64 is refused, though the variant name is wrong (FND-003).

Defaults are not hard-coded in production code. QSpec FR-193 "Solver
budgets" says "this requirement states no number", and QSL FR-250 assigns 60
and 1,000,000 to TC-705's fixture provider only. Passing the published
bounds in from the caller is therefore the right shape.

The wire member names `max_solver_time` and `max_solver_steps` match FR-331
and backend-provider-v1 `HybridLimits`. The `solver-steps` limit spelling
that QSL-911 tracks is pre-existing and untouched.

Key bytes match the spec. `max_solver_steps` is in request identity and
`max_solver_time` is outside it (QSpec FR-193, FR-331 "Request identity",
QSL FR-250). An unset budget adds no bytes.

`SolverBudget` and `BudgetRefusal` are public with no production caller. That
is acceptable as groundwork: QSL-1035 explicitly owns the RequestItem
`hybrid.limits` wiring and the provider path, which matches the QSL-900
precedent.

The only panic site in production code is `object["limits"]`, which is
guarded by an invariant (FND-004).

## Verdict

**CHANGES REQUESTED.** The rounding, the reader refusals the AC names and the
key-byte rule are correct. However, the PR fails the `string-edge` CI gate
(FND-001, high). The reader and writer also admit budget values above
`PositiveCount`'s 2^53-1 maximum (FND-002, medium). FND-003 and FND-004 are
low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `SolverBudget::read_hybrid` branches on JSON member-name strings (`key.as_str() != "limits"`, and `[TIME_MEMBER, STEPS_MEMBER].contains(&key.as_str())`) outside a `#[string_edge]` function. `xtask string-edge` reports 1 unmarked row at hybrid.rs:558, and `string-edge` is in `CI_CHECKS`, so the aggregate gate fails. Fix: decode `hybrid`/`limits` through a typed serde struct with `deny_unknown_fields`, or mark the wire reader `#[ix_trace_rs::string_edge]`. | qsl-replay/src/hybrid.rs:552-577 |
| FND-002 | medium | Budget members are typed `PositiveCount` (backend-provider-v1 schema: integer 1..9007199254740991; FR-331 invalid-value for a count outside 0..2^53-1). However, `read_hybrid` admits any u64, and `from_request` admits any u64 step count and any rounded time that fits u64. Both readers then admit out-of-domain values, and `hybrid_member` writes them to the wire. The reader should refuse values above 2^53-1, and the writer should refuse them before writing. | qsl-replay/src/hybrid.rs:478-507, 570-580 |
| FND-003 | low | `BudgetRefusal` variants misname causes. A negative wire value (`-1`) is refused `NotAWholeNumber` instead of `NotPositive`. A whole value past u64, from either `round_up_seconds` or the reader, is refused `NotAWholeNumber` although it is whole (the doc's "or exceeds 64 bits" is a separate cause). | qsl-replay/src/hybrid.rs:465, 506, 577 |
| FND-004 | low | `object["limits"]` uses serde_json `Map` indexing, which panics on a missing key. It is safe only because the preceding non-empty and all-keys-equal-"limits" checks hold. `object.get("limits")` folded into the `EmptyOrMalformed` path removes the panic site. | qsl-replay/src/hybrid.rs:561 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | `wire_count` refuses every finite, whole-valued float literal as `OutOfDomain` when it is positive, including in-range non-canonical spellings such as `2.0` or `1e2`. The refusal is still made before consumption, as FR-193 requires for a non-canonical value, but the cause wrongly says "above 2^53-1". A probe test of `{"limits": {"max_solver_time": 2.0}}` returned `Err(OutOfDomain("max_solver_time"))`. Only a float above `MAX_COUNT` should be `OutOfDomain`; any other float should be `NotAWholeNumber`. | qsl-replay/src/hybrid.rs:610-619 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (bba4e0735): `read_hybrid` is marked `#[ix_trace_rs::string_edge]`, and `xtask string-edge` reports 0 rows. This is a legitimate ADR-012 §9 edge, unlike #1426. No typed `hybrid.limits` vocabulary exists in QSL or its dependencies. `read_hybrid` is the one total conversion from wire member names to the typed `SolverBudget`, or to a typed `BudgetRefusal` that names the member. No later code compares these strings. A serde struct with `deny_unknown_fields` would be an alternative, but it is not required. |
| FND-002 | fixed | Round 1 (bba4e0735): `MAX_COUNT = 2^53-1`. `from_request` refuses steps above it and a rounded time above it, and `wire_count` refuses wire counts above it, each as `OutOfDomain`. A new test covers max, max+1, a value past u64, and 2^53-1 plus a fraction rounding up past the maximum. |
| FND-003 | fixed | Round 1 (bba4e0735): a negative integer is now `NotPositive`, and a value past u64 or above 2^53-1 is `OutOfDomain`. `NotAWholeNumber` is reserved for non-whole values. The float residue is FND-005. |
| FND-004 | fixed | Round 1 (bba4e0735): `object.get("limits").and_then(serde_json::Value::as_object)` now feeds the `EmptyOrMalformed("hybrid.limits")` path, so no index panic site remains. |
| FND-005 | still-open | Found in round 1 at bba4e0735 and not yet fixed. It needs the coder to change the float branch at hybrid.rs:610-619 so that only a value above MAX_COUNT is OutOfDomain, and to add a `2.0` assertion. Alternatively the lead may record it deferred, since it is low and the value is still refused. |
