---
id: FR-560
title: "Translate the JIT coverage set and report every other construct as Untranslatable"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-295
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-035
    type: references
  - target: "ix://agent-ix/quire-specification/FR-306"
    type: depends_on
---
# FR-560: Translate the JIT coverage set and report every other construct as Untranslatable

## Description

The JIT backend (`qsl-jit`, ADR-029 EB-1) SHALL publish the constructs it
translates as one typed value, `qsl_jit::coverage() -> &'static JitCoverage`,
translate every selected entry whose constructs all lie in that coverage, and
list every other selected entry as unsupported with FR-294's
`Untranslatable` cause.

`JitCoverage` holds three sets: the expression forms, by their ADR-012 form
names; the value types; and the operations, each a Value-family operator over
named operand types. A construct "lies in the coverage" when its form, the
value type of every operand and result it has, and its operation are each
listed.

### The coverage floor

The coverage SHALL list at least:

- **Forms.** The ADR-012 `Value` family parsing forms `literal`, `operator`,
  `let`, `if`, `call`, `record` and `collection`. A function declaration is
  an entry, not a translated form.
- **Value types.** `Boolean`; `Integer`, both bounded (`Int[lo, hi]`) and
  unbounded; `Rational`; `Decimal` under the selected profile; `Text`; and
  records and collections whose members are of these types.
- **Operations.** Every `Value`-family operator over the listed value types.

The JIT may list more. Every construct it lists is held to the interpreter by
the parity gate (FR-563), and an entry whose constructs all lie in the
coverage is never listed unsupported.

### The cause of an untranslatable entry

For an entry with a construct outside the coverage, `prepare` lists the entry
with `UnsupportedEntry { backend: Jit, cause: Untranslatable { construct,
location } }` (FR-294), naming the first such construct in clause-node order
and its source `Location`:

| The first construct outside the coverage is | `construct` |
| --- | --- |
| a form the coverage does not list, of any family | `ExpressionForm(name)`, `name` its ADR-012 form name |
| a construct of a listed form with an operand, result or binding whose value type the coverage does not list | `ValueType(t)`, `t` that value type |
| a listed form and types, but an operation the coverage does not list | `Operation(k)`, `k` the operation's node key |
| a `call` to a function the JIT lists unsupported | `Operation(k)`, `k` the node key of the `call` |

The callee in the last row is listed with its own cause. When a form is
unlisted, its row wins over the value-type row for the same construct; when
the form is listed and a type is not, the value-type row wins over the
operation row.

## Inputs

A checked package and the selected entries (FR-294).

## Outputs

`JitCoverage`; the prepared entries and `PreparedEntries::unsupported`.

## Behavior

- The JIT shall return its coverage from `qsl_jit::coverage()`, holding at
  least the coverage floor.
- When every construct of a selected entry lies in the coverage, the JIT's
  `prepare` shall translate the entry without listing it unsupported.
- When a selected entry has a construct outside the coverage, the JIT's
  `prepare` shall list the entry unsupported with the `Untranslatable` cause
  the table gives for the first such construct in clause-node order.
- When a selected entry is listed unsupported, the JIT's `prepare` shall
  prepare the other selected entries.
- When a translated entry calls a function the JIT lists unsupported, the
  JIT's `prepare` shall list the calling entry unsupported with
  `Operation(k)`, `k` the node key of the `call`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-560-AC-1 | `qsl_jit::coverage()` lists the seven floor forms, the floor value types (`Boolean`, bounded and unbounded `Integer`, `Rational`, `Decimal`, `Text`, and records and collections of them) and every `Value`-family operator over them. | Test (TC-1380) |
| FR-560-AC-2 | The JIT prepared over each of the 10,000 units the FR-295 parity program generator yields at the default seed, every function selected, lists no entry unsupported; prepared over `add(a: Integer, b: Integer): Integer { a + b }`, `half(a: Rational): Rational { a / 2 }` and `name(t: Text): Text { t }`, it lists none of the three unsupported. | Test (TC-1380) |
| FR-560-AC-3 | For a form and a value type that `coverage()` does not list (the test takes the `SumCase` form `case` and the type `Timestamp` when unlisted, and otherwise the first unlisted ADR-012 form and K2 value type), an entry whose first outside construct is that form is listed with `ExpressionForm(<its name>)`, one whose first outside construct is a parameter of that type with `ValueType(<that type>)`, each with that construct's `Location`; an entry that calls the first entry is listed with `Operation(k)`, `k` the node key of the `call`, while the first entry keeps its own cause; and the other selected entries are prepared. | Test (TC-1381) |
| FR-560-AC-4 | Over a property-generated set of entries built from the listed forms and types, the JIT lists none unsupported, and over the same entries with one construct replaced by an unlisted form, each is listed with that form's `ExpressionForm` cause. | Test (TC-1381) |

## Dependencies

- ADR-029 EB-1, EB-4 rule 6, EB-7: the JIT backend and its unsupported
  entries.
- ADR-012: the family form names.
- [FR-294](FR-294-run-every-execution-backend-behind-one-seam.md): `UnsupportedEntry`, `UntranslatableConstruct` and clause-node order.
- [FR-295](FR-295-return-interpreter-outcomes-from-jit-and-aot.md): the parity program generator.
- [FR-563](FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md): the gate that fails on a coverage gap.
- QSpec FR-306: an entry a backend cannot translate settles unsupported for
  that entry only.

## References

- QSL-393 (V1-A06a): checked-only AOT and JIT.
- [ADR-035](../decisions/ADR-035-jit-code-generator-selection-criteria.md): the code generator, which this coverage does not depend on.
