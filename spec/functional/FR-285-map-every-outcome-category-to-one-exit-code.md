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
| pending: a supplied-trace clause still pending when the trace ends | 0 |
| indeterminate: a finite-window clause (FR-501) whose window is closed and whose evidence stays unresolved | 11 |
| internal failure (including tool failure) | 30 |

The category undefined belongs to non-proof evaluation items: an `execute`
or `run` outcome (FR-100, FR-279) and a clause run (FR-109). Such an item
exits 10 and keeps the label `undefined` in machine output, so it stays
distinct from a violation. A proof item (`prove`, `analyze`) never has the
category undefined: an undefined claim evaluation settles it `refuted` with
cause `UndefinedEvaluation{where, cause}`, category violation (FR-281). An
undefined clause evaluation over a supplied trace (FR-283) is likewise a
violation. Each of these exits 10.

The category indeterminate belongs to a supplied-trace clause under the
finite-window profile (FR-501) whose window is closed and whose evidence still
leaves the claim open (a missing or conflicting valuation, an incomplete input,
missing history, or a straddled bound). It is neither a violation nor a success:
the trace lacks the data to decide the claim. It exits 11, a code of its own, so
a script that tests for exit 10 does not read it as a violation and one that
tests for 0 does not read it as success. The code sits in the evaluation-result
decade beside 10 and ranks below it in severity, so an indeterminate item never
hides a violation in a multi-item outcome. QSpec FR-301 owns the exit-code
catalog; it must add 11 and the order below (see References).

QSpec FR-301 gives exit 22 to an incomplete result and exit 0 to a run
that completed without violation. An inconclusive proof or `analyze` item
did not settle its claim, so it is incomplete and exits 22. A supplied-trace
clause still pending when the trace ends (FR-283) observed no violation, so
it has the category pending, a category of its own that is not QSpec's
inconclusive, and exits 0. The machine-output record of a pending item SHALL
carry the status `pending` and the ids of the pending clauses, so that a
consumer can tell "no violation observed" from "all obligations discharged".

A multi-item outcome exits with the most severe code present, in the order
30, 20, 21, 22, 10, 11, 0 (QSpec FR-301's order with 11 added below 10).

A `StageFailure::Refused` or `StageFailure::Limit` that carries `protocol_results`
(FR-503) takes its category from its own cause alone: the first `Refused` result
in declaration order for `Refused`, and incomplete for `Limit`. The payload
changes no category and no exit, and each `Refused` result is an item of the
multi-item rule above.

`StageFailure` maps to a category as follows: `Refused` takes the category of
its cause, so a profile-gated construct is unsupported (21) and any other
refusal is refusal (20); `Limit` and `Cancelled` are incomplete (22); `Fault`
is internal failure (30). `CallFailure::Input` is refusal, `Cancelled` is
incomplete and `Fault` is internal failure; `CallFailure::Unsupported`
(FR-294, FR-296) is unsupported and `CallFailure::Limit` is incomplete.

## Inputs

An O-16 category (inconclusive for a proof or `analyze` item, pending for a
supplied-trace clause pending at the trace's end); or those inputs for each
item of a multi-item outcome.

## Outputs

One exit code.

## Behavior

- The exit function shall map each O-16 category to the code the table
  states.
- The exit function shall map the category indeterminate to 11.
- The exit function shall map an inconclusive proof or `analyze` item to 22
  and the category pending to 0.
- The machine-output record of a pending item shall carry the status `pending`
  and the ids of the pending clauses.
- When an outcome holds several items, the frontend shall exit with the most
  severe item code under the order 30, 20, 21, 22, 10, 11, 0.
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
| FR-285-AC-1 | For each of the ten table rows, the exit function returns the code the table states: an inconclusive `analyze` item with cause `CertificateRejected` exits 22, FR-283-AC-1's pending clause (category pending) exits 0, and FR-501-AC-4's `Indeterminate` clause exits 11. FR-281-AC-2's `refuted` terminal record reports its category as `Category::Violation`, which the same function maps to 10. | Test (TC-769) |
| FR-285-AC-2 | Multi-item outcomes with item codes {10, 22} exit 22; {0, 30, 20} exit 30; {21, 22} exit 21; {20, 21} exit 20; {0, 10} exit 10; {10, 11} exit 10; {11, 22} exit 22; {0, 11} exit 11; {0} exit 0. | Test (TC-769) |
| FR-285-AC-3 | `StageFailure::Refused` with a profile-gated cause exits 21 and with an `ill_typed` cause exits 20; `StageFailure::Limit` and `StageFailure::Cancelled` exit 22; `StageFailure::Fault` exits 30; `CallFailure::Input`, `Cancelled` and `Fault` exit 20, 22 and 30. | Test (TC-769) |
| FR-285-AC-4 | FR-100's outcome mapping renders FR-100-AC-10's `sum-out-of-domain` evaluation as `{"kind": "undefined", "reason": "sum-out-of-domain"}` and exits 10. | Test (TC-769) |
| FR-285-AC-5 | A `run` whose compile reaches a stage limit (the source byte limit set to 1, and `s3.nodes` set to 1) refuses with the `stage_limit_exceeded` code in category `incomplete`, exits 22 and not 20, and the refusal names the limits field and the configured bound. | Test (TC-769) |
| FR-285-AC-6 | `CallFailure::Unsupported` with cause `NotSupplied` and with cause `Untranslatable` each exit 21, and `CallFailure::Limit` naming `prepare.compile_work_units` exits 22. | Test (TC-769) |
| FR-285-AC-7 | A finite-window clause run (FR-330) whose result is FR-501's `Indeterminate` reports category `indeterminate`, keeps that label and the reason in its machine output, and exits 11; it does not exit 10 or 0. | Test (TC-769) |
| FR-285-AC-8 | A `StageFailure::Refused` carrying `protocol_results` [W `Emitted`, X `Refused` with an `ill_typed` cause] exits 20, one whose first `Refused` result has a profile-gated cause exits 21, and a `StageFailure::Limit` carrying [W `Emitted`, Z `Incomplete`] exits 22; removing the payload from each changes neither category nor exit. | Test (TC-769) |
| FR-285-AC-9 | A clause pending when its supplied trace ends exits 0 with the category `pending`, not `inconclusive`; its machine-output record carries the status `pending` and the ids of the pending clauses, and a run whose clauses are all discharged carries no pending status and no pending ids. | Test (TC-769) |

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
- QSpec FR-301-AC-2 (STD-141): the QSpec half. QSpec must add exit 11 for the
  category indeterminate and the order 30, 20, 21, 22, 10, 11, 0 to its exit
  table; the mirror is tracked on QSL-1500.
