---
id: FR-294
title: "Run every execution backend behind one seam that admits checked input only"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-279
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: references
  - target: "ix://agent-ix/quire-specification/FR-306"
    type: depends_on
---
# FR-294: Run every execution backend behind one seam that admits checked input only

## Description

One trait in layer 5 (`qsl-eval`, module `execution`) holds every execution
backend (ADR-029 EB-1):

- `ExecutionBackend::prepare(&self, package: &CheckedPackage, entries:
  &[EntrySelection], limits: &PrepareLimits, cancel: &Cancel)` returns
  `Result<Self::Prepared, PrepareFailure>`. An entry the backend cannot run
  is recorded inside `Self::Prepared` as unsupported for that entry only.
- `PreparedEntries::unsupported(&self) -> &[(EntrySelection,
  UnsupportedEntry)]` lists the selected entries the backend did not
  prepare, in selection order, each with its cause.
- `PreparedEntries::call(entry, arguments, environment, accounting, cancel)`
  and `PreparedEntries::evaluate(clause, input, accounting, cancel)` each
  return `Result<Evaluation<Value>, CallFailure>`, with the contract of
  `CheckedPackageEvaluation::call` and `evaluate`. Calling an entry the
  backend recorded unsupported returns `CallFailure::Unsupported` with that
  entry's `UnsupportedEntry`.

The seam's types:

| Type | Definition |
| --- | --- |
| `EntrySelection` | `Function(QualifiedName)` or `Clause(ClauseSelection)`: one entry to prepare. |
| `PrepareLimits` | `compile_work_units: u64` and `code_bytes: u64`: the translation work and the generated code size one `prepare` may use. Published defaults 16777216 units and 67108864 bytes; setting names `prepare.compile_work_units` and `prepare.code_bytes` (FR-277). The interpreter translates nothing and charges neither. The counters and charge points are in "Prepare charges" below. |
| `PrepareFailure` | `Limit(LimitExceeded)`, naming the `PrepareLimits` field, its configured value and the counter reached; `Cancelled(CancelCause)`; `Fault(InternalFault)`. It fails the whole `prepare` and has no per-entry variant. |
| `UnsupportedEntry` | `backend: ExecutionBackendChoice` (FR-296), the backend that did not run the entry, and `cause: UnsupportedCause`. |
| `UnsupportedCause` | `NotSupplied`: the caller supplied no such backend (FR-296); or `Untranslatable { construct: UntranslatableConstruct, location: Location }`: the first construct of the entry, in clause-node order, that the backend cannot translate, and its source location. |
| `UntranslatableConstruct` | `ExpressionForm(name)`: a checked expression form the backend does not translate, by its ADR-012 form name (`literal`, `operator`, `let`, `if`, `call`, `record`, `collection`, or the owning family's form name); `ValueType(ValueType)`: a checked value type the backend has no representation for; or `Operation(NodeKey)`: the node key of a checked operation the backend does not implement. |

Beside its admission, fault and cancellation arms, `CallFailure` has
`Unsupported(UnsupportedEntry)` and `Limit(PrepareLimitReached)`; `Limit`
comes only from `prepare` (FR-279). `CallFailure` lives in the `quire-eval`
leaf, which depends on no QSL crate (FR-401), so its arm cannot hold
`qsl-foundation`'s `LimitExceeded` and its `Locus`.
`PrepareLimitReached { setting, bound, actual }` is the leaf's own,
locus-free record of a reached prepare limit: `setting` is
`PrepareSetting::CompileWorkUnits` or `PrepareSetting::CodeBytes`, whose
`name()` is `prepare.compile_work_units` or `prepare.code_bytes`; `bound` is
the configured value; `actual` is the counter the refused charge reached. A
prepare budget is charged over the selected entries, not at a source
location, so the record has no locus. Its O-16 category is incomplete and
its catalog code is `stage_limit_exceeded`.
Unsupported is not an evaluation
outcome (ADR-013 O-16), so it is a `CallFailure` arm and never an
`Evaluation`.

### Prepare charges

`prepare` charges `compile_work_units` one unit for each checked node of a
selected entry's body that the backend translates, before it translates that
node. It charges `code_bytes` the byte length of each block of generated
code, or of the built artifact for AOT, before it keeps that block. At each
of those charges it polls `&Cancel` (FR-276): a cancelled handle returns
`PrepareFailure::Cancelled` with the handle's cause at that charge. A charge
that would exceed its bound is not admitted and returns
`PrepareFailure::Limit` with the counter the charge would have reached. A
`prepare` that stops returns no prepared value.

Three backends implement the seam: the interpreter in `qsl-eval`, which
accepts every entry S6a accepts and runs S6a; AOT in the driver's
`quire-aot`; and JIT in QSL's `qsl-jit` (layer X).

The interpreter is the reference: S6a defines what a checked package means,
and replay runs only through S6a (ADR-029 EB-3, ADR-011 FB-07).

`prepare` takes `&CheckedPackage`, which only S4 constructs (ADR-011 §4), so
every backend admits checked input only, by type (ADR-029 EB-2). AOT's
`generate` step takes the `EmittedPackage` that the same run's `check`
produced, and the IR reader checks it against that run's expected
`package_id` (ADR-011 E5).

## Inputs

As the trait signatures state.

## Outputs

As the trait signatures state.

## Behavior

- Layer 5 shall define `ExecutionBackend` and `PreparedEntries` with the
  signatures the Description states.
- The interpreter backend shall accept every entry S6a accepts, and its
  `call` and `evaluate` shall return what `CheckedPackage::call` and
  `CheckedPackage::evaluate` return for the same inputs.
- When a backend cannot run a selected entry, `prepare` shall return
  `Ok` with that entry listed by `unsupported` with an `Untranslatable`
  cause, and shall prepare the other selected entries.
- When `call` or `evaluate` names an entry its `prepare` recorded
  unsupported, the prepared value shall return `CallFailure::Unsupported`
  with that entry's `UnsupportedEntry`.
- When a backend's `prepare` reaches a checked body node to translate, the
  `prepare` operation shall charge one `compile_work_units` unit and poll
  `&Cancel` before translating the node.
- When a backend's `prepare` has a block of generated code or a built
  artifact to keep, the `prepare` operation shall charge its byte length to
  `code_bytes` and poll `&Cancel` before keeping it.
- The `prepare` operation shall name in each `Untranslatable` cause the
  first untranslatable construct's kind and identity and its `Location`.
- When a backend's translation reaches a `PrepareLimits` bound, the
  backend's `prepare` operation shall return `PrepareFailure::Limit` naming
  that field and its configured value.
- Each backend's `prepare` and each `PreparedEntries` method shall honour
  `&Cancel` as FR-276 states.
- The `replay` facade shall evaluate through S6a for every request.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-294-AC-1 | The interpreter backend, prepared over FR-100-AC-4's unit with every function selected, returns for each AC-4 call the `Evaluation` or `CallFailure` that `CheckedPackage::call` returns for the same arguments and accounting limits; for FR-109's ConfigVersion clause, `evaluate` returns what `CheckedPackage::evaluate` returns. | Test (TC-778) |
| FR-294-AC-2 | Passing package bytes, a `ParsedSource` or an unchecked expression to `prepare`, and package bytes read from a file to AOT's `generate` step, each fails to compile (`compile_fail` doctests with compiling controls). | Test (TC-778) |
| FR-294-AC-3 | A test backend standing in for `jit` that cannot run the third of three selected entries returns `Ok` from `prepare`; `unsupported` lists exactly that entry with `UnsupportedEntry { backend: Jit, cause: Untranslatable { construct, location } }`, `construct` and `location` the construct the test backend names; calling the two other entries returns their interpreter results, and calling the third returns `CallFailure::Unsupported` with the same `UnsupportedEntry`. | Test (TC-778) |
| FR-294-AC-4 | `prepare` and `call` with a handle already cancelled return `PrepareFailure::Cancelled` and `CallFailure::Cancelled`. | Test (TC-778) |
| FR-294-AC-5 | `PrepareLimits::default()` has `compile_work_units` 16777216 and `code_bytes` 67108864. A test backend that charges one compile work unit per entry, prepared over three entries with `compile_work_units` 2, returns `PrepareFailure::Limit` naming `prepare.compile_work_units`, configured value 2 and counter 3; with the default it returns `Ok`. The interpreter prepared over the same entries with both fields 0 returns `Ok`. | Test (TC-778) |
| FR-294-AC-6 | A test backend that follows "Prepare charges", prepared over FR-262-AC-1's `count` alone, records one `compile_work_units` charge per checked body node it translates, each before that node, and one `code_bytes` charge per generated block equal to its byte length, each before the block is kept; with `compile_work_units` one below the body's node count it returns `PrepareFailure::Limit` naming `prepare.compile_work_units` with that bound and the node count as the counter, and with `code_bytes` one below the generated byte total it names `prepare.code_bytes` with the total as the counter; at exactly each count it returns `Ok`. Cancelled with `Requested` from a second thread after the observer has seen 3 charges, it returns `PrepareFailure::Cancelled(Requested)`, no prepared value, and at most one charge after the cancellation. | Test (TC-778) |
| FR-294-AC-7 | A test backend standing in for `jit` that does not translate `let`, has no representation for `Decimal` and does not implement one checked operation, prepared over three entries whose first untranslatable constructs are, in clause-node order, a `let`, a `Decimal` parameter and an application of that operation, lists them as `Untranslatable { construct: ExpressionForm("let"), location }`, `Untranslatable { construct: ValueType(Decimal), location }` and `Untranslatable { construct: Operation(k), location }`, `k` the operation's node key and each `location` that construct's; an entry with a `let` after the operation application names the operation. | Test (TC-778) |

## Dependencies

- ADR-029 EB-1 to EB-3: the seam, checked-only input and the reference.
- ADR-011 §4, E5, FB-07: construction, the IR reader and replay.
- ADR-013 T-4: `CallFailure`.
- [FR-276](FR-276-cancel-a-lifecycle-operation.md): cancellation.
- [FR-279](FR-279-execute-a-checked-entry-as-a-library-operation.md): `execute`.
- [FR-098](FR-098-execute-a-replay-request.md): replay.
- QSpec FR-306: AOT and JIT execution, as the QSpec half in References aligns it.

## Overlap

The driver repository owns `quire-aot`: lowering, generation with the AOT
target, the toolchain build and the child-process run.

## References

- QSL-393 (V1-A06a): checked-only AOT and JIT.
- QSL-390 (ARCH-50): owner ruling 3, recorded on the ticket.
- QSpec FR-306 (STD-141): the QSpec half.
