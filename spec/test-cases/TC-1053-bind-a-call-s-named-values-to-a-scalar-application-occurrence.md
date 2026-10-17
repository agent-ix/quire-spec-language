---
id: TC-1053
title: "Bind a call's named values to a scalar application occurrence in operand order"
type: TC
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-367"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-357"
    type: verifies
  - target: "ix://agent-ix/quire-spec-language/FR-100"
    type: verifies
---
# TC-1053: Bind a call's named values to a scalar application occurrence in operand order

## Description

Verify that `bind_scalar_operands` returns the operand values of one scalar
application occurrence in the application's argument order, independent of the
order the call lists its arguments, with the argument that fed each operand;
that it refuses a missing, extra, duplicate and mistyped argument with FR-100's
own refusals; and that it refuses an occurrence the claim does not own and an
operand it cannot bind.

Catches an operand order taken from the call's argument order, a vector
sorted or deduplicated by parameter name, an inline literal dropped from the
vector, a nested application operand bound by guess, a wrapped or saturated
`i64`, and a binding that evaluates the function.

## Test Procedure

Every unit starts with the complete-V1 header and one profile selection `v`.
Compile and emit each unit as FR-100's `run` does, then call
`bind_scalar_operands` with the stated `Call` and `ScalarOccurrence`.

1. `function sub using v(a: Int[0, 9], b: Int[0, 9]): Int[-9, 9] pure { a - b }`:
   bind arguments listed `b = 2, a = 7`, then `a = 7, b = 2`; compile
   `b - a` as a second unit and bind the same values.
2. `a + 1`, `1 + a`, `a - a` and `-a` over `a: Int[0, 9]`, `a = 4`; and a
   package whose application holds an inline literal term.
3. For `sub`: leave `b` unbound; add an extra argument `c`; bind `a` twice;
   give `a` a Boolean; give `a` the integer `10`; then leave `b` unbound and
   give `a` a Boolean together; and, over a model unit, give a reference
   argument of a foreign model selection, of a foreign universe, and one
   absent from the complete population. Run the same `Call` through `run` and
   compare each refusal. A foreign type is not reachable from a
   source-compiled call, so that cause is verified on the shared admission
   function (TC-792).
4. For `sub`: an occurrence key the node lacks while another exists; a node of
   a second function `other` that shares the parameter nodes of `sub`; a node
   outside `sub`'s body; a node absent from the package; a
   `ScalarOccurrence.function` that differs from `Call.function`; and an
   equality application.
5. `function w using v(a: Integer): Integer pure { a + 1 }` and
   `function n using v(a: Int[0, 9]): Int[0, 12] pure { (a + 1) + 2 }`; bind
   the inner and the outer application of `n`; then resolve a hand-built
   operand naming the nested application's node against the compiled `n`.
6. `function big using v(a: Int[0, 100000000000000000000]): Int[0, 100000000000000000001] pure { a + 1 }`
   with `a = 9223372036854775807` and `a = 9223372036854775808`, and
   `a + 9223372036854775808` over `a: Int[0, 9]`.
7. A function whose evaluation would be undefined (`1 / b > 0 and a + 1 > 0`
   with `b = 0`) with a `work_units` of 1, binding its `+` occurrence; then
   `a + 1` with a `work_units` of 0, which `run` reports `Incomplete`.
8. Bind step 1's request twice, and the FR-357 claim for the same occurrence;
   compare `values()` with the claim's operand values.

## Expected Results

- Step 1: `[7, 2]` for both listings, provenance argument indices `1, 0` and
  `0, 1`, parameters `a` then `b`, each with its parameter node and level;
  `b - a` gives `[2, 7]`.
- Step 2: `[4, 1]` with a `value`/`literal` graph-node `InlineLiteral`
  second operand of range `(1, 1)`; `[1, 4]`; `[4, 4]` from one argument
  index at two ordinals; `[4]`; the inline literal term binds
  `InlineLiteral { node: None }`.
- Step 3: `Err` with `UnboundParameter`, `UnknownParameter`,
  `DuplicateArgument`, `WrongValueKind`, `WrongValueKind`, and for the
  combined case FR-100's first refusal; `foreign_reference` with
  `foreign-model-selection` and `foreign-universe`;
  `dangling_reference`/`absent-target-in-complete-population`. Every one equals
  the refusal `run` returns, and no vector is returned.
- Step 4: `ScalarIdentity` with `Occurrence`, `Node`, `Node`, `Node`,
  `Function` and `Operator`, in that order; `other`'s application binds only
  under `Call.function = other`.
- Step 5: `Refused(Operands(UnboundedRange))`; the inner binds `[a, 1]`; the
  outer, compiled from source, returns `Refused(Operands(UnboundedRange))`;
  the hand-built nested-application operand returns
  `Refused(OperandNotParameter { ordinal: 0 })`; `lt` and its
  quantifier variant return `Refused(OperandNotArgument)` at ordinal 0 with the
  binder's name and level.
- Step 6: `[9223372036854775807, 1]` binds despite the range endpoint beyond
  `i64`; the next value and the literal `9223372036854775808` return
  `Refused(OperandNotNative)` at their ordinals.
- Step 7: the first returns `Bound` with `[4, 1]` and the function is not
  called, though `run` is `Undefined`; the second returns `Bound` with `[4, 1]`
  though `run` is `Incomplete` at `work_units`.
- Step 8: equal results; `values()` equals the claim's ordered operand values.
