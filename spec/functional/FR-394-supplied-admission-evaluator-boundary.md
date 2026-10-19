---
id: FR-394
title: "Keep supplied admission and conversion outside evaluator charges"
type: FR
relationships:
  - target: ix://agent-ix/quire-semantic-value/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-semantic-value/FR-122
    type: depends_on
  - target: ix://agent-ix/quire-exact/FR-380
    type: depends_on
---
# FR-394: Keep supplied admission and conversion outside evaluator charges

## Description

The `qsl-eval` evaluator and its supplied-admission caller SHALL keep supplied-membership and conversion work in their initiating phase budgets, leaving the evaluator meter and charge sequence unchanged by those helpers. This allocation carries the evaluator portion of QSV FR-109-AC-6 and the QSL/evaluator portions of quire-exact FR-372-AC-5 and FR-380-AC-6.

## Inputs

- A checked call with supplied arguments, original cancellation handle, conversion and admission budgets, and an evaluator meter.
- The admitted numeric magnitude bound or its QSpec FR-323 default.

## Outputs

- Existing typed supplied-admission or conversion result and evaluator charge record.

## Behavior

The initiating phase SHALL debit a shared helper once. Supplied membership SHALL not create an evaluated `FunctionCall` or an evaluation meter charge. The QSL caller SHALL resolve an omitted numeric magnitude bound to the published QSpec FR-323 4096-bit default and pass that 4096-bit bound to QSV. The exact helper and direct QSV caller obligations are allocated to their owners in Dependencies.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-394-AC-1 | Every reachable logical event and shared helper has one initiating phase/budget owner. Membership leaves the evaluator's meter and charge sequence unchanged, and conversion-triggered helper work is never also charged as membership. No failure is labelled evaluated `FunctionCall`. | Test |
| FR-394-AC-2 | A QSL caller resolving omitted B to QSpec FR-323's published 4096-bit default supplies that B to QSV. The supplied caller's qualified numeric helper leaves evaluator charges unchanged. | Test |
| FR-394-AC-3 | For the integrated supplied Decimal fixture of QSV FR-122-AC-2, membership, nonmembership, budget denial and original-handle cancellation leave the evaluator meter and charge sequence unchanged; no admission failure is projected as an evaluated `FunctionCall`. | Test |

## Dependencies

- [QSV FR-109](ix://agent-ix/quire-semantic-value/FR-109) owns supplied membership and its budget.
- [QSV FR-122](ix://agent-ix/quire-semantic-value/FR-122) owns bounded helper invocation and typed projection.
- [quire-exact FR-380 AC-7](ix://agent-ix/quire-exact/FR-380) owns the required B parameter and no-default exact-helper clause; [QSV FR-122 AC-3](ix://agent-ix/quire-semantic-value/FR-122) owns the direct QSV caller result when B is absent.
- [quire-exact FR-372](ix://agent-ix/quire-exact/FR-372) and [FR-380](ix://agent-ix/quire-exact/FR-380) own helper semantics.
- [IR-878](https://linear.app/agent-ix/issue/IR-878) tracks allocation.

## Status

PLANNED: evaluator and caller test PRs are pending.
