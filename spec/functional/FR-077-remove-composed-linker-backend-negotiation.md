---
id: FR-077
title: "Remove backend negotiation from the composed linker"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-011
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: traces_to
---
# FR-077: Remove backend negotiation from the composed linker

## Description

The composed linker SHALL perform no backend negotiation in its
request-report path (`linking::composed::requests`). The composed linker's
`requests::report` function SHALL record requested capability pairs as data
only. The composed linker SHALL take no `Backend` parameter in
`requests::report`. The composed linker SHALL produce no
`UnsupportedCapability` or `UnsupportedFamily` disposition from
`requests::report`. This is ADR-010 OBS-003 as resolved by ADR-012 §10:
candidates and routing live only in the registry (FR-075), and dispositions
live only in `quire-contract-codegen`'s `negotiate_*`.

## Inputs

- A requested clause/capability pair, unchanged from FR-036's existing
  request shape, minus any backend argument.

## Outputs

- A request report recording each pair as data (declaration, capability
  kind, `required` flag, request index), with no disposition field that
  reflects backend support.

## Behavior

### No `Backend` parameter

`requests::report`'s signature SHALL take no parameter, of any type, whose
purpose is to supply a backend or backend set for negotiation. The function
SHALL be callable, and SHALL produce a complete request report, with no
backend information available to it.

### No `UnsupportedCapability` or `UnsupportedFamily` disposition

The disposition type `requests::report` returns, or any type reachable from
it, SHALL contain no variant named for, or serving the purpose of,
"unsupported capability" or "unsupported family" as a per-pair disposition.
A pair's capability kind admission (FR-057) is unaffected by this removal:
`absent-kind` and `unknown-kind` remain admission-time refusals, produced
without reading backend state; this requirement removes only the backend-
dependent dispositions, not admission's own refusal causes.

### Requests are still recorded as data

Removing negotiation SHALL NOT remove recording. `requests::report` SHALL
continue to record every admitted pair (declaration, capability kind,
`required` flag, request index) as data available to later stages, so that
unconditional removal of the four-kind negotiation path leaves the request
report complete for every pair QSL admits.

### Four-kind compatibility is not retained

Because `requests::report` no longer negotiates, it SHALL retain no
compatibility path for the pre-canonical four-kind capability request label
enum. A request built against the four-kind vocabulary SHALL be treated as
unsupported by the surrounding admission machinery (FR-057), not specially
accepted by this function.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-077-AC-1 | `requests::report`'s signature contains no parameter of a `Backend`-shaped type; a call site supplying no backend argument compiles and produces a complete request report. | Test (TC-199) |
| FR-077-AC-2 | The disposition type returned by, or reachable from, `requests::report` contains no `UnsupportedCapability` variant and no `UnsupportedFamily` variant. | Test (TC-199) |
| FR-077-AC-3 | Given a set of admitted requested pairs, `requests::report` records every pair (declaration, capability kind, `required` flag, request index) in its output; a test that removes the negotiation call from a reference build still observes the full pair set in the report, showing recording is independent of negotiation. | Test (TC-200) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §10
  (ADR-010 OBS-003) is the normative decision this requirement implements:
  "The composed linker performs no backend negotiation. `requests::report`
  records requests as data and has no `Backend` parameter, no
  `UnsupportedCapability` disposition and no `UnsupportedFamily`
  disposition."
- [FR-036](FR-036-link-composed-native-packages.md) owns the request report
  this requirement amends; ADR-012 §10 (OBS-003) records that FR-036's line
  97, AC-5 and AC-6, and TC-115, change with this removal.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md) is where
  candidates and routing now live, replacing the linker's prior direct read
  of backend support.

## Status

Specified under
[quire-spec-language#185](https://github.com/agent-ix/quire-spec-language/issues/185).
Not yet implemented; `linking::composed::requests` still reads a
caller-declared backend's support at `requests::report` on `main`.
Implementation is unconditional: the prior four-kind compatibility path is
removed, not preserved behind a flag (owner ruling on
[quire-spec-language#229](https://github.com/agent-ix/quire-spec-language/issues/229),
recorded in
[quire-specification#116](https://github.com/agent-ix/quire-specification/issues/116)).
