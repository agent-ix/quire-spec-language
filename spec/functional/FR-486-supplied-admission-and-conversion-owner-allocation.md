---
id: FR-486
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
# FR-486: Allocate supplied admission and conversion work at the QSL evaluator boundary

## Description

The QSL evaluator and its supplied-admission caller shall allocate each supplied-membership and conversion event to its initiating phase and budget. This allocation carries the evaluator and conversion-owner portions of QSV FR-109-AC-6, the QSL caller portion of the supplied-caller controls formerly quire-exact FR-380-AC-6, and the evaluator-effect portions formerly quire-exact FR-372-AC-5 and QSV FR-109-AC-8.

## Inputs

- A checked call with supplied arguments, the original cancellation handle, conversion and admission budgets, and an evaluator meter.
- The admitted numeric magnitude bound, or its QSpec FR-323 default when the QSL caller omits the bound.

## Outputs

- The existing typed supplied-admission or conversion result and the evaluator charge record.

## Behavior

1. The QSL system shall debit each supplied-membership or conversion event once, to the budget of the phase that initiated it. An event is one logical membership check or one logical conversion, so a shared helper invoked on behalf of an event adds no second debit to that event's budget or to any other phase's budget.
2. The QSL system shall not create an evaluated `FunctionCall` or an evaluator-meter charge for supplied membership or conversion.
3. The QSL caller shall resolve an omitted numeric magnitude bound to QSpec FR-323's published 4096-bit default and pass that bound to QSV.
4. The QSL system shall attribute a budget denial or cancellation raised during a membership or conversion event to the phase that initiated that event.
5. The QSL system shall leave the evaluator meter and charge sequence unchanged when a membership or conversion event is denied, cancelled, or completed.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-486-AC-1 | For a conversion-triggered shared helper call and for a membership check, the initiating phase's budget is debited once per logical event, the other phase's budget is not debited for that event, and no admission or conversion failure is labelled as an evaluated `FunctionCall`. | Test |
| FR-486-AC-2 | Inspection of the supplied-argument admission and conversion path in `qsl-replay` (currently `qsl-replay/src/execute/argument.rs`, with budgets in `qsl-replay/src/supplied_limits.rs` and the `supplied.*` limit keys in `qsl-replay/src/limits.rs`) and of the evaluator charge sites in `qsl-eval/src/value/expression/evaluate.rs` (`ChargePoint::FunctionCall`) finds, for each of the reachable events (the membership check, the conversion of a supplied value, the shared-helper call made for either, and the typed projection of a denial or cancellation raised by either), exactly one debit site naming exactly one initiating phase budget. The inspection fails if any event has a second debit to another phase's budget, any debit path reaches the evaluator meter or constructs a `FunctionCall`, or any event has no debit site. | Inspection |
| FR-486-AC-3 | When the QSL supplied caller omits the numeric magnitude bound, the bound it passes to the QSV helper is the 4096-bit default published by QSpec FR-323. | Test |
| FR-486-AC-4 | For the supplied Integer and Rational path of quire-exact FR-380, the QSL caller leaves the evaluator meter and charge sequence unchanged, with the QSL-resolved bound passed through. | Test |
| FR-486-AC-5 | For the integrated supplied Decimal fixture of QSV FR-122-AC-2, a membership check, a nonmembership result, a budget denial and a cancellation through the original `Cancel` each leave the evaluator meter and charge sequence unchanged. | Test |
| FR-486-AC-6 | When the membership budget is denied, or the original `Cancel` fires during a membership event, the evaluator meter and charge sequence are unchanged and the failure is attributed to the membership phase. | Test |
| FR-486-AC-7 | When the conversion budget is denied, or the original `Cancel` fires during a conversion event, the evaluator meter and charge sequence are unchanged and the failure is attributed to the conversion phase, not to membership and not to a `FunctionCall`. | Test |

## Dependencies

- [QSV FR-109](ix://agent-ix/quire-semantic-value/FR-109) owns supplied membership, its budget, and the evaluator-side allocation formerly in AC-6 and AC-8.
- [QSV FR-122](ix://agent-ix/quire-semantic-value/FR-122) owns bounded helper invocation, typed projection, and the direct-caller missing-bound behavior in AC-3; the QSL default in this requirement does not change that owner boundary.
- [quire-exact FR-372](ix://agent-ix/quire-exact/FR-372) and [FR-380](ix://agent-ix/quire-exact/FR-380) own exact-helper semantics and the shared-helper allocation formerly in their AC-5 and AC-6.
- [QSpec FR-323](ix://agent-ix/quire-specification/FR-323) publishes the 4096-bit omitted-bound default used by the QSL caller.
- [IR-878](https://linear.app/agent-ix/issue/IR-878) tracks this owner allocation.

## Status

PLANNED: the evaluator and caller test PRs are pending.
