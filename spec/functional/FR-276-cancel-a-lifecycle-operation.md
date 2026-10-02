---
id: FR-276
title: "Cancel a lifecycle operation through a caller-owned handle"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: references
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
---
# FR-276: Cancel a lifecycle operation through a caller-owned handle

## Description

`Cancel` is a cloneable handle the caller owns (ADR-029 LC-3). Every QSL
lifecycle operation (FR-275) SHALL poll it wherever it charges a work meter
or a stage limit, so a cancelled run stops within one charge. A cancellation
carries a `CancelCause`: `Requested`, when the caller asks for it, or
`Deadline`, when the frontend that owns the timer reaches its deadline. The
library reads no clock.

`StageFailure` gains the variant `Cancelled(CancelCause)` and `CallFailure`
gains the variant `Cancelled(CancelCause)` (ADR-029 Amendments, ADR-013 T-4).

## Inputs

`&Cancel`, passed to every operation. Any clone of the handle can cancel it,
from any thread, with either cause.

## Outputs

- `StageFailure::Cancelled(CancelCause)` from a cancelled operation that
  returns `Staged`.
- `CallFailure::Cancelled(CancelCause)` from a cancelled `execute`.
- Inside `analyze`, each item still open when the handle is cancelled settles
  FR-331 `incomplete` with the cancellation cause kept (ADR-013 O-16); items
  already settled keep their records.

Every cancelled outcome is O-16 category incomplete, which exits 22
(FR-285).

## Behavior

- Each lifecycle operation shall check the `Cancel` handle at every charge of
  a work meter or a stage limit.
- When the handle is cancelled, each lifecycle operation shall stop at its
  next charge and return its cancelled failure with the handle's
  `CancelCause`.
- When an operation is called with a handle that is already cancelled, the
  operation shall return its cancelled failure before its first charge.
- When an operation is cancelled, the operation shall return no output
  artifact and no partial artifact.
- When the handle is cancelled during `analyze`, the operation shall settle
  each open item `incomplete` with the cancellation cause, and shall keep
  each item already settled.
- No QSL library operation shall read a clock to decide a deadline; the
  frontend that owns the timer cancels with cause `Deadline`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-276-AC-1 | Each of `parse`, `select`, `check`, `package`, `analyze`, `monitor`, `replay`, `inspect` and `render`, called over its FR-275-AC-1 input with a handle already cancelled with `Requested`, returns `StageFailure::Cancelled(Requested)` and no output; `execute` returns `CallFailure::Cancelled(Requested)`. | Test (TC-757) |
| FR-276-AC-2 | A `check` over a generated source of 200,000 declarations, cancelled with `Deadline` from a second thread once the test's work-meter observer has seen 1,000 charges, returns `StageFailure::Cancelled(Deadline)`, no `CheckedPackage`, and the observer records at most one charge after the cancellation. | Test (TC-757) |
| FR-276-AC-3 | An `execute` of a function whose evaluation charges more than 10,000,000 work units, run with every accounting limit at `u64::MAX` and cancelled from a second thread after the observer has seen 1,000 charges, returns `CallFailure::Cancelled(Requested)` and no value. | Test (TC-757) |
| FR-276-AC-4 | An `analyze` request of two items, where the test engine settles the first and then blocks on the second until the handle is cancelled, returns the first item's record unchanged and the second settled `incomplete` with cause cancelled. | Test (TC-757) |
| FR-276-AC-5 | The category of `StageFailure::Cancelled` of each cause and of `CallFailure::Cancelled` is incomplete, and its exit code is 22. | Test (TC-757) |

## Dependencies

- ADR-029 LC-3: the handle, the causes and the polling rule.
- ADR-013 T-4 and O-16: the failure types and the incomplete category.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the operations.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md): the exit code.
- QSpec FR-300: cancellation on every request.

## References

- QSL-390 (ARCH-50), QSL-393 (V1-A06).
- QSpec FR-300 (STD-141): the QSpec half.
