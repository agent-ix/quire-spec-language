---
id: FR-076
title: "Settle unmet capability demand as unsupported, never a refusal or hold"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-011
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
---
# FR-076: Settle unmet capability demand as unsupported, never a refusal or hold

## Description

An item whose capability kind no registered backend advertises SHALL settle
`unsupported`, with a warning naming that capability kind and, when the
request named one, the named backend. This is the solver-absence policy
(quire-specification#116; FR-290 "Backend absence"; ADR-012 §7.3). It SHALL
NOT be produced as a refusal, an error, or a hold that waits for a future
registration; the registry SHALL supply the empty candidate set as ordinary
data and SHALL NOT itself raise a refusal-shaped or hold-shaped result for
this case.

## Inputs

- A requested item's capability kind and any named `BackendId`.
- The registry's candidate-set computation (FR-075) for that item.

## Outputs

- An empty candidate set, passed on unchanged as the item's `candidates`
  value (quire-specification FR-331).
- The eventual `unsupported` disposition and its warning are settled by
  `quire-contract-codegen`'s `negotiate_*`
  ([quire-contract-codegen#86](https://github.com/agent-ix/quire-contract-codegen/issues/86)),
  which is the only negotiation point (ADR-012 §7.1, §7.2); this
  requirement's registry contributes the empty set that makes that
  disposition possible and asserts that the set is never converted to
  anything else on its way there.

## Behavior

### The registry never converts absence into a refusal or a hold

When an item's candidate set is empty, the registry SHALL return that empty
set as an ordinary, well-formed value. The registry SHALL NOT raise an
error, panic, or refusal in this case, and SHALL NOT defer or queue the item
pending a future registration. No stage in this requirement produces a hold
(ADR-012 §7.3, FR-057 "Absence, unsupported, refusal, timeout and hold").

### Absence is distinct from an unregistered named backend

Backend absence (an empty candidate set because no registrant advertises the
kind, or a registered named backend does not) is a different case from a
request naming a `BackendId` the registry does not hold at all (the
unknown-backend marker, FR-075-AC-3). Both are ordinary registry outputs;
neither is a registry-level refusal.

### The registry carries the naming data the warning needs

The registry's output for an empty candidate set SHALL retain the item's
capability kind and, when the request named a `BackendId`, that identity, so
that `negotiate_*`'s warning can name both without re-deriving them from
elsewhere.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-076-AC-1 | Given a registry with no registrant advertising an item's capability kind, the registry's candidate-set computation returns an empty set as an ordinary value; no error, panic or refusal-shaped result is observed at the registry boundary. | Test (TC-197) |
| FR-076-AC-2 | Given the same empty-candidate-set case, the registry's output retains the item's capability kind and, when the request named a backend, that backend's identity, so a caller can name both in a downstream warning without a separate lookup. | Test (TC-197) |
| FR-076-AC-3 | A test harness that asserts on the registry's return type shows no variant, error kind or panic path corresponds to "hold" or "refused because no backend registered"; an empty candidate set and a registration-time refusal (FR-075-AC-4) are distinct, differently-shaped outputs. | Test (TC-198) |

## Dependencies

- [FR-075](FR-075-compute-candidates-from-registered-backends.md) computes
  the candidate set this requirement's empty case is drawn from.
- [FR-290](ix://agent-ix/quire-specification/FR-290) "Backend absence" and
  FR-290-AC-4 are the normative statement that backend absence settles
  `unsupported`, warned, and are the ruling this requirement's registry-side
  half implements; the disposition itself is settled downstream by
  `negotiate_*`, not by this requirement.
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §7.3
  states that no backend is invoked and no artifact is emitted for an absent
  item, and that the outcome is terminal for the request, not a hold.
- Owner ruling recorded in
  [quire-specification#116](https://github.com/agent-ix/quire-specification/issues/116):
  "a claim no registered backend can discharge settles `unsupported` with a
  warning, never a hold."

## Status

Specified under
[quire-spec-language#185](https://github.com/agent-ix/quire-spec-language/issues/185).
Not yet implemented. The settled `unsupported` disposition itself is
produced by `negotiate_*`, which
[quire-contract-codegen#86](https://github.com/agent-ix/quire-contract-codegen/issues/86)
implements; this requirement's exit also waits on that ticket because only
`negotiate_*` settles the outcome.
