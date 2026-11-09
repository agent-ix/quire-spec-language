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
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-035
    type: references
  - target: "ix://agent-ix/quire-specification/FR-306"
    type: depends_on
---
# FR-560: Translate the JIT coverage set and report every other construct as Untranslatable

## Description

The JIT backend (`qsl-jit`, ADR-029 EB-1) SHALL publish the constructs it
translates as one typed value, `qsl_jit::coverage() -> &'static BackendCoverage`,
translate every selected entry whose constructs all lie in that coverage, and
list every other selected entry as unsupported with FR-294's
`Untranslatable` cause.

`BackendCoverage` is a seam type in `qsl-eval`'s `execution` module, beside
FR-294's types, so the parity runner (FR-563) reads a coverage without
depending on `qsl-jit`. It holds three sets: the expression forms, by their ADR-012 form
names; the value types; and the operations, each a Value-family operator over
named operand types. A construct "lies in the coverage" when its form, the
value type of every operand and result it has, and its operation are each
listed.

### The coverage floor

The coverage SHALL list at least:

- **Forms.** The ADR-012 `Value` family parsing forms `literal`, `name`,
  `operator`, `let`, `if`, `call`, `record` and `collection`. `name` is the
  read of a parameter or of a `let` binding, the checked syntax's `Name`
  variant, named by the lower-casing rule below. A function declaration is
  an entry, not a translated form.
- **Value types.** `Boolean`; `Integer`, both bounded (`Int[lo, hi]`) and
  unbounded; `Rational`; `Decimal` under the selected profile; `Text`; and
  records and collections whose members are of these types. A union type
  is not a record: the coverage lists it only if the JIT translates the
  union's construction and `case` (the `SumCase` family), and v1 does not, so
  a construct with a union operand, result or binding is outside the
  coverage. A representation of coverage value types that groups several
  checked type kinds under one name (a `Record` entry that also covers a
  union as a composite) lists a union only when the JIT translates it.
- **Operations.** Every `Value`-family operator over the listed value types, except `present`, which the coverage does not list. The `record` form includes a record's field read (`e.f` over a record value); the `collection` form excludes the binder queries `forall` and `exists`, which the coverage does not list.

The JIT may list more. Every construct it lists is held to the interpreter by
the parity gate (FR-563), and an entry whose constructs all lie in the
coverage is never listed unsupported.

### Clause entries and the records beside the evaluation

A clause entry's seam result is the `Evaluation` (FR-294), which the JIT
compares with the interpreter's; the trail and receiver path FR-265 and
FR-107 record beside the evaluation are not part of that result. A clause
whose constructs all lie in the coverage is translated, and the JIT never
lists it unsupported, and never reports a `Location` or a cause, because the
trail or receiver path is not reproduced: the closed `UnsupportedCause` set
(FR-294) has no such member, and the parity gate (FR-563) compares no such
record. A clause that reads `self` is outside the coverage by the `self` row
below.

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

The node key `k` of a checked operation is the `NodeKey` of the FR-322 node
that FR-093 lowers the operation to, which `check` exposes on the checked
operation (FR-093 "One node per checked expression"); the `Location` is the
operation's own site.

The v1 coverage lists no construct of the `StateModel`, `ProtocolClause` or
`SumCase` family, and none of the `Value` constructs named below. The first
such construct of an entry is
`ExpressionForm(name)`, `name` the construct's ADR-012 name, for each of:

| Construct | `name` | Family (ADR-012) |
| --- | --- | --- |
| a universal binder query, `forall(x in c: p)` | `forall` | `Value` |
| an existential binder query, `exists(x in c: p)` | `exists` | `Value` |
| a presence test, `present(e)` | `present` | `Value` |
| a dereference of a model element reference | `deref` | `StateModel` |
| a field read whose operand is an object value or a `deref` (a record's field read is the floor's `record` form) | `field` | `StateModel` |
| a reachability test over a state clause's population | `reaches` | `StateModel` |
| the `self` of a state clause | `self` | `ProtocolClause` |
| a `case` over a union | `case` | `SumCase` |

The names are the variant names of the checked syntax (the binder-query
variants `Forall` and `Exists`, `Present`, `Deref`, `Field`, `SelfRef`,
`Reaches`, `Case`), lower-cased. The interpreter, which
FR-563 holds as the parity reference, is the only evaluator of such an entry;
the JIT never runs it and `execute` never falls back (FR-296).

The scalar constructs the v1 coverage lists are arithmetic, ordering,
equality, boolean connectives, `let`, `if` and application (`call`) over
`Integer` and `Boolean`, with the other floor value types and operations
above.

An entry the JIT cannot prepare because the operating system refuses
executable memory (a `PROT_EXEC` `mmap` or `mprotect` denied, as on a
hardened Linux) is listed unsupported with
`UnsupportedCause::EnvironmentRefused(ExecutableMemoryRefused)` (FR-294), for
every selected entry, and `prepare` returns `Ok`. This cause belongs to QSL's
seam type `UnsupportedCause`; it is not a result cause of QSpec's FR-331
provider envelope, whose `unsupported` causes are a closed set that holds no
executable-memory member. A report that serializes it for a FR-331 consumer
carries no `cause` member for it: it carries the diagnostic of the exit-21
unsupported category (FR-285) naming the backend `jit` and the refusal. The refusal is not a
`PrepareFailure::Fault`, and a `call` on such an entry returns
`CallFailure::Unsupported` with that entry (FR-294), which the driver
reports as unsupported, exit 21 (FR-285).

The callee in the last row is listed with its own cause. When a form is
unlisted, its row wins over the value-type row for the same construct; when
the form is listed and a type is not, the value-type row wins over the
operation row.

## Inputs

A checked package and the selected entries (FR-294).

## Outputs

`BackendCoverage`; the prepared entries and `PreparedEntries::unsupported`.

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
- When a selected entry's first construct outside the coverage is a
  `forall` or `exists` binder query, a presence test, a dereference, a
  field read on an object value, a reachability test, the `self` of a state
  clause or a `case`, the JIT's `prepare` shall list the entry unsupported
  with the `ExpressionForm` cause the table above names.
- When the operating system refuses the JIT executable memory, the JIT's
  `prepare` shall return `Ok` with every selected entry listed unsupported
  with `EnvironmentRefused(ExecutableMemoryRefused)`, and shall not return
  `PrepareFailure::Fault`.
- When a translated entry calls a function the JIT lists unsupported, the
  JIT's `prepare` shall list the calling entry unsupported with
  `Operation(k)`, `k` the node key of the `call`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-560-AC-1 | `qsl_jit::coverage()` lists the eight floor forms, the floor value types (`Boolean`, bounded and unbounded `Integer`, `Rational`, `Decimal`, `Text`, and records and collections of them) and every `Value`-family operator over them. | Test (TC-1380) |
| FR-560-AC-2 | The JIT prepared over each of the 10,000 units the FR-295 parity program generator yields at the default seed, every function selected, lists unsupported only entries whose first outside construct is one the table names, and lists no entry whose constructs all lie in the coverage; prepared over `add(a: Integer, b: Integer): Integer { a + b }`, `half(a: Rational): Rational { a / 2 }` and `name(t: Text): Text { t }`, it lists none of the three unsupported. | Test (TC-1380) |
| FR-560-AC-3 | For a form and a value type that `coverage()` does not list (the test takes the `SumCase` form `case` and the type `Timestamp` when unlisted, and otherwise the first unlisted ADR-012 form and K2 value type), an entry whose first outside construct is that form is listed with `ExpressionForm(<its name>)`, one whose first outside construct is a parameter of that type with `ValueType(<that type>)`, each with that construct's `Location`; an entry that calls the first entry is listed with `Operation(k)`, `k` the node key of the `call`, while the first entry keeps its own cause; and the other selected entries are prepared. | Test (TC-1381) |
| FR-560-AC-4 | Over a property-generated set of entries built from the listed forms and types, the JIT lists none unsupported, and over the same entries with one construct replaced by an unlisted form, each is listed with that form's `ExpressionForm` cause. | Test (TC-1381) |
| FR-560-AC-5 | Over one entry each for `forall`, `exists`, `present`, `deref`, a field read on an object value, `reaches`, `self` and `case`, `prepare` lists each entry unsupported with `Untranslatable { construct: ExpressionForm(<its name from the table>), location }`, `location` that construct's, and `unsupported` holds no other entry; the FR-563 runner over the same entries reports each with its `UnsupportedEntry` and records no `coverage-gap`. | Test (TC-1381) |
| FR-560-AC-6 | With a test executable-memory provider that denies every request (standing in for a `PROT_EXEC` refusal), `prepare` over three selected entries returns `Ok`, `unsupported` lists all three with `UnsupportedCause::EnvironmentRefused(ExecutableMemoryRefused)`, `PrepareFailure` is not returned, `call` on each returns `CallFailure::Unsupported` with that entry, and the exit code of the resulting outcome is 21; with the provider allowing the request, the same entries are translated. | Test (TC-1381) |
| FR-560-AC-7 | A selected state clause whose constructs all lie in the coverage (no `self`) is translated and not listed unsupported; its `evaluate` returns the interpreter's `Evaluation`, and no `unsupported` entry, `Location` or cause exists for a trail or receiver path. | Test (TC-1381) |
| FR-560-AC-8 | An entry whose parameter is of a union type, with no `case` in it, is listed `Untranslatable { construct: ValueType(<the union type>), location }` with that parameter's `Location`, while `coverage()` lists no union type; `coverage()` lists the form `name`, and `add(a: Integer, b: Integer): Integer { a + b }` reads two names and is translated. | Test (TC-1381) |

## Dependencies

- ADR-029 EB-1, EB-4 rule 6, EB-7: the JIT backend and its unsupported
  entries.
- ADR-012: the family form names.
- [FR-294](FR-294-run-every-execution-backend-behind-one-seam.md): `UnsupportedEntry`, `UntranslatableConstruct` and clause-node order.
- [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md): the node key of a checked operation.
- [FR-295](FR-295-return-interpreter-outcomes-from-jit-and-aot.md): the parity program generator.
- [FR-563](FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md): the gate that fails on a coverage gap.
- QSpec FR-306: an entry a backend cannot translate settles unsupported for
  that entry only.

## References

- QSL-393 (V1-A06a): checked-only AOT and JIT.
- [ADR-035](../decisions/ADR-035-jit-code-generator-selection-criteria.md): the code generator, which this coverage does not depend on.
