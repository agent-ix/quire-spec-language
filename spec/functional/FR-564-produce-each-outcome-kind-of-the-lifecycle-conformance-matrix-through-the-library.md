---
id: FR-564
title: "Produce each outcome kind of the lifecycle conformance matrix through the library"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-279
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-283
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-291
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-299
    type: references
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
---
# FR-564: Produce each outcome kind of the lifecycle conformance matrix through the library

## Description

FR-299 runs one source through the operations of ADR-029's first slice and
reaches success and cancellation. The conformance matrix extends it to every
outcome kind a user can meet, for each lifecycle verb.

QSL's library operations SHALL produce, for each applicable cell of the
matrix below, an outcome of that cell's kind, whose FR-286 document has the
cell's category and whose FR-285 exit code is the cell's.

### The eight outcome kinds

| Kind | O-16 category (FR-285) | What tells it apart in the document | Exit |
| --- | --- | --- | --- |
| success | success | | 0 |
| violation | violation | | 10 |
| refusal | refusal | the refusal's catalog code | 20 |
| unsupported | unsupported | the unsupported cause | 21 |
| timeout | incomplete | cause `Deadline` (FR-276), or a provider's `timed-out` (FR-291) | 22 |
| cancellation | incomplete | cause `Requested` (FR-276) | 22 |
| incomplete | incomplete | the exhausted limit (FR-277) | 22 |
| tool-failure | internal failure | a failure of a backend or tool outside QSL's evaluator: a supplied backend's `Fault`, or a provider's `tool-failure{defect}` (FR-291) | 30 |

### The matrix

Each cell names the input that produces its kind. "n/a" cells are not
reachable through that operation, for the stated reason.

| Verb | success | violation | refusal | unsupported | timeout | cancellation | incomplete | tool-failure |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `check` | FR-275-AC-1's source | n/a (1) | FR-100-AC-5's `inv` (`ill_typed`) | FR-285-AC-3's profile-gated source | FR-276-AC-2 (`Deadline`) | FR-276-AC-1 (`Requested`) | FR-277-AC-2, `s3.nodes` one below | n/a (2) |
| `package` | FR-275-AC-1's package | n/a (1) | n/a (3) | FR-185-AC-7's `Service` | handle cancelled `Deadline` before the call | FR-276-AC-1 (`Requested`) | FR-277-AC-1, a `package` limit one below | n/a (2) |
| `execute` | FR-100-AC-1's `seven` | n/a (4) | FR-279-AC-1's `id(12)` | FR-296-AC-2 (`jit` not supplied) | FR-276-AC-3 cancelled `Deadline` | FR-276-AC-3 cancelled `Requested` | FR-100-AC-6 (`work_units` 0) | FR-295-AC-4's injected backend `Fault` |
| `analyze` | FR-281-AC-1 | FR-281-AC-2 | FR-281-AC-9's binding refusal | FR-281-AC-3's unsupported item alone | FR-281-AC-12 (`deadline_ms`) | FR-281-AC-12, caller cancellation first | FR-281-AC-6 (state budget 1) | n/a (2) |
| `monitor` | FR-283-AC-2's lasso, `tested` | FR-283-AC-1 | FR-283-AC-3's admission failure | FR-283-AC-6 (fairness premise) | handle cancelled `Deadline` before the call | FR-276-AC-1 (`Requested`) | FR-283-AC-12 (`work_units` 0) | n/a (2) |
| `prove` (QSL half) | FR-281-AC-1 through `qsl_analyze::provider::run` | FR-281-AC-2 through `provider::run` | FR-281-AC-11's `Admission` refusal (`declined`) | FR-281-AC-3's unsupported item through `provider::run` | FR-291 `incomplete`, `timed-out` | FR-291 `incomplete`, cancelled | FR-281-AC-6 through `provider::run` | FR-291 `failed`, `tool-failure{crash}` |

Reasons for the n/a cells:

1. `check` and `package` decide no claim.
2. The operation runs no backend or tool outside QSL's evaluator. Its
   internal faults are FR-285's internal failure and are covered there
   (FR-285-AC-3).
3. Every refusal other than an unsupported construct is made by `check`
   (S3) before `package` runs.
4. `execute` returns a value; a `Boolean` `false` is a completed value, and
   a clause verdict is the clause-run path's (FR-109), reached through
   `monitor`.

`prove` is the driver's operation (ADR-029 OP-1). Its QSL half is what QSL
contributes to a `prove` item: the terminal records of QSL engines served as
providers through `qsl_analyze::provider::run` (FR-280, FR-281), and the
terminal records FR-291 builds from a plugin run's settlement. Each becomes
one FR-286 item.

Every cell's outcome is a one-item outcome or a stage or call failure, so
its exit is its own category's.

## Inputs

Each cell's input, with default limits except where the cell names a limit,
and an uncancelled `Cancel` except in the timeout and cancellation cells.

## Outputs

For each applicable cell, the library outcome, its FR-286 document and its
FR-285 exit code.

## Behavior

- For each applicable cell, the named operation over the cell's input shall
  return an outcome of the cell's kind.
- The outcome's FR-286 document shall carry the kind's category and the
  cause that tells it apart, and its FR-285 exit code shall be the kind's.
- A timeout cell's document shall carry the cause `Deadline` or `timed-out`,
  and a cancellation cell's the cause `Requested`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-564-AC-1 | Each of `check`'s six applicable cells returns its kind: success exit 0; refusal `ill_typed` exit 20; unsupported exit 21; timeout `StageFailure::Cancelled(Deadline)` exit 22; cancellation `StageFailure::Cancelled(Requested)` exit 22; incomplete `LimitExceeded` naming `s3.nodes` exit 22. | Test (TC-1388) |
| FR-564-AC-2 | Each of `package`'s five applicable cells returns its kind: success exit 0; unsupported `unsupported_construct` exit 21; timeout `Cancelled(Deadline)`, cancellation `Cancelled(Requested)` and incomplete `LimitExceeded` naming the `package` limit field, each exit 22. | Test (TC-1389) |
| FR-564-AC-3 | Each of `execute`'s seven applicable cells returns its kind: `seven` completes 7, exit 0; `id(12)` `CallFailure::Input`, exit 20; `jit` not supplied `CallFailure::Unsupported` cause `NotSupplied`, exit 21; `CallFailure::Cancelled(Deadline)` and `CallFailure::Cancelled(Requested)`, exit 22; `Incomplete` naming `work_units`, exit 22; the injected backend `Fault`, exit 30, no value. | Test (TC-1390) |
| FR-564-AC-4 | Each of `analyze`'s seven applicable cells returns its kind: `proved` exit 0; `refuted` exit 10; `StageFailure::Refused(AnalyzeRefusal::Binding)` exit 20; `unsupported` exit 21; `incomplete` `cancelled{source: deadline}` and `cancelled{source: requested}`, exit 22; `incomplete` with the state-budget cause, exit 22. | Test (TC-1391) |
| FR-564-AC-5 | Each of `monitor`'s seven applicable cells returns its kind: `tested` exit 0; violation exit 10; the admission refusal exit 20; unsupported `missing-fairness-premise` exit 21; `StageFailure::Cancelled(Deadline)` and `StageFailure::Cancelled(Requested)`, exit 22; `Incomplete` naming `work_units`, exit 22. | Test (TC-1392) |
| FR-564-AC-6 | Each of the eight `prove` (QSL half) cells yields one FR-286 item of its kind: `proved` exit 0; `refuted` exit 10; `declined` with cause `invalid-input` exit 20; `unsupported` exit 21; `incomplete` `timed-out` and `incomplete` cancelled, exit 22; `incomplete` with the state-budget cause, exit 22; `failed` with `tool-failure{crash}`, exit 30. | Test (TC-1393) |
| FR-564-AC-7 | Every outcome of AC-1 to AC-5 serializes to an FR-286 document whose `operation` names its verb and whose `category` is its kind's, and every AC-6 item serializes to an FR-286 `items` entry whose `category` is its kind's; serializing each twice gives equal bytes, and each exit code is the one FR-285's function returns for that category. | Test (TC-1394) |

## Dependencies

- ADR-029 LC-1, LC-3, CB-4, OP-1 to OP-3.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md) to [FR-279](FR-279-execute-a-checked-entry-as-a-library-operation.md), [FR-281](FR-281-analyze-claims-with-in-process-engines.md), [FR-283](FR-283-monitor-a-supplied-trace-offline.md): the operations and the cell inputs.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md), [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): exit codes and documents.
- [FR-291](FR-291-bound-plugin-runs-and-settle-plugin-failures.md): plugin settlement records.
- [FR-299](FR-299-serve-one-value-family-source-through-every-qsl-library-operation-the-cli-calls.md): the first-slice source this matrix extends.
- QSpec FR-300, FR-301: the lifecycle API and the CLI exit codes.

## Overlap

The driver repository runs every cell through the `quire` CLI and its own
`prove` (FR-565).

## References

- QSL-390 (ARCH-50), QSL-392 (ARCH-52).
