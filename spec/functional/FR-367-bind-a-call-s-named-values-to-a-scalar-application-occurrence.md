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

- The package the call runs against: Contract IR's admitted
  `CheckedPackageV2` of the unit's emitted bytes, which the caller admits once
  with `read_checked_package` under the evidence `admission_evidence` returns,
  and the `AdmittedModels` and `CheckedUnit` those bytes were emitted from (the
  I1 and S4 products FR-100's `run` holds before its package stage). The
  caller SHALL pass the admission of the bytes emitted from that
  `CheckedUnit`; the entry checks the pairing by `package_id` and refuses a
  mismatch (Behavior 1).
- A `Call` (FR-100): `function`, `arguments`, `accounting`, `supplied`.
- A `ScalarOccurrence`, the scalar claim's location (FR-357 Inputs, operator
  level), with exactly these typed members:

| Member | Value |
| --- | --- |
| `function` | The enclosing function's declared name; it SHALL equal `Call.function` |
| `application` | The scalar application node (`WireNodeId`) |
| `occurrence` | The occurrence key (`OccurrenceKey`) of that node |

## Outputs

`bind_scalar_operands(admitted, models, checked, call, occurrence, environment,
cancel) -> Result<BindOutcome, Box<RunRefusal>>`, where `admitted` is the
caller's `&CheckedPackageV2`, `models` the `&AdmittedModels`, `checked` the
`&CheckedUnit`, `call` a `&Call`, `occurrence` a `&ScalarOccurrence`,
`environment` the `&ObjectEnvironment` FR-100's `run` takes and `cancel` the
`&Cancel` handle polled as `run` polls it. The entry takes no package bytes:
it neither reads nor admits the emitted package, and reads every operand from
`admitted` as given.

The entry is shaped as FR-100's `run` returns its outcome. The `Err` arm
is FR-100's own `RunRefusal`, unchanged: the call-admission refusals of
Behavior 1 and the internal failure (`RunRefusal::Fault`, FR-100 "Internal
failure at S6a"). The `Ok` arm is exactly one of:

- `BindOutcome::Bound(BoundScalarOperands)`;
- `BindOutcome::Refused(BindRefusal)`, a package, occurrence or operand refusal below.

The binding evaluates nothing, so it reports no `Incomplete` outcome: an
accounting limit the evaluation would reach does not stop it, and a
supplied-value admission stop is the `Err` arm's `RunRefusal::Supplied`, as
`run` returns it.

`admission_evidence(models, checked, emitted, imports) ->
Result<CheckedPackageEvidence, Box<RunRefusal>>` returns the evidence the
caller's one admission needs: the emitter's own evidence, every domain
package document the unit's `model` declarations selected, and the admitted
packages of the dependency closure the unit's `import`s reached, read under
the I2 limits `imports`. It reads no byte of the emitted package. A refusal is
the internal failure `RunRefusal::Fault`.

`BoundScalarOperands` has accessors for these typed members:

| Member | Value |
| --- | --- |
| `occurrence` | The `ScalarOccurrence` as supplied |
| `operands` | `&[BoundOperand]`, one per application argument, in the application's wire argument order |
| `values()` | `Vec<i64>`: the `value` of each `BoundOperand` in the same order, the generated oracle's argument order |

`BoundOperand` has these typed members:

| Member | Value |
| --- | --- |
| `ordinal` | Zero-based position in the application's arguments (Contract IR `CheckedScalarOperand.ordinal`) |
| `value` | The operand's concrete `i64` |
| `range` | The operand's inclusive range as Contract IR returns it (`i128` endpoints) |
| `source` | `OperandSource`, below |

`OperandSource` is exactly one of:

- `Argument { argument, parameter, node, level }`: `argument` is the index of
  the supplying `CallArgument` in `Call.arguments`; `parameter` is its declared
  name; `node` is the parameter node (`WireNodeId`) the operand references;
  `level` is that node's declared level as Contract IR's
  `parameter_declaration(node)` returns it (decimal text).
- `InlineLiteral`: the operand is an integer literal term of the application's
  arguments and no argument fed it. QSL lowers an integer literal operand of
  `add`, `sub`, `mul` and `negate` to such a term, so this is the form a
  source-compiled package yields. Its value is the literal's own value, which
  equals the singleton of its range.

`BindRefusal` has exactly these variants, each carrying the data named:

| Variant | Carries | Code and cause |
| --- | --- | --- |
| `AdmittedPackage` | the admitted package's `package_id` digest, and the checked unit's emitted `package_id` digest or none when the unit was never emitted | `stale_dependency`/`content-mismatch` |
| `ScalarIdentity` | a cause: FR-357's `Node`, `Operator` or `Occurrence`, or this requirement's `Function` | `stale_dependency`/`content-mismatch` |
| `Operands` | Contract IR's `CheckedScalarOperandError` | `unsupported_construct`/`expression-form` |
| `OperandNotParameter` | `ordinal` | `unsupported_construct`/`expression-form` |
| `OperandNotArgument` | `ordinal`, `parameter` name, `level` | `unsupported_construct`/`expression-form` |
| `OperandNotNative` | `ordinal`, the value's decimal text | `invalid_runtime_input` |

The call-admission refusals keep FR-100's own codes: `UnknownParameter`,
`DuplicateArgument`, `UnboundParameter` and `WrongValueKind`/`Admission` of a
value of the wrong kind or outside its type are `invalid_runtime_input`; an
`Admission` carrying a foreign model selection, type or universe is
`foreign_reference` (`foreign-model-selection`, `foreign-type`,
`foreign-universe`); `DanglingReference` is `dangling_reference`
(`absent-target-in-complete-population`); `MissingDeclaration` and
`UnsupportedResult` are FR-100's. Contract IR's `MalformedBody`, and a
parameter whose declared name differs from the called function's parameter at
the joined node, are internal failures (`RunRefusal::Fault`), never a
`BindRefusal`.

## Behavior

1. The entry SHALL first compare the admitted package's `package_id` with the
   `package_id` the emitter computed for the checked unit, and SHALL return
   `Refused(AdmittedPackage)` when they differ or the unit was never emitted,
   before any call admission or occurrence check. The entry SHALL then admit the call as FR-100's `run` does before the
   evaluation: it SHALL select `Call.function` (`select`), join each argument
   to the parameter of the same declared name, and convert each value to its
   parameter's declared type. It SHALL return `Err` with the `RunRefusal`
   FR-100 states, in FR-100's order, for: an argument that names no parameter
   (an extra argument, `UnknownParameter`); two arguments that name one
   parameter (`DuplicateArgument`); a parameter with no argument (a missing
   argument, `UnboundParameter`); a value that is not of its parameter's
   declared type, including a Boolean for an integer parameter or an integer
   outside a bounded parameter's range (a mistyped argument, `WrongValueKind`);
   and the model-aware `Admission` and `DanglingReference` causes. Admission
   refusals precede every occurrence refusal.
2. The entry SHALL then check the occurrence as FR-357's operator entry
   checks a claim's scalar node, with the same typed causes: `Function` when
   `occurrence.function` differs from `Call.function`; `Node` when
   `application` is no node of the package or lies outside the called
   function's body; `Operator` when it is no application of `add`, `sub`,
   `mul` or `negate`; `Occurrence` when `occurrence` is not one of the node's
   occurrences. Each returns `Refused(ScalarIdentity)`. A missing occurrence
   refuses even if another occurrence of the same node exists.
3. The entry SHALL read the operands with Contract IR's
   `scalar_application_operands(application, occurrence)` in the order it
   returns them, and SHALL NOT reorder, deduplicate, sort or skip any operand:
   `a - a` binds two operands, each with provenance to the same
   `CallArgument`, and `b - a` binds `b` first. Any error of that accessor
   returns `Refused(Operands)` with the error unchanged, and no operand list.
4. For each operand the entry SHALL decide its source in this order.
   - An `InlineLiteral` operand of Contract IR binds as `InlineLiteral` with
     the singleton of its range as its value.
   - A `GraphChild`: the entry SHALL call Contract IR's
     `parameter_declaration(&node)`. If it refuses `NotParameter` (the operand
     is a nested application or another non-literal node), the entry SHALL
     return `Refused(OperandNotParameter)` with the operand's `ordinal`: this
     requirement binds a parameter or a literal, and it evaluates no
     subexpression.
   - If `parameter_declaration` returns a declaration, the entry SHALL join the
     node to the called function's parameter of that node id (the join
     FR-098-AC-2 states) and take the value from the `CallArgument` that
     supplied that parameter. A parameter node that is no parameter of the
     called function, such as the binder of a `let` or quantifier (QSL lowers a
     binder to a `value`/`parameter` node at the next level), has no supplied
     value; the entry SHALL return `Refused(OperandNotArgument)` with the
     `ordinal`, the declared name and the level.
   This source maps to FR-357's `OperandIdentity` as follows: an `Argument` is
   `GraphChild(node)` (the node is a reference in the application) and an
   `InlineLiteral` is FR-357's `InlineLiteral`.
   Two operands that reference one parameter node bind the same argument
   twice, at their own ordinals. The `level` is carried in the `source`; it is
   not used to choose the argument.
5. The entry SHALL return `Refused(OperandNotNative)` for an operand value
   (an argument's or a literal's) that is not representable as `i64`, checking
   the operands in ordinal order and refusing the lowest ordinal. An endpoint of
   an operand's range beyond `i64` does not refuse: the check is on the value,
   so a parameter declared `Int[0, 100000000000000000000]` binds a value up to
   `9223372036854775807`. A value outside its operand's range cannot reach this
   step: a parameter operand's range is its declared type's, which step 1
   admitted, and a literal's is its own value.
6. The returned `values()` SHALL equal the operands' values in `operands`
   order. Binding the same `Call` and occurrence twice SHALL give equal
   results. The order of `Call.arguments` SHALL NOT change `values()`; it
   changes only the `argument` indices in the provenance.
7. The entry SHALL charge nothing to a meter and SHALL report an accounting
   limit only where `run` reports it before its evaluation: a supplied-value
   admission stop is `Err(RunRefusal::Supplied)`, as `run` returns it. A call
   that `run` reports `Incomplete` because its evaluation reaches a limit
   (`work_units`, or `value_occurrences` charged by the evaluation) still
   binds.
8. This requirement changes no catalog code, no FR-357 request member and no
   FR-100 behavior: `run` is unchanged, and the driver passes `values()` and
   `operands` unchanged to the generated oracle and its receipt.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-367-AC-1 | Over `function sub using v(a: Int[0, 9], b: Int[0, 9]): Int[-9, 9] pure { a - b }`, a `Call` with arguments listed `b = 2`, `a = 7` and the occurrence of the `-` application bind `values()` to `[7, 2]`; the same call with the arguments listed `a`, `b` gives `[7, 2]`. The provenance of ordinal 0 is `Argument` with index 1, then 0, parameter `a`, and of ordinal 1 the reverse, parameter `b`, each with its parameter node and level. A swapped `b - a` in a separately compiled package gives `[2, 7]`. | Test (TC-1053) |
| FR-367-AC-2 | Over `a + 1` with `a = 4`, `values()` is `[4, 1]`; the second operand is an inline integer literal term of the application, has `source` `InlineLiteral` and range `(1, 1)`, and no graph node holds the literal; over `1 + a` it is `[1, 4]`. Over `a - a` two operands bind, `[4, 4]`, with `Argument` sources of the one argument index at ordinals 0 and 1. Over `-a` one operand binds. | Test (TC-1053) |
| FR-367-AC-3 | A missing argument (parameter `b` left unbound) returns `Err(UnboundParameter { parameter: b })`; an extra argument naming `c` returns `Err(UnknownParameter { parameter: c })`; `a` bound twice returns `Err(DuplicateArgument)`. Each is `invalid_runtime_input`, returns no operand, and equals the refusal `run` returns for the same call. | Test (TC-1053) |
| FR-367-AC-4 | A mistyped argument, a Boolean value for the integer `a` and an integer `10` for `a: Int[0, 9]`, returns `Err(WrongValueKind)` at `a`'s declared position, `invalid_runtime_input`, and equals the refusal `run` returns. A reference argument whose model selection, type or universe is foreign returns `Err(Admission)` with `foreign_reference` and its cause (`foreign-model-selection`, `foreign-type` or `foreign-universe`), and a reference absent from the complete population returns `Err(DanglingReference)` with `dangling_reference`/`absent-target-in-complete-population`; each equals the refusal `run` returns. With an unbound `b` and a mistyped `a` the refusal is the one FR-100's order gives, and no occurrence refusal precedes any of them. `foreign-type` is not reachable from a source-compiled call, because a reference parameter's declared type always belongs to a selected model; its cause is verified on the shared admission function (TC-792) and not at this entry. | Test (TC-1053) |
| FR-367-AC-5 | An occurrence key the application node does not have returns `Refused(ScalarIdentity(Occurrence))` (`stale_dependency`/`content-mismatch`) even when the node has another occurrence and the arguments are valid; an `application` node of another function, a node outside the called function's body and a node absent from the package each return `ScalarIdentity(Node)`; a `ScalarOccurrence.function` that differs from `Call.function` returns `ScalarIdentity(Function)` (the cause this requirement adds); an equality application returns `ScalarIdentity(Operator)`. Two functions of one package that share parameter nodes (FR-092-AC-5) bind each to its own body's application only. | Test (TC-1053) |
| FR-367-AC-6 | `function w using v(a: Integer): Integer pure { a + 1 }` returns `Refused(Operands(UnboundedRange))` (`unsupported_construct`/`expression-form`) with no operand list. In `function n using v(a: Int[0, 9]): Int[0, 12] pure { (a + 1) + 2 }` compiled from source, the inner application binds `[a, 1]` and the outer returns `Refused(Operands(UnboundedRange))` with no partial list: source compilation types the inner application as an unbounded `Integer`, and Contract IR's errors precede operand classification (Behavior 3). `Refused(OperandNotParameter { ordinal })` is the case of an operand list Contract IR admitted whose graph child is neither a literal nor a parameter; it is tested with a hand-built operand naming the nested application's node, resolved against the compiled package, and binds no partial list. | Test (TC-1053) |
| FR-367-AC-7 | For `function big using v(a: Int[0, 100000000000000000000]): Int[0, 100000000000000000001] pure { a + 1 }`, whose range endpoint exceeds `i64`, the value `9223372036854775807` binds as `[9223372036854775807, 1]` and `9223372036854775808` returns `Refused(OperandNotNative)` at ordinal 0 (`invalid_runtime_input`), never a wrapped or saturated value. A literal operand whose value exceeds `i64` (`a + 9223372036854775808`) returns `Refused(OperandNotNative)` at its ordinal. | Test (TC-1053) |
| FR-367-AC-8 | Binding the same request twice gives equal results. Binding does not call the function: a call whose evaluation would exceed `work_units` still binds when its arguments are admitted. The `undefined` half is not exercised for three kernel undefined reasons, which the checker discharges statically: an integer division by a divisor not proved nonzero, a `reduce` over a source not proved nonempty (`Unproved(NonemptyReduction)`), and a `sum` whose range is not proved inside its declared domain (`Unproved(Range)`) each refuse the unit at check, over a scalar or a `Reference` argument alike. Not covered: none-value, ieee-not-finite. A call that `run` reports `Incomplete` at `work_units` still binds, and binding reports no `Incomplete` outcome. | Test (TC-1053) |
| FR-367-AC-9 | The `BoundScalarOperands` returned for a claim equals, member for member, what FR-357's operator-level claim for the same occurrence holds as its ordered operand values, and `values()` is accepted unchanged as the `operands` of a generated oracle's native request (CG `KaniScalarNativeRequest.operands`); a differently ordered or reconstructed vector is a different binding. | Test (TC-1053) |
| FR-367-AC-10 | Over `function lt using v(a: Int[0, 9]): Int[0, 10] pure { let x = a in x + 1 }`, the `+` occurrence returns `Refused(OperandNotArgument { ordinal: 0, parameter: x, level })` with the binder's declared name and level (`unsupported_construct`/`expression-form`) and binds no partial list; the same holds for a quantifier-bound variable operand, such as `x` in `forall(x in deref(n).items: x + 1 > 0)` over a `Reference<M::Node>` argument. | Test (TC-1053) |
| FR-367-AC-11 | The entry binds from the caller's one admission: with the emitted unit and its bytes dropped after the caller's `read_checked_package` call, `bind_scalar_operands` over the admitted `CheckedPackageV2` and the unit's `AdmittedModels` and `CheckedUnit` binds `sub`'s `-` occurrence to `[7, 2]` for `b = 2`, `a = 7`. The entry has no byte or `EmittedUnit` input, so by signature it can neither read nor admit the emitted package a second time; the compiler enforces this, and the test observes the binding only. | Test (TC-1053) |
| FR-367-AC-12 | An admitted package that is not the admission of the checked unit's own emitted bytes, such as the admission of `sub` alone passed with the `CheckedUnit` of `sub` plus a second function `neg`, returns `Refused(AdmittedPackage)` (`stale_dependency`/`content-mismatch`) carrying both `package_id` digests, even though `sub`'s `-` occurrence is a node of both packages; the same occurrence binds `[7, 2]` over the second unit's own admission. | Test (TC-1053) |

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
