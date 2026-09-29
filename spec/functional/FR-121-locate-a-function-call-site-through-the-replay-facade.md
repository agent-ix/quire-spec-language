---
id: FR-121
title: "Locate a function's call site through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
---
# FR-121: Locate a function's call site through the replay facade

## Description

QSL SHALL provide `qsl_replay::call_site`, a second public entry of the
layer-6 replay facade (ADR-011 §6.1, ADR-013 TK-01) beside `replay`
(FR-098), for a consumer that must build an FR-071 replay request itself
rather than execute one: the compiled package's own `package_id` and, for
one named function, each declared parameter paired with its own node id.

`call_site` compiles a standalone unit through S1 to S4 -- the same
`qsl_replay::spine::compile` FR-027's `compile` command uses -- with no
dependency input, no domain package input and the default spine stage
limits, and resolves `function` by the same one-segment name lookup in the
compiled package's declarations FR-098's selection uses (OQ-5). It derives
each parameter's node id the same way `replay` does, so a `ReplayRequestWire`
keyed by `call_site`'s pairs is keyed the way `replay` actually accepts. It
builds no `replay` call and no `spine::Call`; it names a call site, it does
not make one.

CG needs this because it reaches QSL only through `qsl_replay` (ADR-011
FB-05) and may not call `qsl_replay::spine` itself (ADR-011 §3 FB-05, T-12
rule (a): `spine` is public only for `command`), yet must know a function's
parameter node ids to build the request `replay` will later execute.

Pairing each parameter with its declared name, rather than returning node
ids alone, is required by ADR-013 O-25: a consumer joins a witness row or an
argument to its parameter by declared identity, never by position.

## Inputs

- `source`: the unit's FR-001 source identity.
- `path`, `bytes`: the unit's authored path and source bytes.
- `function`: the `QualifiedName` naming the function to locate.

## Outputs

- A `CallSite` on success: `package_id` (`DigestRecord`) and `parameters`
  (`Vec<(Identifier, WireNodeId)>`, in declared order), or a typed
  `CallSiteRefusal` with no partial result.

## Behavior

- `call_site` SHALL compile `bytes` with `qsl_replay::spine::compile` under
  no dependency input, no domain package input and the default spine stage
  limits, and SHALL carry a compile refusal as `CallSiteRefusal::Compile`,
  its rendered message, with no `spine` type reaching the public error. A
  source that imports a library, selects a domain package, or needs
  non-default stage limits refuses this way; a client that needs one of
  those needs a widened entry, not this one.
- `call_site` SHALL resolve `function` by one-segment name lookup in the
  compiled package's declarations (OQ-5). A `function` with zero or more
  than one segment, or naming no declared function, SHALL refuse
  `CallSiteRefusal::UnknownFunction`, pairing the `QualifiedName` with the
  compiled package's own `package_id` -- never a bare `QualifiedName`
  (FR-088-AC-6, TC-258).
- `call_site` SHALL pair each of the resolved function's declared
  parameters, in declared order, with its own node id, derived the same way
  `replay`'s own executor derives it (FR-098), so the same parameter always
  gets the same node id from either entry.
- A broken invariant -- the resolved function's own node is not itself a
  function node, its parameter count disagrees with its checked signature,
  or one of its declared parameter names is not itself a valid `Identifier`
  -- SHALL refuse `CallSiteRefusal::Fault(InternalFault)`.
- `call_site` SHALL read no path, environment variable, clock or search
  location, and SHALL give the same result for the same input.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-121-AC-1 | For a unit declaring one Boolean predicate `p(x: Int[0, 9]): Boolean`, `call_site` over `p` returns `parameters` holding exactly one pair, its `Identifier` equal to `x`, and its `WireNodeId` is the one `replay` accepts: a `ReplayRequestWire` keyed by that exact pair does not refuse `UnknownParameter` or `UnboundParameter`. | Test (TC-516) |
| FR-121-AC-2 | A `function` naming no declared function of the compiled package refuses `CallSiteRefusal::UnknownFunction`, pairing the `QualifiedName` with the compiled package's own `package_id`, never a bare `QualifiedName`. | Test (TC-516) |

## Dependencies

- [FR-027](FR-027-export-compiled-native-package.md): the spine compile
  `call_site` runs.
- [FR-088](FR-088-clause-name-and-type-identity.md): no identity treats a
  bare `QualifiedName` as sufficient; `UnknownFunction` pairs it with the
  package it was looked up in (FR-088-AC-6, TC-258).
- [FR-098](FR-098-execute-a-replay-request.md): the replay facade
  `call_site` is a second entry of, and the parameter node id derivation it
  shares.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1, §3 FB-05, T-12 rule (a).
- ADR-013 O-25, C-11, TK-01.

## Status

Implemented under QSL-317: `qsl_replay::call_site`, sharing its parameter
node-key derivation with `qsl_replay::replay`'s own selection
(`callable_parameter_keys`), verified by TC-516.
