---
id: FR-092
title: "Evaluate bounded past temporal operators"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-008
    type: implements
  - target: ix://agent-ix/quire-specification/FR-090
    type: references
---
## Description

When evaluating an admitted bounded past operator, the temporal evaluator shall
distinguish positions before an authoritative execution origin from required
history that was omitted or evicted.

## Inputs

A temporal subject and anchor, inclusive integer bounds `0 <= a <= b`, total
Boolean predicate valuations, and either an authoritative execution origin or
the complete required history identified by the observation binding.

## Outputs

A true or false past-time result, or a separate incomplete/refused disposition
that identifies missing history, an unsupported profile/operator or a resource
failure.

## Behavior

For an anchor whose history is complete back to an authoritative origin, the
false-extension profiles shall treat atomic predicates before that origin as
false and Boolean constants as unchanged. They shall use:

| Operator | Exact past meaning |
| --- | --- |
| `once[a,b] p` | Some offset `d` in `[a,b]` makes `p` true at `i-d` |
| `historically[a,b] p` | Every offset `d` in `[a,b]` makes `p` true at `i-d` |
| `p since[a,b] q` | Some `d` in `[a,b]` makes `q` true at `i-d`, and every `e` in `[a,d)` makes `p` true at `i-e` |
| `p triggered[a,b] q` | The Boolean dual of `not p since[a,b] not q` under the same profile |

A supplied history cutoff is not an execution origin. If the cutoff does not
prove that every needed position is inside the available history, then the
evaluator shall report missing history rather than apply false extension.

The timestamped finite-window profile shall quantify admitted past instants by
inclusive clock distance.

The timestamped finite-window evaluator shall require complete history over the
window before applying empty-window duals.

The v1 profile shall define strong previous as `once[1,1]`.

If a later profile admits weak previous, then that profile shall define its
boundary separately from strong previous.

The v1 temporal profiles shall refuse unbounded past and implicit history
reconstruction.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-092-AC-1 | Independent truth tables distinguish all four past operators, inclusive endpoints, zero-width intervals, nonzero lower bounds and each origin boundary. | Test (TC-112) |
| FR-092-AC-2 | The same visible suffix yields a Boolean when its start is the authoritative execution origin and missing-history incomplete when it is merely a cutoff. | Test (TC-112) |
| FR-092-AC-3 | `p since[1,2] q` uses `p` only between the lower bound and its witness, symmetrically with the selected future `until` convention. | Test (TC-112) |
| FR-092-AC-4 | Missing, evicted, stale or binding-mismatched history never becomes false padding or a conclusive Boolean. | Test (TC-118) |
| FR-092-AC-5 | Current TL future-only profiles are reported unsupported for every past operator until a separately reviewed past-time TL profile establishes exact correspondence. | Test (TC-117) |

## Dependencies

- [FR-090](./FR-090-select-temporal-profile-and-clock.md).
- The future/past profile-evolution work remains separately owned in the TL
  ecosystem; ticket existence is not implementation evidence.
