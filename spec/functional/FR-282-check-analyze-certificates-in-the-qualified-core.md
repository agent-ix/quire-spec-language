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
| Product-closure checker | hyperproperties and state-graph claims | ADR-023 HX-7 |

A trap refutation (ADR-022 GV-2) carries no certificate and is settled by
no checker in this table. Its closure check is FR-170's trap replay, also a
layer-6 entry in `qsl-replay` inside the qualified core (ADR-029 CB-2, RU-2):
it re-explores the trap's forward closure with its own code (ADR-022 GX-3,
GC-4), and a closure that reaches a target node settles `inconclusive`,
`Verdicts`, naming that node, so no `CertificateRule` names a trap.

Each is a layer-6 entry in `qsl-replay`, beside `replay`, and QSL settles
the native engine's terminal record there (ADR-029 OP-2). A proof from a
native engine with none of these checkers settles `proved`, labelled `uncertified`
(FR-281, FR-290). A checker recompiles the package from its
byte provision, as `replay` does, and checks the certificate with its own
code. It reads nothing from the engine's search but the certificate. The EN-5
checker has two forms with one set of conditions (FR-201 "Checking"). Over a
package S4 emits, its wire form recompiles from the byte provision. Over a
package S4 refuses to emit (FR-185), its
checked-subject form runs over the `CheckedPackage` that `analyze` holds; a
separate frontend gets that package by re-checking the provision's source
bytes (S1 to S3, in the qualified core) into a `CheckedPackage`, admitting
the `ModelSystem` from it and the subject binding, and running the same form.
No emitted package and no engine code is involved, and the conditions are the
same.

A certificate is a document `analyze` writes. A checker accepts or rejects
it in the run that produced it, and equally in a later run of a separate
frontend, such as the qualified binary's `check-certificate` verb (ADR-029
CB-3 item 4).

## Inputs

- The certificate document.
- The package's byte provision (ADR-013 O-26), as `replay` takes it, and for
  an EN-5 certificate the subject binding (FR-281) that admits its `ModelSystem`.
- The claim item's occurrence key.
- The checker's limits and `&Cancel`.

## Outputs

`Staged<CertificateVerdict>`: accepted, or rejected with a typed rejection
cause; or a `StageFailure` when the package cannot be recompiled.

## Behavior

- Each certificate checker shall recompile the package from the byte
  provision.
- Each certificate checker shall decide the certificate using the
  recompiled package and the certificate document only.
- If the certificate does not establish the claim under the checker's rules,
  then the checker shall reject it with a typed cause naming the first
  failing obligation.
- A certificate checker shall return the same verdict for the same
  certificate bytes and byte provision whichever run or frontend calls it.
- For FR-201's closed reachability, acyclic state-predicate and acyclic expected-reward profiles, the fresh-process facade SHALL
  admit the bounded source `ByteProvision`, re-check S1 to S3 into a
  `CheckedPackage`, admit the `ModelSystem` and selected claim binding,
  read the certificate document through that profile's strict reader and
  invoke the independent core checker with the caller's limits and
  `&Cancel`. It SHALL extract/reuse the source completeness and provision
  identity admission in `qsl-replay/src/request.rs`, together with existing
  content-hash validation wherever required by the provision's source
  contracts; it SHALL NOT duplicate those semantics or invent a new hash
  identity. Source and document byte ceilings SHALL be
  enforced before unbounded allocation or source lookup. The facade SHALL
  observe cancellation throughout admission, parsing and checking.
- That facade SHALL expose the checked-source path independently of S4
  emission. `compile_package` runs S4 and therefore SHALL NOT be required
  to obtain this package: random/workload packages refused by FR-185 still
  reach the checker through qualified S1 to S3. The facade SHALL NOT require
  a fake emitted package or fabricated `ReplayRequestWire` for this path. The existing
  replay-wire entry retains its own decode/recompile obligations.
- The fresh checker executable SHALL link no provider or `qsl-analyze`
  dependency and SHALL use only the certificate document, provision,
  checked subject/claim binding and caller controls as evidence. It SHALL
  enforce FR-201's identity and independently rederived objective comparison
  before accepting the document.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-282-AC-1 | The EN-5 certificate from FR-281-AC-1, written to bytes by `analyze` and passed with the byte provision and the subject binding to the EN-5 checker in a fresh process that links only qualified-core crates, is accepted: the process re-checks the provision's sources into a `CheckedPackage`, admits the `ModelSystem` and runs the checked-subject form. | Test (TC-764) |
| FR-282-AC-2 | The same certificate with one probability bound changed is rejected with a cause naming the failing obligation. | Test (TC-764) |
| FR-282-AC-3 | A zone certificate from an ADR-026 zone-search run over a dense-time model is accepted by the zone checker, and the same certificate with one zone removed is rejected. | Test (TC-764) |
| FR-282-AC-5 | The EN-1 closure certificate of an invariant proved over a finite model is accepted by the EN-1 closure checker; the same certificate with one reachable product state removed is rejected with a cause naming the successor it no longer contains. | Test (TC-764) |
| FR-282-AC-6 | The EN-1 component certificate of `always eventually q` under `fair weak inc`, proved over a finite model, is accepted by the SCC and ranking checker; the same certificate with two components swapped out of topological order is rejected with a cause naming the edge that runs backward. | Test (TC-764) |
| FR-282-AC-7 | The simulation-relation certificate of a refinement that holds is accepted by the simulation-relation checker; the same certificate with one pair removed from the relation is rejected with a cause naming the concrete step left without a matching abstract step. | Test (TC-764) |
| FR-282-AC-8 | The product-closure certificate of a two-copy hyperproperty proved over a finite model is accepted by the product-closure checker; the same certificate with one product-state key removed is rejected with a cause naming the successor it no longer contains. | Test (TC-764) |
| FR-282-AC-9 | **RETIRED (staleness check withdrawn).** The certificate `package_id` check is deleted. | Test (TC-1141), retired |
| FR-282-AC-10 | **RETIRED (staleness check withdrawn).** The certificate `package_id` check is deleted. | Test (TC-1141), retired |
| FR-282-AC-11 | An exact acyclic eventual certificate from a random/workload source package that S4 refuses is written, then accepted in a fresh executable linking only qualified-core crates after bounded source admission and S1 to S3 checking. No emitted-package bytes or fabricated replay request are supplied. The same executable rejects changed values, refuses mismatched identity/objective, and preserves typed source/hash failures. | Test |
| FR-282-AC-12 | With a source provision or certificate document one byte above its caller ceiling, the fresh checker stops/refuses before reading beyond that ceiling; pre-cancellation and cancellation during graph expansion stop without acceptance or a truth value. Exhausted state, edge and arithmetic budgets retain the actual named setting and consumption. | Test |
| FR-282-AC-13 | A real FR-197 Exact bounded event-position certificate for FR-201-AC-13 is written by analyze and accepted from its canonical document bytes in a fresh qualified-core-only process using the source provision and actual subject/claim binding. The process independently reconstructs formula-monitor keys and terminal closure through qsl-eval, without qsl-analyze or provider linkage. Changing a running monitor horizon, binding or bounded rank prevents acceptance. | Test |
| FR-282-AC-14 | Analyze writes the exact until and always proof documents of FR-201-AC-15 and AC-16; a fresh executable linking only qualified-core crates accepts their canonical bytes after independent source S1 to S3 admission and actual subject/claim binding, including random/workload packages that S4 cannot emit. It independently derives the original and stopped graphs, predicate labels and terminal-stutter boundary semantics. Changed formula with changed extensional labels, event, optimization, failure/target set, binding or complete key prevents acceptance; a threshold beyond the certified side rejects. Existing reachability/v1 bytes and verdicts are preserved. | Test |
| FR-282-AC-15 | For AC-14, source/document byte ceilings, state/edge/arithmetic limits, pre-cancellation and cancellation during independent graph expansion preserve typed nonacceptance with the actual named limit, consumption or cancellation. No analyzer/provider linkage, emitted-package bytes, fabricated replay request, solver labels, local state ids or new witness wire are used. | Test |

| FR-282-AC-16 | Analyze emits the real expected-reward proof documents of FR-201-AC-19 to AC-21. A fresh executable linking only qualified-core crates accepts their canonical bytes from the bounded source provision and actual S3 subject/claim binding, including random/workload source packages refused by S4. It independently rederives reward identity, exact unit normalization, semantic actions/draws/costs, original acyclicity and stopped Bellman equations, with no qsl-analyze/provider dependency, emitted-package bytes, fabricated replay request or solver graph. Source admission failures retain their typed refusal; changed claim/reward/target/unit identity, complete key or an unmet threshold prevents acceptance. Changed source costs that falsify the equations reject, while equivalent independently rederived costs may accept. | Test |
| FR-282-AC-17 | For AC-16, source/document byte ceilings, state/edge/rational/key-representation budgets, pre-cancellation and cancellation during independent reward evaluation/graph expansion preserve the actual typed failure, named limit/consumption or cancellation without acceptance or truth values. Both older probability document profiles retain their canonical bytes and verdicts. | Test |

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
