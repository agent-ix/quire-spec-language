---
id: FR-050
title: "Publish authenticated temporal selections in compiled protocol v2"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-002, type: implements }
  - { target: ix://agent-ix/quire-spec-language/US-004, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-043, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-044, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-045, type: references }
  - { target: ix://agent-ix/quire-specification/FR-090, type: depends_on }
  - { target: ix://agent-ix/quire-protocol/IT-001, type: references }
---
# FR-050: Publish authenticated temporal selections in compiled protocol v2

## Description

When a compiled native package contains a selected temporal declaration, the
compiler SHALL emit and admit that declaration's exact definition artifact and
clock configuration through `quire.compiled-protocol/2`.

This is the narrow temporal producer extension accepted on compiler
[#40](https://github.com/agent-ix/quire-spec-language/issues/40) after merged L5
revision `72507f856457ba0922719bd5d9f5cadcce4058cd`. It extends the reviewed
[FR-042](FR-042-publish-compiled-protocol-artifacts.md) contract without changing
the bytes, meaning or strict reader for `quire.compiled-protocol/1`.

## Inputs

The constructor-private native family admission used by FR-042; exact selected
temporal definitions and original dependency bytes; a declaration-to-definition
selection for every temporal declaration; one explicitly selected immutable
clock configuration per selection; the independent expected artifact/contract/baseline/producer/source/
dependency/model selections used by the reader; and FR-042's caller-lowered
construction/admission limits.

An L5 trace, observed timestamp, runtime watermark, installed default clock,
display name or inferred unit is not a compiler clock configuration.

## Outputs

Canonical `quire.compiled-protocol/2` bytes and their complete raw-byte digest,
or FR-042's typed invalid, unsupported or resource-incomplete disposition with
no partial package. The caller remains responsible for placing that digest in
an independently authorized external `ArtifactRef`; the compiler does not mint
its authority, identity or revision. A successful strict `/2` read returns a constructor-private
`protocol_artifact::v2::AdmittedPackage`; it does not authenticate `/1` bytes or
produce a temporal/protocol assessment result.

## Interface and wire model

The public Rust consumer entry point SHALL be
`protocol_artifact::v2::read(bytes, expected, limits) -> Report<AdmittedPackage>`.

The public Rust producer entry point SHALL be
`protocol_artifact::native::admit_v2(proofs, selected, temporal, limits) -> Report<AdmissionV2>`.

When a Rust consumer locates the committed `/2` handoff, the compiler crate
SHALL publish `protocol_artifact::handoff::PUBLISHED_HANDOFF` and the exact
member-name constants `PUBLISHED_OFFER_FILE`,
`PUBLISHED_ARTIFACT_REFERENCE_FILE`, `PUBLISHED_SELECTION_FILE`,
`PUBLISHED_MUTATION_MANIFEST_FILE`, `PUBLISHED_CHECKSUMS_FILE` and
`MUTATION_MANIFEST_FORMAT`. Their values SHALL respectively name
`compiled-protocol-v2.json`, `compiled-protocol-v2.ref.json`,
`expected-v2.json`, `mutations/manifest.json`, `SHA256SUMS` and
`quire.protocol.v2-mutations/1`. A consumer SHALL NOT require an environment
variable or redeclare those producer-owned values to address the handoff.

The producer's `temporal` input SHALL be a complete table of
`native::TemporalSelection` records keyed by exact source `ArtifactRef` and
declaration byte span, with expected definition identity/revision/artifact and
one typed clock configuration.

`AdmissionV2` SHALL expose immutable canonical bytes, digest and the strict `/2`
admitted view while retaining the same constructor-private native authority as
FR-042's producer. Its enclosing `Report<AdmissionV2>` SHALL expose effective
limits and usage exactly once, following the existing FR-042 report pattern.

The public L5 entry points SHALL be
`temporal::evaluate_v2(&protocol_artifact::v2::AdmittedPackage, declaration, trace, limits)`,
`temporal::evaluate_with_progress_v2(..., ledger)` and
`temporal::mapping_support_v2(&protocol_artifact::v2::AdmittedPackage, declaration, closure)`,
with the same result types as their existing strict `/1` counterparts.

The existing `temporal::evaluate`, `evaluate_with_progress` and
`mapping_support` entry points SHALL retain their `/1` signatures and
unauthenticated-premise behavior for compatibility.

The strict reader's `v2::Expected` SHALL contain the complete inherited
FR-042 `Expected` selections plus an independently supplied complete table of
`v2::ExpectedTemporal` records. Each expected temporal record selects the exact
source artifact and `ExpectedDeclaration`, definition identity/revision/artifact
and typed clock configuration; none is obtained from offered `/2` bytes.

The `/2` package SHALL retain every `/1` member in the same order and meaning,
change only the versioned wire/media/schema selections, and append one required
`temporal_bindings` member after `declarations`.

Each `TemporalBinding` SHALL contain the declaration index, its definition index
and exactly one closed `ClockConfiguration` alternative.

The clock alternatives SHALL be exactly:

| Profile identity | Canonical configuration |
| --- | --- |
| `quire.temporal.event-position.false-extension/v1` | `{kind:"event_position",sequence_authority:Name}` |
| `quire.temporal.fixed-sample.false-extension/v1` | `{kind:"fixed_sample",epoch:Number,period:Number,unit:Name}` |
| `quire.temporal.timestamped-event.finite-window/v1` | `{kind:"timestamped_event",timestamp_unit:Name}` |

The binding's definition index SHALL equal the selected declaration's existing
`profile` index.

The selected `Definition` SHALL supply the exact identity and revision, and its
`artifact` index SHALL select one exact dependency `ArtifactRef` whose digest
matches the independently supplied original definition bytes.

The fixed-sample period SHALL be a positive reduced exact number representable
by FR-038 without floating conversion, sign repair or zero substitution.

The epoch SHALL be an exact FR-038 number in the same declared unit domain as
the period.

Every authority and unit name SHALL be nonempty and within FR-042's UTF-8 byte
limit without trimming, normalization or alias resolution.

For the v2-specific L5 entry points, `Trace.clock.parameters` SHALL contain
exactly the keys selected by the admitted clock alternative: only
`sequence_authority`; exactly `epoch`, `period` and `unit`; or only
`timestamp_unit`, respectively.

The trace's `epoch` and `period` strings SHALL be the compact canonical JSON
encoding of the corresponding FR-038 `Number` object; the authority and unit
strings SHALL equal the admitted names byte-for-byte.

## Behavior

The native `/2` producer SHALL emit one binding for every and only temporal
declaration in canonical declaration-index order.

The native `/2` producer SHALL select the clock alternative from the exact
definition identity rather than from supplied configuration shape.

The native `/2` producer SHALL match each temporal selection to the exact
source/declaration locus discharged by the supplied proof report and to its
registered definition before copying the selected configuration.

If a temporal definition is unknown, missing its exact dependency bytes or
paired with a missing, surplus, duplicate or wrong-profile configuration, then
the producer SHALL return the corresponding typed unsupported or invalid
disposition before emitting bytes.

The strict `/2` reader SHALL reject a missing, surplus, duplicate, out-of-order,
foreign-owner or out-of-range temporal binding.

The strict `/2` reader SHALL match every binding and clock field against one
independently supplied `ExpectedTemporal` record before granting admission.

The strict `/2` reader SHALL reject a binding whose definition index differs
from its declaration profile or whose configuration alternative differs from
that exact registered profile.

The strict `/2` reader SHALL recompute the selected definition dependency digest
from the independently supplied original bytes rather than trust the offered
artifact reference alone.

The strict `/2` reader SHALL apply FR-042's canonical byte comparison, closed
shape validation, complete inventory checks and charge-before-work limits to the
new records without changing their existing accounting identity.

The existing `protocol_artifact::read` SHALL remain a strict `/1` reader that
rejects `/2` wire/media/schema selections and the added member.

The `/2` reader SHALL reject `/1` rather than default temporal bindings or infer
them from declaration bodies, traces or the environment.

Each v2-specific trace-evaluation entry point SHALL compare a trace's profile identity,
revision, clock binding name and exact parameter map to the admitted `/2`
selection before visiting positions; a mismatch is a typed binding refusal and
no longer an unauthenticated retained premise.

`mapping_support_v2` takes no runtime trace. It SHALL require the declaration's
admitted binding to select the same definition/profile before classifying the
requested surrounding-execution closure; it SHALL NOT claim to authenticate a
parameter map that is not an input to mapping classification.

The progress ledger identity used by `evaluate_with_progress_v2` SHALL include
the admitted package digest, declaration, definition identity/revision and exact
clock configuration. Progress recorded for `/1`, another `/2` artifact or a
different authenticated clock configuration SHALL NOT settle this evaluation.

Changing a selected definition artifact, sequence authority, epoch, period,
unit or timestamp unit SHALL change canonical `/2` bytes and the external
artifact digest without changing the admitted native source.

The compiler SHALL NOT interpret clock progress, settle temporal truth, map a TL
formula, create observation records or emit a protocol result through this
extension.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-050-AC-1 | A package with one declaration under each registered profile and an independently selected complete temporal table emits exactly three canonical declaration-indexed bindings containing the exact definition identity/revision/artifact digest and the matching event-position, fixed-sample or timestamped configuration. | Test (TC-138) |
| FR-050-AC-2 | Missing, surplus, duplicate, reordered, foreign-owner and out-of-range bindings refuse independently; a definition/profile mismatch and every wrong clock alternative refuse without defaulting or shape-based selection. | Test (TC-138) |
| FR-050-AC-3 | Empty/oversized names, zero/negative/unreduced/overflowing periods, substituted epoch/unit/authority, and substituted definition identity/revision/artifact/bytes/digest produce their exact typed causes without a partial package. | Test (TC-138) |
| FR-050-AC-4 | `/1` bytes are readable only by the strict `/1` reader; `/1` rejects `/2`, `/2` rejects `/1`, and neither reader upgrades, strips or invents a temporal selection. | Test (TC-138) |
| FR-050-AC-5 | Exact and one-short FR-042 limits cover added binding entries, strings, numbers and canonical bytes with the existing accounting identity; a sufficient retry starts fresh and reproduces the same bytes. | Test (TC-138) |
| FR-050-AC-6 | The v2-specific L5 evaluation entry points compare all authenticated clock fields before positions and retain them in results, while the existing `/1` entry points keep their signatures and historical unauthenticated scope and cannot establish native-to-temporal correspondence. | Test (TC-138) |
| FR-050-AC-7 | The public handoff path, five member filenames and mutation format constants resolve the committed `/2` inventory and decoded manifest without an environment variable or consumer-owned duplicate vocabulary. | Test (TC-138) |

## Dependencies

[FR-042](FR-042-publish-compiled-protocol-artifacts.md) owns the unchanged `/1`
schema, canonical encoding, selection model, limits and producer authority. This
requirement owns only the parallel strict `/2` package/view and its temporal
binding table. [FR-043](FR-043-evaluate-bounded-native-temporal.md),
[FR-044](FR-044-activate-temporal-obligations.md) and
[FR-045](FR-045-classify-temporal-mapping-support.md) remain the L5 evaluator,
activation and mapping interfaces. Standard FR-090 owns temporal profile and
clock meaning. E owns settlement; F owns concrete observations and progress;
B owns protocol conformance/results.
