---
id: FR-094
title: "Interpret temporal progress, history and closure"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-008
    type: implements
  - target: ix://agent-ix/quire-specification/FR-091
    type: references
  - target: ix://agent-ix/quire-specification/FR-092
    type: references
---
## Description

When an evaluator consumes a validated progress or closure assertion, it shall
apply that assertion only to the named clock, subjects, decision scope,
surrounding execution, interval and history boundary. The evaluator shall
preserve the independent progress/closure states and the distinction between
complete and incomplete inputs.

## Inputs

An active temporal obligation; its required future horizon and past-history
bound; exact decision-scope and surrounding-execution identities; validated
progress/completeness/closure assertions supplied by the observation authority;
and any earlier result identities affected by them.

## Outputs

An updated temporal disposition carrying assessment execution, independent
decision-scope and surrounding-execution progress/closure, settlement basis and
exact decision support through the shared result contract, or a typed
contradiction, incomplete or refused result. Historical result bytes are
retained when a later assertion invalidates their premises.

## Behavior

Progress shall be monotonic within its identified binding. A regressing or
same-revision conflicting assertion shall produce a typed contradiction or
refusal and shall not roll back progress, restamp closure or replace earlier
result bytes.

Progress shall cover an inclusive boundary.

A timestamped watermark at `W` permits settlement of a
deadline `D` only when `W >= D` and completeness covers every required source,
subject and valuation through `D`. A fixed-sample progress assertion advances
sample ticks even when no business predicate is true. Event-position time shall
not advance during silence merely because wall time passed.

A complete closed decision scope shall authorize the selected closed-boundary
rule and settlement basis `closed-scope`. It shall not restamp the surrounding
execution as closed. An open decision scope shall leave unobserved admitted
continuations possible, but may settle from a `decisive-witness` or
`decisive-counterexample` only under [FR-091](./FR-091-evaluate-bounded-future.md).
All other open futures remain `unsettled` and pending.

The evaluator shall classify end-of-file, consumer shutdown or a file labelled
closed without complete required observations as an incomplete input.

The evaluator shall not apply false extension or finite-window empty truth to
an incomplete input.

Assessment execution, decision-scope progress/closure, surrounding-execution
progress/closure, truth, settlement basis and completeness shall remain
independent. The temporal handoff shall retain the exact decision-support set.
A missing fact inside that set makes the associated truth unavailable; a
missing fact outside a completed support set remains a completeness/adequacy
gap without falsifying or delaying that settled truth.

Past evaluation shall require history through its computed lower boundary or an
authoritative execution origin. A later record inside an interval previously
declared complete is a completeness contradiction supplied by Agent F's
contract. The evaluator shall supersede or invalidate affected results without
rewriting their historical bytes. The v1 evaluator shall not infer watermark
authority or automatically repair settled history.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-094-AC-1 | A timestamped response exactly at an inclusive deadline participates before progress settles the window; the same response after the deadline does not. | Test (TC-115) |
| FR-094-AC-2 | With no business event, complete timestamp/sample progress through a deadline settles the applicable bounded obligation, while event-position time does not advance. | Test (TC-115) |
| FR-094-AC-3 | Open/closed decision scope, open/closed surrounding execution, completed/failed assessment execution and complete/incomplete input remain independently represented; closing one axis cannot close or complete another. | Test (TC-116) |
| FR-094-AC-4 | Progress for a foreign clock, subject, source set or binding cannot settle the obligation. | Test (TC-115) |
| FR-094-AC-5 | A late completeness contradiction identifies the assertion and affected historical result; no earlier bytes or profile identity are restamped. | Test (TC-116) |
| FR-094-AC-6 | Missing predecessor/history on restart requires replay or an admitted state-restoration contract and cannot resume as a fresh complete stream. | Test (TC-118) |
| FR-094-AC-7 | `closed-scope`, `decisive-witness`, `decisive-counterexample`, `unsettled` and `unavailable` are accepted only with their valid truth, scope and exact deciding-fact combinations; every one-axis substitution is refused. | Test (TC-116) |
| FR-094-AC-8 | A same-binding progress regression or conflicting assertion revision produces a typed contradiction/refusal and cannot roll back progress, closure or an earlier result identity. | Test (TC-115) |

## Dependencies

- Agent F owns the validity, provenance and replay form of observation,
  progress, completeness and contradiction assertions. This requirement owns
  only their temporal interpretation.
- Agent B owns the shared result-dimension wire contract.
