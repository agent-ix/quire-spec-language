---
id: FR-095
title: "Preserve exact native-to-TL temporal correspondence"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-008
    type: implements
  - target: ix://agent-ix/quire-specification/FR-090
    type: references
  - target: ix://agent-ix/quire-specification/FR-091
    type: references
  - target: ix://agent-ix/quire-specification/FR-092
    type: references
---
## Description

When lowering an admitted native temporal clause, the bridge shall emit a TL
subject only for a versioned correspondence whose admitted domain preserves the
native operator, predicate, clock, capture, history and closure meaning.

## Inputs

An exact native temporal subject; total-Boolean predicate projections governed
by [contract-IR issue #63](https://github.com/agent-ix/quire-contract-ir/issues/63);
selected observation and clock bindings; and an exact bridge-definition
identity, revision and digest naming the TL syntax and evaluator profiles it
targets.

## Outputs

A generated TL formula/valuation request plus a source/result-correspondence
record, or a typed unsupported/refused mapping naming every unmatched semantic
dimension. The generated TL artifact is internal derived output and never
editable source.

## Behavior

The initial correspondence shall use this support table:

| Native request | Existing TL target | Disposition |
| --- | --- | --- |
| Event-position false-extension, bounded future core, complete execution | `mltl.closed-trace/v1` | Supported when every bridge premise below holds |
| Event-position false-extension, bounded future core, open prefix | `mltl.online-prefix/v1` | Supported when every bridge premise below holds |
| Fixed-sample false-extension, bounded future core | The same TL profiles | Supported only with one total valuation per required sample and retained epoch/period/unit correspondence |
| Timestamped-event finite-window | Current TL profiles | Unsupported; no index conversion is inferred |
| Any bounded past operator | Current TL profiles | Unsupported until a separately reviewed TL past profile exists |
| Finite-window future semantics | `mltl.closed-trace/v1` | Unsupported because TL extends atoms false beyond closure |

For a supported mapping, the bridge shall preserve the native source
digest/revision and exact clause/expression span; clause/requirement identity;
language and temporal profile definition; model/type/predicate bindings;
evaluation anchor; immutable capture environment; clock/observation binding;
interval; closure/history premises; TL formula/profile/proposition identities;
and bridge definition.

For every mapped evaluation result, the correspondence record shall also
preserve assessment-execution state; decision-scope progress/closure;
surrounding-execution progress/closure; truth; settlement basis; the exact
decision-support identities; completeness; and any superseded-result relation.
Those dimensions remain native/shared-result metadata outside the TL formula
and valuation request when the selected TL profile does not encode them. Equal
formula bytes or equal TL truth shall not manufacture, erase or merge them.

The bridge shall lower only total native Boolean predicates to TL propositions.

The bridge shall not coerce numeric, nullable, unavailable or pending values.

The bridge shall keep TL proposition identifiers distinct from native
predicate, signal and capture identities.

The bridge shall not treat a formula match without its valuation, capture and
clock mapping as semantic correspondence.

For an event-position or fixed-sample future mapping, the temporal bridge shall
preserve the exact TL lower-bound convention and false-extension rule in
[FR-091](./FR-091-evaluate-bounded-future.md).

The temporal bridge shall not use a backend, syntax or historical result to
repair an unknown mapping, substitute a profile or synthesize missing
correspondence.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-095-AC-1 | Independent native and TL evaluators emit equal truth/pending outcomes for every member of the declared finite exhaustive bounded-future formula/trace population in the event-position mapping domain. | Test (TC-117) |
| FR-095-AC-2 | The one-position `always[0,1] p` vector rejects a finite-window-to-`mltl.closed-trace/v1` equivalence claim. | Test (TC-117) |
| FR-095-AC-3 | Timestamped-event and past-time requests return typed unsupported mappings at current TL versions without emitting a substitute formula. | Test (TC-117) |
| FR-095-AC-4 | Mutating any native/TL profile, source/span, model/predicate, clock, capture, interval, closure/history or bridge identity breaks correspondence and names that axis. | Test (TC-117) |
| FR-095-AC-5 | A numeric, nullable, unavailable or pending native value cannot enter the TL proposition population. | Test (TC-117) |
| FR-095-AC-6 | Fixed-sample lowering retains epoch/period/unit, reports an absent runtime sample as incomplete, and refuses a changed period or event-count reinterpretation even where the TL formula bytes match. | Test (TC-113) |
| FR-095-AC-7 | A supported TL result is accepted only when the correspondence record preserves assessment execution, both progress/closure axes, settlement basis, exact decision support, completeness and supersession; mutating or omitting one axis cannot yield a healthy native result. | Test (TC-117) |

## Dependencies

- [Contract-IR issue #63](https://github.com/agent-ix/quire-contract-ir/issues/63)
  owns typed predicate projection.
- [Contract-IR issue #64](https://github.com/agent-ix/quire-contract-ir/issues/64)
  owns the public bridge interface ticket; this private requirement supplies
  temporal meaning without copying private planning into that repository.
- TL syntax/evaluation retains its own semantic-profile and release authority.
