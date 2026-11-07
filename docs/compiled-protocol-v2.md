# Compiled protocol package wire v2

Normative data contract owned by
[FR-050](../spec/functional/FR-050-publish-authenticated-temporal-artifacts.md).
Version 2 authenticates each temporal declaration's definition artifact and
clock configuration. It embeds the version-1 package, including its shared
`Model` records, and changes no temporal meaning.

## Version selection

| Identity | Exact value |
| --- | --- |
| Payload wire | `quire.compiled-protocol/2` |
| Media type | `application/vnd.quire.compiled-protocol+json;version=2` |
| Schema/type | `quire.compiled-protocol.schema/2` / `CompiledProtocolPackage` |
| Byte encoding | `quire.protocol.compact-json/1` |
| Native numbers | `quire.protocol.numeric/1` |

The external `ix.artifact-ref/3-draft` has `kind:"linked-package"` and
`wire:{identity:"quire.compiled-protocol",version:"2"}`. The reference carries
no content digest. Version-1 references and payloads retain version `1` and their
original bytes.

All encoding, number, name, canonical ordering, selection, model, producer,
source, dependency, declaration and limit rules from
[version 1](compiled-protocol-v1.md) apply unchanged. The two versions have
separate strict Rust readers. Neither reader adds, removes or translates fields.
The inherited shared-reference vocabulary keeps concrete `population` and
`window` assessment documents distinct from one another and from the compiled
artifact. A temporal binding's authenticated clock configuration selects how a
window is interpreted; it is not itself the window identity, coverage or
membership document.

## Closed version-2 delta

Version 2 appends the required `temporal_bindings` member after `declarations`:

```text
ClockConfiguration =
   {kind:"event_position",sequence_authority:Name}
 | {kind:"fixed_sample",epoch:Number,period:Number,unit:Name}
 | {kind:"timestamped_event",timestamp_unit:Name}

TemporalBinding = {declaration:U,definition:U,clock:ClockConfiguration,clock_name:Name}

CompiledProtocolPackageV2 = {
 wire:"quire.compiled-protocol/2",media:"application/vnd.quire.compiled-protocol+json;version=2",
 schema:"quire.compiled-protocol.schema/2",type:"CompiledProtocolPackage",
 encoding:"quire.protocol.compact-json/1",numeric:"quire.protocol.numeric/1",
 contract:Ref,producer:Producer,baseline:Ref,language:Language,
 package_definition:U,features:Features,sources:[Source],dependencies:[Dependency],
 definitions:[Definition],models:[Model],types:[Type],declarations:[Declaration],
 temporal_bindings:[TemporalBinding]
}
```

`U`, `Name`, `Number`, `Definition`, `Dependency` and every inherited record have
their exact version-1 definitions. A fixed-sample `period` is positive and
reduced; `epoch` and `period` are exact numbers in the declared `unit`, with no
binary-float conversion. The other alternatives contain no extra field.

There is exactly one binding for every temporal declaration and none for other
declaration families. Bindings sort by `declaration` and cannot repeat.
`definition` equals that declaration's `profile` index. The selected definition
identity/revision is the indexed `Definition`; its `artifact` selects the exact
dependency `ArtifactRef`. The clock tag is determined by the selected exact
profile identity:

| Definition identity | Required clock tag and fields |
| --- | --- |
| `quire.temporal.event-position.false-extension/v1` | `event_position`: `sequence_authority` |
| `quire.temporal.fixed-sample.false-extension/v1` | `fixed_sample`: `epoch`, `period`, `unit` |
| `quire.temporal.timestamped-event.finite-window/v1` | `timestamped_event`: `timestamp_unit` |

`clock_name` is the declaration's clock name, typed (ADR-012 section 9): the
same name the inherited version-1 binding carries with a `clock:` prefix in its
own `name` member, without that prefix. A reader takes `clock_name` directly;
it never re-derives a clock name by parsing the inherited binding's name.
Admission refuses a `clock_name` that disagrees with the inherited version-1
binding's `clock:`-prefixed spelling.

Missing, surplus, duplicate, out-of-order, foreign-owner or out-of-range
bindings refuse. Definition/profile disagreement, wrong clock tag, invalid exact
number, empty/oversized name, a `clock_name` disagreeing with the inherited
binding's `clock:` spelling, and unavailable original definition bytes also
refuse. Unknown selected profiles remain unsupported. No trace, environment or
installed default supplies a missing field.

## Rust boundary

The strict consumer entry point is:

```rust,ignore
protocol_artifact::v2::read(bytes, expected, limits) -> Report<AdmittedPackage>
```

`v2::Expected` contains the complete inherited version-1 `Expected` value and a
complete independently supplied `ExpectedTemporal` table. An expected temporal
record selects one exact source artifact plus `ExpectedDeclaration`, definition
identity/revision/artifact and typed clock configuration. Admission never builds
that table from the offer.

Version-2 selection failures use `Error::V2(v2::Refusal)`. `Refusal` carries
public typed `HeaderField`, `BindingCause`/`InventorySide`, `BindingIndex`,
`SelectionSide`, `DeclarationField`, `DefinitionField`/`ArtifactField`, and `ClockField`
discriminants. `Refusal::code()` returns the stable machine code (for example
`v2.definition.artifact.identity` or `v2.clock.sequence-authority`); consumers do
not parse diagnostics to distinguish axes. Closed-JSON shape errors such as a
missing or renamed required clock member remain `Error::Json`, while valid
objects with a wrong alternative or value receive the exact v2 selection code.

`Error::code()` extends that guarantee to every other axis, so a consumer never
matches on a variant to tell two refusals apart. It returns `json` for a
closed-JSON shape error and dotted codes under
`allocation`, `numeric.`, `producer.`, `invalid.`, `unsupported.` and
`incomplete.` for the remaining variants; a version-2 refusal delegates to
`Refusal::code()` and keeps its established `v2.` spelling. Codes are
append-only and are never message-derived.

The constructor-private compiler entry point is:

```rust,ignore
protocol_artifact::native::admit_v2(proofs, selected, temporal, limits)
    -> Report<AdmissionV2>
```

The selected `temporal` input is a complete `native::TemporalSelection` table
keyed by exact source artifact and declaration byte span. Each record includes
the expected definition identity/revision/artifact and one typed clock
configuration. The producer matches those selections to the proof report rather
than accepting a same-named declaration or shape. `AdmissionV2` exposes
canonical bytes and a strict admitted view, never a freely
constructible external artifact authority token. Its enclosing `Report` owns
usage and effective limits. L3/L4 add no native `state` evaluator: the artifact-graph `state::evaluate_v2`
path and its `state::Refusal::UnpublishedArtifact` were retired under FR-049
(ADR-011 §7.3 M-6c, FR-405 item 1). A strictly read v2 package is evaluated
through the checked-term entry points of `qsl-eval`
(`CheckedPackageEvaluation::{call, evaluate, evaluate_protocol_slot}` and the
checked-family functions), with the package identity established by its
compile/read path; no caller-supplied artifact identity can be substituted. L5 adds `temporal::evaluate_v2`,
`evaluate_with_progress_v2` and `mapping_support_v2` over the v2 admitted type;
the existing `/1` signatures remain unchanged. The v2 trace-evaluation entry
points compare the trace's profile, revision,
binding name and exact parameter map before visiting positions. Event-position
uses only `sequence_authority`; fixed-sample uses exactly `epoch`, `period` and
`unit`, with the numbers encoded as compact canonical FR-038 Number JSON;
timestamped-event uses only `timestamp_unit`. L5 does not interpret clock
progress during artifact admission.

`mapping_support_v2(package, declaration, closure)` has no trace input; it
checks the declaration's authenticated definition/profile binding before using
the existing surrounding-execution-closure classifier and makes no parameter-map
claim. Its `Classification::authenticated()` accessor returns the
`AuthenticatedSelection` containing the selected package reference, declaration,
definition identity/revision and definition artifact reference. Version-1 classification
returns `None`; neither path authenticates clock parameters without a trace.
`evaluate_with_progress_v2` keys retained progress by the admitted package
reference, declaration, definition identity/revision and exact clock configuration,
so another artifact, `/1` evaluation or changed clock selection cannot lend it
progress.

## Reproducible producer-to-consumer handoff

A consumer handoff names the final published compiler implementation
that retains this contract and names the exact public surface it consumes:

- `protocol_artifact::v2::{Expected, ExpectedDefinition, ExpectedTemporal,
  AdmittedPackage, read, encode_candidate}` and
  `protocol_artifact::v2::wire::{Package, TemporalBinding,
  ClockConfiguration}`;
- `protocol_artifact::native::{TemporalSelection, AdmissionV2, admit_v2}`;
- the `qsl-eval` checked-term entry points
  `CheckedPackageEvaluation::{call, evaluate, evaluate_protocol_slot}`; and
- `temporal::{AuthenticatedSelection, Classification, evaluate_v2,
  evaluate_with_progress_v2, mapping_support_v2}`.

The producing caller supplies the actual parser/linker/proof report plus the
complete `native::Selections`: original native source and model artifacts,
unreconstructed dependency bytes and their exact artifact identities, and a
complete temporal selection table. `admit_v2` returns a
`Report<AdmissionV2>`. The report, not `AdmissionV2`, owns the effective limits,
usage and typed refusal. On success `AdmissionV2::bytes()` exposes the raw
canonical `/2` payload, while `admitted()` exposes
only the constructor-private local view.

The external linked-package `ArtifactRef` is a separate consumer-authorized
identity selection. Its version is `2`; A does not mint consumer authority by
returning that reference. Before
reading, the caller independently constructs `v2::Expected` from its selected
artifact, contract, baseline, producer, source, dependency and model inventories
plus its own complete `ExpectedTemporal` table. It then calls `v2::read` with
caller-selected limits and retains the resulting typed `Report`, including a
resource-incomplete or invalid/unsupported refusal without a partial admitted
package.

The executable recipe is
[`tests/compiled_protocol_v2.rs`](../tests/it/compiled_protocol_v2.rs), supported by
[`tests/support/native_protocol/mod.rs`](../tests/support/native_protocol/mod.rs).
It starts with real native compilation, reconstructs reader expectations from
the original inputs rather than emitted wire, and covers cross-version offers,
selection/configuration substitutions,
same-meaning recanonicalized producer input, exact limits, and the L5 adapters.
These fixtures demonstrate A's producer and reader boundary only; they do not
claim that B admitted the artifact or completed an ecosystem acceptance run.

### Checked-package handoff directory

The directory a downstream crate reads is written by the checked-handoff route,
not by a `/2` release producer. `checked_handoff::write` (feature
`checked-handoff`) compiles a complete-V1 unit in process through
`emit_checked_v2`, reads the emitted package back through Contract IR's reader
and writes a new directory. The legacy handoff writer, its
`handoff-writer` feature, its two examples and the `examples/protocol-handoff/`
recipe inputs are retired (FR-517-AC-10).

The directory contains:

- `checked-package.json`: the emitted checked package;
- `selection.json`: the selection sidecar (`checked_handoff::Selection`),
  derived from the in-process checked unit and the request, never from the
  emitted bytes;
- `inventory.json`: the inventory of the unit;
- the exact input bytes and the model artifacts the sidecar names; and
- `mutations/manifest.json` and `mutations/*.json`, when the request asks for
  the adverse corpus: the base package, the sidecar and every mutation with its
  expected refusal. Every verdict on a package comes from
  `read_checked_package` and `Selection::check`.

The repository commits no instance of this directory. A consumer's build or
test calls `checked_handoff::write` to generate it into a directory the
consumer chooses, then parses the package and compares it with the sidecar
through `Selection::check` against the borrowed
`quire_contract_model::CheckedPackageV2` surface, with no copy of the format.

The executable recipe is the `checked-handoff` integration suite
[`tests/it/checked_handoff.rs`](../tests/it/checked_handoff.rs). The mutation
controls for the `/2` reader remain in
[`tests/compiled_protocol_v2.rs`](../tests/it/compiled_protocol_v2.rs), including
cross-version, every temporal selection family, artifact-reference and
same-meaning recanonicalized-input substitutions. These controls qualify A's
handoff only and do not assert B or campaign acceptance.
