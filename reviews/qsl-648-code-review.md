---
id: SR-1366
title: "Code review of quire-spec-language PR #657: adapting to IR-662 i128 model bounds and literals (QSL-648)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@713238dc0b663181a2aa311a6776e09ea663c277; PR #657 own diff b90d17b13...713238dc0 (stacked on #651): src/checking/**, src/lowering/wire.rs, src/model_source/lower.rs, src/protocol_artifact/{models.rs,native/types.rs,native/values.rs}, src/runtime/**, src/state/evaluation.rs, tests/it/{composed_proofs,composed_types,integer_lowering,linking,native_model,native_model_profiles,native_model_qualification}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-041
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-038
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: reviews
---
# Code review of quire-spec-language PR #657

## Summary

Ticket: QSL-648. Code review with the rust-review lane folded in. Scope is only this PR's own commits over #651 (`git diff b90d17b13...713238dc0`). The IR lock (3ed1f7ce), #651's owner member and the QSL-642 source-side checks are out of scope by ruling.

The PR widens every QSL site that reads IR `IntegerType`/`RationalType` bounds or builds `IntegerLiteral` to `i128`. It drops the i64 narrowing at most sites. It adds one i64 narrowing with a `NumericDomain` refusal where a checked rational becomes a protocol number (`native/values.rs`).

Measured at 713238dc0 with focused tests (`integer_lowering`, `composed_proofs`, `composed_types`, `native_model_profiles`, `native_protocol_emission`): 78 passed. Probes in a throwaway worktree at the same head:
- `model_source::read` now admits a native-rule-model/2 rational with `maximum_denominator` 9223372036854775808 and with 18446744073709551615. Both refused before IR-662.
- With the shared `Exact` scalar's denominator raised to u64::MAX, `native::admit` of the FR-042-AC-2 numeric-domain fixture fails with `Numeric(ComponentOutOfRange { component: Decimal })`, not `Invalid(NumericDomain)`.
- A literal `rational(1, 9223372036854775808)` under that domain types and discharges, then refuses `Invalid(NumericDomain)` at the new `values.rs` guard.
- At PR head, `lhs_arg = 9223372036854775808` (the original i64+1 case) still yields `LiteralDomain`. Before the PR, `rational(9223372036854775808,9223372036854775808)` refused `RationalNormalization`. It now types, because it normalizes to 1/1.

Protocol-number edge: FR-038 fixes protocol numeric components at signed-64, with denominators in 1..i64::MAX. So a writer-side `NumericDomain` refusal for a wider checked rational in `values.rs` matches the spec. The model-scalar writer in `native/types.rs` lost its equivalent guard; see FND-001.

Examined:
- FR-041-AC-3 (examined): "The closed rational JSON variant preserves i64 numerator extrema, integers adjacent to 2^53 and valid maximum-denominator boundaries exactly. Zero/negative/too-large denominator, reversed numerator bounds, missing/duplicate/unknown fields, wrong tags, floating values and out-of-width integers refuse through the actual decoder/IR constructor."
- FR-041 statement (examined): "The IR constructor requires numerator minimum no greater than maximum and maximum denominator in 1..i64::MAX inclusive."
- FR-038 Inputs (examined): "A signed-64 integer or a normalized rational whose numerator is signed-64 and whose denominator is in 1..9223372036854775807."
- FR-040-AC-3 / AC-4 / AC-8 (examined, via composed_proofs `excluded_numeric_meanings_keep_the_actual_type_refusal_and_original_locus`)
- src/checking/composed/proofs/engine/queries.rs, walk.rs; src/checking/composed/solver/literals.rs, validation.rs; src/checking/constraints.rs, proof.rs, proof/walk.rs; src/lowering/wire.rs; src/model_source/lower.rs; src/protocol_artifact/models.rs; src/runtime/evaluation.rs, validation/values.rs; src/state/evaluation.rs (examined): mechanical widening; no defect found except as listed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Model admission now accepts a rational `maximum_denominator` in i64::MAX+1..=u64::MAX. FR-041 says the ceiling is 1..i64::MAX and FR-041-AC-3 says a too-large denominator refuses. The PR deletes the two test cases that held that refusal and does not edit FR-041. Such a model then fails late at protocol admission with `Numeric(ComponentOutOfRange)`, because `native/types.rs` dropped its `i64::try_from(..) -> NumericDomain` guard and the protocol `integer()` writer now takes any i128. That is not the `NumericDomain` refusal the PR description claims. Fix in this PR: refuse a denominator above i64::MAX at model source with the original scalar occurrence, keep the two cases in `invalid_ir_bounds_keep_the_original_scalar_occurrence`, and keep `integer()` i64-typed, or narrow it with `NumericDomain`. The alternative is to amend FR-041 to a new ceiling and test the protocol-edge refusal. | src/model_source/lower.rs:131-140; src/protocol_artifact/native/types.rs:377; src/protocol_artifact/native/types.rs:655-665; tests/it/native_model_profiles.rs:419-424 |
| FND-002 | medium | The changed cases in composed_proofs and composed_types replace the i64+1 inputs instead of adding to them. The i64+1 inputs now exercise new code: the i128 parse plus the domain comparison in `literals.rs`, which still yields LiteralDomain, and rational normalization of `rational(2^63,2^63)` to 1/1, which now types. After the swap, neither behaviour is asserted anywhere. Keep the 2^127 cases for parse overflow. Restore the 2^63 integer case as LiteralDomain. Add the 2^63 rational as a typed, normalized-1/1 case. | tests/it/composed_proofs.rs:736-743; tests/it/composed_types.rs:647-650 |
| FND-003 | low | `u128::try_from(..).unwrap_or(u128::MAX)` silently turns an out-of-domain value into the widest one, and for `total_denominator` that is the accepting direction. IR guarantees a positive denominator, so this is dead today, but it is a fallback that admits. Use `let Ok(..) = .. else { return false };`, as the rest of the function does. | src/checking/composed/proofs/engine/queries.rs:379-381 |
| FND-004 | low | The new `NumericDomain` guard for a checked rational wider than i64 has no test. The probe above shows it is reachable once a wide-denominator model admits (FND-001). If FND-001 is fixed at model source, the guard becomes defensive. Either way, a one-case test pins the classification. | src/protocol_artifact/native/values.rs:338-344 |

## Verdict

One high, one medium and two low findings. The widening itself is mechanical and correct, and the IR-facing sites now compare in i128 without lossy casts. FND-001 is a spec claim the code now violates (FR-041's denominator ceiling), and its tests were removed rather than adapted. Not mergeable until FND-001 and FND-002 are fixed in this PR. FND-003 and FND-004 are small and belong in the same fix round.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The new test `a_rational_literal_wider_than_the_protocol_wire_is_refused_before_emission` claims in its doc comment that the type check refuses `rational(1, 2^63)` with `LiteralDomain`. It asserts only that some declaration is refused, so a refusal for any other reason, in either unit, passes. Assert the `LiteralDomain` cause on `Bounded` at the literal's span. FND-004 does not need this test (the guard is unreachable), so it earns its place only as an FR-041 ceiling check on literals, and that needs the cause asserted. Otherwise delete it. | tests/it/native_numeric_domain_emission.rs:149-184 |

## Dispositions

Round 1, reviewed at 67237309027c179ed07aceb9ba4ccab01a42f5b0. Full `cargo test --locked --test it`: 878 passed, 2 ignored; `--lib lowering::wire`: 2 passed.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 672373090 (lower.rs refusal; tests restored in 88502cb52). IR's `RationalType::new(.., 0)` yields IR's own `InvalidNumericBounds`, message "rational numerator or denominator bounds are invalid" (true for a too-large ceiling), IR's rational-bounds path, and QSL's scalar-occurrence span. This is sound, not misleading. A struct literal is possible (`ir::Diagnostic` fields are public), but it would copy IR's private path constant, so the indirection is the lesser cost. types.rs restored the i64 `NumericDomain` narrowing. |
| FND-002 | fixed | 88502cb52 |
| FND-003 | fixed | 88502cb52 |
| FND-004 | rejected | After FND-001, model numerators are i64 and denominators are <= i64::MAX, so a normalized in-domain rational always fits i64 and the guard is unreachable from source. It is not ceremony to delete: `ExactRational::new` takes i64, so an i128 -> i64 conversion is required, and the fallible form mapped to `NumericDomain` is the correct idiom (the alternatives are a panic or a truncating cast). Testing it would need a forged TypeReport, so no test. |

Round 2, reviewed at 78b7bbd80afa1f6122f36171e86a784efc5deb48.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | 78b7bbd80 |
