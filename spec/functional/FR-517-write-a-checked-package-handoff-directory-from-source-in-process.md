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
---
# FR-517: Write a checked-package handoff directory from source, in process

## Description

FR-042-AC-10 and AC-15 (retired as historical in M-6d) required that real
accepted source pass the compiler stages and produce a handoff that quire-protocol
consumes unchanged, written in process by a library call into a new directory,
deterministically, behind a feature in the normal dependency surface. The writer
`protocol_artifact::handoff::{write_v1, write_v2}` still does this for the
`quire.compiled-protocol/1` and `/2` wires. The deletion table (rows PA A20 to
A22, W4 to W7, AC-10, AC-15) found no successor: nothing in the checked-package
path writes a directory, a selection sidecar or a mutation manifest, and
quire-protocol never reads a checked package (GAP-10, GAP-15). This requirement
specifies the successor writer. FR-518 specifies the mutation corpus it writes.

The legacy writer, its two examples, the `handoff-writer` feature and the
`expected*.json` files stay until FR-517-AC-1 to AC-9, AC-11 to AC-14 and FR-518 are green and
QSL-1549 has repointed quire-protocol (FR-499's keep row, FR-521 row 2); the
change that deletes them replaces nothing: this writer is already its successor.

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
| `inventory.json` | `qsl_replay::qsl_inventory` over the unit, with the selected formal identity (FR-514); written once QSL-1512 provides `qsl_inventory` over the spine unit (FR-517-AC-11) |
| `selection.json` | the selection sidecar, `quire.checked-handoff.selection/1` |
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
| `model.json` scalars `ObjectId` (text, 128), `Amount` 1..20, `Total` 0..100, `Tally` 0..5, `Wide` (signed 64-bit) | one domain package document with the same bounds, selected by a `model` declaration; `Amount` and `Total` carry no unit until QSL-1558 (FR-517-AC-13), for the refusals FR-517-AC-15 pins |
| scalars `Ratio` and `Exact` (bounded rationals) | none (FR-517-AC-12) |
| records `Workflow`, `Notice`, `Provider`; sequence `receipts` (maximum 5); the compensations' `attempts`, `retry` and `recover` binders typed by the object type `Workflow` | the object types `Workflow` and `Provider`, the record value type `Notice` and the bounded sequence `receipts`; `Workflow` stays the `over`, role and forward-binder type; the `attempts` type and the `retry` binders are the record value type `Attempt` (field `tally`), and the `recover` binder the record value type `Recovery` (field `receipts`) |
| `predicates` and `state` bodies: cross-unit calls, bounded `sum`/`size` queries, the extrema literals `-9223372036854775808` and `9223372036854775807` | predicates and functions in a library unit imported by the main unit, with the same literals |
| `temporal` bodies, three clock kinds: event position, fixed sample, timestamped event | four `temporal` clauses, each under a `profile` selection; clock configurations are given in the request (QSL-1557); `temporal_bindings` holds one row per temporal clause and per compensation template, in ascending `NodeRef` order |
| `workflow` body: roles `Service` and `Provider`, channel `Decisions` with two `Notice` receives and an all-branch join, `Outcome` choice with complementary Boolean guards, `Workflow::apply` with pre and post, and the compensations `Full` (commit `Main::Committed`) and `Partial` (commit `never`) | one `protocol` with the same roles, channel, join, choice, effect and two `compensate` templates (FR-383), whose `recover` conditions compare the receipt sum with the captured target (FR-517-AC-14); until a `sum` query binder is admitted inside `recover` (QSL-1555), **R** substitutes a `size` bound on the receipts and a test of the captured target |

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
`Recovery`, and their `recover` conditions are those of FR-517-AC-14, with the
`size` substitute above until QSL-1555 (QSL-1556). A `recover` condition that
reads its recovery binder is checked and emitted, and analysis of **R**
reports that compensation item `unsupported` with cause
`recovery-view{predicate}` (QSpec FR-427): no rule derives a recovery view
from a model state, so this is the expected result, not a defect of **R**.

The unit `U` on `Amount` and `Total` is not portable yet. In the model it is
an alias of `ix://quire/unit/U` whose `min` and `max` name the alias (QSpec
FR-154 "Unit bounds"; FR-230-AC-26), and each source declares
`dimension Money;` and `unit U : Money = rational(1, 1);`. Unit-typed members
check against unit literals (`amount >= 1 U` in `BeforeApply`), but **R**
refuses at three points (QSL-1558):

- A measurement unit that no expression of the compiled source reaches is not
  emitted, although the selected model's types name it, so Contract IR's reader refuses the first
  model `typeRef` that reaches it, `Recovery.receipts` of type `Amount`:
  `missing_declaration`/`missing-name` at `/types/7/fields/0/typeRef`. **R**'s
  library and its fixed-sample and timestamped-event units read no unit-typed
  member.
- A model alias has no node and is not a source type name (FR-230), so
  `sum<M::Total>` and `capture targetFull: M::Total` refuse
  `UnresolvedTypeName` at assembly.
- `sum<N>` checks an integer summand only (FR-093's `Query{visit: Sum}` row
  lowers to `quire.op.collection.sum.integer`), so the `Healthy` invariant's
  receipt sum over `Quantity<U>` refuses `IllTyped(TypeMismatch)` at the
  clause, where QSpec FR-145 and FR-322 define `quire.op.collection.sum` per
  numeric family, the quantity family included.

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
| FR-517-AC-1 | `write(&request_for_R, dir)` creates exactly the paths of the Outputs table other than `inventory.json` (with no `mutations/` for `corpus = None`), and `checked-package.json` equals the bytes `emit_checked_v2::emit` returns for the same request. | Test (TC-1312) |
| FR-517-AC-2 | The package read from `checked-package.json` with `read_checked_package` and evidence assembled from the written files is admitted, and `Selection::check` passes; the sidecar's clause ids, names, requirement entries, activation forms, profile selections, definition authority and `TemporalBinding` rows, those of clauses and those of compensation templates, equal the values read from the checked unit held in memory by the writer. | Test (TC-1312) |
| FR-517-AC-3 | The event-position package of **R** emits one `protocol_clause`, a `Full` and a `Partial` compensation node with different keys paired with the one `Main::Applied` effect, two temporal clauses and four `temporal_bindings` rows in ascending `NodeRef` order; the fixed-sample and timestamped-event packages each emit one temporal clause and one row; the rows cover the clock kinds `EventPosition`, `FixedSample` and `TimestampedEvent`. | Test (TC-1312) |
| FR-517-AC-4 | **R** carries the extrema `-9223372036854775808` and `9223372036854775807` as `Wide` values of the emitted package, and no declared bound is widened to admit them. | Test (TC-1312) |
| FR-517-AC-5 | Two `write` calls with equal requests give byte-identical trees (every file, in path order). | Test (TC-1312) |
| FR-517-AC-6 | `write` onto an existing directory returns `Error::Exists` and leaves every byte of the directory unchanged; a failure injected after the temporary tree is built (read-back difference, I/O error on the third file) leaves no directory or temporary directory at or beside the requested path. | Test (TC-1312) |
| FR-517-AC-7 | A request whose source the compiler refuses returns the compiler's refusal and writes nothing; a request whose emitted package the reader refuses, and a request for which `ReadBackDiffers` is injected, write nothing. | Test (TC-1312) |
| FR-517-AC-8 | `checked_handoff` is behind the feature `checked-handoff`, adds no entry to `Cargo.lock`, and a downstream crate with only that feature enabled compiles `write` and `Selection::check` without building this crate's examples or dev-dependencies. | Test (TC-1312) |
| FR-517-AC-9 | A package with one `temporal_bindings` clock value changed to another well-formed value is admitted by `read_checked_package` and refused by `Selection::check` naming its `SelectionAxis`; no function of the module parses a checked package except through `read_checked_package` (`git grep` finds no second reader). | Test (TC-1312) |
| FR-517-AC-10 | Until FR-517-AC-1 to AC-9, AC-11 to AC-14 and FR-518-AC-1 to AC-12 are green, `handoff::write_v1`, `write_v2`, the five `PUBLISHED_*` constants (`PUBLISHED_V1_OFFER_FILE`, `PUBLISHED_V1_SELECTION_FILE`, `PUBLISHED_OFFER_FILE`, `PUBLISHED_SELECTION_FILE`, `PUBLISHED_MUTATION_MANIFEST_FILE`), the `handoff-writer` feature, the two examples and `examples/protocol-handoff` are present; the change that removes any of them cannot merge while FR-521 row 2 is not green. | Inspection (TC-1312, TC-1316) |
| FR-517-AC-11 | With `qsl_inventory` over the spine unit available (QSL-1512), each handoff directory written for **R** contains `inventory.json` equal to `qsl_replay::qsl_inventory` over the unit with the selected formal identity. | Test (TC-1312) |
| FR-517-AC-12 | With the spine admitting a bounded rational value type (QSL-1522, FR-505), **R** carries the rationals `9223372036854775807/9223372036854775806` and `-9223372036854775808/9223372036854775807` as `Exact` values of the emitted package. | Test (TC-1312) |
| FR-517-AC-13 | With the model admitting a unit on a bounded scalar (QSL-1558, FR-497), the `Amount` and `Total` scalars of **R** carry the unit `U`. | Test (TC-1312) |
| FR-517-AC-14 | With a `sum` query binder admitted inside `recover` (QSL-1555), the `recover` condition of `Full` is that the receipt sum equals the captured target, and that of `Partial` is that the receipt sum is positive and below the captured target. | Test (TC-1312) |
| FR-517-AC-15 | `emit_checked_v2` run with `--clock-selections` naming a file that holds the clock configuration of each temporal clause and compensation template of an **R** package emits the bytes `emit_checked_v2::emit` returns for the equal `Request`; an empty object, a repeated name, an unknown `kind`, a member the configuration does not define and a member it requires that is missing are each a refusal naming the file that writes no bytes, and a name matching no clause or template is the `UnknownClause` refusal that writes no bytes. | Test (TC-1312) |
| FR-517-AC-16 | Until QSL-1558, over **R**'s model with `Amount` and `Total` as aliases of `ix://quire/unit/U`: a source whose `pre` compares the `Amount` parameter with `1 U` is written and admitted; a source that reads no unit-typed member is refused by the reader with `missing_declaration`/`missing-name` at `/types/7/fields/0/typeRef`; `sum<M::Total>` refuses `UnresolvedTypeName` for `M::Total`; and `sum<Cash>` over the receipts, with `type Cash = Quantity<U>;`, refuses `IllTyped(TypeMismatch)` at its state clause. | Test (TC-1312) |

## Dependencies

- **Upstream:** QSL-1512 (`qsl_inventory` over the spine unit, FR-517-AC-11), QSL-1522 (FR-517-AC-12), QSL-1558 (FR-517-AC-13, AC-16), QSL-1555 (FR-517-AC-14), FR-042-AC-10 and AC-15 (retired), FR-337, FR-383, FR-480,
  FR-505, FR-514, FR-515, `emit_checked_v2`, `qsl_package::emit_checked_with_temporal_selections`.
- **Downstream:** quire-protocol IT-001 (QSL-1549).
