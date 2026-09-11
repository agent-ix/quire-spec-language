---
id: FR-043
title: "Evaluate bounded native temporal obligations under one selected profile"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-003, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-044, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-048, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-090, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-091, type: references }
  - { target: ix://agent-ix/quire-specification/FR-092, type: references }
  - { target: ix://agent-ix/quire-specification/FR-095, type: references }
---
# FR-043: Evaluate bounded native temporal obligations under one selected profile

## Description

When a caller supplies an admitted `quire.compiled-protocol/1` temporal
requirement, its exactly one selected temporal profile and an independently
supplied observation trace, the evaluator SHALL return the obligation's temporal
truth, its settlement basis and its retained premises, or a located incomplete or
refused result that names the missing dimension.

This requirement owns the compiler-side temporal semantics of the emitted
artifact. It does not own observation transport, protocol conformance results or
the native-to-TL bridge; a mapping request that the current bridge cannot express
remains an explicit refusal under
[FR-044](./FR-044-activate-temporal-obligations.md)'s retained native subject.

## Inputs

The emitted temporal body (current-input binder, clock binding, activation,
captures and the closed temporal operation graph), the declaration's admitted
registered profile identity, and one caller-constructed observation trace.

A trace supplies its own profile discriminant and carries, per admitted position:
the clock coordinate in that profile's domain, a total valuation for every
`holds` leaf handle reachable from the declaration root, and the position's
admitted order key where the profile declares one. The trace separately carries
its decision-scope progress state (open or closed), its completeness assertion and
its authoritative origin, if any.

Caller-lowered evaluation limits arrive as explicit values under
[NFR-008](../non-functional/NFR-008-bound-temporal-evaluation.md). The evaluator
consumes no ambient clock, installed default profile, file ordering or backend
capability report.

## Outputs

A temporal assessment carrying truth (`true`, `false` or `pending`), the
settlement basis, the exact decision support that established it, the retained
clock/profile/trace premises and the separate activation dimension; or a typed
`Incomplete` or `Refused` result naming the affected clause, position and
dimension.

## Behavior

The evaluator SHALL reject a trace whose profile discriminant differs from the
declaration's admitted registered profile identity, and SHALL NOT substitute,
widen or default a profile, clock, sample period, epoch, unit or sequence
authority.

The evaluator SHALL interpret one interval tick under the selected profile:
one admitted semantic-event position for
`quire.temporal.event-position.false-extension/v1`; one declared-period sample
from the declared epoch for `quire.temporal.fixed-sample.false-extension/v1`; and
one tick of the declared timestamp unit for
`quire.temporal.timestamped-event.finite-window/v1`.

The evaluator SHALL evaluate all eight bounded operators — `eventually`,
`always`, `until`, `release`, `once`, `historically`, `since` and `triggered` —
over inclusive integer intervals with `0 <= a <= b`, and SHALL evaluate `not`,
`and`, `or` and `implies` as untimed Boolean connectives over temporal operands.

The evaluator SHALL treat `holds(expr)` as an atomic temporal valuation that is
distinct from a source constant. Under the two false-extension profiles, when the
decision scope is closed and complete, an atomic valuation SHALL be false at
integer offsets outside the scope while a temporal constant SHALL retain its
value. Under the finite-window profile, quantification SHALL range only over
admitted instants inside the window and SHALL add no synthetic atom after
closure.

The evaluator SHALL settle a Boolean on an open prefix only when every admitted
continuation preserves it with complete exact decision support; otherwise the
result SHALL be `pending` with an unsettled basis.

The evaluator SHALL refuse `until`, `release`, `since` and `triggered` when two
participating positions carry equal clock coordinates without an admitted order
key, and SHALL NOT substitute ingestion order, arrival order or trace index.

The evaluator SHALL classify a missing required valuation, sample or history
interval as incomplete against its exact position, and SHALL NOT read it as a
false atom, an empty population or a settled truth.

The evaluator SHALL retain the profile identity, clock binding, trace
completeness assertion and authoritative origin in the result's premises.

If a limit, profile, clock or binding selection differs from the one that
produced a retained result, then the evaluator SHALL compute a new result rather
than reuse that retained one.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-043-AC-1 | On one complete closed position with `p` true, `always[0,1] holds(p)` is false and `always[0,1] true` is true under both false-extension profiles; no evaluation or graph rewrite folds the two into one result. | Test (TC-122) |
| FR-043-AC-2 | The same `holds` valuations and the same numeric interval evaluated under event-position, fixed-sample and timestamped-event selections yield three results retaining three distinct profile identities and premises, with the finite-window case true where the false-extension cases are false. | Test (TC-122) |
| FR-043-AC-3 | A trace whose profile discriminant, clock identity, sample period, epoch, timestamp unit or sequence authority differs from the declaration's admitted selection refuses before evaluation and names the affected clause; no default or nearest-compatible selection is inserted. | Test (TC-122) |
| FR-043-AC-4 | All eight bounded operators evaluate over inclusive intervals including the zero-width `[k,k]` form, and `once[1,1]` expresses strong previous; a zero-width interval remains distinct from an absent position. | Test (TC-122) |
| FR-043-AC-5 | `until`, `release`, `since` and `triggered` refuse when participating positions share a clock coordinate with no admitted order key, while the same trace with an admitted order key evaluates; trace index is never substituted. | Test (TC-122) |
| FR-043-AC-6 | An open decision scope returns `pending` with an unsettled basis unless every admitted continuation preserves the Boolean; a witnessed `eventually[0,30] holds(p)` settles true on an open scope without closing the execution. | Test (TC-122) |
| FR-043-AC-7 | A missing required valuation, absent fixed sample under a valid binding, or omitted history interval reports incomplete against its exact position and never becomes false, empty or settled. | Test (TC-122) |
| FR-043-AC-8 | A result retains its profile, clock, completeness and origin premises, and a changed profile, clock binding or limit configuration produces a distinct result identity rather than reusing the earlier one. | Test (TC-122) |

## Dependencies

- **Upstream**: [FR-042](./FR-042-publish-compiled-protocol-artifacts.md) emits the
  temporal body, clock binding requirement and closed operation graph consumed here.
- **Upstream**: [US-003](../usecase/US-003-evaluate-bounded-state.md) drives honest
  bounded outcomes.
- **Peer**: [FR-044](./FR-044-activate-temporal-obligations.md) owns the separate
  activation and capture dimension.
- **Constrained by**:
  [NFR-008](../non-functional/NFR-008-bound-temporal-evaluation.md).
- Shared source requirements: `ix://agent-ix/quire-specification/FR-048`,
  `ix://agent-ix/quire-specification/FR-090`. The shared temporal meaning is
  owned by agent E; this requirement specifies the compiler's evaluation of the
  artifact it emits, not the shared semantics themselves.
