---
id: FR-245
title: "Check a zone certificate in the qualified core"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-238
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-244
    type: depends_on
---
# FR-245: Check a zone certificate in the qualified core

## Description

`qsl_replay::check_zone_certificate`, a layer-6 facade entry beside
`replay_model_trace`, SHALL verify a zone certificate with its own DBM code
in exact arithmetic, reading nothing from the search but the certificate
(ADR-026 CF-3, CF-4, CF-6). It is the in-core certificate checker through
which a zone-engine proof enters the qualified core: the checker is inside
the core and the engine outside it, and a zone-engine `proved` counts only
once the checker accepts its certificate.

## Use case

An assessor qualifies only the core. A zone-engine proof reaches a
qualified verdict because the core recompiles the package, recomputes
every successor of every certificate node and checks every coverage and
component reason itself.

## Inputs

```rust
pub fn check_zone_certificate(
    request: ReplayRequest<'_>,            // FR-098: package, byte provision, limits
    item: &ObligationIdentity,
    certificate: &ZoneCertificate,         // FR-244
    limits: CertificateCheckLimits,
    poll: impl FnMut() -> bool,
) -> Result<CertificateVerdict, CertificateRefusal>;

pub struct CertificateCheckLimits {
    pub max_certificate_edges: u64,   // default 16_777_216 (2^24)
    pub max_certificate_bytes: u64,   // default 1_073_741_824 (2^30)
}
```

## Outputs

- `CertificateVerdict::Accepted`, `Rejected(RejectReason)` naming the first
  failing node, edge or component, or `Stopped(IncompleteCause,
  CertificateLimit)`.
- `CertificateRefusal` for a stale package or a mismatched identity.

## Behavior

- The checker SHALL recompile and check the package by FR-098's rules and
  rebuild the subject's `ModelSystem`.
- If the certificate's obligation identity differs from `item` or from the
  recompiled item's identity, then the checker SHALL refuse `stale_dependency`/`revision-mismatch`, naming both
  identities.
- The checker SHALL recompute every successor of every node by every
  enabled transition identity with DBM code separate from EN-6's.
- The checker SHALL check each of the following:
  - the listed edges cover every enabled identity from each node;
  - each successor is covered by its named target under the aLU simulation
    with the certificate's LU bounds;
  - the certificate's LU bounds are at least the bounds the checker
    computes from the recompiled model and claim automaton;
  - the initial symbolic state is covered by the named initial node;
  - no node is bad;
  - every edge respects the component numbering, and each component's
    stated reason holds;
  - for the time-lock-freedom item, each node's divergence path exists in
    the certificate and ends at a divergent target;
  - the `admitted` path starts at the initial node, follows certificate
    edges, and ends at a quiescent node or at a node of a component with a
    cycle of positive total delay (ADR-026 TD-6).
- The first failure in node order SHALL return `Rejected` naming it.
- The checker SHALL run under `CertificateCheckLimits` and `poll`; reaching
  a limit SHALL return `Stopped` naming the limit and its value.
- The checker's Kani harnesses SHALL prove its DBM operations free of
  overflow and in agreement with a reference implementation on bounded
  dimensions.
- **Verdict fit.** `Accepted` SHALL let the item settle
  `Proved{basis: ZoneCertified}`; `Rejected` SHALL settle it
  `inconclusive`, `CertificateRejected`; `Stopped` SHALL settle it V-7
  (FR-235).
- The checker SHALL read no path, environment variable, clock or search
  output beyond the certificate, and SHALL give the same verdict for the
  same inputs.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-245-AC-1 | FR-244-AC-1, AC-2 and AC-3's certificates are each `Accepted`. | Test (TC-700) |
| FR-245-AC-2 | Each tampering is `Rejected`, naming the failing part: one node removed; one target zone shrunk so it no longer covers its successor; one LU bound lowered below the computed bound; one edge pointing to a node of smaller component index; one component reason naming a fairness constraint that an edge inside it takes; one node replaced by a bad node; the `admitted` path cut so it ends at a node that is neither quiescent nor in a component with a positive-delay cycle. | Test (TC-700) |
| FR-245-AC-3 | AC-1's certificate checked against the identity of another item refuses `stale_dependency`/`revision-mismatch` naming both; after a source edit that changes the package identity it refuses by FR-098's rule. With `max_certificate_edges` 1 it returns `Stopped` naming the limit and the value 1. | Test (TC-700) |
| FR-245-AC-4 | The Kani harnesses for the checker's `close`, `constrain`, `reset`, `up`, `includes` and aLU test pass for dimension at most 3 with bounds in `[-8, 8]`, proving no overflow and agreement with the reference implementation. | Test (TC-700) |

## Dependencies

- ADR-026 §8.1 CF-1, CF-3, CF-4, CF-6, TD-6; ADR-011 §6.1 (layer 6) and E11; ADR-013
  O-09 and O-16; ADR-014 B-5.
- [FR-098](FR-098-execute-a-replay-request.md),
  [FR-235](FR-235-settle-a-timed-verdict-as-a-terminal-record.md),
  [FR-238](FR-238-represent-zones-as-difference-bound-matrices.md),
  [FR-244](FR-244-emit-a-zone-certificate-with-every-proof.md).

## References

- The qualified core and its certificate checkers: ADR-029 CB-2 and RU-2
  (draft).
- QSpec half: QSpec FR-418 (Linear STD-139) owns the certificate wire
  (ADR-026 OV-12).
- Muntac, a verified certificate checker for timed automata, 2025 (ADR-026
  References).
