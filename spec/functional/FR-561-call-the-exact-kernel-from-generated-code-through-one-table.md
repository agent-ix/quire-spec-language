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

`qsl-jit` SHALL define one `KernelTable`: the fixed set of `extern "C"` shim
symbols of `qsl_jit::exec` (FR-568), one per `quire-exact` or
`quire-semantic-value` operation. The table is fixed at link time. Generated
code names each shim as a module symbol that the code generator resolves
against `qsl_jit::exec` when it finalizes the code, so no function pointer to
a shim sits in writable memory: not in the `EntryFrame` (FR-568), not in a
heap table and not in a stack slot. A prepared value links only the shims its
entries use. Generated code SHALL reach the kernel and the runtime only
through those shims, and SHALL call no other address except the blocks of the
same prepared value. A register-indirect call is allowed only when its
register is loaded by an absolute relocation that names a `KernelTable` shim
symbol, as non-PIC Cranelift code calls an imported symbol; a call through a
pointer read from writable memory (the frame, a heap table or a GOT-style
slot) is not allowed. The translator declares the prepared value's own functions with final
(colocated) linkage, so a call between its own blocks is a direct PC-relative
call and never a register-indirect one.

Each kernel-table entry:

- takes its operands as typed handles to values the runtime owns (the
  handle types of the semantic-value runtime, each a typed reference
  into storage the `EntryFrame` (FR-568) lists), and the call's meter, which
  holds the call's cancellation handle (FR-277);
- runs the kernel operation the interpreter runs for the same checked
  operation, with the same charges at the same points (EB-4 rule 4), polling
  cancellation through the meter at each charge (FR-276);
- returns a typed status: a value handle; `Undefined` with the kernel's
  typed cause; `Incomplete` with the exhausted limit (FR-277); `Cancelled`
  with the handle's cause; or `Fault`.

Generated code passes every non-value status up to the `call` or `evaluate`
result unchanged, adding the `Location` of the checked operation that made
the call, exactly as the interpreter attaches it. It never turns a status into
another status, and it never retries a call.

## Inputs

The prepared entries, their arguments, environment or admitted observations,
the call's limits and `&Cancel`, whose handle the call's meter holds.

## Outputs

The `Evaluation<Value>` or `CallFailure` of FR-294's `call` and `evaluate`.

## Behavior

- The JIT shall define the `KernelTable` as shim symbols fixed at link time,
  one per kernel or runtime operation.
- The JIT shall hold no function pointer to a shim in writable memory.
- Generated code shall call only `KernelTable` entries and blocks of the same
  prepared value, a call being a direct call or a register-indirect call
  loaded by an absolute relocation naming a `KernelTable` shim symbol, and
  never a call through a pointer read from writable memory.
- A `KernelTable` entry shall poll cancellation through the meter.
- A `KernelTable` entry shall take no cancellation handle of its own.
- Each kernel-table entry shall run the interpreter's kernel operation for
  the checked operation, with the interpreter's charges at the interpreter's
  charge points.
- When a kernel-table entry returns `Undefined`, `Incomplete`, `Cancelled` or
  `Fault`, the generated code shall return that status as the run's outcome
  with the interpreter's cause and `Location`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-561-AC-1 | With a recording `KernelTable` (the same shim symbols built with a test recording hook, never a table a frame supplies), `add(a: Integer, b: Integer): Integer { a + b }` called with `a` = 2^100 and `b` = 1 records exactly one kernel `add` call with those operands, returns 2^100 + 1, and its recorded charges equal the interpreter's charge sequence for the same call. | Test (TC-1382) |
| FR-561-AC-2 | `q(a: Integer, b: Integer): Integer { a div b }` called with `b` = 0 returns `Undefined` with cause `division-by-zero` and the `Location` of the `div`, as the interpreter does; with `work_units` one below its measured spend it returns the interpreter's `Incomplete`; with a recording table that returns `Fault` from the `div` entry it returns `Fault` and no value; with `&Cancel` cancelled `Requested` before the call it returns `CallFailure::Cancelled(Requested)`. | Test (TC-1382) |
| FR-561-AC-3 | For every block the JIT prepares over the 10,000 FR-295 generated units at the default seed, each call in the block's relocation records is a direct call whose target is a `KernelTable` shim symbol or a block of the same prepared value, or a register-indirect call whose register is loaded by an absolute relocation naming a `KernelTable` shim symbol, and the block holds no call through a pointer loaded from writable memory; over the run at least one block calls a `KernelTable` shim. A test code generator that emits one direct call to an address outside both, and one that emits an indirect call through a pointer loaded from the frame, each make the check fail, naming the block and the target. | Test (TC-1383) |
| FR-561-AC-4 | `EntryFrame` has no field whose type is a function pointer or a `KernelTable`, checked by a test inside `qsl_jit::exec` over the structure's field types; with `&Cancel` cancelled `Requested` after the call has started and before its first charge, the call returns `CallFailure::Cancelled(Requested)` at that charge, polled through the meter. | Test (TC-1383) |

## Dependencies

- ADR-029 EB-4 rules 2, 4 and 7.
- [FR-294](FR-294-run-every-execution-backend-behind-one-seam.md): `call`, `evaluate` and their results.
- [FR-295](FR-295-return-interpreter-outcomes-from-jit-and-aot.md): the outcomes and the generator.
- [FR-276](FR-276-cancel-a-lifecycle-operation.md), [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md): cancellation and limits; the meter that holds the cancellation handle.
- [FR-562](FR-562-take-a-machine-width-fast-path-only-where-checked-bounds-prove-the-result-fits.md): the operations that skip the table.
- QSpec FR-306: generated code calls only the kernel and the semantic-value
  runtime.

## References

- QSL-393 (V1-A06a).
- ADR-035 CG-1 item 3: the code generator must expose relocation records.
