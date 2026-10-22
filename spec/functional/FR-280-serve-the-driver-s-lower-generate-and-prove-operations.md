---
id: FR-280
title: "Serve the driver's lower, generate and prove operations at the QSL boundary"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-193
    type: references
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-331"
    type: depends_on
---
# FR-280: Serve the driver's lower, generate and prove operations at the QSL boundary

## Description

The orchestrating driver owns `lower`, `generate` and `prove` (ADR-029 LC-1,
OP-1), because CG depends on QSL and a QSL crate that called CG would close a
package cycle (ADR-011 FB-11). The driver repository specifies those
operations. This requirement states what QSL supplies at that boundary:

- the `EmittedPackage` from the same run's `package` (FR-278), which is the
  input `lower` takes and the input `prove` hands a process provider;
- for each `analyze` engine (FR-281), its FR-331 provider manifest, so the
  registry can route items to it (FR-288);
- an entry that runs the items of an FR-331 provider request routed to a QSL
  engine through `analyze`, and returns FR-331 results whose terminal records
  equal those `analyze` settles for the same items, so `prove` and `analyze`
  produce the same record for an item (ADR-029 OP-2). The entry reads the
  run's `CheckedPackage`, the package `analyze` reads, and no emitted package;
- QSL's FR-331 terminal record type (`TerminalRecord`, ADR-013 O-24), the
  one record type both paths write.

The entry takes the checked package because a QSL engine runs in process
over the checked subject, and because some packages a QSL engine settles have
no emitted form: S4 refuses to emit a package that holds a random parameter,
a workload, a reward or a probabilistic claim (FR-185), while its checked
package remains. EN-4 decides a probabilistic claim over that checked subject
(FR-189), and a sampled witness replays over the same subject (FR-193). An
entry that took an `EmittedPackage` could serve no EN-4 item through `prove`,
since every package that holds a probabilistic claim is one S4 refuses.

## Inputs

For the provider entry: the run's `CheckedPackage` from `check` (FR-278),
the subject binding (FR-120), the FR-331 provider request's `manifest` and
its items routed to one QSL engine with the request's `deadline_ms`, the
provider limits and `&Cancel`. The entry takes no `EmittedPackage` and reads no `package`,
`package_id`, `dependencies` or `provision` member: the checked package and
its resolved dependencies are already in process (QSpec FR-331, "In-process
providers").

## Outputs

FR-331 results: one terminal record per routed item.

## Behavior

- QSL shall publish one FR-331 provider manifest for each `analyze` engine,
  listing the (kind, mode) pairs, domains, options and bounds the engine
  accepts.
- Each engine's manifest shall be data of that engine's own module in
  `qsl-analyze`, and `qsl-route` shall hold the registry and FR-288's
  conversion and no engine's manifest.
- An engine that settles `probabilistic-satisfaction` items shall list the
  evidence kinds it settles (FR-057, FR-075).
- EN-4's manifest shall list (`probabilistic-satisfaction`, `bounded`) with
  the evidence kind `statistical` and no other, and with the `domains`
  `population` and `integer`, the boundable domains its sampler draws over
  (FR-057 refuses a `bounded` pair without `domains`).
- When the driver passes an FR-331 provider request routed to a QSL engine,
  the QSL provider entry shall run the routed items through `analyze` over
  the `CheckedPackage` it is given and return one FR-331 terminal record per
  routed item.
- The provider entry shall take the checked package and no emitted package,
  so it serves an item over a checked package that S4 refuses to emit
  (FR-185) as it serves an item over one that S4 emits.
- When an EN-4 item settles `measured` through the provider entry, the
  provider entry shall keep in its record the sampled witnesses `analyze`
  keeps for the same item.
- The provider entry shall keep each sampled witness replayable through
  `replay_sampled_witness` over the checked subject it was given (FR-193).
- The terminal record the provider entry returns for an item shall equal the
  record `analyze` returns for the same item, checked package, subject
  binding and limits.
- No QSL crate shall depend on a CG, RT or driver crate. An IR crate that
  ADR-011 §6.1 sanctions (`quire-contract-model`) is allowed.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-280-AC-1 | Each QSL engine's provider manifest is read by the FR-331 manifest reader and converted by FR-288's conversion into a `BackendDescriptor` whose advertised (kind, mode) pairs equal the claim kinds the engine settles in TC-762. | Test (TC-761) |
| FR-280-AC-2 | For each TC-762 item, an FR-331 provider request that routes that item to its QSL engine, passed to the provider entry with the `CheckedPackage` of the same source, returns a terminal record equal to the one `analyze` returns for that item. | Test (TC-761) |
| FR-280-AC-3 | The `cargo tree` of every QSL crate lists no CG, RT or driver crate; an IR crate ADR-011 §6.1 sanctions (`quire-contract-model`) is allowed (FR-284's direction check). | Test (TC-767) |
| FR-280-AC-4 | EN-4's manifest is read by the FR-331 manifest reader and converted by FR-288's conversion into a `BackendDescriptor` that `BackendDescriptor::admit` accepts, advertising exactly (`probabilistic-satisfaction`, `bounded`) with the evidence-kind set `{statistical}` and the `domains` `{population, integer}`. A registry holding it and a descriptor advertising the same pair with `{exact}` gives a `statistical` probabilistic item the EN-4 `BackendId` alone, and an `exact` item the other alone. | Test (TC-761) |
| FR-280-AC-5 | `Service` (FR-185-AC-7), whose `emit_checked` refuses `unsupported_construct`/`not-yet-implemented`, is passed to the provider entry as its `CheckedPackage`, with its subject binding and an FR-331 provider request routing the `2 ms` claim of ADR-024 §7.2 over `Service` (the claim of FR-193-AC-1 and FR-193-AC-5) to EN-4 with evidence kind `statistical`, the Okamoto method (FR-189-AC-1), seed 7 and `max_witnesses` 3. The entry returns one record, `measured` with decision `rejected`, equal to the record `analyze` returns for that item over the same checked package, binding and limits; its three sampled witnesses equal `analyze`'s and each replays `Reproduced` through `replay_sampled_witness` over `Service`'s checked subject. No `EmittedPackage` of `Service` exists in the run; `Service`'s `emit_checked` is refused before and after the entry runs. | Test (TC-761) |

## Dependencies

- ADR-029 LC-1, OP-1, OP-2: the operation owners and `analyze` inside
  `prove`.
- ADR-011 FB-11: the package direction.
- ADR-013 O-24: the terminal record.
- [FR-278](FR-278-parse-select-check-and-package-as-library-operations.md): `package`.
- [FR-281](FR-281-analyze-claims-with-in-process-engines.md): `analyze`.
- [FR-288](FR-288-build-the-registry-from-provider-manifests.md): the manifest conversion.
- [FR-057](FR-057-admit-shared-capability-kinds.md): the admitted kinds and the `domains` rule.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md): the descriptor's evidence kinds and the candidate sets.
- [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md): S4's refusal of a probabilistic package.
- [FR-189](FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md): EN-4 over the checked subject.
- [FR-193](FR-193-keep-and-replay-sampled-witnesses.md): sampled-witness replay over the checked subject.
- QSpec FR-300: the `lower` and `generate` operations; QSpec FR-331: the
  provider envelope and its in-process form.

## Overlap

The driver repository owns the requirements for `lower`, `generate` and
`prove`, the `Provider` trait (ADR-029 PV-3) and its adapter over this entry,
including how `prove` hands this entry the checked package when `package`
refuses to emit.
CG owns the process-provider negotiation arm (ADR-029 PV-4).

## References

- QSL-393 (V1-A06): typed library APIs.
- QSL-390 (ARCH-50).
- QSpec FR-300 (STD-141): the QSpec half.
