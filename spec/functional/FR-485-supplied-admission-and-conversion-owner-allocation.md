---
id: FR-485
title: "Allocate supplied admission and conversion work at the QSL evaluator boundary"
type: FR
relationships:
  - target: ix://agent-ix/quire-semantic-value/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-semantic-value/FR-122
    type: depends_on
  - target: ix://agent-ix/quire-exact/FR-372
    type: depends_on
  - target: ix://agent-ix/quire-exact/FR-380
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-323
    type: depends_on
---
# FR-485: Allocate supplied admission and conversion work at the QSL evaluator boundary

## Description

The QSL evaluator and its supplied-admission caller shall allocate each supplied-membership and conversion event to its initiating phase and budget. This allocation carries the evaluator and conversion-owner portions of QSV FR-109-AC-6, the QSL caller portion of quire-exact FR-380-AC-6, and the evaluator-effect portion of quire-exact FR-372-AC-5 and QSV FR-109-AC-8.

## Inputs

- A checked call with supplied arguments, the original cancellation handle, conversion and admission budgets, and an evaluator meter.
- The admitted numeric magnitude bound, or its QSpec FR-323 default when the QSL caller omits the bound.

## Outputs

- The existing typed supplied-admission or conversion result and the evaluator charge record.

## Behavior

The initiating phase shall debit a shared helper exactly once. Supplied membership shall not create an evaluated `FunctionCall` or an evaluator-meter charge. The QSL caller shall resolve an omitted numeric magnitude bound to QSpec FR-323's published 4096-bit default and pass that bound to QSV. A direct QSV caller that omits the required bound follows QSV FR-122-AC-3; the QSL default does not change that owner boundary.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-485-AC-1 | Every reachable supplied-membership and conversion event, including a shared helper, has one initiating phase and budget owner. The QSL evaluator caller keeps the evaluator and conversion-owner allocation from QSV FR-109-AC-6; conversion-triggered helper work is not also charged as membership, and no admission or conversion failure is labelled as an evaluated `FunctionCall`. | Test |
| FR-485-AC-2 | When the QSL supplied caller omits the numeric magnitude bound, it supplies QSV with the 4096-bit default published by QSpec FR-323. The caller passes the required bound through to the QSV helper; a direct QSV caller with no bound retains the behavior specified by QSV FR-122-AC-3. | Test |
| FR-485-AC-3 | For the integrated supplied Decimal fixture of QSV FR-122-AC-2, membership, nonmembership, budget denial, and cancellation through the original handle leave evaluator meter and charge sequence unchanged. The evaluator-effect allocation follows quire-exact FR-372-AC-5 and QSV FR-109-AC-8; supplied-admission failures remain attributed to their initiating phase. | Test |

## Dependencies

- [QSV FR-109](ix://agent-ix/quire-semantic-value/FR-109) owns supplied membership, its budget, and evaluator-side allocation in AC-6 and AC-8.
- [QSV FR-122](ix://agent-ix/quire-semantic-value/FR-122) owns bounded helper invocation, typed projection, and the direct-caller missing-bound behavior in AC-3.
- [quire-exact FR-372](ix://agent-ix/quire-exact/FR-372) and [FR-380](ix://agent-ix/quire-exact/FR-380) own exact-helper semantics and their shared-helper/evaluator-effect allocation.
- QSpec FR-323 publishes the 4096-bit omitted-bound default used by the QSL caller.
- [IR-878](https://linear.app/agent-ix/issue/IR-878) tracks this owner allocation.

## Status

PLANNED: the evaluator and caller test PRs are pending.
