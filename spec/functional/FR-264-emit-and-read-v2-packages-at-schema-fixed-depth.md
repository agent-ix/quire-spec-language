---
id: FR-264
title: "Emit and read v2 checked packages whose depth the schema fixes"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-027
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-258
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-264: Emit and read v2 checked packages whose depth the schema fixes

## Description

QSpec FR-322 defines a v2 node `body` by a stratified grammar in which no
production names itself or a higher stratum, so the JSON depth of every v2
package is fixed by the schema, whatever the model's size (ADR-030 D-2).
The S4 emitter and QSL's I2 read SHALL keep to that grammar: the emitter
writes every package in it, and the read bounds v2 packages by size and work
limits only (ADR-030 D-4.9).

## Behavior

1. **Emission.** The S4 v2 emitter SHALL write every node body in QSpec
   FR-322's stratified body grammar, with each composite subterm as its own
   node reached by `reference` (FR-258).
2. **Read limits.** QSL's I2 read SHALL bound a v2 artifact by `i2.input_bytes`,
   `i2.nodes`, `i2.edges`, `i2.occurrences`, `i2.diagnostics`, `i2.work_units` and
   `i2.native_join_storage`, passed to IR's checked-package reader, with published
   defaults of 16777216 bytes, 10000 nodes, 100000 edges, 100000
   occurrences, 10000 diagnostics, 1000000 work units and 16777216 logical storage units (the last supplied by the outer configuration, FR-255), each with no
   ceiling, and no depth limit
   or setting. When IR's reader reports one of those limits, QSL SHALL return `StageFailure::Limit` with
   its kind, bound, count reached, setting (FR-255) and the locus FR-096
   defines.
3. **Malformed wire.** When IR's reader refuses a node body outside FR-322's
   stratified grammar, QSL's I2 read SHALL return the refusal with code
   `malformed_wire` (QSpec FR-322-AC-40).
4. **Fixed-depth decoded nodes.** The decoded nodes of a `V2Read` SHALL be
   fixed-depth types, so that cloning, comparing, formatting for debug and
   dropping a `V2Read` visit its nodes without native recursion.
5. **Emission output ceiling.** The S4 v2 emitter SHALL bound the canonical
   bytes it writes for one package by `emit.output_bytes` (FR-255), a ceiling
   the caller owns: every emit entry takes it as an argument, and no emit path
   below the outer configuration is unbounded. The bound is inclusive: a
   package whose canonical bytes number exactly the bound is emitted, and the
   emitter writes no bytes for one byte more. A package the emitter returns
   from its remembered emission counts the same bytes against the bound
   supplied by the call. When the bound is reached the emitter SHALL return
   `EmitRefusal::OutputLimit`, whose catalog code is `stage_limit_exceeded`
   with cause `input-bytes-exceeded`, carrying kind `input bytes`, the
   configured bound, the byte count the refused write would have reached, and
   setting `emit.output_bytes`; the compile lifecycle returns it as its
   `Limit` refusal. The published default is 16777216 bytes, supplied by the
   outer configuration (FR-255).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-264-AC-1 | On a thread with a 512 KiB stack, a package holding a 100,000-term sum, compiled under limits raised to fit it, is emitted, read back through QSL's I2 read with `i2.*` raised to fit it, and verified at its emitted `package_id`. The `V2Read` clones, compares equal to its clone, formats for debug and drops on the same thread. | Test (TC-738) |
| FR-264-AC-2 | Every node body of every package the emitter writes for the TC-415 corpus and for AC-1 is in FR-322's stratified grammar, checked by a walk of the emitted JSON that classifies each body position as Leaf, Group, Tuple, Member or Body. | Test (TC-738) |
| FR-264-AC-3 | A v2 artifact identical to an emitted one except that one `application` argument is an inline `application` term is refused by QSL's I2 read with code `malformed_wire`, with no limit outcome. | Test (TC-739) |
| FR-264-AC-4 | A v2 artifact with more nodes than `i2.nodes` at bound `B` returns `StageFailure::Limit` with kind node count, bound `B`, IR's consumed count and setting `i2.nodes`, with `Locus::Artifact`; read again with `i2.nodes` raised through the v2 read limits' builder and through FR-255's settings operation given `i2.nodes=<n>` (FR-255), which the driver CLI exposes as `--limit` (ADR-029 CB-1), it verifies. | Test (TC-739) |
| FR-264-AC-5 | On a thread with a 512 KiB stack and the default limits, a v2 artifact whose JSON is 100,000 arrays deep is refused by QSL's I2 read with code `malformed_wire`, with no limit outcome and no outcome naming a depth. | Test (TC-739) |

| FR-264-AC-6 | A package whose canonical emitted bytes number `N` is emitted at `emit.output_bytes` `N`, with bytes equal to the bytes emitted under a larger bound. At bound `N - 1` it returns `EmitRefusal::OutputLimit` with code `stage_limit_exceeded`, cause `input-bytes-exceeded`, kind input bytes, bound `N - 1`, an actual count above `N - 1` and setting `emit.output_bytes`, and writes no bytes. At bound 0 it returns the same refusal with bound 0. | Test (TC-738) |
| FR-264-AC-7 | A package emitted once and then offered again to an emit entry at a bound below its remembered byte length returns the same `OutputLimit` refusal with actual `N`; at bound `N` it returns the remembered emission. The compile lifecycle given `emit.output_bytes` `N - 1` returns `CompileRefusal::Limit` naming `emit.output_bytes`, and given `N` emits. | Test (TC-738) |

## Dependencies

- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  D-2, D-4.9 and D-8 overlap item O-1.
- [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md) and
  [FR-258](FR-258-check-and-lower-expressions-at-any-depth.md) define the
  terms lowering writes.
- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)
  defines the I2 reader's limit loci.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md) names the
  `i2.*` and `emit.output_bytes` settings.

## Overlap

Enforcing FR-322's stratified grammar in IR's checked-package reader, and
removing that reader's depth limit and recursive decode, are IR's work
(ADR-030 D-8 O-1, Linear IR-495), which IR has merged. QSL's read has no
depth member, no depth limit kind and no clamp.

## References

- QSpec FR-322 (checked-package artifact and its v2 body grammar).
- QSpec FR-322 AC-39 to AC-42, the stratified v2 body grammar (Linear
  STD-143, which supersedes STD-125).
- Linear IR-495 (IR's reader work for overlap item O-1).
- Linear QSL-381.
