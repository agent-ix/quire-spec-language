---
id: FR-367
title: "Bind an admitted Call's named values to a scalar application occurrence in generated-oracle operand order"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: depends_on
---
# FR-367: Bind an admitted Call's named values to a scalar application occurrence in generated-oracle operand order

## Description

A driver that runs a generated scalar oracle needs the concrete operands of
one scalar application occurrence as an ordered `&[i64]` (the generated
oracle's argument order), and needs to know which supplied value fed which
operand. FR-100's `Call` holds its arguments as an unordered list keyed by
parameter name, and Contract IR's `scalar_application_operands` returns the
static operand structure and ranges only. This requirement adds the join:
`qsl_replay::spine::bind_scalar_operands` admits a `Call` as FR-100 does,
selects the occurrence the caller's scalar claim names (FR-357), reads the
occurrence's operands from Contract IR, and returns each operand's value in
operand order with the argument that supplied it. The driver maps nothing by
guess: a name that does not join, an argument left over, a value of the wrong
type or an occurrence the claim does not own refuses with a typed cause, and
no partial vector is returned.

The binding evaluates nothing. It runs no function body, reads no generated
artifact and reports no verdict.

## Inputs

- The compiled and emitted package the call runs against, as FR-100's `run`
  holds it after its compile and package stages, with its `package_id`.
- A `Call` (FR-100): `function`, `arguments`, `accounting`, `supplied`.
- A `ScalarOccurrence`, the scalar claim's location (FR-357 Inputs, operator
  level), with exactly these typed members:

| Member | Value |
| --- | --- |
| `function` | The enclosing function's declared name; it SHALL equal `Call.function` |
| `application` | The scalar application node (`WireNodeId`) |
| `occurrence` | The occurrence key (`OccurrenceKey`) of that node |

## Outputs

`bind_scalar_operands(package, call, occurrence) -> Result<BoundScalarOperands,
BindRefusal>`.

`BoundScalarOperands` has accessors for these typed members:

| Member | Value |
| --- | --- |
| `package_id` | The package's `package_id` |
| `occurrence` | The `ScalarOccurrence` as supplied |
| `operands` | `&[BoundOperand]`, one per application argument, in the application's wire argument order |
| `values()` | `Vec<i64>`: the `value` of each `BoundOperand` in the same order, the generated oracle's argument order |

`BoundOperand` has these typed members:

| Member | Value |
| --- | --- |
| `ordinal` | Zero-based position in the application's arguments (Contract IR `CheckedScalarOperand.ordinal`) |
| `value` | The operand's concrete `i64` |
| `range` | The operand's inclusive range as Contract IR returns it |
| `source` | `OperandSource`, below |

`OperandSource` is exactly one of:

- `Argument { argument, parameter, node, level }`: `argument` is the index of
  the supplying `CallArgument` in `Call.arguments`; `parameter` is its declared
  name; `node` is the parameter node (`WireNodeId`) the operand references;
  `level` is that node's declared level as Contract IR's
  `parameter_declaration(node)` returns it (decimal text).
- `InlineLiteral`: the operand is an inline integer literal term of the
  application; its value is the singleton of its range and no argument fed it.

`BindRefusal` has exactly these variants, each carrying the data named:

| Variant | Carries | Code and cause |
| --- | --- | --- |
| `Admission` | the FR-100 `RunRefusal`, unchanged | that refusal's own: `missing_declaration`, `unsupported_construct`, or `invalid_runtime_input` for `UnknownParameter`, `DuplicateArgument`, `UnboundParameter`, `WrongValueKind`, `Admission`, `DanglingReference` |
| `ScalarIdentity` | a cause: FR-357's `Node`, `Operator` or `Occurrence`, or this requirement's `Function` | `stale_dependency`/`content-mismatch` |
| `Operands` | Contract IR's `CheckedScalarOperandError` | `unsupported_construct`/`expression-form` |
| `OperandNotParameter` | `ordinal` | `unsupported_construct`/`expression-form` |
| `OperandNotNative` | `ordinal`, the value's decimal text | `invalid_runtime_input` |

A defect the compiled package cannot produce (Contract IR's `MalformedBody`,
or a parameter whose declared name differs from the called function's
parameter at the joined node) is an internal failure (FR-100 "Internal
failure at S6a"), never a `BindRefusal`.

## Behavior

1. The entry SHALL admit the call as FR-100's `run` does before the
   evaluation: it SHALL select `Call.function` (`select`), join each argument
   to the parameter of the same declared name, and convert each value to its
   parameter's declared type. It SHALL refuse with `Admission`, carrying the
   `RunRefusal` FR-100 states, for: an argument that names no parameter (an
   extra argument, `UnknownParameter`); two arguments that name one parameter
   (`DuplicateArgument`); a parameter with no argument (a missing argument,
   `UnboundParameter`); and a value that is not of its parameter's declared
   type, including a Boolean for an integer parameter or an integer outside a
   bounded parameter's range (a mistyped argument, `WrongValueKind` or the
   model-aware `Admission`). Admission refusals precede every occurrence
   refusal.
2. The entry SHALL then check the occurrence as FR-357's operator entry
   checks a claim's scalar node, with the same typed causes: `Function` when
   `occurrence.function` differs from `Call.function`; `Node` when
   `application` is no node of the package or lies outside the called
   function's body; `Operator` when it is no application of `add`, `sub`,
   `mul` or `negate`; `Occurrence` when `occurrence` is not one of the node's
   occurrences. Each refuses `ScalarIdentity`. A missing occurrence refuses
   even if another occurrence of the same node exists.
3. The entry SHALL read the operands with Contract IR's
   `scalar_application_operands(application, occurrence)` in the order it
   returns them, and SHALL NOT reorder, deduplicate, sort or skip any operand:
   `a - a` binds two operands, and `b - a` binds `b` first. Any error of that
   accessor refuses `Operands` with the error unchanged, and no operand list is
   returned.
4. For a `GraphChild` operand the entry SHALL read the node with Contract IR's
   `parameter_declaration(&node)`. If it refuses `NotParameter` (the operand
   is a nested application or a graph-held literal), the entry SHALL refuse
   `OperandNotParameter` with the operand's `ordinal`: this requirement
   binds a parameter or an inline literal, and it evaluates no
   subexpression. If it returns a declaration, the entry SHALL join the node to
   the called function's parameter of that node id (the join FR-098-AC-2
   states) and take the value from the `CallArgument` that supplied that
   parameter. Two operands that reference one parameter node bind the same
   argument twice, at their own ordinals. The `level` is carried in the
   `source`; it is not used to choose the argument.
5. For an `InlineLiteral` operand the entry SHALL take the value as the
   singleton of the operand's range, with `source` `InlineLiteral`.
6. The entry SHALL refuse `OperandNotNative` for an operand value, or an
   endpoint of its range, that is not representable as `i64`, and SHALL check
   the operands in ordinal order and refuse the lowest ordinal. A value
   outside its operand's range cannot reach this step: a parameter operand's
   range is its declared type's, which step 1 admitted, and an inline
   literal's is its own value.
7. The returned `values()` SHALL equal the operands' values in `operands`
   order. Binding the same `Call` and occurrence twice SHALL give equal
   results. The order of `Call.arguments` SHALL NOT change `values()`; it
   changes only the `argument` indices in the provenance.
8. The entry SHALL charge only the argument conversion FR-098 states under
   `Call.accounting`; a conversion that reaches a limit reports the `Incomplete`
   FR-100 reports and returns no binding.
9. This requirement changes no catalog code, no FR-357 request member and no
   FR-100 behavior: `run` is unchanged, and the driver passes `values()` and
   `operands` unchanged to the generated oracle and its receipt.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-367-AC-1 | Over `function sub using v(a: Int[0, 9], b: Int[0, 9]): Int[-9, 9] pure { a - b }`, a `Call` with arguments listed `b = 2`, `a = 7` and the occurrence of the `-` application bind `values()` to `[7, 2]`; the same call with the arguments listed `a`, `b` gives `[7, 2]`. The provenance of ordinal 0 is `Argument` index 1, then 0, parameter `a`, and of ordinal 1 the reverse, parameter `b`, each with its parameter node and level. A swapped `b - a` in a separately compiled package gives `[2, 7]`. | Test (TC-1053) |
| FR-367-AC-2 | Over `a + 1`, `values()` is `[a, 1]`, the second operand has `source` `InlineLiteral` and range `(1, 1)`; over `1 + a` it is `[1, a]`. Over `a - a` two operands bind with `Argument` sources of the one argument index, at ordinals 0 and 1. Over `-a` one operand binds. | Test (TC-1053) |
| FR-367-AC-3 | A missing argument (parameter `b` left unbound) refuses `Admission(UnboundParameter { parameter: b })`; an extra argument naming `c` refuses `Admission(UnknownParameter { parameter: c })`; `a` bound twice refuses `Admission(DuplicateArgument)`. Each is `invalid_runtime_input`, returns no operand, and equals the refusal `run` returns for the same call. | Test (TC-1053) |
| FR-367-AC-4 | A mistyped argument, a Boolean value for the integer `a` and an integer `10` for `a: Int[0, 9]`, refuses `Admission` with `WrongValueKind` at `a`'s declared position, `invalid_runtime_input`, and equals the refusal `run` returns. With an unbound `b` and a mistyped `a` the refusal is the one FR-100's order gives, and no occurrence refusal precedes either. | Test (TC-1053) |
| FR-367-AC-5 | An occurrence key the application node does not have refuses `ScalarIdentity(Occurrence)` (`stale_dependency`/`content-mismatch`) even when the node has another occurrence and the arguments are valid; an `application` node of another function, a node outside the called function's body, a node absent from the package and a `ScalarOccurrence.function` that differs from `Call.function` each refuse `ScalarIdentity` with `Node` or `Function` (the cause this requirement adds for a differing function name); a `==`-style non-arithmetic application refuses `Operator`. The two functions of one package that share parameter nodes (FR-092-AC-5) bind each to its own body's application only. | Test (TC-1053) |
| FR-367-AC-6 | `function w using v(a: Integer): Integer pure { a + 1 }` refuses `Operands(UnboundedRange)` (`unsupported_construct`/`expression-form`) with no operand list. In `function n using v(a: Int[0, 9]): Int[0, 11] pure { (a + 1) + 2 }` the inner application binds `[a, 1]`, and the outer, whose first operand is a graph child that is no parameter, refuses `OperandNotParameter { ordinal: 0 }` and binds no partial list. | Test (TC-1053) |
| FR-367-AC-7 | For `function big using v(a: Int[0, 100000000000000000000]): Int[0, 100000000000000000001] pure { a + 1 }`, a value of `9223372036854775808` refuses `OperandNotNative` at ordinal 0 (`invalid_runtime_input`) and `9223372036854775807` binds; a range endpoint past `i64` refuses `OperandNotNative`, never a wrapped or saturated value. | Test (TC-1053) |
| FR-367-AC-8 | Binding the same request twice gives equal results. Binding does not call the function: a call whose evaluation would be `undefined` or exceed `work_units` still binds when its arguments are admitted, and a conversion that reaches `value_occurrences` returns `Incomplete` and no binding. | Test (TC-1053) |
| FR-367-AC-9 | The `BoundScalarOperands` returned for a claim equals, member for member, what FR-357's operator-level claim for the same occurrence holds as its ordered operand values, and `values()` is accepted unchanged as the `operands` of a generated oracle's native request (CG `KaniScalarNativeRequest.operands`); a differently ordered or reconstructed vector is a different binding. | Test (TC-1053) |

## Dependencies

- FR-100 (the `Call`, the argument join, conversion and refusals, `select`).
- FR-098 (AC-2: the join of arguments to the function's parameters by
  parameter node id; conversion charges).
- FR-357 (the operator claim, its `ScalarIdentity` causes, operand ranges and
  the `i64` operand order the generated oracle takes).
- FR-092 (parameter nodes carry no owner; the same node serves every function
  that declares the same name, level and type).
- Contract IR FR-038: `scalar_application_operands` and
  `parameter_declaration` (the typed parameter declaration), read-only here.

## References

- TC-1053 exercises the binding and each refusal.
