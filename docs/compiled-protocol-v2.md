# Compiled protocol package wire v2

Normative data contract owned by
[FR-050](../spec/functional/FR-050-publish-authenticated-temporal-artifacts.md).
Version 2 authenticates each temporal declaration's definition artifact and
clock configuration. It does not change any version-1 field or temporal meaning.

## Version selection

| Identity | Exact value |
| --- | --- |
| Payload wire | `quire.compiled-protocol/2` |
| Media type | `application/vnd.quire.compiled-protocol+json;version=2` |
| Schema/type | `quire.compiled-protocol.schema/2` / `CompiledProtocolPackage` |
| Byte encoding | `quire.protocol.compact-json/1` |
| Native numbers | `quire.protocol.numeric/1` |

The external `ix.artifact-ref/3-draft` has `kind:"linked-package"` and
`wire:{identity:"quire.compiled-protocol",version:"2"}`. Its digest is SHA-256
over the complete canonical version-2 payload. Version-1 references and payloads
retain version `1` and their original bytes.

All encoding, number, name, canonical ordering, selection, model, producer,
source, dependency, declaration and limit rules from
[version 1](compiled-protocol-v1.md) apply unchanged. The two versions have
separate strict Rust readers. Neither reader adds, removes or translates fields.

## Closed version-2 delta

Version 2 appends the required `temporal_bindings` member after `declarations`:

```text
ClockConfiguration =
   {kind:"event_position",sequence_authority:Name}
 | {kind:"fixed_sample",epoch:Number,period:Number,unit:Name}
 | {kind:"timestamped_event",timestamp_unit:Name}

TemporalBinding = {declaration:U,definition:U,clock:ClockConfiguration}

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
dependency `ArtifactRef`, and admission recomputes that reference's digest from
the independently supplied original bytes. The clock tag is determined by the
selected exact profile identity:

| Definition identity | Required clock tag and fields |
| --- | --- |
| `quire.temporal.event-position.false-extension/v1` | `event_position`: `sequence_authority` |
| `quire.temporal.fixed-sample.false-extension/v1` | `fixed_sample`: `epoch`, `period`, `unit` |
| `quire.temporal.timestamped-event.finite-window/v1` | `timestamped_event`: `timestamp_unit` |

Missing, surplus, duplicate, out-of-order, foreign-owner or out-of-range
bindings refuse. Definition/profile disagreement, wrong clock tag, invalid exact
number, empty/oversized name and unavailable original definition bytes also
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

The constructor-private compiler entry point is:

```rust,ignore
protocol_artifact::native::admit_v2(proofs, selected, temporal, limits)
    -> Report<AdmissionV2>
```

The selected `temporal` input is a complete `native::TemporalSelection` table
keyed by exact source artifact and declaration byte span. Each record includes
the expected definition identity/revision/artifact and one typed clock
configuration. The producer matches those selections to the proof report rather
than accepting a same-named declaration or shape. `AdmissionV2`
exposes canonical bytes and a strict admitted view, never a freely constructible
authority token. L5 adds `evaluate_v2`, `evaluate_with_progress_v2` and
`mapping_support_v2` over the v2 admitted type; the existing `/1` signatures
remain unchanged. The v2 entry points compare the trace's profile, revision,
binding name and exact parameter map before visiting positions. Event-position
uses only `sequence_authority`; fixed-sample uses exactly `epoch`, `period` and
`unit`, with the numbers encoded as compact canonical FR-038 Number JSON;
timestamped-event uses only `timestamp_unit`. L5 does not interpret clock
progress during artifact admission.
