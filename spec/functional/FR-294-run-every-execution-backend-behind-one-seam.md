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
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-286
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
  settles `Unsupported` for that entry only.
- `PreparedEntries::call(entry, arguments, environment, accounting, cancel)`
  and `PreparedEntries::evaluate(clause, input, accounting, cancel)` each
  return `Result<Evaluation<Value>, CallFailure>`, with the contract of
  `CheckedPackageEvaluation::call` and `evaluate`.
- `PrepareFailure` holds per-entry `Unsupported` causes, `Limit`,
  `Cancelled` and `Fault`.

### Prepare limits

`PrepareLimits` has two fields, each an FR-255 setting with a published
default:

| Field | Setting | Limit kind | Counter | Default |
| --- | --- | --- | --- | --- |
| `work_units` | `prepare.work_units` | work budget | one unit per checked node the backend translates, charged before it translates the node | 16777216 units |
| `code_bytes` | `prepare.code_bytes` | input bytes | the byte length of each block of generated code or built artifact, charged before the backend keeps it | 67108864 bytes |

Reaching either returns `PrepareFailure::Limit(LimitExceeded)` with the
limit kind (`work-budget-exceeded` or `input-bytes-exceeded`), the
configured bound, the count the refused charge would have reached and the
setting (FR-277, FR-255 Behavior 1, catalog code `stage_limit_exceeded`),
and no prepared value. It is category incomplete and exits 22 (FR-285).
`prepare` polls `&Cancel` at every charge (FR-276). The interpreter backend
translates nothing, so it charges neither field.

### The unsupported-entry cause

An entry the backend cannot run settles `UnsupportedEntry { entry,
construct }`: `entry` is the `EntrySelection` as the caller passed it, and
`construct` is an `UnsupportedConstruct`, a closed set:

| Variant | Names |
| --- | --- |
| `ExpressionForm { form, location }` | the checked expression form the backend does not translate, and its `Location` |
| `ValueType { ty, location }` | the checked value type the backend has no representation for, and the `Location` of the first node of that type |
| `Operation { operation, location }` | the checked operation identity the backend does not implement, and the `Location` of its application |

`construct` names the first construct in the entry's checked body, in the
interpreter's evaluation order, that the backend cannot run. When at least
one selected entry is prepared, `prepare` returns the prepared value, and its
`unsupported()` lists the `UnsupportedEntry` of every other selected entry in
selection order. When no selected entry can be prepared, `prepare` returns
`PrepareFailure::Unsupported` holding every selected entry's
`UnsupportedEntry`, in selection order. Serialized in an FR-286 document, an
`UnsupportedEntry` carries QSpec FR-271's catalog code
`unimplemented_capability` (valid complete-V1 meaning the selected producer
does not implement), its O-16 category is unsupported and it exits 21
(FR-285). The interpreter backend never settles an entry `Unsupported`.

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
- When a backend cannot run an entry, `prepare` shall settle that entry
  `UnsupportedEntry` naming the entry and the first construct it cannot run,
  and prepare the other selected entries.
- When no selected entry can be prepared, `prepare` shall return
  `PrepareFailure::Unsupported` holding every selected entry's
  `UnsupportedEntry` in selection order.
- When a `PrepareLimits` charge would exceed its bound, `prepare` shall
  return `PrepareFailure::Limit(LimitExceeded)` naming the field's setting,
  the bound and the count the charge would have reached, and no prepared
  value.
- Each backend's `prepare` and each `PreparedEntries` method shall honour
  `&Cancel` as FR-276 states.
- The `replay` facade shall evaluate through S6a for every request.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-294-AC-1 | The interpreter backend, prepared over FR-100-AC-4's unit with every function selected, returns for each AC-4 call the `Evaluation` or `CallFailure` that `CheckedPackage::call` returns for the same arguments and accounting limits; for FR-109's ConfigVersion clause, `evaluate` returns what `CheckedPackage::evaluate` returns. | Test (TC-778) |
| FR-294-AC-2 | Passing package bytes, a `ParsedSource` or an unchecked expression to `prepare`, and package bytes read from a file to AOT's `generate` step, each fails to compile (`compile_fail` doctests with compiling controls). | Test (TC-778) |
| FR-294-AC-3 | A test backend that cannot run one of three selected entries returns a prepared value for the other two and a `PrepareFailure` entry cause `Unsupported` for the third; calling the two prepared entries returns their interpreter results. | Test (TC-778) |
| FR-294-AC-4 | `prepare` and `call` with a handle already cancelled return `PrepareFailure::Cancelled` and `CallFailure::Cancelled`. | Test (TC-778) |
| FR-294-AC-5 | A test backend that cannot translate one expression form, prepared over three selected entries where the second's body uses that form at a known `Location` and the third's body uses an unrepresentable value type, returns a prepared value whose `unsupported()` is, in order, `UnsupportedEntry { entry: <second>, construct: ExpressionForm { form, location } }` with that form and `Location`, and the third's `ValueType` entry naming the type and the `Location` of its first node. Selecting only the second and third returns `PrepareFailure::Unsupported` holding the same two values in the same order. Each serializes in an FR-286 document with code `unimplemented_capability`, category unsupported and exit 21. The interpreter backend prepared over the same three entries lists no unsupported entry. | Test (TC-778) |
| FR-294-AC-6 | A test backend that charges one `prepare.work_units` unit per translated node and the byte length of each generated block to `prepare.code_bytes`, prepared over FR-262-AC-1's `count` with `work_units` one below the node count it translates, returns `PrepareFailure::Limit(LimitExceeded)` with kind `work-budget-exceeded`, that bound, the would-be count and setting `prepare.work_units`, and no prepared value; with `code_bytes` one below its generated byte count, kind `input-bytes-exceeded` and setting `prepare.code_bytes`; with each at exactly its count, `prepare` succeeds. Both failures exit 22. `PrepareLimits::default()` holds 16777216 work units and 67108864 code bytes, and the interpreter backend prepares `count` with both fields at 0. | Test (TC-778) |

## Dependencies

- ADR-029 EB-1 to EB-3: the seam, checked-only input and the reference.
- ADR-011 §4, E5, FB-07: construction, the IR reader and replay.
- ADR-013 T-4: `CallFailure`.
- [FR-276](FR-276-cancel-a-lifecycle-operation.md): cancellation.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md) and [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md): the `prepare.*` settings and the limit outcome.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md) and [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): exit codes and the outcome document.
- QSpec FR-271: `unimplemented_capability`; QSpec FR-461: setting names.
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
