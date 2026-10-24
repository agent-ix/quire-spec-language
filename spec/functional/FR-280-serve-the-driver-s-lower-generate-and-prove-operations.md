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
- an entry, `qsl_analyze::provider::run`, that runs the items of an FR-331
  provider request routed to a QSL engine through `analyze`, and returns
  terminal records equal to those `analyze` settles for the same items, so
  `prove` and `analyze` produce the same record for an item (ADR-029 OP-2).
  The entry reads the run's `CheckedPackage`, the package `analyze` reads,
  and takes no `EmittedPackage`;
- QSL's FR-331 terminal record type (`TerminalRecord`, ADR-013 O-24), the
  one record type both paths write.

The entry and every type it takes are QSL's, in layer A `qsl-analyze`, so
no QSL crate names a driver type:

```rust
pub fn run(
    package: &CheckedPackage,
    binding: &SubjectBinding,
    items: &[RoutedItem],
    engines: &Engines,
    limits: &AnalyzeLimits,
    cancel: &Cancel,
) -> Vec<TerminalRecord>;

pub struct RoutedItem {
    pub item: ClaimItem,
    pub backend: BackendId,
}
```

`SubjectBinding`, `ClaimItem`, `Engines` and `AnalyzeLimits` are `analyze`'s
own types (FR-281); `backend` is the QSL engine `negotiate_*` settled the
item `supported` on and FR-057's routing routed it to. The driver passes
`Engines::qsl()`. The
driver's PV-3 `Provider::run` for a QSL engine adapts to this entry: it
converts its `ProviderRequest` items and the request's `deadline_ms` to
`RoutedItem`s and `AnalyzeLimits`, passes the run's `CheckedPackage` and
subject binding, drops `emitted`, which `prove` passes as `None` when
`package` refuses to emit and as `Some` otherwise, and returns the records
as its `ProviderResults`. The entry runs each item on its routed engine and
returns one record per item in `items` order; a routed backend that
`engines` does not hold settles the item `invalid-request` with
`invalid_capability`/`unknown-backend`, as `analyze` settles it (FR-281). A subject binding that FR-120
admission does not admit settles every routed item `declined` with the
admission failure's catalog code, since the entry returns records only:
cause `invalid-input` for `AdmissionFailure::Refused` and `incomplete-input`
for `AdmissionFailure::Incomplete`, ADR-013 O-16's refusal row. The entry takes the checked
package because a QSL engine runs in process
over the checked subject, and because some packages a QSL engine settles have
no emitted form: S4 refuses to emit a package that holds a random parameter,
a workload, a reward or a probabilistic claim (FR-185), while its checked
package remains. EN-4 decides a probabilistic claim over that checked subject
(FR-189), and a sampled witness replays over the same subject (FR-193). An
entry that read an `EmittedPackage` could serve no EN-4 item through `prove`,
since every package that holds a probabilistic claim is one S4 refuses.

## Inputs

For the provider entry: the run's `CheckedPackage` from `check` (FR-278),
the `SubjectBinding` (FR-120, FR-281), the `RoutedItem`s of the FR-331
provider request routed to QSL engines, the `AnalyzeLimits` holding the
request's `deadline_ms`, and `&Cancel`. The entry takes no `emitted` and reads no `package`,
`package_id`, `dependencies` or `provision` member: the checked package and
its resolved dependencies are already in process (QSpec FR-331, "In-process
providers").

## Outputs

`Vec<TerminalRecord>`: one terminal record per routed item, in `items`
order, which the driver's adapter writes as FR-331 results.

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
- EN-1's manifest shall list exactly one (kind, mode) pair,
  (`temporal-satisfaction`, `bounded`), the reachable-invariant, bounded,
  liveness and deadlock-freedom items its explicit-state search decides over
  a finite subject (FR-126); the `domains` `population` and `integer`, the
  boundable domains the subject's universes and declared ranges bound; the
  fairness kinds `weak` and `strong`, under which it decides (FR-126,
  FR-130); no evidence kinds, since it settles no
  `probabilistic-satisfaction` item; and no `dynamics` member, so it is a
  candidate for a temporal item that carries no hybrid-dynamics requirement
  and never for one that carries it (QSpec FR-193 "Candidates"). Its
  `BackendId` is its own and differs from every other engine's in
  `Engines::qsl()`.
- When the driver passes routed items of an FR-331 provider request to
  `qsl_analyze::provider::run`, the provider entry shall run each item on its
  routed engine through `analyze` over the `CheckedPackage` it is given and
  return one terminal record per routed item, in `items` order.
- The provider entry shall read the checked package and take no
  `EmittedPackage`, so it serves an item over a checked package that S4
  refuses to emit (FR-185) as it serves an item over one that S4 emits.
- If FR-120 admission does not admit the subject binding, then the provider
  entry shall settle every routed item `declined` with the admission
  failure's catalog code, cause `invalid-input` for a refusal and
  `incomplete-input` for an incomplete admission.
- When an EN-4 item settles `measured` through the provider entry, the
  provider entry shall keep in its record the sampled witnesses `analyze`
  keeps for the same item.
- The provider entry shall keep each sampled witness replayable through
  `replay_sampled_witness` over the checked subject it was given (FR-193).
- When FR-120 admission admits the subject binding, the terminal record the
  provider entry returns for a routed item shall equal the record `analyze`
  returns for the same item settled `Supported` on the same backend, with
  the same engines, checked package, subject binding and limits.
- If FR-120 admission does not admit the subject binding, then the
  equal-record rule shall not apply: the provider entry returns one
  `declined` record per item, while `analyze` returns
  `StageFailure::Refused` and no record.
- No QSL crate shall depend on a CG, RT or driver crate. An IR crate that
  ADR-011 §6.1 sanctions (`quire-contract-model`) is allowed.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-280-AC-1 | Each QSL engine's provider manifest is read by the FR-331 manifest reader and converted by FR-288's conversion into a `BackendDescriptor` whose advertised (kind, mode) pairs equal the claim kinds the engine settles in TC-762. | Test (TC-761) |
| FR-280-AC-2 | For each TC-762 item, a `RoutedItem` routing that item to its QSL engine, passed to `qsl_analyze::provider::run` with the `CheckedPackage`, `SubjectBinding` and `AnalyzeLimits` of the same run, returns a terminal record equal to the one `analyze` returns, over `Engines::qsl()`, for that item settled `Supported` on the same backend. | Test (TC-761) |
| FR-280-AC-3 | The `cargo tree` of every QSL crate lists no CG, RT or driver crate; an IR crate ADR-011 §6.1 sanctions (`quire-contract-model`) is allowed (FR-284's direction check). | Test (TC-767) |
| FR-280-AC-4 | EN-4's manifest is read by the FR-331 manifest reader and converted by FR-288's conversion into a `BackendDescriptor` that `descriptor_from_manifest` accepts, advertising exactly (`probabilistic-satisfaction`, `bounded`) with the evidence-kind set `{statistical}` and the `domains` `{population, integer}`. A registry holding it and a descriptor advertising the same pair with `{exact}` gives a `statistical` probabilistic item the EN-4 `BackendId` alone, and an `exact` item the other alone. | Test (TC-761) |
| FR-280-AC-5 | `Service` (FR-185-AC-7), whose `emit_checked` refuses `unsupported_construct`/`not-yet-implemented`, is passed to the provider entry as its `CheckedPackage`, with its subject binding and a `RoutedItem` routing the `2 ms` claim of ADR-024 §7.2 over `Service` (the claim of FR-193-AC-1 and FR-193-AC-5) to EN-4 with evidence kind `statistical`, the Okamoto method (FR-189-AC-1), seed 7 and `max_witnesses` 3. The entry returns one record, `measured` with decision `rejected`, equal to the record `analyze` returns for that item over the same checked package, binding and limits; its three sampled witnesses equal `analyze`'s and each replays `Reproduced` through `replay_sampled_witness` over `Service`'s checked subject. No `EmittedPackage` of `Service` exists in the run; `Service`'s `emit_checked` is refused before and after the entry runs. | Test (TC-761) |
| FR-280-AC-6 | AC-2's request with its items in reverse order returns their records in that reverse order, each equal to AC-2's record for the same item. The same items with a binding whose `universes` name a population the package does not declare return one `declined` record per item, cause `invalid-input`, code `invalid_runtime_input`/`wrong-role-mapping`; with a binding whose `initial` digest is absent from `snapshots`, one `declined` record per item, cause `incomplete-input`, code `unavailable_observation`/`missing-required-artifact`; in both, no engine runs. `qsl_analyze::provider::run`'s signature names no type from a driver, CG or RT crate (FR-280-AC-3's `cargo tree` check holds with the entry built). | Test (TC-761) |
| FR-280-AC-7 | EN-1's manifest is read by the FR-331 manifest reader and converted by FR-288's conversion into a `BackendDescriptor` that `descriptor_from_manifest` accepts, advertising exactly (`temporal-satisfaction`, `bounded`) with the `domains` `{population, integer}`, the fairness kinds `{weak, strong}`, no evidence kinds and no `dynamics`, under a `BackendId` that differs from EN-4's. A registry holding it and a descriptor advertising `temporal-satisfaction` with `dynamics` `["hybrid"]` gives a temporal item that carries no `requirements` the EN-1 `BackendId` alone, and an item carrying `"requirements": ["hybrid-dynamics"]` the other alone; neither settles `ambiguous-backend`. | Test (TC-761) |

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
`prove`, the `Provider` trait (ADR-029 PV-3) and its adapter over
`qsl_analyze::provider::run`,
including how `prove` hands this entry the checked package when `package`
refuses to emit.
CG owns the process-provider negotiation arm (ADR-029 PV-4).

## References

- QSL-393 (V1-A06): typed library APIs.
- QSL-390 (ARCH-50).
- QSpec FR-300 (STD-141): the QSpec half.
