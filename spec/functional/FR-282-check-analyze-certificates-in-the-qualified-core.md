---
id: FR-282
title: "Check analyze certificates in the qualified core"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-284
    type: references
---
# FR-282: Check analyze certificates in the qualified core

## Description

`analyze` enters the qualified core only through its certificate checkers
(ADR-029 CB-2). The checkers are:

| Checker | Certifies | Defined by |
| --- | --- | --- |
| Zone certificate checker | dense-time reachability and Büchi emptiness | ADR-026 CF-1 to CF-5 |
| EN-5 certificate checker | exact probability bounds | ADR-028 CE-1 to CE-5 |
| EN-1 closure checker | safety: the reached product states are closed and none is bad | ADR-018 PC-3 |
| SCC and ranking checker | liveness and fairness: every cycle lies in one component whose witness holds | ADR-018 PC-4 |
| Simulation-relation checker | refinement: the mapping is a simulation of the abstract model | ADR-020 |
| Product-closure checker | hyperproperties and state-graph claims, including the trap closure of ADR-022 | ADR-023 HX-7, ADR-022 GX-3 |

Each is a layer-6 entry in `qsl-replay`, beside `replay`, and QSL settles
the native engine's terminal record there (ADR-029 OP-2). A proof from a
native engine with none of these checkers settles `proved`, labelled `uncertified`
(FR-281, FR-290). A checker recompiles the package from its
byte provision, as `replay` does, and checks the certificate with its own
code. It reads nothing from the engine's search but the certificate.

A certificate is a document `analyze` writes. A checker accepts or rejects
it in the run that produced it, and equally in a later run of a separate
frontend, such as the qualified binary's `check-certificate` verb (ADR-029
CB-3 item 4).

## Inputs

- The certificate document.
- The package's byte provision (ADR-013 O-26), as `replay` takes it.
- The claim item's occurrence key.
- The checker's limits and `&Cancel`.

## Outputs

`Staged<CertificateVerdict>`: accepted, or rejected with a typed rejection
cause; or a `StageFailure` when the package cannot be recompiled.

## Behavior

- Each certificate checker shall recompile the package from the byte
  provision and check that the recompiled `package_id` equals the one the
  certificate names.
- If the recompiled `package_id` differs from the certificate's, then the
  checker shall reject the certificate with a cause naming both identities.
- Each certificate checker shall decide the certificate using the
  recompiled package and the certificate document only.
- If the certificate does not establish the claim under the checker's rules,
  then the checker shall reject it with a typed cause naming the first
  failing obligation.
- A certificate checker shall return the same verdict for the same
  certificate bytes and byte provision whichever run or frontend calls it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-282-AC-1 | The EN-5 certificate from FR-281-AC-1, written to bytes by `analyze` and passed with the byte provision to the EN-5 checker in a fresh process that links only qualified-core crates, is accepted. | Test (TC-764) |
| FR-282-AC-2 | The same certificate with one probability bound changed is rejected with a cause naming the failing obligation; with its `package_id` replaced by another package's, it is rejected with a cause naming both identities. | Test (TC-764) |
| FR-282-AC-3 | A zone certificate from an ADR-026 zone-search run over a dense-time model is accepted by the zone checker, and the same certificate with one zone removed is rejected. | Test (TC-764) |
| FR-282-AC-4 | Over ADR-022 §7.2's game model, the closure check accepts the trap at `Lost`, whose forward closure `{Lost}` holds no target node, after exploring that closure itself; given a trap at a node whose forward closure reaches a target node, it rejects with a cause naming that target node. | Test (TC-764) |
| FR-282-AC-5 | The EN-1 closure certificate of an invariant proved over a finite model is accepted by the EN-1 closure checker; the same certificate with one reachable product state removed is rejected with a cause naming the successor it no longer contains. | Test (TC-764) |
| FR-282-AC-6 | The EN-1 component certificate of `always eventually q` under `fair weak inc`, proved over a finite model, is accepted by the SCC and ranking checker; the same certificate with two components swapped out of topological order is rejected with a cause naming the edge that runs backward. | Test (TC-764) |
| FR-282-AC-7 | The simulation-relation certificate of a refinement that holds is accepted by the simulation-relation checker; the same certificate with one pair removed from the relation is rejected with a cause naming the concrete step left without a matching abstract step. | Test (TC-764) |
| FR-282-AC-8 | The product-closure certificate of a two-copy hyperproperty proved over a finite model is accepted by the product-closure checker; the same certificate with one product-state key removed is rejected with a cause naming the successor it no longer contains. | Test (TC-764) |

## Dependencies

- ADR-029 CB-2, CB-3: the checkers in the core.
- ADR-018, ADR-020, ADR-022, ADR-023, ADR-026, ADR-028 (drafts): the
  checkers' rules.
- ADR-013 O-26: the byte provision.
- [FR-098](FR-098-execute-a-replay-request.md): recompilation from the byte provision.
- [FR-281](FR-281-analyze-claims-with-in-process-engines.md): `analyze` calls the checkers.
- [FR-284](FR-284-keep-the-qualified-core-separable-by-crate.md): the checkers sit in the core.

## References

- QSL-390 (ARCH-50): ruling RU-2 and the checker list ruled with RU-6,
  recorded on the ticket.
- The checkers' defining drafts: PR #562 (ADR-018: EN-1 closure, SCC and
  ranking), PR #564 (ADR-020: simulation relations), PR #571 (ADR-023:
  product closure), PR #573 (ADR-026: zone), PR #579 (ADR-028: EN-5).
