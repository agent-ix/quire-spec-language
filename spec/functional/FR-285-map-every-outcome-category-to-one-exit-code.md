---
id: FR-285
title: "Map every outcome category to one exit code through one total function"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
  - target: "ix://agent-ix/quire-specification/FR-301"
    type: depends_on
---
# FR-285: Map every outcome category to one exit code through one total function

## Description

QSL F `diagnostic`, which holds the ADR-013 O-16 category type, SHALL define
one total function from an O-16 category to an exit code (ADR-029 CB-4).

The category type is `Category`, in `qsl-foundation`'s `diagnostic` module.
It is the one O-16 category type: a stage or call outcome's category and a
terminal record's category (ADR-013 O-24) are both `Category`, and no proof
result has a category type of its own.
Every frontend takes its exit code from this function only: QSL's `command`
operation (FR-027, FR-100), the driver's `quire` binary and a qualified
binary.

| O-16 category | Exit code |
| --- | --- |
| success | 0 |
| violation | 10 |
| undefined | 10 |
| refusal | 20 |
| unsupported | 21 |
| incomplete (limit, cancellation, deadline, budget) | 22 |
| inconclusive: a proof or `analyze` item | 22 |
| inconclusive: a supplied-trace clause still pending when the trace ends | 0 |
| internal failure (including tool failure) | 30 |

The category undefined belongs to non-proof evaluation items: an `execute`
or `run` outcome (FR-100, FR-279) and a clause run (FR-109). Such an item
exits 10 and keeps the label `undefined` in machine output, so it stays
distinct from a violation. A proof item (`prove`, `analyze`) never has the
category undefined: an undefined claim evaluation settles it `refuted` with
cause `UndefinedEvaluation{where, cause}`, category violation (FR-281). An
undefined clause evaluation over a supplied trace (FR-283) is likewise a
violation. Each of these exits 10.

QSpec FR-301 gives exit 22 to an incomplete result and exit 0 to a run
that completed without violation. An inconclusive proof or `analyze` item
did not settle its claim, so it is incomplete and exits 22. A supplied-trace
clause still pending when the trace ends (FR-283) observed no violation, so
it exits 0.

A multi-item outcome exits with the most severe code present, in QSpec
FR-301's order: 30, 20, 21, 22, 10, 0.

`StageFailure` maps to a category as follows: `Refused` takes the category of
its cause, so a profile-gated construct is unsupported (21) and any other
refusal is refusal (20); `Limit` and `Cancelled` are incomplete (22); `Fault`
is internal failure (30). `CallFailure::Input` is refusal, `Cancelled` is
incomplete and `Fault` is internal failure.

## Inputs

An O-16 category, and for inconclusive whether the item is a proof or
`analyze` item or a pending supplied-trace clause; or those inputs for each
item of a multi-item outcome.

## Outputs

One exit code.

## Behavior

- The exit function shall map each O-16 category to the code the table
  states.
- The exit function shall map an inconclusive proof or `analyze` item to 22
  and a supplied-trace clause pending at the trace's end to 0.
- When an outcome holds several items, the frontend shall exit with the most
  severe item code under the order 30, 20, 21, 22, 10, 0.
- Each `StageFailure` and `CallFailure` variant shall have the O-16 category
  the Description states.
- A non-proof evaluation item whose category is undefined shall keep the
  label `undefined` in its machine-output record.
- Each QSL frontend shall derive its exit code from this function only.
- Every QSL outcome and terminal record shall report its category as the
  one `qsl-foundation` `Category`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-285-AC-1 | For each of the nine table rows, the exit function returns the code the table states: an inconclusive `analyze` item with cause `CertificateRejected` exits 22, and FR-283-AC-1's pending clause exits 0. FR-281-AC-2's `refuted` terminal record reports its category as `Category::Violation`, which the same function maps to 10. | Test (TC-769) |
| FR-285-AC-2 | Multi-item outcomes with item codes {10, 22} exit 22; {0, 30, 20} exit 30; {21, 22} exit 21; {20, 21} exit 20; {0, 10} exit 10; {0} exit 0. | Test (TC-769) |
| FR-285-AC-3 | `StageFailure::Refused` with a profile-gated cause exits 21 and with an `ill_typed` cause exits 20; `StageFailure::Limit` and `StageFailure::Cancelled` exit 22; `StageFailure::Fault` exits 30; `CallFailure::Input`, `Cancelled` and `Fault` exit 20, 22 and 30. | Test (TC-769) |
| FR-285-AC-4 | FR-100's outcome mapping renders FR-100-AC-10's `sum-out-of-domain` evaluation as `{"kind": "undefined", "reason": "sum-out-of-domain"}` and exits 10. | Test (TC-769) |

## Dependencies

- ADR-029 CB-4: the exit table, ruling RU-1.
- ADR-013 O-16, T-4: the categories and the failure types.
- ADR-011 §5: one total exit function.
- [FR-027](FR-027-export-compiled-native-package.md), [FR-100](FR-100-run-a-named-function-through-the-spine.md): QSL `command`'s exit codes.
- QSpec FR-301: the six exit codes and the severity order, with undefined at
  10 as the QSpec half in References aligns it.

## References

- QSL-390 (ARCH-50): ruling RU-1, recorded on the ticket, and the
  team-leader decision ADR-029's References records, which reconciles RU-1
  with the QSL-366 ruling for proof items.
- QSpec FR-301-AC-2 (STD-141): the QSpec half.
