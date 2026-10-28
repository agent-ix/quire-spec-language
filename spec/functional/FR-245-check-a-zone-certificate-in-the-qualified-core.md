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
    cancel: &Cancel,                       // FR-276
) -> Result<CertificateVerdict, CertificateRefusal>;

pub struct CertificateCheckLimits {
    pub max_certificate_edges: u64,   // default 16_777_216 (2^24)
    pub max_certificate_bytes: u64,   // default 1_073_741_824 (2^30)
}
```

## Outputs

- `CertificateVerdict::Accepted`, `Rejected(CertificateRejection)` naming
  the first failing rule and its node, edge, component, initial discrete
  state or member, or `Stopped(IncompleteCause, CertificateLimit)`.
- `CertificateRefusal` for a stale package or a mismatched identity.

The checker uses FR-338's `CertificateRejection{rule, at}` with FR-338's
`InitialMissing`, `SuccessorMissing`, `BadState`, `BackwardEdge` and
`WitnessFails` and FR-163's `Malformed`. FR-338's `CertificateLocus` gains
`ZoneNode { node: u64 }`, `ZoneEdge { edge: u64 }` and
`ZoneComponent { component: u64 }`, positions in `nodes` and `edges` and a
component index; the checker also uses FR-169's `ModelState` for the
initial discrete state's state-key digest record and FR-163's
`CertificateMember`. QSpec FR-331 writes them as its zone node, zone edge,
zone component, State and certificate member forms; the member `lu_bounds`
is written `bounds`.

| Failure | `rule` | `at` |
| --- | --- | --- |
| A required member is absent: `components` for a CF-2 form, `divergence` for the time-lock-freedom item, `positions` or `refinement` for a refinement form | `Malformed` | `CertificateMember` naming it |
| In a refinement form, for a concrete initial state `i`, `check_initial` yields another position than its initial node lists, or it passes and that node is marked `locked`, or it fails (as `InitialNotAbstract`) and that node is not a `locked` entry node with the reserved index `positions.len()` | `InitialMismatch` | `ZoneNode` of the initial node of `i` |
| In a refinement form, `check_step` fails (`Fails`, `Undefined`, or `Undetermined` that is not meter-only) on an edge whose target is not a `locked` entry node | `StepFails` | `ZoneEdge` of that edge, with the failing verdict as FR-149's locus carries it |
| In a refinement form, a passing edge's target position or `taken` differs from what `check_step` yields, or an edge leaves the locked region for a positioned node | `PostDiffers` | `ZoneEdge` of that edge |
| The certificate's LU bounds are below the bounds the checker computes | `Malformed` | `CertificateMember("bounds")` |
| No node of the initial discrete state covers the initial symbolic state; in a refinement form, `initials[i]` does not cover concrete initial state `i`'s symbolic state, for any `i` | `InitialMissing` | `ModelState` of that initial discrete state |
| A node has no listed edge for an enabled identity, or a successor is not covered by its edge's named target | `SuccessorMissing` | `ZoneNode` of that node |
| A node is bad and not marked `locked`, or a `locked` node reaches (itself included) a quiescent node or a node of a component with a cycle of positive total delay | `BadState` | `ZoneNode` of that node |
| An edge goes to a node of smaller component index | `BackwardEdge` | `ZoneEdge` of that edge |
| A component's stated reason does not hold | `WitnessFails` | `ZoneComponent` of that component |
| A node's divergence path is absent or does not end at a divergent target, or the `admitted` path does not start at the initial node (in a refinement form, at a positioned `initials[i]`), follow certificate edges and end at a quiescent node or a node of a component with a cycle of positive total delay | `WitnessFails` | `ZoneNode` of that node; for `admitted`, its last node |

## Behavior

- The checker SHALL recompile and check the package by FR-098's rules and
  rebuild the subject's `ModelSystem`.
- If the certificate's obligation identity differs from `item` or from the
  recompiled item's identity, then the checker SHALL refuse `stale_dependency`/`content-mismatch`, naming both
  identities.
- **Refinement form.** For a certificate with `refinement` present
  (FR-244 "Refinement form") the checker SHALL recompile and check both
  packages by FR-098's rules, refuse `stale_dependency`/`content-mismatch`
  when the refinement node, either package or the obligation differs from
  the item's (FR-144 "Certificate identity"), and replace the claim
  automaton and its badness test by `check_step` (FR-141) on each edge, its
  own code, run on the positions the certificate lists. It SHALL:
  - run `check_initial` on each concrete initial state `i`, in index order,
    against its initial node (`refinement.initials[i]`, FR-244), and reject
    `InitialMismatch` at the `ZoneNode` of that node unless either
    `check_initial` passes with exactly that node's position and the node is
    not `locked`, or `check_initial` fails and that node is a `locked` entry
    node with the reserved index `positions.len()`. ADR-026 TV-1 governs per
    initial state, exactly as it governs an edge: a failing `check_initial`
    at a time-locked initial node is the failing step of the empty prefix
    and is accepted as an edge into a `locked` entry node is, and the
    `locked` node's soundness is the `BadState` rule's (no quiescent node and
    no positive-delay cycle reachable from it, the node itself included), so a failing `check_initial`
    at a node that is not marked `locked` is rejected, and a `locked` mark on
    an initial node whose forward closure is not time-locked is rejected
    `BadState`. Another initial state's failure never decides this one;
  - run `check_step` on every edge out of a positioned node, in node then
    edge order, before the DBM checks of that node, and reject `StepFails`
    for `Fails`, `Undefined` and a non-meter-only `Undetermined` unless the
    target is a `locked` entry node, `PostDiffers` when the target's
    position or the edge's `taken` differs from the yield;
  - classify as FR-149 does: an `Undetermined` whose cause is only the
    evaluation meter returns `Stopped(LimitReached{evaluation-meter, …})`
    and settles V-7, never a rejection;
  - take the locked region (FR-244 "Refinement form") on concrete steps
    only. A node carrying the reserved index `positions.len()` that is not
    in the locked region, or a locked entry node whose `locked` mark is
    absent, is judged by the rules above: the edge entering it is judged
    by `check_step` and the mark's absence gives `StepFails` at that edge;
    an initial entry node has no entering edge, so its absent mark gives
    `InitialMismatch` at its `ZoneNode`.
  Every other check below applies unchanged.
- The checker SHALL recompute every successor of every node by every
  enabled transition identity with DBM code separate from EN-6's.
- The checker SHALL check each of the following:
  - the listed edges cover every enabled identity from each node;
  - each successor is covered by its named target under the aLU simulation
    with the certificate's LU bounds;
  - the certificate's LU bounds are at least the bounds the checker
    computes from the recompiled model and the claim automaton, built by
    `qsl-eval`'s property-automaton translation (ADR-018 LA-5);
  - the initial symbolic state is covered by the named initial node; in a refinement form, each concrete initial state `i`'s symbolic state is covered by `initials[i]`;
    - no node is bad unless it is marked `locked`, and from each `locked`
    node no quiescent node and no node of a component with a cycle of
    positive total delay is reachable over certificate edges;
  - every edge respects the component numbering, and each component's
    stated reason holds;
  - for the time-lock-freedom item, each node's divergence path exists in
    the certificate and ends at a divergent target;
  - the `admitted` path starts at the initial node (in a refinement form, at any positioned `initials[i]`, so a locked `initials[0]` does not decide it), follows certificate
    edges, and ends at a quiescent node or at a node of a component with a
    cycle of positive total delay (ADR-026 TD-6).
- The checker SHALL check the `Malformed` members first, then the initial
  coverage, then the nodes in node order, and SHALL return `Rejected` with
  the rule and locus the table above gives the first failure.
- The checker SHALL run under `CertificateCheckLimits` and FR-276's
  `Cancel` handle; reaching a limit SHALL return `Stopped` naming the
  limit, its value and its setting.
- The checker's Kani harnesses SHALL prove its DBM operations free of
  overflow and in agreement with a reference implementation on bounded
  dimensions.
- **Verdict fit.** `Accepted` SHALL let the item settle
  `Proved{basis: ZoneCertified, certification: Certified}`; `Rejected` SHALL settle it
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
| FR-245-AC-3 | AC-1's certificate checked against the identity of another item refuses `stale_dependency`/`content-mismatch` naming both; after a source edit that changes the package identity it refuses by FR-098's rule. With `max_certificate_edges` 1 it returns `Stopped` naming the limit and the value 1. | Test (TC-700) |
| FR-245-AC-4 | The Kani harnesses for the checker's `close`, `constrain`, `reset`, `up`, `includes` and aLU test pass for dimension at most 3 with bounds in `[-8, 8]`, proving no overflow and agreement with the reference implementation. | Test (TC-700) |
| FR-245-AC-5 | FR-244-AC-4's certificate is `Accepted`; the same certificate with the `locked` mark removed is `Rejected` `BadState` at the bad node; a certificate for the variant in which `ping` resets `x` that marks `(B, x <= 1)` `locked` is `Rejected` `BadState` at that `locked` node, from which the `ping` cycle of positive total delay is reachable. | Test (TC-700) |
| FR-245-AC-6 | The checker's DBM code is separate from EN-6's: no crate of layers K to 6, SV or R, and no tool crate, names `qsl-analyze` in its normal, dev or build dependencies, so `qsl-replay`'s checker cannot call the engine's DBM (ADR-026 CF-6). | Test (TC-700) |
| FR-245-AC-7 | Rejection rules and loci for AC-2's tamperings: the node removed rejects `SuccessorMissing` at the `ZoneNode` whose edge named it; the shrunk target, `SuccessorMissing` at the edge's source node; the lowered LU bound, `Malformed` at `CertificateMember("bounds")`; the backward edge, `BackwardEdge` at its `ZoneEdge`; the wrong component reason, `WitnessFails` at its `ZoneComponent`; the bad node, `BadState` at its `ZoneNode`; the cut `admitted` path, `WitnessFails` at its last node. A CF-2 certificate without `components` rejects `Malformed` at `CertificateMember("components")` before any successor is recomputed, and one whose initial node does not cover the initial symbolic state rejects `InitialMissing` at the `ModelState` of the initial discrete state. Each rejection is written with QSpec FR-331's rule spelling at its zone node, zone edge, zone component, State or certificate member form. | Test (TC-1197) |
| FR-245-AC-8 | FR-244-AC-5's certificates are each `Accepted`. The refinement certificate of FR-430-AC-6 with the `locked` mark removed from the post-state of `undo` is `Rejected` `StepFails` at the `ZoneEdge` of the `undo` edge; the `TReq` certificate with `positions` removed is `Rejected` `Malformed` at `CertificateMember("positions")` before any successor is recomputed; FR-430-AC-7's refinement (abstract initial `sent` true, so the half is `Violated`) with a forged certificate whose initial node lists the abstract state `sent` false (and carries no `locked` mark) is `Rejected` `InitialMismatch` at the initial `ZoneNode`; the `TReq` certificate with one edge's target position replaced, or one `taken` changed to `Stutter`, is `Rejected` `PostDiffers` at that `ZoneEdge`; the `TReq` certificate checked under a request whose evaluation meter limit is 0 returns `Stopped(LimitReached)` naming `evaluation-meter` at the first `check_step` (FR-141-AC-9's `HeavyEnq` shape is the second fixture), never `Rejected`; AC-7's `Undetermined` variants return `Undecided` from the engine and have no certificate to check; checked against a refinement item over another abstract package it refuses `stale_dependency`/`content-mismatch`. | Test (TC-700) |
| FR-245-AC-9 | Fixture `Init3`: abstract `Req` (`r.sent`, initial `false`; `send` sets it) refined by timed `TInit` (`time dense`; `r` with `sent: Bool`, `k: Int[0, 2]` with no initial value, so the concrete initial states are enumerated in ascending `k` and indices 0, 1, 2 have `k` 0, 1, 2; initial `sent` is `false` for `k` 0 and 2 and `true` for `k` 1; clock `x`; `send` with guard `x >= 1`, precondition `not self.sent and self.k <> 1`; time invariant `when self.k = 1 { x <= 0 }`; step map `send -> send`; mapping `sent -> sent`). Index 1 fails `check_initial` (`InitialNotAbstract{initial: 1}`) and is time-locked; indices 0 and 2 pass and diverge. EN-6 returns `Holds`; the certificate's `initials` has three nodes, index 1 a `locked` entry node with the reserved index, and the checker returns `Accepted` (`Proved`). Rejections, each at the initial `ZoneNode` of the named index: the `locked` mark removed from index 1, or added to index 0, is `InitialMismatch`; `initials[2]` replaced by a node that does not cover index 2's symbolic state is `InitialMissing` at that state's `ModelState`. With the initial states reordered so the locked one is index 0, the certificate is `Accepted` and `admitted` starts at a positioned `initials[i]`. The variant without the time invariant returns `Violated` from EN-6 with `InitialNotAbstract{initial: 1}` and an empty prefix; a forged `Holds` certificate marking its initial node `locked` is `BadState` at that node (a `locked` node reaches itself). Rejections use QSpec FR-331's rule spelling. | Test (TC-700) |

## Dependencies

- ADR-026 §8.1 CF-1, CF-3, CF-4, CF-6, TD-6; ADR-011 §6.1 (layer 6) and E11; ADR-013
  O-09 and O-16; ADR-014 B-5.
- [FR-098](FR-098-execute-a-replay-request.md),
  [FR-235](FR-235-settle-a-timed-verdict-as-a-terminal-record.md),
  [FR-238](FR-238-represent-zones-as-difference-bound-matrices.md),
  [FR-244](FR-244-emit-a-zone-certificate-with-every-proof.md).

## References

- The qualified core and its certificate checkers: ADR-029 CB-2 and RU-2,
  Linear QSL-390.
- QSpec half: QSpec FR-418 (Linear STD-139) owns the certificate wire
  (ADR-026 OV-12).
- Muntac, a verified certificate checker for timed automata, 2025 (ADR-026
  References).
