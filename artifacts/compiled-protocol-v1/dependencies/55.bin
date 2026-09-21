---
id: FR-091
title: "Evaluate bounded future temporal operators"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-008
    type: implements
  - target: ix://agent-ix/quire-specification/FR-090
    type: references
---
## Description

The temporal evaluator shall apply the inclusive interval and boundary rules of
the selected profile to every admitted bounded future operator without
shortening the interval to the supplied file.

## Inputs

A temporal subject from [FR-090](./FR-090-select-temporal-profile-and-clock.md),
an admitted evaluation anchor, total Boolean predicate valuations, and validated
progress/closure information for the subject's required observations.

## Outputs

A true, false or pending temporal truth disposition with the exact decision
scope, settlement basis and decision-support identities needed by the shared
result contract, or a separate incomplete or refused disposition with the
affected clause, interval and missing premise.

## Behavior

For the two false-extension profiles, let the admitted positions be
`D = {0, ..., n-1}`. At every integer position outside `D`, an atomic predicate
is false while Boolean constants retain their values. At anchor `i`, for
inclusive integers `0 <= a <= b`, the evaluator shall use:

| Operator | Exact false-extension meaning |
| --- | --- |
| `eventually[a,b] p` | Some offset `d` in `[a,b]` makes `p` true at `i+d` |
| `always[a,b] p` | Every offset `d` in `[a,b]` makes `p` true at `i+d` |
| `p until[a,b] q` | Some `d` in `[a,b]` makes `q` true at `i+d`, and every `e` in `[a,d)` makes `p` true at `i+e` |
| `p release[a,b] q` | The Boolean dual of `not p until[a,b] not q` under the same profile |

The lower-bound convention for `until` is deliberate: positions before `a` do
not satisfy its left operand. It matches `mltl.closed-trace/v1` and shall not be
replaced by a different unbounded-until convention under the same identity.

For the timestamped finite-window profile, an eligible future instant has an
admitted clock distance in `[a,b]`. Existential and universal operators quantify
only eligible admitted instants; `until` uses the admitted order and requires
its left operand at every eligible earlier instant from the lower bound to the
witness. Empty existential windows are false and empty universal windows are
true only after the observation authority establishes completeness through the
upper endpoint; before then the result is pending or incomplete. `release`
remains the Boolean dual.

The evaluator shall return a Boolean for an open decision scope only when the
selected profile proves that every admitted continuation preserves it. Such a
result shall identify `decisive-witness` for true or
`decisive-counterexample` for false and shall retain the exact valuation,
capture, clock, progress and profile facts that form its decision-support set.
Closure of that decision scope authorizes `closed-scope`; it does not imply
closure of the surrounding execution. An undecided open scope shall remain
pending with settlement basis `unsettled`.

The evaluator shall represent missing required current valuations or an
incomplete closed input as incomplete rather than pending future time or false.
The evaluator shall make truth unavailable when a fact is missing inside the
exact decision-support set. The evaluator shall retain a fact missing outside a
completed decision-support set in completeness and any dependent
adequacy/package result without changing an already settled temporal truth.

The v1 temporal profiles shall refuse unbounded intervals.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-091-AC-1 | Independent truth tables cover all four operators, every inclusive endpoint, zero-width intervals, nonzero lower bounds, nesting and short traces. | Test (TC-111) |
| FR-091-AC-2 | On a one-position closed trace where atomic `p` is true, `always[0,1] p` is false under false-extension and true under a complete finite-window interpretation; the profiles cannot share one result identity. | Test (TC-112) |
| FR-091-AC-3 | `p until[1,2] q` may be true with `p` false at the anchor when `q` is true at offset one, pinning the selected lower-bound convention. | Test (TC-111) |
| FR-091-AC-4 | A decisive open decision scope may settle early only with the matching decisive basis, complete exact decision support and proof over every admitted continuation; every other unclosed future remains pending, while a closed incomplete input reports incomplete rather than applying false extension. | Test (TC-116) |
| FR-091-AC-5 | Timestamped empty existential/universal window truth is emitted only after complete progress through the inclusive upper endpoint; an eligible event exactly at the deadline participates before settlement. | Test (TC-115) |
| FR-091-AC-6 | Unbounded intervals, unavailable total valuations and exhausted semantic work return non-Boolean dispositions. | Test (TC-118) |

## Dependencies

- [FR-090](./FR-090-select-temporal-profile-and-clock.md).
- Total Boolean predicate projection from
  [contract-IR issue #63](https://github.com/agent-ix/quire-contract-ir/issues/63).
