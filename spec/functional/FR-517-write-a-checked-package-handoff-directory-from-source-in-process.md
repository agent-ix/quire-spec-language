---
id: FR-517
title: "Write a checked-package handoff directory from source, in process"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-514
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-500
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-505
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-531
    type: references
---
# FR-517: Write a checked-package handoff directory from source, in process

## Description

FR-042-AC-10 and AC-15 (retired as historical in M-6d) required that real
accepted source pass the compiler stages and produce a handoff that quire-protocol
consumes unchanged, written in process by a library call into a new directory,
deterministically, behind a feature in the normal dependency surface. The writer
`protocol_artifact::handoff::{write_v1, write_v2}` did this for the
`quire.compiled-protocol/1` and `/2` wires. The retired deletion table (rows PA A20 to
A22, W4 to W7, AC-10, AC-15) found no successor when this requirement was
written: nothing in the checked-package path wrote a directory, a selection
sidecar or a mutation manifest, and quire-protocol never read a checked package
(GAP-10, GAP-15). This requirement specifies the successor writer. FR-518
specifies the mutation corpus it writes.

The legacy writer, its two examples, the `handoff-writer` feature and the
`expected*.json` files were deleted by QSL-840 once FR-517-AC-1 to AC-9, AC-11
to AC-14, AC-17 to AC-20 and FR-518 were green and QSL-1549 had repointed
quire-protocol (FR-499's keep row, FR-521 row 2). That change replaced nothing:
this writer was already its successor.

## Use case

A downstream crate's test compiles the recipe fresh and asks QSL to write a
handoff into a directory it names. It then reads `checked-package.json` with
Contract IR's `read_checked_package`, using evidence assembled from the files
beside it, and checks the package against `selection.json`, which was derived
from the source before any bytes were emitted.

## Inputs

`quire_spec_language::checked_handoff::write(&CheckedHandoffRequest, &Path)`,
behind the Cargo feature `checked-handoff` and adding no dependency:

- `source`: one complete-V1 (`1-draft`) unit, its library sources, and its
  domain package documents, as the bytes `emit_checked_v2::Request` takes (the
  `emit_checked_v2::emit` entry, not its command line);
- `package_name`, `execution_points` and `temporal_selections` as
  `emit_checked_v2` and `qsl_package::emit_checked_with_temporal_selections`
  take them (`temporal_selections` is a new optional member of the request and
  also selects a `compensate` template by its template name);
- `corpus`: `None` or `Standard` (FR-518).
- `native_join`: the finite work and logical-storage ceilings of the identity file's native joins (FR-531 "The native join budget"); its work and storage are `i2.work_units` and `i2.native_join_storage`. The writer has no default for it; one budget of it serves the write, and a denied charge returns `Error::NativeJoinExhausted` with Contract IR's refusal and writes no directory.

The `emit_checked_v2` command line carries `temporal_selections` as
`--clock-selections <json-file>`: a JSON object from the exact authored name of
a `temporal` clause or `compensate` template to its clock configuration in the
`TemporalBinding` `clock` wire form, for example `{"Due": {"kind":
"event_position", "sequence_authority": "events-authority"}}`. Each member is
one `ClockSelection`, in file order. An empty object, a repeated name or a
value that is not a well-formed clock configuration is a refusal naming the
file; a name that matches no clause or template is the request's
`UnknownClause` refusal. Without the option the request holds no selection.

## Outputs

A new directory containing exactly:

| Path | Content |
| --- | --- |
| `checked-package.json` | the exact bytes `emit_checked_v2::emit` returns |
| `sources/<label>` | the unit and library sources, exact bytes; `<label>` is the `source:<path>` label the package embeds |
| `models/<n>.json` | each domain package document, exact bytes, in selection order |
| `inventory.json` | `qsl_replay::qsl_inventory` over the unit, with the formal identity the writer selects (FR-514), in the encoding "The inventory file" states; the writer runs the replay spine over the unit's source to obtain the `check` outcome `qsl_inventory` takes (FR-517-AC-11). |
| `selection.json` | the selection sidecar, `quire.checked-handoff.selection/1` |
| `identities.json` | the required identity file, `quire.checked-handoff.identities/4`: selected declaration/field rows and final-emission source-record, source-union, source-operation and source-object rows, in the encoding FR-531 "The identity file" states; a bytes-only consumer reads it with the package (FR-531) |
| `equality.json` | the equality file, `quire.checked-handoff.equality/1`: the equality application rows, equality fact rows and selected source-unit rows of the in-process unit, in the encoding FR-531 "The equality file" states, read by a bytes-only consumer with the package to evaluate a protocol slot whose walk holds an `==` or `!=` application (FR-533) |
| `mutations/manifest.json`, `mutations/<case>.json` | present only for `corpus = Standard` (FR-518) |

The sidecar holds, per temporal clause: the clause `NodeId`, declared name, the
protocol requirement entries that name it (FR-449), its activation form
(`origin` or `each`, with the trigger binder name and whether a guard is
written), the
profile selection (identity and revision) of its law, the lock definition
authority, and its expected `TemporalBinding` row (`clock_name` and clock
configuration); per `compensate` template, in the `compensations` member:
its `node`, `name` and expected `binding` (`TemporalBinding` row with
`clock_name` and `clock`; `binding` is absent for a clause or template to which
the request gives no clock selection); and per package: the contract version, the read limits the
writer used (`CheckedPackageReadLimits`), the evidence rows (which `models/`
files are domain package documents and which `sources/` files are libraries) and
the counts of nodes by form. Every value is taken from the in-process checked
unit and the request, never from the emitted bytes.

`checked_handoff::Selection::read(bytes)` parses the sidecar and
`Selection::check(&self, &CheckedPackageV2) -> Result<(), SelectionRefusal>`
compares an admitted package with it, counting every `temporal_bindings` row,
those of clauses and those of compensation templates; `SelectionRefusal` carries one
`SelectionAxis` (FR-518). Both are QSL-owned so a consumer parses and compares
with no copy of the format.

The writer builds `equality.json` from the in-process unit's `Emission` rows
(`equality_producer_identity_rows`, `equality_fact_witness_rows` and
`selected_source_unit_rows`). The emitted package bytes are used for one join only:
every node and key member is written as the wire value of that node's `CheckedNodeId`
(the node id object of FR-531 "The identity file"), which the
writer reads from `WireGraph::of_package_bytes` over the emitted bytes, as for the
`node` member of `inventory.json`. A unit that holds no
equality application writes the file with `applications`, `facts` and `source_units`
empty, so the directory has the same paths for every unit.

### The inventory file

`inventory.json` is written once beside `checked-package.json`. Contract IR
publishes no wire form of a `SourceInventoryOutcome`, so the encoding is QSL's,
and it follows `selection.json` and `identities.json`: one JSON object,
pretty-printed with two-space indent, members in the order below, closed (an
unknown or repeated member is a refusal), UTF-8, ending in one newline,
byte-identical for equal inputs. The writer builds every row from the outcome
`qsl_inventory` returns for the in-process unit. The emitted package bytes are
used for one join only: a requirement row holds a structural `CheckedNodeId`
(`qsl_semantics::check::wire_id::structural_id` of the occurrence's node key),
and its `node` member is the wire value of that node's `CheckedNodeId` in the package (an object
with `package`, `path` and `ordinal`, as `selection.json` and `identities.json` spell node ids), which the writer
reads from `WireGraph::of_package_bytes` over the emitted bytes, as
`Selection::derive` does for `selection.json`. A node the package does not
name is the writer's `Error::Unnameable`, and nothing is written.

| Member | Content |
| --- | --- |
| `format` | the string `quire.checked-handoff.inventory/1` |
| `owner` | the string `qsl` (`SourceInventoryOwner::Qsl`) |
| `scope` | the string `qsl_checked_requirements_and_pending_obligations` |
| `source` | the formal identity as `{ "document": <text>, "revision": <integer> }`; every row's source is this value, so rows carry none |
| `completeness` | the string `complete`; the writer refuses a unit the compiler does not check (FR-517-AC-7), so it never writes an `Incomplete` inventory |
| `observed_count` | the number of rows, a JSON integer equal to the length of `rows` |
| `rows` | the outcome's rows in the order `qsl_inventory` yields them: the requirement records in report order, then the pending obligations in report order |

A row is a closed object with a `kind`. A requirement record is `{ "kind":
"requirement_record", "node": <node reference>, "role": <text>, "ordinal":
<integer> }`, with `node` the wire value of the package's `CheckedNodeId` for that node (the node
reference of FR-531 "The identity file"), `role` the requirement's role text and
`ordinal` its occurrence ordinal. A pending obligation is `{ "kind":
"pending_obligation", "declaration": <integer>, "unit": 0, "start": <integer>,
"end": <integer>, "capability_kind": <text> }`, with `declaration` the ordinal
of the requirement form that holds the obligation's span (FR-514), `unit` always
`0`, `start` and `end` the span's byte offsets in the unit's source, and
`capability_kind` the exact capability kind name. The inventory has no
`expression` member: the spine holds no expression handle (FR-514). Numbers are
JSON integers; no value is a float or a digest.

The formal identity is selected by one rule, so that equal requests give equal
files and no caller input is needed. `qsl_inventory` takes Contract IR's
`SourceIdentity`, whose `document` is a `SourceDocumentId`: it starts with an
ASCII letter and holds only ASCII letters, digits, `.`, `_` and `-` (`:` and
`/` are rejected), so the `source:<path>` label of `sources/<label>` cannot be
the document. `document` is the file name of the unit's source path (the last
path component, extension included) with every character outside that alphabet
replaced by `_`, and prefixed with `u` when the result is empty or does not
start with an ASCII letter; `revision` is `1`. The three packages of **R**
(`unit-event-position.native`, `unit-fixed-sample.native`,
`unit-timestamped-event.native`) therefore have the documents
`unit-event-position.native`, `unit-fixed-sample.native` and
`unit-timestamped-event.native`. The rule adds no disambiguator and no digest:
one handoff holds one unit, so its document is unique within it, and units whose
file names sanitize to the same text are told apart by the `sources/<label>`
beside the file, which keeps the full path label. The formal identity is a
parameter of `qsl_inventory` and not part of the checked unit (FR-514), so this
rule is the writer's.

`checked_handoff::Inventory::read(bytes)` parses the file, behind the same
feature as `Selection::read`, and returns `InventoryReadError`, a closed set:
`Malformed` (not JSON, not this shape, an unknown or repeated member or row
member, a `kind` that is neither row kind, a non-integer number, a `completeness`
other than `complete`, or an `observed_count` unequal to the length of `rows`)
and `Format` (another `format` string, naming it). It returns no partial value.

Vector. The unit is the file `unit.native` holding
`language "ix:native" edition "1-draft";`, `profile v = "quire.value.complete/v1";`
and `function f using v(x: Int[0, 9], p: Boolean): Integer pure { if p then x + 1 else x + 1 }`,
each on its own line, so its formal identity is document `unit.native`, revision
1. `qsl_inventory` over it (measured on the spine) returns four rows: two
requirement records with role `expression` and ordinals 0 and 1, then two
pending obligations of capability kind `value-validity` for declaration 0 at
byte spans 150 to 155 and 161 to 166. `N0` and `N1` stand for the `CheckedNodeId` wire values (a JSON object, written with
the same two-space indentation at the depth of the row's members) of the node
the two records name in the emitted package, which are not fixed here; both
records are occurrences of one node. The bytes are exactly these lines, each ending in a line feed,
with no trailing space:

```
{
  "format": "quire.checked-handoff.inventory/1",
  "owner": "qsl",
  "scope": "qsl_checked_requirements_and_pending_obligations",
  "source": {
    "document": "unit.native",
    "revision": 1
  },
  "completeness": "complete",
  "observed_count": 4,
  "rows": [
    {
      "kind": "requirement_record",
      "node": N0,
      "role": "expression",
      "ordinal": 0
    },
    {
      "kind": "requirement_record",
      "node": N1,
      "role": "expression",
      "ordinal": 1
    },
    {
      "kind": "pending_obligation",
      "declaration": 0,
      "unit": 0,
      "start": 150,
      "end": 155,
      "capability_kind": "value-validity"
    },
    {
      "kind": "pending_obligation",
      "declaration": 0,
      "unit": 0,
      "start": 161,
      "end": 166,
      "capability_kind": "value-validity"
    }
  ]
}
```

Refusal vectors for `Inventory::read`: the vector with `"observed_count": 3` (four rows) is
`Malformed`; with `"format": "quire.checked-handoff.inventory/2"` is `Format`
naming that string; with a row `"kind": "criterion"` is `Malformed`; with
`"completeness": "incomplete"` is `Malformed`; with `"start": 10.0` is
`Malformed`; with a second `"owner"` member is `Malformed`; the empty input is
`Malformed`.

## Behavior

- **In process, no side channel.** The writer calls the library entries only: no
  shell, no stdout parser, no alternate frontend and no hand-written fixture
  supplies the package (FR-042-AC-10's rule, kept).
- **Read back before writing.** The writer reads the emitted bytes with
  `quire_contract_model::read_checked_package` and evidence it assembled from the
  files it is about to write, and compares the admitted package with the
  in-process checked unit (node forms, node ids, temporal bindings). Any
  difference returns `Error::ReadBackDiffers` and writes nothing. With
  `corpus = Standard` it also replays every case (FR-518) before writing.
- **Deterministic.** Two calls with equal requests write byte-identical trees.
- **New directory only.** An existing path returns `Error::Exists` and its
  contents are untouched. The tree is built in a sibling temporary directory and
  moved into place, so a failure leaves no partial directory at the requested
  path (an improvement on the legacy writer, whose README allows a partial
  directory).
- **No fork.** The writer and `Selection::check` read packages only through
  Contract IR's reader and the borrowed `CheckedPackageV2` surface.

### The recipe maps to complete-V1 source

The IT-001 recipe (`examples/protocol-handoff/`) is a located rule model
(`model.json`) and five `0-draft` bodies (`predicates`, `state`, `temporal`,
`temporal-v2`, `workflow`). The checked-package producer takes complete-V1
source (QSL-1501 symbol map, row C), so the recipe is ported, not wrapped. The
ported recipe is **R**; its elements and their spine requirements:

| Recipe element | In **R** |
| --- | --- |
| `model.json` scalars `ObjectId` (text, 128), `Amount` 1..20, `Total` 0..100, `Tally` 0..5, `Wide` (signed 64-bit) | one domain package document with the same bounds, selected by a `model` declaration; `Amount` and `Total` are aliases of the unit `ix://quire/unit/U` that keep their bounds as member constraints (FR-517-AC-13) |
| scalars `Ratio` and `Exact` (bounded rationals) | two Rational value types of the model with no `unit` (FR-505: `unit` is optional, and the legacy scalars carry none): `Ratio` with numerator -1..1 and denominator 1..2, and `Exact` with numerator -9223372036854775808..9223372036854775807 and denominator 1..9223372036854775807; the compiled source reaches each, so the package carries it (FR-517-AC-12) |
| records `Workflow`, `Notice`, `Provider`; sequence `receipts` (maximum 5); the compensations' `attempts`, `retry` and `recover` binders typed by the object type `Workflow` | the object types `Workflow` and `Provider`, the record value type `Notice` and the bounded sequence `receipts`; `Workflow` stays the `over`, role and forward-binder type; the `attempts` type and the `retry` binders are the record value type `Attempt` (field `tally`), and the `recover` binder the record value type `Recovery` (field `receipts`) |
| `predicates` and `state` bodies: cross-unit calls, bounded `sum`/`size` queries, the extrema literals `-9223372036854775808` and `9223372036854775807` | predicates and functions in a library unit imported by the main unit, with the same literals |
| `temporal` bodies, three clock kinds: event position, fixed sample, timestamped event | four `temporal` clauses, each under a `profile` selection; clock configurations are given in the request (QSL-1557); `temporal_bindings` holds one row per temporal clause and per compensation template, in ascending `NodeRef` order |
| `workflow` body: roles `Service` and `Provider`, channel `Decisions` with two `Notice` receives and an all-branch join, `Outcome` choice with complementary Boolean guards, `Workflow::apply` with pre and post, and the compensations `Full` (commit `Main::Committed`) and `Partial` (commit `never`) | one `protocol` with the same roles, channel, join, choice, effect and two `compensate` templates (FR-383), whose `recover` conditions compare the receipt sum with the captured target (FR-517-AC-14) |

The shared library of **R** SHALL reach both unit-less Rational types through
typed predicate parameters and Rational literals. Each of the three packages
SHALL carry the two extreme literals with the emitted bounded Rational type of
`Exact`, not merely as source text or values of an unbounded Rational type.
`Ratio` keeps numerator bounds -1..1 and denominator bounds 1..2; `Exact` keeps
numerator bounds -9223372036854775808..9223372036854775807 and denominator bounds
1..9223372036854775807. Neither declaration gains a `unit`.

The legacy recipe's `Workflow` attempt and recovery binders are not portable,
and **R** replaces their type rather than the element. An attempt record is a
record or an event type (FR-381 "Activation, attempts and deadline"; QSpec
FR-466 reference check 1 on `attempts.record_type`), so `attempts 3 of
M::Workflow` is refused. The retry relation and the recovery predicate bar
model state, and a field read of an object-typed binder is a model-state read
(FR-381 "Names a stage may read", FR-381-AC-14; QSpec FR-464 "Slots that bar
model state"), so `earlier.tally` and `recovered.receipts` over `Workflow`
binders are refused. The compensations keep every stage, bound and retry
relation of the legacy recipe over the record value types `Attempt` and
`Recovery`, and their `recover` conditions are those of FR-517-AC-14: the
reader admits the `sum` query binder that a `recover` slot introduces itself. A `recover` condition that
reads its recovery binder is checked and emitted, and analysis of **R**
reports that compensation item `unsupported` with cause
`recovery-view{predicate}` (QSpec FR-427): no rule derives a recovery view
from a model state, so this is the expected result, not a defect of **R**.

The unit `U` on `Amount` and `Total` is carried by the model. In the model it is
an alias of `ix://quire/unit/U` whose `min` and `max` name the alias (QSpec
FR-154 "Unit bounds"; FR-230-AC-26), and each source declares
`dimension Money;` and `unit U : Money = rational(1, 1);`. Unit-typed members
check against unit literals (`amount >= 1 U` in `BeforeApply`). A measurement
unit that only the selected model's types name is emitted with
its dimension and target closure (FR-094); the library and fixed-sample and
timestamped-event units no longer need an expression that reads a unit-typed member.
Model aliases are source type names: `M::Total` resolves to its admitted
unit-bearing type. Its model `min`/`max` constraints retain their original
alias authority as member bounds (FR-230), without widening them or bounding
unrelated values of `U`. `capture targetFull: M::Total` therefore checks.
`sum<N>` checks a summand of the numeric family of `N` (FR-517-AC-20): an
integer total takes any integer summand, a rational total any rational, a decimal
total any decimal, and a float or quantity total a summand of that very float
type or unit (a bounded quantity of the unit is a quantity of it), and a carried-quantity total a summand of that very carried type. It lowers
to the family's `quire.op.collection.sum.<family>` identity (FR-093) and
evaluates by the family's addition under the value-accounting collection sum
schedule: a total the checker cannot prove inside `N`'s domain is refused as an unproved range (FR-046, FR-100),
and the empty sum is the family's zero, which `N` must admit. So **R**'s
`Healthy` invariant sums its receipts over `Quantity<U>` without the earlier
`IllTyped(TypeMismatch)`.

**R** is three packages, one per clock kind, because the emitter admits one
temporal profile selection per unit and the checker refuses a unit that selects
three. `write` writes one handoff directory for each package, and FR-518's cases
run in the package of their clock kind, so the corpus is split by kind across
the three directories. The packages hold:

| Package | Temporal clauses | Protocol, compensations and effect | `temporal_bindings` rows |
| --- | --- | --- | --- |
| event position | `Due` and `DueLate`, so the row-ordering case can swap them | one `protocol_clause`, the `Full` and `Partial` compensations, the `Main::Applied` effect | four: two clauses and two compensation templates |
| fixed sample | `DueSample` | none | one |
| timestamped event | `DueTimestamp` | none | one |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-517-AC-1 | `write(&request_for_R, dir)` creates exactly the paths of the Outputs table of the Outputs table, `inventory.json` included (with no `mutations/` for `corpus = None`), and `checked-package.json` equals the bytes `emit_checked_v2::emit` returns for the same request. | Test (TC-1312) |
| FR-517-AC-2 | The package read from `checked-package.json` with `read_checked_package` and evidence assembled from the written files is admitted, and `Selection::check` passes; the sidecar's clause ids, names, requirement entries, activation forms, profile selections, definition authority and `TemporalBinding` rows, those of clauses and those of compensation templates, equal the values read from the checked unit held in memory by the writer. | Test (TC-1312) |
| FR-517-AC-3 | The event-position package of **R** emits one `protocol_clause`, a `Full` and a `Partial` compensation node with different keys paired with the one `Main::Applied` effect, two temporal clauses and four `temporal_bindings` rows in ascending `NodeRef` order; the fixed-sample and timestamped-event packages each emit one temporal clause and one row; the rows cover the clock kinds `EventPosition`, `FixedSample` and `TimestampedEvent`. | Test (TC-1312) |
| FR-517-AC-4 | **R** carries the extrema `-9223372036854775808` and `9223372036854775807` as `Wide` values of the emitted package, and no declared bound is widened to admit them. | Test (TC-1312) |
| FR-517-AC-5 | Two `write` calls with equal requests give byte-identical trees (every file, in path order). | Test (TC-1312) |
| FR-517-AC-6 | `write` onto an existing directory returns `Error::Exists` and leaves every byte of the directory unchanged; a failure injected after the temporary tree is built (read-back difference, I/O error on the third file) leaves no directory or temporary directory at or beside the requested path. | Test (TC-1312) |
| FR-517-AC-7 | A request whose source the compiler refuses returns the compiler's refusal and writes nothing; a request whose emitted package the reader refuses, and a request for which `ReadBackDiffers` is injected, write nothing. | Test (TC-1312) |
| FR-517-AC-8 | `checked_handoff` is behind the feature `checked-handoff`, adds no entry to `Cargo.lock`, and a downstream crate with only that feature enabled compiles `write` and `Selection::check` without building this crate's examples or dev-dependencies. | Test (TC-1312) |
| FR-517-AC-9 | A package with one `temporal_bindings` clock value changed to another well-formed value is admitted by `read_checked_package` and refused by `Selection::check` naming its `SelectionAxis`; no function of the module parses a checked package except through `read_checked_package` (`git grep` finds no second reader). | Test (TC-1312) |
| FR-517-AC-10 | The legacy writer is absent: `protocol_artifact::handoff` (`write_v1`, `write_v2`, the five `PUBLISHED_*` constants), the `handoff-writer` feature, the two examples `native_protocol_handoff` and `native_protocol_v2_handoff`, `examples/protocol-handoff` and `tests/it/handoff_writer.rs` do not exist, and no Cargo manifest of the workspace names the feature. The gate that kept them (FR-517-AC-1 to AC-9, AC-11 to AC-14, AC-17 to AC-20 and FR-518-AC-1 to AC-12 green, FR-521 row 2) was met when they were deleted. | Test (TC-1312) |
| FR-517-AC-11 | Each handoff directory written for **R** contains `inventory.json`, and its bytes equal the encoding "The inventory file" states of the outcome `qsl_replay::qsl_inventory` returns over the unit, run through the replay spine, with the formal identity that section selects; the file's rows are non-empty, `observed_count` equals their number, and two writes of equal requests give equal files. | Test (TC-1312) |
| FR-517-AC-12 | **R**'s model declares the Rational value types `Ratio` and `Exact` with no unit, the compiled source reaches both, and the emitted package of **R** carries the rationals `9223372036854775807/9223372036854775806` and `-9223372036854775808/9223372036854775807` as `Exact` values, with no declared bound widened to admit them. | Test (TC-1312) |
| FR-517-AC-13 | **R**'s model declares `Amount` and `Total` as aliases of the unit `ix://quire/unit/U`, each keeping its `min` and `max` naming the alias (`Amount` 1 and 20, `Total` 0 and 100), and each of the three **R** sources declares `U`. The emitted package of **R** holds the node of `U`, types the `targetFull` and `targetPartial` captures by it, and lowers **R**'s receipt sums to `quire.op.collection.sum.quantity`. | Test (TC-1312) |
| FR-517-AC-14 | With a `sum` query binder admitted inside `recover`, the `recover` condition of `Full` is that the receipt sum equals the captured target, and that of `Partial` is that the receipt sum is positive and below the captured target. | Test (TC-1312) |
| FR-517-AC-15 | `emit_checked_v2` run with `--clock-selections` naming a file that holds the clock configuration of each temporal clause and compensation template of an **R** package emits the bytes `emit_checked_v2::emit` returns for the equal `Request`; an empty object, a repeated name, an unknown `kind`, a member the configuration does not define and a member it requires that is missing are each a refusal naming the file that writes no bytes, and a name matching no clause or template is the `UnknownClause` refusal that writes no bytes. | Test (TC-1312) |
| FR-517-AC-16 | Over **R**'s model with `Amount` and `Total` as aliases of `ix://quire/unit/U`, a source whose `pre` compares the `Amount` parameter with `1 U` is written and admitted. `sum<Cash>` over the receipts, with `type Cash = Quantity<U>;`, is written and admitted (FR-517-AC-20). | Test (TC-1312) |
| FR-517-AC-17 | `Inventory::read` over the bytes `write` produced for **R** returns rows equal to those of the in-process outcome, and over each refusal vector of "The inventory file" returns the stated `InventoryReadError` and no value; two reads of equal bytes return equal results. | Test (TC-1312) |
| FR-517-AC-18 | A measurement unit that the selected model's types name is part of the emitted package even when no expression of the compiled source reaches it: the package holds its `dimension` and `unit` nodes, and Contract IR's reader admits a package whose model `typeRef` names it. Over **R**'s model with `Amount` and `Total` as aliases of `ix://quire/unit/U`, a source that reads no unit-typed member is written and admitted. | Test (TC-1312) |
| FR-517-AC-19 | A model alias is a source type name: over **R**'s model, `sum<M::Total>` and `capture targetFull: M::Total` resolve to the alias's unit-bearing type, and neither refuses `UnresolvedTypeName`. The capture checks, and the sum checks as a quantity sum (FR-517-AC-20). The alias's original `min`/`max` bounds remain selected-model member constraints and enforce the same endpoint values as before, without widening them or imposing them on other aliases of `U`. | Test (TC-1312) |
| FR-517-AC-20 | `sum<N>` checks a summand of any numeric family QSpec FR-145 and FR-322 give a `quire.op.collection.sum` identity for (integer, rational, decimal, float, `Quantity<U>` and the carried-quantity family), and lowers to that family's identity; over **R**, the `Healthy` invariant's receipt sum over `Quantity<U>` checks and evaluates to the sum of the receipts' magnitudes in `U`, and a `sum<N>` whose `N` is a carried `Quantity<U, C>` over summands of that very type checks, lowers to `quire.op.collection.sum.carried_quantity` and evaluates (QSpec FR-142-AC-33 to AC-38). | Test (TC-1312) |
| FR-517-AC-21 | The handoff directory written for TC-1070 fixture E contains `equality.json`, whose bytes equal the encoding FR-531 states of the rows of the in-process unit, with the three row lists non-empty (the unit holds integer `=` applications) and each row's node members naming nodes of `checked-package.json`; the directory written for the unit of the inventory vector (`unit.native`, which holds no `==` or `!=` application) contains `equality.json` with the three lists empty; two writes of equal requests give byte-identical files. | Test (TC-1312) |
| FR-517-AC-22 | A writer given a unit whose equality rows name a node the package does not hold returns `Error::Unnameable` and writes nothing, as for `inventory.json`; the written file holds no source text, no digest and no member the encoding of FR-531 does not list. | Test (TC-1312) |
| FR-517-AC-23 | The writer emits the required `identities/4` source-record, source-union, source-operation and source-object lists from the same final emission as the package: original opaque keys, final structural/type references, original member identities and order round-trip through native joins. The reader requires all four lists and refuses the old format or an omitted list; a `identities/3` document is the old format and is refused; no optional compatibility path or source fallback supplies them. | Test (TC-1312) |

## Dependencies

- **Upstream:** the inventory writer (FR-517-AC-11, AC-17) runs the replay spine over the unit and calls `qsl_inventory` over it (FR-514). FR-517-AC-12 needs no spine change: FR-505 admits a Rational value type without a unit, and the criterion fixes only what **R**'s model declares and the source reaches. FR-517-AC-13 is gated on the three spine capabilities of FR-517-AC-18 to AC-20 (the unit a model type names is emitted, a model alias is a source type name, `sum` over a quantity), all delivered. FR-517-AC-14 holds: the reader admits a binder that a protocol slot introduces itself, such as the `sum` query binder inside `recover`. FR-042-AC-10 and AC-15 (retired), FR-337, FR-383, FR-480,
  FR-505, FR-514, FR-515, `emit_checked_v2`, `qsl_package::emit_checked_with_temporal_selections`.
- **Downstream:** quire-protocol IT-001 (QSL-1549).
