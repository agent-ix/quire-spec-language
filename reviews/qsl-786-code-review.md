---
id: SR-12042
title: "QSL-786 PR #921 code review: String, UUID and Timestamp literals"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ba47de62637888e49170fb6561d9559a26e65c8a; qsl-cst/src/token.rs, qsl-forms/src/syntax.rs, qsl-forms/src/value.rs, qsl-semantics/src/check/check/native_literal.rs, qsl-semantics/src/check/check/typing.rs, qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/node_key/mod.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/src/check/family.rs, qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/checked_dispatch.rs"
review_set: subset
---

## Summary

Ticket: QSL-786. PR quire-spec-language#921 at head ba47de626, diff `origin/main...origin/qsl-786-impl` (20 files). Code review with the rust-review lane folded in. Read-only: no build or test run. Major bugs only, as briefed.

Checked: `decode_quoted` as the one decoder shared by the lexer and S2; `ExprNode::String` typed `String` with no hint use; `NativeConstructor` interception of unqualified `UUID`/`Timestamp` calls before `Application::resolve`; the canonical-text UUID check through `Uuid::from_canonical_text`; the Timestamp argument forms (integer, directly negated integer, S2-folded `i128::MIN`); the `IntegerOutsideI128` positive-literal rule with limit `i128::MAX` for a negated overflow; `Timestamp(-0)` folding to `"0"`; refusal locations (argument vs call); the ambiguity check against declared functions and type forms; lowering of `Value::String/Uuid/Timestamp` to `LiteralValue::String/Uuid/Timestamp` with kinds `string`/`uuid`/`timestamp`; and the family encoding of the new form.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The ambiguous-name refusal for a native literal call does not name the native constructor as a candidate, and for a declared type form it names no declaration. `single_candidate` emits `CheckCause::AmbiguousName { loci: [use, Body root of function UUID] }`; `AmbiguousName` has no slot for a span-less native candidate (the assembly rule it is meant to mirror carries `native: bool`). When the clash is a declared tuple (or other type form) named `UUID`/`Timestamp`, the refusal is the `type_named` one with `loci: [use]` only, so the declaration span is missing too. FR-091-AC-42 and the "Name resolution" section require the use, each declaration's span, and the native constructor without a span. | qsl-semantics/src/check/check/native_literal.rs:112-137, qsl-semantics/src/check/refusal.rs:626-631, qsl-semantics/src/check/type_form.rs:148-151 |

## Verdict

PASS (no high findings). FND-001 is a medium spec deviation in the refusal payload; the code and cause are right, so the main path is correct. It is cheap to fix in-PR: carry a `native: bool` (or a span-less candidate) on the check-time ambiguity and give the type-form clash its declaration loci, then assert both in the test.

The rest is sound:

- `decode_quoted` is the lexer's own decoder, so S2 cannot drift from the lexer; S2 maps a decode failure to `unexpected`, which the lexer makes unreachable.
- The String literal is always `ValueType::String`; a `Text`, `UUID` or `Reference` slot then refuses through the existing mismatch path.
- `Timestamp(-170141183460469231731687303715884105728)` reaches check as the S2-folded `Integer(i128::MIN)` and is admitted; `Timestamp(-170141183460469231731687303715884105729)` is `Negate(Integer(2^127+1))` and refuses on the magnitude with limit `Max` and the positive value, at path `[0, 0]`, as FR-091-AC-43 states. `-0` is canonicalised to `"0"`, so `Timestamp(-0)` and `Timestamp(0)` are one value and one lowered node.
- Wrong argument forms refuse `type-mismatch` at the argument, and wrong arity at the call. `M::UUID` is not intercepted (exact-name match), so it resolves as a qualified call.
- Lowering keys the three literal kinds distinctly from `text`, so `"abc"` as String and as Text cannot share a node; equal literals share by key.
- No panics, unsafe, or unchecked integer conversions added.
