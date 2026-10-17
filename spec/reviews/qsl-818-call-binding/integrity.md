---
id: SR-11763
title: "Integrity review of FR-367 and TC-1053 (QSL-818, PR #915)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@35042dc696ecfa1a71a62ecbb58f10529b8fb0f4; spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md, spec/test-cases/TC-1053-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md, spec/spec.md"
review_set: subset
---
# Integrity review of FR-367 and TC-1053

## Summary

Ticket: QSL-818. PR quire-spec-language#915, head 35042dc69. Checked FR-367's Behavior against its own Outputs and ACs, against Contract IR origin/main `scalar_application_operands` (crates/quire-contract-model/src/checked_package/v2/scalar_operands.rs) and the planned `parameter_declaration` (IR FR-038 "Typed parameter declaration", AC-258..261), and against QSL's own lowering (qsl-semantics/src/check/lowering.rs). Three internal contradictions: the inline-literal path is unreachable for QSL-compiled packages so AC-2/AC-6/AC-7/AC-8 contradict Behavior step 4; AC-7 contradicts Behavior step 6 on an out-of-i64 range endpoint; and AC-8's `Incomplete` has no arm in the declared return type.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | QSL lowers an integer literal expression to a `value`/`literal` graph node referenced from the application (lowering.rs `literal` -> `value_node` -> `LeafTerm::reference`; qsl-replay tests note "This package lowers every operand as a node"), so IR returns `GraphChild`, `parameter_declaration` refuses `NotParameter`, and Behavior step 4 refuses `OperandNotParameter` for the `1` in `a + 1`. AC-2 (`InlineLiteral`, `[a, 1]`, `1 + a`), AC-6 inner `[a, 1]`, AC-7 and AC-8 `a + 1` binding all contradict step 4; step 5's `InlineLiteral` path is unreachable from a source-compiled package. | FR-367-AC-2, FR-367-AC-6, FR-367-AC-7, FR-367-AC-8, FR-367 Behavior 4-5 |
| FND-002 | high | Behavior step 6 refuses `OperandNotNative` when an operand's range endpoint is not `i64`. In AC-7 `a: Int[0, 100000000000000000000]` has upper endpoint 10^20 > i64::MAX, so every value refuses, yet AC-7 says `9223372036854775807` binds. | FR-367-AC-7, FR-367 Behavior 6 |
| FND-003 | high | Behavior 8 and AC-8 return FR-100's `Incomplete` and no binding, but the signature is `Result<BoundScalarOperands, BindRefusal>` and `BindRefusal` has no `Incomplete` variant; `Admission` carries a `RunRefusal`, and FR-100 reports `Incomplete` as a `CallOutcome`, not a `RunRefusal`. AC-8's limit case has no typed result. | FR-367-AC-8, FR-367 Outputs, FR-367 Behavior 8 |

## Verdict

FIX. Operand order and arity match IR (wire argument order, one entry per argument, `a - a` two operands, `-a` one); the join by parameter node id matches FR-098-AC-2; FR-357 cause reuse for Node/Operator/Occurrence matches FR-357's operator entry; every AC has a TC-1053 step; spec.md has both the contains edge and the index row.

## New findings (disposition pass 1)

Reviewed at agent-ix/quire-spec-language@530ecc8c4 (`git diff 35042dc..530ecc8`).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | AC-6 now declares `function n using v(a: Int[0, 9]): Int[0, 12] pure { (a + 1) + 2 }`, but TC-1053 step 5 still compiles `n` with result `Int[0, 11]`, a different unit whose body range `[3, 12]` exceeds its declared result. The TC fixture no longer matches the AC it verifies. | FR-367-AC-6, TC-1053 step 5 |
| FND-005 | medium | `OperandSource::InlineLiteral { node: Some(n) }` names a source-compiled literal operand that FR-357's operator claim must identify as `GraphChild(n)`: FR-357's `InlineLiteral` is only an inline literal term, and its preimage check refuses `NotInlineLiteral` for a `reference` argument (qsl-replay execute/scalar_site.rs `check_operands`). FR-367 states no mapping from `OperandSource` to FR-357's `OperandIdentity`, so a driver that maps the like-named variants builds a claim FR-357 refuses for every literal operand, while AC-9 claims the binding matches the FR-357 claim. | FR-367 Outputs (OperandSource), FR-367-AC-9, FR-357 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 530ecc8c4 |
| FND-002 | fixed | 530ecc8c4 |
| FND-003 | fixed | 530ecc8c4 |
