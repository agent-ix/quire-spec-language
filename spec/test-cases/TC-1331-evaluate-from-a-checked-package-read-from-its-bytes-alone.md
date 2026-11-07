---
id: TC-1331
title: "Evaluate from a checked package read from its bytes alone"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-531
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: references
  - target: ix://agent-ix/quire-contract-ir/FR-357
    type: references
  - target: ix://agent-ix/quire-contract-ir/FR-358
    type: references
---
# TC-1331: Evaluate from a checked package read from its bytes alone

## Description

Verify the bytes-only entry and its selected-monitor preparation boundary.
Scope: FR-531-AC-1 through FR-531-AC-33. **R** is the package of FR-517.
The isolated bytes-only consumer test crate depends on `quire-contract-model`,
`qsl-package` and `qsl-eval` only. Producer transport and selected-monitor
preparation tests additionally obtain authentic final emissions, admitted model
inputs and the FR-251 plan context through their public QSL APIs before handing
rows or bytes to the consumer.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1, 3 | in a child run given only **R**'s handoff directory: `read_checked_package`, `BarePackage::new`, one temporal call, one slot call | both return a result; the source unit is never compiled |
| 2 | 2 | compile-fail probes of a `BarePackage` from bytes, from a decoded graph and from a term; API-surface listing of the signatures | each fails to compile; no signature names `CheckedPackage`, `Emission` or `AdmittedPackageRead` |
| 3 | 4 | each clause form and each slot kind **R** holds; a non-temporal node; a non-protocol node | results; `clause-root-unresolved` and FR-390-AC-4's refusal, empty charge logs |
| 4 | 5 | slot call with no supply, foreign rows, unknown rows; the temporal call with no supply; `read_checked_package` over a lock that selects a dependency the evidence lacks | `MissingIdentityWitness`, `ForeignIdentityWitness`, `UnknownIdentityWitness` as exit-21 `Admission`, meter untouched; the temporal call returns; the reader refuses `MissingImport`, so no package is admitted and no dependency refusal exists |
| 5 | 6 | a package without `temporal_bindings`; without a profile row; with an unknown profile definition; with an infinite-trace profile | `Unavailable` with the clock `Missing`; `clause-unreadable` `profile-selection`; `clause-unreadable` `profile-unknown`; `not-finite-window-profile` |
| 6 | 7 | encode **R**'s results; search for the clause names, `source:` labels and source map regions; read the progress binding | none present; the binding holds `package.name()` |
| 7 | 8 | **R**'s directory: read `identities.json`; compare with the encoding of the in-process rows; write twice; omit the file from the supply | slot call evaluates; bytes equal; two writes equal; `MissingIdentityWitness` for the slot call only |
| 8 | 9 | enumerate the public methods of `EvidencePackage` and `CheckedPackageEvaluation` against the inventory table | one row each; an unlisted method fails the test |
| 9 | 10 | charge logs of bare and in-process calls; `new`, `with_supply`, view creation | equal; no `Meter` taken; a refusing call leaves the meter identical |
| 10 | 12 | `ReadIdentityRows::read` over a non-JSON file, an unknown or repeated member, another `format`, a row the package does not hold, a form that does not match its identity | `Malformed`, `Malformed`, `Format`, `Unjoined`, `Unjoined`; no rows |
| 11 | 13 | **R**'s identity file: every identity kind; a unit with kind `compound`; a declared unit with an absent node reference; a `declared_unit` row whose identity is a record; a forged row that joins | encodings as stated; rows equal the writer's; `Malformed`; `Unjoined`; `Unjoined`; accepted with no authority claimed |
| 12 | 11 | read FR-515, FR-516, FR-520 | the out-of-scope sentence is gone; each has a "Consumer entry" section naming FR-531 |
| 13 | 14 | TC-1070 fixture E's handoff directory: read `equality.json`; compare with the encoding of the in-process rows; write twice | `Refund`'s retry relation evaluates; bytes equal; two writes equal |
| 14 | 15 | `ReadEqualityRows::read` over a non-JSON file, an unknown or repeated member, an unknown fact tag, another `format`, a row the package does not hold, a row that is no equality application | `Malformed`, `Malformed`, `Malformed`, `Format`, `Unjoined`, `Unjoined`; no rows |
| 15 | 16 | a slot call on fixture E's `Refund` retry relation with `equality: None`, foreign rows, the package's own rows without the application's row; `Refund`'s `trigger.guard` | `MissingEqualityWitness`, `ForeignEqualityWitness`, `UnknownEqualityWitness` as exit-21 `Admission` at `CallAdmissionLocus::Package`, meter untouched; evaluates |
| 16 | 17 | the vector rows of "The equality file"; a row of every fact tag, value-type tag, owner kind and a compound unit in a fact row; malformed rationals; a compound unit in `source_units` | bytes equal the vector; rows round-trip with byte-identical rewrites; `Malformed` |
| 17 | 18 | rows with a `carried_quantity` fact value type over each carrier, a `bounded_quantity` with and without bounds, a `map` with and without a bound, and the example of "Value encodings"; a bound in another unit; an unknown carrier tag | write then read returns equal rows, byte-identical rewrite, the example value-type subtree as JSON, members in order; `Malformed` for the last two |
| 18 | 19 | a genuine final source-record emission, including renamed members and nested opaque types | required `/2` rows round-trip unchanged; native IR record join and `source_record_view` equal the producer row |
| 19 | 20 | old format; missing required arrays; repeated/unknown members; malformed or over-depth opaque key, including an isolated deep-key writer/reader control; stale/foreign/missing/conflicting record rows | typed parser/native row refusal; an empty supplied record set refuses at the actual native request and maps to public `MissingTypeRepresentation`, with no structural-key success fallback |
| 20 | 21 | a fixture's `with_supply` on a budget measured at (W, S), then (W - 1, S) and (W, S - 1) | the package; `ExhaustedWork`; `ExhaustedStorage`; consumed counters as IR reports; never `UnknownIdentityWitness`, a fault or an empty set |
| 21 | 22 | foreign rows, unknown rows and no supply through an admitting budget; unknown rows through a budget one unit short | the three witness refusals at the slot call; the budget denial from `with_supply` |
| 22 | 23 | two `with_supply` on one budget; selection, world and projection calls on the first package | cumulative counters; the second refused at the first crossing charge; no further native charge from the calls |
| 23 | 24 | for `with_supply`, `from_read` and `from_emission`, also reuse one budget for two joins at (2W, 2S), (2W - 1, 2S) and (2W, 2S - 1); independently measure each entry's native work W and storage S: evaluation view, `from_read`, `from_emission`, handoff writer; run each at (W, S), (W - 1, S), (W, S - 1) | exact bounds succeed; work and storage one short respectively refuse with the correct setting and `ViewFailure::Exhausted`, `PackageMismatch::Exhausted` or `Error::NativeJoinExhausted`; a refused writer leaves no directory; opaque-budget denials preserve the original configured bound, native locus and setting, and actual is cumulative consumed plus first unpaid charge |
| 24 | 25 | signature inspection of the six entries; a search for `NativeJoinBudget::new` in library sources | mutable-reference parameters; none below an entry; no `Copy` or `Clone` on `BarePackage` |

For AC-26, write and read an authentic source-union emission with nullary and positional payload members, comparing complete original rows and native joined rows and byte-identical rewrites. For AC-27, reject format `/2`, absent `source_unions`, empty member identities, stale member names, foreign source owners and conflicting repeated rows with typed refusals and no partial read. Verify absent union witnesses through the bare union refusal tests.

For AC-29, compare the complete equality application, fact and source-unit rows
of an authentic final emission at ample and exact measured native allowances.
Exercise its source-admission and handoff-writing paths. Repeat on one budget,
then lower each resource by one and try zero storage, asserting the typed first
unpaid kind, amount and locus, cumulative admitted prefix and absence of an
authoritative partial witness. Observe the adapter's allocations so a denied
materialization step and compact refusal cannot allocate their payloads.

For AC-30, use source ordered and unordered enums, a model enum shape and
sequence, set and bounded-collection rows from one final emission. Compare
complete native rows and original Exact identities after conversion, including
canonical variant order. Compose equality, enum and collection adapters on one
budget and exercise exact, one-short and zero-storage boundaries. Preserve an
earlier first-denial locus across a later attempted conversion. A map with a key
that the target IR collection row cannot carry refuses with its typed
unsupported cause; it never becomes a keyless collection. Re-reading borrowed
retained rows adds no owned-storage charge.
Use one genuine emission retaining both a sequence and a map: converting and
projecting the selected sequence succeeds with its original type and nodes,
while selecting the map returns its typed unsupported cause and keeps its key.

For AC-31, obtain an authentic emission with nonempty source records, members
and unions. Materialize its identity rows and compare every retained original
declaration, field, member, union key and ordering to the producer's claims;
then pass the rows through the actual native identity join and type projection.
Measure the native join prefix separately from the adapter's later declaration,
field, record/member and union copies, requiring their sum to equal the shared
budget's cumulative consumption. Repeat with exact allowances, one below each
resource and zero storage. A refusal returns no partial rows and preserves the
original configured bound, admitted prefix and first unpaid row locus. Compose
a second adapter on that same budget and require no restored allowance.

For AC-32, build two genuine tick-monitor emissions with the same authored
names but distinct original package/model identities. Through FR-251's public
plan context, obtain the first emission's plan, obligations, admitted order and
progress association. Join its converted selected rows through Contract IR and
perform the actual selected type-closure projection. Compare complete native
values, original node/declaration/member identities and Exact types. Present
the second emission's association or omit each required authority in turn;
require the typed refusal and no executable context, without fabricating a
plan, graph or owner receipt. Dropping producer preparation objects does not
change the retained context. Borrowed context reads add no owned-storage charge.

For AC-33, use a real allocation observer, warmed independently from its payload
fixtures. Prepare fresh owner-produced lazy numeric values, including `2^127`,
`-(2^127 + 1)` and larger heap-backed values, without displaying or inspecting
them before the measured call. Observe only materialization or conversion;
retain its result and perform assertions after observation ends. Require
observed allocations on a successful owned copy. Denial before the first
allocation observes none; a later denial may retain allocations admitted before
its first stop but allocates no denied payload or owned diagnostic. Exercise
second-row work and storage denials, asserting the actual row locus and original
bound remain unchanged after an attempted different charge or adapter call.
Native budget construction and a fresh observer are not replacements for those
shared-budget controls.

Bind each new test with a two-id attribute such as
`#[trace("TC-1331", "FR-531-AC-29")]` or
`#[trace("TC-1331", "FR-531-AC-30")]`. Bind AC-31, AC-32 and AC-33 with their own
two-id attributes; a source-enum locus test names AC-30 rather than the equality
materialization criterion.

## Expected Results

Each row's Expected column and each added criterion's complete value, refusal,
provenance and accounting assertions. Materialized rows alone grant no native
admission or execution authority. This procedure does not claim that a pending
implementation, generated backend or whole IR-721 campaign has passed.
