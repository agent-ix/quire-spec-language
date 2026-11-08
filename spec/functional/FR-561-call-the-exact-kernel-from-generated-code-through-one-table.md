---
id: FR-561
title: "Call the exact kernel from generated code through one kernel table"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-295
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-562
    type: references
  - target: "ix://agent-ix/quire-specification/FR-306"
    type: depends_on
---
# FR-561: Call the exact kernel from generated code through one kernel table

## Description

ADR-029 EB-4 rules 2 and 7 say generated code calls only the kernel and the
semantic-value runtime, and calls the kernel for every value operation that
no checked bound puts on a machine-width fast path (FR-562). This
requirement fixes that boundary.

`qsl-jit` SHALL build one `KernelTable` per prepared value: a fixed set of
entry points, one per `quire-exact` or `quire-semantic-value` operation the
prepared entries use. Generated code SHALL reach the kernel and the runtime
only through those entries, and SHALL call no other address except the
blocks of the same prepared value.

Each kernel-table entry:

- takes its operands as handles to values the runtime owns, and the call's
  meter and `&Cancel`;
- runs the kernel operation the interpreter runs for the same checked
  operation, with the same charges at the same points (EB-4 rule 4), polling
  `&Cancel` at each charge (FR-276);
- returns a typed status: a value handle; `Undefined` with the kernel's
  typed cause; `Incomplete` with the exhausted limit (FR-277); `Cancelled`
  with the handle's cause; or `Fault`.

Generated code passes every non-value status up to the `call` or `evaluate`
result unchanged, adding the `Location` of the checked operation that made
the call, exactly as the interpreter attaches it. It never turns a status into
another status, and it never retries a call.

## Inputs

The prepared entries, their arguments, environment or admitted observations,
the call's limits and `&Cancel`.

## Outputs

The `Evaluation<Value>` or `CallFailure` of FR-294's `call` and `evaluate`.

## Behavior

- The JIT shall build one `KernelTable` per prepared value, with one entry
  per kernel or runtime operation its entries use.
- Generated code shall call only `KernelTable` entries and blocks of the same
  prepared value.
- Each kernel-table entry shall run the interpreter's kernel operation for
  the checked operation, with the interpreter's charges at the interpreter's
  charge points.
- When a kernel-table entry returns `Undefined`, `Incomplete`, `Cancelled` or
  `Fault`, the generated code shall return that status as the run's outcome
  with the interpreter's cause and `Location`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-561-AC-1 | With a recording `KernelTable`, `add(a: Integer, b: Integer): Integer { a + b }` called with `a` = 2^100 and `b` = 1 records exactly one kernel `add` call with those operands, returns 2^100 + 1, and its recorded charges equal the interpreter's charge sequence for the same call. | Test (TC-1382) |
| FR-561-AC-2 | `q(a: Integer, b: Integer): Integer { a div b }` called with `b` = 0 returns `Undefined` with cause `division-by-zero` and the `Location` of the `div`, as the interpreter does; with `work_units` one below its measured spend it returns the interpreter's `Incomplete`; with a recording table that returns `Fault` from the `div` entry it returns `Fault` and no value; with `&Cancel` cancelled `Requested` before the call it returns `CallFailure::Cancelled(Requested)`. | Test (TC-1382) |
| FR-561-AC-3 | For every block the JIT prepares over the 10,000 FR-295 generated units at the default seed, each call target in the block's relocation records is a `KernelTable` entry of the same prepared value or a block of the same prepared value; a test code generator that emits one call to an address outside both makes this check fail, naming the block and the target. | Test (TC-1383) |

## Dependencies

- ADR-029 EB-4 rules 2, 4 and 7.
- [FR-294](FR-294-run-every-execution-backend-behind-one-seam.md): `call`, `evaluate` and their results.
- [FR-295](FR-295-return-interpreter-outcomes-from-jit-and-aot.md): the outcomes and the generator.
- [FR-276](FR-276-cancel-a-lifecycle-operation.md), [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md): cancellation and limits.
- [FR-562](FR-562-take-a-machine-width-fast-path-only-where-checked-bounds-prove-the-result-fits.md): the operations that skip the table.
- QSpec FR-306: generated code calls only the kernel and the semantic-value
  runtime.

## References

- QSL-393 (V1-A06a).
- ADR-035 CG-1 item 3: the code generator must expose relocation records.
