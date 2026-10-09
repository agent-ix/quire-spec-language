---
id: FR-069
title: "Implement the typed proof-result envelope"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: traces_to
---
# FR-069: Implement the typed proof-result envelope

## Description

QSL SHALL implement a typed proof-result envelope, defined in the layer-6
`replay` module and part of its public API (ADR-011 §6.1, FB-05; ADR-013
O-24, C-23), that reads an FR-331 `quire.backend-provider/v1` terminal
record into exactly one of the seven ADR-013 O-16 outcome categories the
proof column of the O-16 category table produces (success, violation,
refusal, unsupported, incomplete, inconclusive, internal failure), and
carries the `backend` member (ADR-013 O-19). The eighth O-16 category,
`undefined`, is excluded by construction: the O-16 category table marks it
"not produced" for the proof column — `undefined` is reachable only through
FR-323's evaluation outcome, not through an FR-331 terminal record — so this
envelope's reader has no `undefined` arm to map into. CG reaches this
envelope only through the `replay` module's public API (ADR-011 FB-05); no
second proof-result type exists anywhere in QSL.

This requirement builds the reader and the envelope type only. It reads an
already-produced FR-331 record; it does not invoke a backend, run Kani, or
implement CG's C-09 map from `KaniOutcomeKind` and the E9 replay result to
an FR-331 terminal value.

## Inputs

- An FR-331 `quire.backend-provider/v1` envelope's `results`, `dispositions`
  and `accounting` members for one requested item, and its `manifest`
  member for the `backend` identity.

## Outputs

- A typed proof-result envelope carrying exactly one O-16 category, the
  FR-331 terminal record, and the `backend` member; or a structured refusal
  when the input envelope itself is malformed or unsupported.

## Behavior

- The envelope's reader SHALL map every FR-331 result/disposition value to
  its ADR-013 O-16 category through one exhaustive function with no `_`
  fallback arm (ADR-013 O-16 category table).
- A backend result `tested` SHALL remain `tested`. Only a Kani run with at
  least one SUCCESS check maps to `proved`.
- A Kani run whose obligation has zero SUCCESS checks SHALL stay
  `Proved { basis: Checks { success_checks: 0 }, certification: Certified }` (ADR-013 C-09: no separate vacuous variant,
  no `NonZero` count) and map to `inconclusive` with the typed cause
  `kani_vacuous_proof`, never to `proved`'s `success` (ADR-013 O-16 proof
  column).
- A terminal value `Inconclusive` SHALL carry a typed cause, `ReplayParity`
  with the replay's `DisagreementCause` or `ReplayRefused` with the
  refusal's catalog code, and SHALL map to `inconclusive`; the envelope's
  cause is that value's own.
- A terminal record SHALL be keyed by its typed `request_index` (QSpec
  FR-331), one per index, never by display text.
- The reader SHALL refuse an envelope whose encoded size exceeds the
  configured reader bound, and SHALL NOT return a truncated or
  partially-populated envelope in that case.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-069-CON-1 | The envelope type SHALL have no public constructor other than the FR-331 reader, so no caller can construct a proof-result envelope naming an arbitrary category directly. | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-069-AC-1 | Given an FR-331 terminal record for each of the seven O-16 categories the proof column produces, including a vacuous `Proved` (zero SUCCESS checks, category `inconclusive`, cause `kani_vacuous_proof`), an `Inconclusive` value with each of `replay_parity` and `replay_refused` (category `inconclusive`, its own cause, each record keyed by its `request_index`), and a `declined` result with a QSL catalog code and one with a STD-001 registry code, even an unregistered one (category `refusal`, the two never remapped onto each other), and a `tested` backend result (category `success`, distinct from and never promoted to `proved`), the reader maps each to its exact O-16 category with no collapsing. | Test (TC-177) |
| FR-069-AC-3 | Given a positive envelope, a construct → serialize → read round trip preserves the `backend` member (the backend identity) and every per-item disposition byte-for-byte. | Test (TC-179) |
| FR-069-AC-4 | Given an envelope whose encoded size exceeds the configured reader bound, the reader refuses with a bound-exceeded cause and returns no truncated or partially-populated envelope. | Test (TC-178) |

## Dependencies

- **Upstream**: [US-010](../usecase/US-010-carry-a-proof-witness-or-replay-outcome-without-a-shadow-type.md);
  ADR-013 O-16 (category table), O-19 (`backend` member), O-24; QSpec
  [FR-331](https://github.com/agent-ix/quire-specification/blob/main/spec/objects/interfaces/FR-331-backend-provider-envelope.md)
  (Accepted complete-V1 boundary value contract) supplies the `results`,
  `dispositions`, `counterexamples`, `accounting`, `manifest` and tool-lock
  wire this envelope reads.
- **Shared types**: [#213](https://github.com/agent-ix/quire-spec-language/issues/213)
  (ARCH-20) is the owning ticket for the shared O-16 category type and the
  kernel `Outcome`/`Refusal` primitives this envelope's category-mapping
  function reuses unchanged (S-1, S-5a); this requirement defines no
  parallel category or outcome type.
- **Downstream**: [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md)
  carries the counterexample this envelope's `violation` category names.

## Status

Implemented: `qsl_replay::read_backend_provider_envelope`, verified by
TC-177 to TC-179.
