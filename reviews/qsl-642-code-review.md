---
id: SR-1361
title: "Code review of quire-spec-language PR #655: i128 integer bounds and literals end to end (QSL-642)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@90bc007a7d5a66e7ca1122e8cf431c4970625d15; PR #655 diff against origin/main: qsl-forms/src/value.rs, qsl-semantics/src/check/{assemble.rs,check/typing.rs,field_refinement.rs,mod.rs,node_key/mod.rs,node_key/tests.rs,refusal.rs,type_form.rs,lowering/tests.rs,lowering/tests/wide_integers.rs}, qsl-semantics/src/model/{domain_package.rs,index.rs,intake.rs}, qsl-semantics/src/value/{counter.rs,member.rs,mod.rs}, qsl-semantics/tests/it/{model_conformance.rs,model_systems.rs}, qsl-replay/src/{execute.rs,execute/argument.rs,execute/parity_identity.rs,execute/tests.rs,request.rs,spine.rs,spine/wide_integer_tests.rs,witness.rs,witness/value.rs,witness/value_text.rs,witness/value_text/tests.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
---
# Code review of quire-spec-language PR #655

## Summary

Ticket: QSL-642. Code review with the rust-review lane folded in. Reviewed head 90bc007a7. Judged against the plan lead's rulings: i128 ceiling with `IntegerOutsideI128` (`ill_typed`/`type-mismatch`) at check; counters a JCS number to 2^53-1 and a decimal string beyond, `UnsafeInteger` deleted; the narrow `-2^127` fold; FR-033-AC-6 planned on IR-662; `meter_over`; the deleted 50-vector count assert is ceremony; the IR-631 parity pin test is a canonical identity digest.

- S2 fold (qsl-forms/src/value.rs:1142-1176). `is_directly_negated_two_pow_127` folds only when the unary `-` operand has exactly one significant token, an integer literal of value 2^127. Parentheses add tokens, so `-(2^127)` and `- (2^127)` keep `Negate` and refuse at check. Layout and comments between `-` and the literal fold, as FR-091 states. `-(2^127 - 1)` keeps its `Negate`. This matches the narrow-fold ruling.
- Literal ceiling (qsl-semantics/src/check/check/typing.rs:637-649). `ExprNode::Integer` is refused when outside i128, at the literal's location. This is the only `ExprNode::Integer` typing arm. `family.rs` and `assemble.rs` only encode or skip literals, so no path lets a literal bypass the check.
- Bound ceiling (type_form.rs `int_interval`). The lower bound is judged before the upper. The refusal goes through `AssemblyCause::IllFormedBounds`, and `type_fault_class` and `CheckCause::code`/`cause` give `ill_typed`/`type-mismatch`. The rule covers only `Int`; `Rational` bounds go through `interval` untouched, which FR-091 scopes ("caps nothing else").
- Counters (value/counter.rs). One `Serialize` gives a number at or below 2^53-1 and `collect_str` beyond it. It is used for recursion size and ordinal, group_reference ordinal and member position. `UnsafeInteger` and `exact_integer` are gone, and `NodeKeyRefusal` has no magnitude variant.
- Domain-package intake (intake.rs `bound_operand`). It takes a JSON i64, or a string that `parse::<i128>()` reads and `to_string()` round-trips, so `+5`, `0018` and `-0` refuse. The refusal names the keyword and the value. `ScalarTypeRecord`, `RecordIndex` and field refinement are widened to i128 with no narrowing cast.
- Replay. `WitnessValue::Integer` and `WitnessValueType::I128` are widened. The value-text encoder already writes `Integer` as `number.to_string()` (a decimal string), so wide inputs are exact on the wire.
- Native IR lowering (src/lowering.rs:554-560) still parses a literal into the IR's `i64` and refuses ("checked integer literal cannot be represented") rather than truncating. That is the IR-662 gap the rulings accept.
- Rust idioms. No new `unsafe`, no `as` casts and no production panics. One silent saturation (FND-001) and one dead public method (FND-002).

Examined:
- qsl-forms/src/value.rs fold and `is_directly_negated_two_pow_127` (examined)
- qsl-semantics/src/check/type_form.rs `IntegerSite`, `I128Limit`, `IntegerOutsideI128`, `int_interval` (examined)
- qsl-semantics/src/check/check/typing.rs integer literal arm (examined)
- qsl-semantics/src/check/refusal.rs new cause, code and tag (examined)
- qsl-semantics/src/check/node_key/mod.rs counter changes (examined)
- qsl-semantics/src/value/counter.rs, value/member.rs (examined)
- qsl-semantics/src/model/intake.rs `bound_operand` and its tests (examined)
- qsl-semantics/src/check/field_refinement.rs, model/index.rs, model/domain_package.rs widening (examined)
- qsl-replay witness widening and `ReplaySource::measured_bytes` (examined)
- qsl-replay/src/spine/wide_integer_tests.rs, execute/tests.rs TC-913, lowering/tests/wide_integers.rs, node_key/tests.rs, model_conformance.rs (examined)
- src/lowering.rs integer literal lowering (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `impl From<usize> for Counter` maps a failed `u64::try_from` to `u64::MAX` and says in a comment that this "never runs". If it ever did run, the key would hash a wrong counter with no signal. Make the 64-bit assumption a compile-time fact (for example `const _: () = assert!(usize::BITS <= 64);` and then a plain conversion) rather than a silent saturating fallback. | qsl-semantics/src/value/counter.rs:28-35 |
| FND-002 | low | `I128Limit::decimal()` is a new public method with no caller and no test anywhere in the workspace. The refusal carries `limit` as the enum, and nothing renders it. Delete it, or use it where the cause is rendered. | qsl-semantics/src/check/type_form.rs:75-84 |
| FND-003 | low | `ReplaySource::measured_bytes` still charges an `Integer` assignment 8 bytes, and its doc still says "at most 8 bytes for a scalar". `WitnessValue::Integer` is now `i128` (16 bytes), so after the widening the `replay.input_bytes` measure undercounts integer inputs. Charge 16 for `Integer` and update the doc. | qsl-replay/src/witness.rs:400-411 |

## Verdict

The core changes are correct and match the rulings: the S2 fold, the literal and bound ceiling with its typed cause and catalog code, the counter spelling, i128 intake and refinement, and the i128 witness. The three findings are low, and none gives a wrong result on any path the PR's tests reach. Mergeable once the findings are fixed in this PR.

Gates: focused runs at 90bc007a7 through `locked-build.sh` with `QSPEC_DIR` set to QSpec main. qsl-semantics lib QSL-642 tests: 9 passed, including the QSpec `integer_encoding_vectors` conformance. `check::lowering::tests`: 68 passed, all FR-092 vectors. qsl-semantics `it --features test-support` (`wide_domain`, `r08`, `model_systems`): 22 passed. qsl-replay lib (`wide_integer`, `tc_913`, `tc_444`, `parity_identity`, `value_text`): 40 passed, including the IR-631 pin. `cargo clippy -p qsl-semantics -p qsl-replay -p qsl-forms --all-targets -D warnings` is clean. Full `make ci` was not run, per the brief.

## Dispositions

Round 1, reviewed at fe7a43674bff9296046a1222336d6a8606839e5b.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 73f15d363 |
| FND-002 | fixed | 73f15d363 |
| FND-003 | fixed | fe7a43674 |
