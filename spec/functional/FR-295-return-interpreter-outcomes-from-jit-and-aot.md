---
id: FR-295
title: "Return the interpreter's outcomes from the JIT and AOT backends"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-306"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-331"
    type: depends_on
---
# FR-295: Return the interpreter's outcomes from the JIT and AOT backends

## Description

A JIT or AOT run SHALL return the same `Evaluation` as the interpreter for
the same package, entry, arguments, environment and accounting limits
(ADR-029 EB-4):

1. **Determinism.** The result depends only on those inputs. Generated code
   reads no clock, random source, environment or thread state, and visits
   collections in the interpreter's canonical order.
2. **Exact arithmetic.** Every value operation has `quire-exact` semantics:
   integers and rationals are exact, and decimal and IEEE operations follow
   the selected profile exactly. Generated code takes a machine-width fast
   path only where a checked bound shows the exact result fits, and calls the
   kernel for every other operation.
3. **Outcomes.** `Completed`, `Undefined`, `Refused` and `Incomplete` match
   the interpreter's, with the same typed cause, the same `Location` and the
   same loss records.
4. **Budgets.** For every accounting limit, the run is `Incomplete` with the
   same charge point and limit as the interpreter's run. Charges may be
   grouped if this holds.
5. **Limits.** Node, byte and work limits bound a run, checked explicitly
   (FR-277); depth is not a limit, and no run recurses on the native stack. Compile work and code size are bounded by
   `PrepareLimits`, and reaching one is a `Limit` failure of `prepare`.
6. **Failures.** An internal error of the backend is `Fault` (exit 30) and
   never a verdict. An entry the backend cannot translate settles
   `Unsupported` for that entry.
7. **No effects.** Generated code performs no I/O and calls only the kernel
   and the semantic-value runtime.

The outcome carries no member naming the backend that ran.

Each non-interpreter backend runs the conformance corpus and generated
programs against the interpreter, and any difference fails the gate (ADR-029
EB-5, QSpec FR-306-AC-1).

QSL's code-generation boundary for AOT is the v2 package (ADR-029 EB-6). CG
generates from S5 IR, negotiated per item before emission. An item not
settled `supported` emits nothing (QSpec FR-331-AC-2). The generated artifact
is FR-331 `artifacts` bytes with a source map keyed by occurrence key. A
runtime monitor is another `generate` target behind the same boundary.

## Inputs

A checked package, an entry, arguments, an environment, accounting limits
and `PrepareLimits`.

## Outputs

`Evaluation<Value>` or `CallFailure`, as the interpreter's; `PrepareFailure`
from `prepare`.

## Behavior

- For every input the interpreter completes, refuses, finds undefined or
  stops incomplete, a JIT or AOT run shall return the same outcome, typed
  cause, `Location` and loss records.
- When an accounting limit is reached, a JIT or AOT run shall return
  `Incomplete` naming the same limit at the same charge point as the
  interpreter's run.
- When a caller's size or work limit is reached, a JIT or AOT run shall
  return the same outcome as the interpreter's run.
- A JIT or AOT run shall not fail by exhausting the native stack at any
  nesting depth.
- When a `PrepareLimits` bound is reached, `prepare` shall return a `Limit`
  failure naming the bound.
- If a backend reaches an internal error, then it shall return `Fault`.
- The outcome shall hold no member naming the backend that ran.
- The parity gate shall run each non-interpreter backend over the
  conformance corpus and generated programs, and fail on any difference from
  the interpreter.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-295-AC-1 | Over the conformance corpus and 10,000 property-generated programs with arguments, each backend's outcome, cause, `Location` and loss records equal the interpreter's; a deliberately wrong backend that rounds one decimal operation fails the gate, naming the program and both outcomes. | Test (TC-779) |
| FR-295-AC-2 | For FR-100-AC-6's `seven` with `work_units` 0 to 5, and for a property-generated set of programs and accounting limits, each backend's `Incomplete` names the same limit at the same charge point as the interpreter's. | Test (TC-780) |
| FR-295-AC-3 | FR-262-AC-1's `count` called with `100000`, on a thread with a 512 KiB stack, returns the interpreter's outcome on each backend, both with `work_units` sized for it and with `work_units` one below the run's measured spend; a `PrepareLimits` code-size bound of 1 makes `prepare` return a `Limit` failure naming it. | Test (TC-780) |
| FR-295-AC-4 | A backend made to hit an injected internal error returns `Fault`, exit 30, and no value. The JSON outcome documents (FR-286) of one call on the interpreter and on each other backend are byte-equal. | Test (TC-780) |

## Dependencies

- ADR-029 EB-4 to EB-7: equivalence, parity, the AOT boundary and the JIT.
- [FR-294](FR-294-run-every-execution-backend-behind-one-seam.md): the seam.
- [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md): caller limits.
- QSpec FR-306: AOT and JIT parity, as the QSpec half in References aligns it; QSpec FR-331-AC-2:
  an unsupported item emits nothing.

## Overlap

The driver repository owns `quire-aot` and the AOT target with CG; a
benchmark picks the JIT's code generator, which these rules bind either way.

## References

- QSL-393 (V1-A06a): checked-only AOT and JIT.
- QSL-390 (ARCH-50): owner ruling 3, recorded on the ticket.
- RES-56: the JIT code-generator benchmark.
- QSpec FR-306 (STD-141): the QSpec half.
