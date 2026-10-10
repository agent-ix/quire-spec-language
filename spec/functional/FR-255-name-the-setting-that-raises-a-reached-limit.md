---
id: FR-255
title: "Name the setting that raises a reached limit, at every entry point"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-027
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-146
    type: depends_on
---
# FR-255: Name the setting that raises a reached limit, at every entry point

## Description

A limit outcome SHALL tell the caller everything needed to run the input
again under a larger limit (ADR-030 D-3). Every configurable resource limit
of QSL's front end and engines has one stable setting name. The same name
raises the limit from the library, from a replay request (every setting but
`replay.input_bytes`) and from the driver CLI (`quire`, ADR-029 CB-1), which
exposes the caller limits through QSL's settings operation, wherever that
entry point takes the setting. Every `stage_limit_exceeded` outcome names the
limit, its value and its setting. An execution budget set in a request's
accounting limits (below) has no setting name here.

The ecosystem rule that depth is never a limit kind, and that a reached
limit names how to raise it, is QSpec's (ADR-030 D-1; QSpec FR-460 and
FR-461). This requirement specifies how QSL's
compiler, readers and engines carry it.

## Setting names

A setting name follows QSpec FR-461's naming rule. The table is the set of
names QSL accepts at every entry point, and lists every limit of the
requirements named below. Each row's default is the published default a
caller gets when it does not configure the limit (ADR-014 §2, "inherited").
Every limit has no ceiling: a caller raises or lowers it, and the value is
used as given. The requirement that owns each limit defines its counter:
NFR-001 (`s1.*`), NFR-011 (`s3.*`), FR-082 (`environment.*`), FR-260 (`intake.*`),
NFR-012 (`model.*`, `admission.*`), FR-106 (`observation.*`), FR-111
(`library.*`), FR-099 (`dependency.*`), FR-098 (`replay.*`), FR-264 (`i2.*`),
FR-003 (`format.*`) and FR-101 (`explore.*`). A limit of an operation QSL has
not yet ported takes its row, with its limits type, when that operation lands.

| Setting | Stage | Limit kind | Library field | Default |
| --- | --- | --- | --- | --- |
| `s1.input_bytes` | S1 | input bytes | `qsl_cst::Limits` source bytes | 1048576 bytes |
| `s1.tokens` | S1 | token count | `qsl_cst::Limits` tokens | 100000 tokens |
| `s1.nodes` | S1 | node count | `qsl_cst::Limits` nodes | 50000 nodes |
| `s1.work_units` | S1 | work budget | `qsl_cst::Limits` parser work | 256 units per significant token, plus 256 |
| `s3.nodes` | S3 | node count | `CheckingLimits` nodes | 100000 units |
| `s3.input_bytes` | S3 | input bytes | `CheckingLimits` declaration preimage bytes | 16777216 bytes |
| `s3.work_units` | S3 | work budget | `CheckingLimits` work budget | 16777216 units |
| `s3.decimal_scale` | S3 assembly | work budget | `AssemblyLimits` decimal scale | 4096 digits |
| `environment.ancestor_steps` | S3 type-environment admission | edge count | `TypeEnvironmentLimits` ancestor_steps | 16777216 edges |
| `environment.work_units` | S3 type-environment admission | work budget | `TypeEnvironmentLimits` work_units | 16777216 units |
| `intake.input_bytes` | I1 semantic-IR intake | input bytes | the intake limits' document bytes | 67108864 bytes |
| `model.declaration_records` | model normalization | node count | `ModelNormalizationLimitsV1` declaration_records | 100000 records |
| `model.derivation_facts` | model normalization | node count | `ModelNormalizationLimitsV1` derivation_facts | 1600000 facts |
| `model.effective_declarations` | model normalization | node count | `ModelNormalizationLimitsV1` effective_declarations | 1600000 declarations |
| `model.dispatch_candidates` | model normalization | node count | `ModelNormalizationLimitsV1` dispatch_candidates | 1600000 candidates |
| `model.hashed_bytes` | model normalization | input bytes | `ModelNormalizationLimitsV1` hashed_bytes | 268435456 bytes |
| `model.work_units` | model normalization | work budget | `ModelNormalizationLimitsV1` work_units | 16777216 units |
| `model.ancestor_steps` | model normalization | edge count | `ModelNormalizationLimitsV1` ancestor_steps | 16777216 edges |
| `model.family_steps` | model normalization | edge count | `ModelNormalizationLimitsV1` family_steps | 16777216 edges |
| `admission.population_members` | population admission | node count | `PopulationAdmissionLimitsV1` population_members | 100000 members |
| `admission.work_units` | population admission | work budget | `PopulationAdmissionLimitsV1` work_units | 16777216 units |
| `admission.ancestor_steps` | population admission | edge count | `PopulationAdmissionLimitsV1` ancestor_steps | 16777216 edges |
| `observation.input_bytes` | observation admission | input bytes | `ObservationLimits` document bytes | 1048576 bytes |
| `observation.objects` | observation admission | node count | `ObservationLimits` objects per document | 10000 objects |
| `observation.values` | observation admission | node count | `ObservationLimits` values per document | 100000 values |
| `library.definitions` | library resolution | node count | `PackageLimits` definitions | 4096 definitions |
| `library.dependency_edges` | library resolution | edge count | `PackageLimits` dependency edges | 16384 edges |
| `library.artifact_bytes` | library resolution | input bytes | `PackageLimits` artifact bytes | 16777216 bytes |
| `library.single_artifact_bytes` | library resolution | input bytes | `PackageLimits` single artifact bytes | 1048576 bytes |
| `dependency.libraries` | S4 source resolution | node count | `DependencyLimits` libraries | 4096 libraries |
| `dependency.import_edges` | S4 source resolution | edge count | `DependencyLimits` import_edges | 16384 edges |
| `dependency.source_bytes` | S4 source resolution | input bytes | `DependencyLimits` source_bytes | 16777216 bytes |
| `identity.input_bytes` | identity encoding | input bytes | `AssemblyLimits` identity (`IdentityLimits` input_bytes), carried into checking, bundle linking and exploration keys | 16777216 bytes |
| `replay.input_bytes` | replay envelope readers | input bytes | the replay readers' encoded-byte bound | 1048576 bytes |
| `format.output_bytes` | format | input bytes | `qsl_cst::format::FormatLimits` output_bytes | 1048576 bytes |
| `i2.input_bytes` | I2 v2 reader | input bytes | the v2 read limits' artifact bytes | 16777216 bytes |
| `i2.nodes` | I2 v2 reader | node count | the v2 read limits' nodes | 10000 nodes |
| `i2.edges` | I2 v2 reader | edge count | the v2 read limits' edges | 100000 edges |
| `i2.occurrences` | I2 v2 reader | occurrence count | the v2 read limits' occurrences | 100000 occurrences |
| `i2.diagnostics` | I2 v2 reader | diagnostic count | the v2 read limits' diagnostics | 10000 diagnostics |
| `i2.work_units` | I2 v2 reader | work budget | the v2 read limits' work | 1000000 units |
| `explore.states` | exploration | node count | exploration `Limits` max_states | 10000000 states |
| `explore.transitions` | exploration | edge count | exploration `Limits` max_transitions | 100000000 transitions |

S2 has no limit of its own (FR-257). The `format.output_bytes` row bounds the
UTF-8 bytes the formatter would return, including formatting whitespace,
comments and the final newline. QSL uses the existing input-bytes limit kind
and its `input-bytes-exceeded` catalog cause for this output byte counter; the
row's setting and bound identify that the formatter's emitted output is the
counter. The formatter has no replay `stage_limits` entry because replay does
not run the format operation. Its typed `FormatLimits` field and the settings
operation remain the library entry points for raising it. `replay.input_bytes` bounds the replay
request and envelope bytes themselves, so it is raised from the library and
the CLI, and a request's own `stage_limits` does not carry it. An execution
budget under `quire.value.accounting/v1` is set in a request's accounting limits,
not through `--limit` or `CallerLimits`, and is named by its accounting
counter, for example `work_units`, which settles as `Incomplete` (QSpec
FR-461 Behavior 5); it has no row in this table. A configurable limit QSL adds later takes a name by the same
rule and a row in this table.

## Behavior

1. **The outcome names the setting.** When a stage refuses a charge because
   it would exceed a configured limit, QSL SHALL return that stage's limit
   outcome carrying the limit kind, the configured bound, the count the
   refused charge would have reached, the setting name of that limit, and
   the locus FR-096 defines for the producer. For a stage limit the outcome
   is `LimitExceeded`. A stage whose requirement gives its limits another
   code (format's `resource_exhausted`/`input-bytes-exceeded`, intake's
   `resource_exhausted`/`intake-limit-exceeded`, library resolution's
   `resource_exhausted`/`insufficient-next-charge`) keeps that code and carries
   the same four values. The catalog record of each such outcome SHALL carry
   the setting name as its `setting` field, beside `kind`, `bound` and
   `actual`.
2. **One mapping per limits type.** Each stage's limits type SHALL map each
   of its fields to its setting name in one place, and every producer of a
   `LimitExceeded` for that stage SHALL take the setting from that mapping.
   `FormatLimits` SHALL map its `output_bytes` field to `format.output_bytes`
   through the shared `SettingLimits` interface; the formatter's byte-budget
   diagnostic SHALL use that mapping.
3. **Rendering.** When QSL renders a `stage_limit_exceeded` diagnostic, the
   text SHALL follow QSpec FR-461's rendering, filled from the outcome of
   Behavior 1.
4. **The library raises every limit by its field.** Each stage's limits
   type SHALL offer a builder method for each field in the table, which sets
   that field's bound and changes nothing else. `FormatLimits::with_output_bytes`
   SHALL set the formatter's field, and `format_with_limits` SHALL consume the
   resulting `FormatLimits` value.
5. **The replay request raises every limit by name.** Replay SHALL read the
   request's `stage_limits` as a map from setting name to bound and pass
   every entry through to its stage (FR-263).
6. **The settings operation raises every limit by name.** The driver CLI
   exposes the caller limits as `--limit <name>=<value>`, repeatable
   (ADR-029 CB-1). QSL SHALL provide the library settings operation that the
   driver CLI calls with those operands: given a list of `<name>=<value>`
   operands, where `<name>` is a setting name from the table and `<value>` a
   non-negative decimal integer, it SHALL return the stages' limits with each
   named limit at that value. For `format.output_bytes`, the returned caller
   limits SHALL expose the resulting `FormatLimits` to the formatter. When
   `<name>` is not in the table, `<value>` is
   not a non-negative decimal integer, or one name is given twice, it SHALL
   return a usage refusal that names the operand, and no stage runs.
7. **Absent settings take their defaults.** When a caller does not
   configure a limit at an entry point, QSL SHALL run the stage with that
   limit at its default from the table.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-255-AC-1 | For each row of the setting table, driving its stage or operation past that limit at a configured bound `B` returns the row owner's specified limit outcome with the row's limit kind, bound `B`, the count the refused charge would have reached, and the row's setting name, and its catalog record carries the outcome code with fields `kind`, `bound`, `actual` and `setting` holding those values. The `format.output_bytes` case uses FR-003's `resource_exhausted` outcome over emitted UTF-8 bytes. | Test (TC-720, TC-931) |
| FR-255-AC-2 | A function `f` whose body is `1 + 1 + 1` (five expression nodes), checked with `s3.nodes` at 4, renders as "S3 node limit 4 reached (5) at <locus>; raise it with `--limit s3.nodes=<n>` or the request's `stage_limits` entry `s3.nodes`", where <locus> renders FR-096's locus of the node whose entry failed, in QSpec FR-461's rendering. | Test (TC-720) |
| FR-255-AC-3 | Each stage's limits type, including `FormatLimits`, maps every one of its fields to exactly one setting name, the names across all limits types are distinct, and the union of those names equals the setting table. Setting a field through its builder method changes that field's bound and leaves every other field at its prior value. | Test (TC-720) |
| FR-255-AC-4 | For each row of the setting table, an input or operation that reaches that limit at its default succeeds once the row's setting is raised to fit it, through each applicable entry point: the library limits type's builder method, the replay request's `stage_limits` entry of that name, and the settings operation given the operand `<name>=<value>`. `replay.input_bytes`, `format.output_bytes`, and a row whose stage a replay does not run are exercised through the builder and the settings operation; the formatter receives the resulting `FormatLimits`. | Test (TC-721, TC-931) |
| FR-255-AC-5 | The settings operation given `s9.nodes=1`, given `s3.nodes=ten`, given `s3.nodes=-1`, and given `s3.nodes=5` and `s3.nodes=6` together, each returns a usage refusal naming the offending operand, and no stage runs. | Test (TC-721) |
| FR-255-AC-6 | A compile with no limit configured at any entry point runs each stage at the defaults the table lists, and the effective limits a checked package records equal those defaults; an unconfigured format operation uses 1048576 output bytes. | Test (TC-721, TC-931) |

## Status

The setting table, the one-mapping-per-limits-type rule, the builders, the
settings operation, a request's `stage_limits` and the defaults are
implemented (TC-720, TC-721). Backing of the criteria:

- FR-255-AC-1 is stage-driven for the step ceilings and the byte limits: the step
  ceilings (`model.ancestor_steps`, `admission.ancestor_steps`,
  `model.family_steps`) carry their setting and the count reached in the
  model refusal cause itself, and `intake.input_bytes` in the intake
  refusal (FR-260-AC-4).
- FR-255-AC-3 compares the limits types against the table in this file.
- FR-255-AC-4 is stage-driven through the settings operation and a request's
  `stage_limits` for `s1.tokens`, `s3.nodes`, `s3.work_units` and
  `identity.input_bytes`, and through the stage's own limit raise for
  `library.*`, `admission.population_members`, `admission.work_units` and
  `model.dispatch_candidates`; every other row is checked at the entry
  points (the setting is accepted and sets its field) and not re-run
  through its stage.
- The `format.output_bytes` row and the `FormatLimits` to settings-operation
  seam are specified here and remain pending the formatter integration in
  QSL-605 (TC-931).

## Dependencies

- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  D-3 decides the setting names, the three entry points and the rendering.
- ADR-029 CB-1: the user CLI is the driver's (`quire`), and it exposes the
  caller limits through the settings operation.
- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)
  defines `LimitExceeded`, its kinds and each producer's locus.
- [FR-098](FR-098-execute-a-replay-request.md) and
  [FR-263](FR-263-replay-at-any-depth-under-the-request-limits.md) carry the
  request's `stage_limits`.
- [FR-003](FR-003-format-native-source.md) owns the formatter's output bytes,
  refusal code and no-partial-output rule.
- [ADR-014](../decisions/ADR-014-temporal-trace-and-boundedness-architecture.md)
  §2: an absent bound is inherited from its published default.
- QSpec FR-461 owns the naming rule and the rendering this requirement
  applies.

## References

- QSpec FR-146: no checking depth limit, and a host stack limit is never an
  outcome.
- QSpec FR-460, the ecosystem depth rule; FR-461, the limit, value and
  setting a reached limit names; and FR-323 `stage_limits`, the request's
  setting entries (Linear STD-143, which supersedes STD-125).
- Linear QSL-381.
