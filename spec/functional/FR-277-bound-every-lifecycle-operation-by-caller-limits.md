---
id: FR-277
title: "Bound every lifecycle operation by caller-configurable limits"
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
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: references
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
---
# FR-277: Bound every lifecycle operation by caller-configurable limits

## Description

Every bound a QSL lifecycle operation (FR-275) enforces SHALL be a field of
the limits value the caller passes, with a published default the caller can
replace (ADR-029 LC-4). Reaching a bound SHALL refuse with `LimitExceeded`
(ADR-013 T-4), which names the limit, its configured value and the limits
field that raises it, and is never read as success.

Nesting depth is not a limit. Node, byte and work limits bound every stage,
and no stage recurses on the native stack (ADR-030 D-1). ADR-030 owns the
depth rule; this requirement states only that the lifecycle operations take
their limits from the caller.

## Inputs

The operation's limits value. Each field has a published default, available
as the limits type's `Default` value.

## Outputs

On reaching a bound, `StageFailure::Limit(LimitExceeded)`; for `execute`'s
accounting limits, the evaluation outcome `Incomplete` naming the counter;
for `analyze`, each open FR-331 item settled `incomplete` with the limit as
its cause.
`LimitExceeded` carries the limit kind, the configured value, the counter at
the failed charge, the `Locus` where the producer knows one (FR-096), and the
name of the limits field that sets the bound.

## Behavior

- Each lifecycle operation shall read every bound it enforces from the
  limits value its caller passes.
- Each limits type shall give every field a published default that the
  caller replaces field by field.
- When an operation reaches a bound, it shall return `LimitExceeded` naming
  the limit kind, the configured value, the counter reached and the limits
  field that raises it.
- A reached limit shall never produce a success outcome.
- Each lifecycle operation shall process input at any nesting depth that
  fits its node, byte and work limits, and shall not fail by exhausting the
  native stack.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-277-AC-1 | For each field of the limits types of all twelve operations (`parse`, `format`, `select`, `check`, `check_fences`, `package`, `execute`, `analyze`, `monitor`, `replay`, `inspect` and `render`), running the operation over its TC-755 step 6 input with that field set to one below the counter the input reaches names that field: `LimitExceeded` with the field's limit kind, configured value and limits-field name; for `execute`, the outcome `Incomplete` naming that counter; for `analyze`, each open item `incomplete` with that limit as its cause. Setting the field to the counter reached succeeds. | Test (TC-758) |
| FR-277-AC-2 | On a thread with the platform's default stack, a source whose body is a 100,000-term sum checks successfully when the caller raises `s3.nodes` to fit it, and with `s3.nodes` one below its node count refuses with `LimitExceeded` naming `s3.nodes`; no outcome names a depth. | Test (TC-758) |

## Dependencies

- ADR-029 LC-4: caller limits.
- ADR-030 D-1: depth is not a limit.
- ADR-013 T-4: `LimitExceeded`.
- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md): the locus a limit carries.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the operations.
- QSpec FR-300: explicit limits on every request.

## References

- QSL-390 (ARCH-50), QSL-393 (V1-A06).
- QSL-381: the depth ruling ADR-030 records.
- QSpec FR-300, FR-460, FR-461 (STD-141, STD-143): the QSpec half.
