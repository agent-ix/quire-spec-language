---
id: FR-531
title: "Evaluate from a checked package read from its bytes alone"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-520
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-532
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-533
    type: references
  - target: ix://agent-ix/quire-protocol/FR-045
    type: references
  - target: ix://agent-ix/quire-protocol/FR-046
    type: references
---
# FR-531: Evaluate from a checked package read from its bytes alone

## Description

A consumer that reads a handoff directory (FR-517) with Contract IR's
`read_checked_package` holds a `quire_contract_model::CheckedPackageV2` and holds
no QSL source `CheckedPackage`, no `Emission` and no `AdmittedPackageRead`. Until
this requirement the QSL evaluation entries needed one of those: `EvidencePackage`
is built by `from_emission` or `from_read`, both over an `AdmittedPackageRead`,
which exists only for a package QSL itself emitted from source and which retains
that source (`AdmittedPackageRead::source`); and `CheckedPackageEvaluation` is
sealed and implemented for the in-process `CheckedPackage`, its reader-backed
evaluation view still taking the slot selection, the binder names and the source
`CheckedBody` from that retained source. FR-515 recorded an evaluator over
`formula_graph()` with no source graph as out of scope. It is in scope: the
consumer of the handoff, quire-protocol, evaluates compensation slots and temporal
evidence from the package alone, and has nothing else to pass.

This requirement states the one entry that takes a `CheckedPackageV2` and nothing
from the source side, what it admits, what it derives from the package's bytes,
how it refuses, how its outputs spell identity where the bytes carry no source
names or spans, and that it gives the same results and the same charges as the
routes that start from source. FR-532 states the temporal evidence call from this
entry; FR-533 states the protocol-slot call from it.

## Use case

quire-protocol reads `checked-package.json` of a handoff directory with
`read_checked_package`, holds the admitted package, and calls QSL to evaluate a
compensation's retry relation and to assess a temporal obligation. It builds the
evaluation value from the package it holds, plus the files beside it (the domain
package documents and the identity rows, below), and never compiles source.

## Semantic authority and boundary

QSL owns evaluation (FR-107, FR-390, FR-516); Contract IR owns the reader and the
package's admission. This requirement adds no checked-package member and no Contract
IR code. It does not re-verify what the reader admitted: a package the reader
admitted is the package, and a difference between it and some source is the
caller's concern, not a refusal here (FR-515-AC-24 states the refusals of the two
routes that do hold a source).

## Inputs

In `qsl_package` (behind `std`, no new dependency):

```rust
pub struct BarePackage<'p> { /* private */ }

impl<'p> BarePackage<'p> {
    pub fn new(package: &'p CheckedPackageV2) -> Self;
    pub fn with_supply(self, supply: BareSupply<'p>) -> Self;
    pub fn package(&self) -> &'p CheckedPackageV2;
}

pub struct BareSupply<'p> {
    pub identities: &'p ReadIdentityRows,
}

impl ReadIdentityRows {
    pub fn read(identities_json: &[u8], package: &CheckedPackageV2)
        -> Result<Self, IdentityFileRefusal>;
}
```

- `package` is the `CheckedPackageV2` that `read_checked_package` returned. A
  caller cannot build a `BarePackage` from bytes, from a decoded graph or from a
  term; `BarePackage` has no other constructor.
- `ReadIdentityRows` is the QSL-owned read-evidence wrapper of FR-390: the selected
  model identity rows (IR's `SelectedExactDeclarationRow` and
  `SelectedExactFieldRow`) paired with the package they were produced for. A
  consumer obtains it with `ReadIdentityRows::read(bytes, package)` from the
  `identities.json` of the handoff directory and the package (below), and by no
  other route. Only the protocol-slot call reads it (FR-533); the temporal
  evidence call reads nothing but the package (FR-532).
- The dependency closure is not a supply. `BarePackage` derives the admitted
  closure from the direct dependencies the reader retained in the package, which
  the consumer passed to `read_checked_package` as read evidence; a consumer
  builds no `AdmittedPackages`, which has no public constructor. QSL reads the
  retained dependencies through the read access Contract IR gives to the packages
  a `CheckedPackageV2` holds. That access is an IR dependency (IR-1325, a public
  read-only accessor for the retained direct dependencies; owner luna:IR:codex2): until it is public
  the slot call over a package that has dependencies is unmet parity, and a
  package with none is unaffected.
- The domain package documents (FR-056's `PackageInput`) stay a caller input of
  observation admission, as for the in-process route; they are bytes the caller
  holds, not source.

`qsl_eval::bare` re-exports `BarePackage`, implements `CheckedPackageEvaluation`
for it and gives `EvidencePackage` the constructor
`EvidencePackage::from_package(&'p BarePackage<'p>) -> Result<Self,
PackageMismatch>`.

## Outputs

The values of the existing calls, unchanged: `Result<ProtocolSlotEvaluation,
CallFailure>` (FR-390) and `Result<EvidenceResult, CallFailure>` (FR-516). The
entry defines no result type of its own.

## Behavior

### Requirements

- The entry SHALL take a `CheckedPackageV2` and no value of the source side.
- The entry SHALL derive every fact of the table below from the package's bytes.
- The bare route SHALL give the same result, the same refusal and the same charge
  log as the in-process route for one package and one request.
- The entry SHALL refuse a call only with a cause, dimension or reason named here
  or in FR-390, FR-515, FR-516 and FR-532.
- Every capability of the in-process route SHALL be available from the bare
  package or be listed in the inventory as unmet parity with its reason; none
  is dropped.

### What the entry derives from the bytes

| Fact | Where it is read | In-process route reads it from |
| --- | --- | --- |
| clause identity, `over`, activation, captures, requirement entries | `temporal_subject` (FR-515) over the package | the same view |
| formula operations, intervals, operand edges and `holds` atoms | `formula_graph()` | the checked formula, cross-checked to the view |
| instance-bound atoms | the checked predicate view's `binders()` (FR-515) | the same view |
| profile, and the evaluator profile it names | the lock's `profile_selections` row of the clause's `temporal_profile` law | the same row, and the source clause's profile |
| clock configuration and clock name | the `temporal_bindings` row keyed `{ declaration, role, ordinal }` | the same row, and the source clause's clock name |
| activation map | `activation_relation(package)` (FR-449 rows) | the same |
| package identity of a progress binding and of a result identity | the package's `name()` | the source identity labels |
| protocol slot, binders, binder types and levels, locations, callees | the admitted protocol nodes and `protocol_locations` rows (FR-390, FR-464) | the source `CheckedBody` and checked package |
| model selections and type environment of an observation | the package's `model_root` nodes and the caller's domain package documents | the source graph's model selections and scope |
| dependency closure | the direct dependencies the package retains from its read evidence | the source package's closure |
| clause name, capture names and capture value types | the occurrence's `temporal_clause_names` row, the captures argument's binding names and the typed result types of the capture initializer terms | the source clause's `name()` and `captures()` |
| arithmetic exactness and bounds | the lock and the nodes' types | the same, through the source |

Every row of the in-process column that names the source is replaced on the
bare route by the bytes column, and FR-532 and FR-533 state that the in-process
routes then read the bytes column too, so that one value exists per fact.

### What the bytes do not carry

The package holds no source text and no source spans beyond its source map (FR-528
integrity claims and `CheckedSourceMapEntry` regions), no source arena order of a
checked formula, and no clause name on the clause node itself (IR-1278 gates the
`name()` of FR-515). The declared name of each clause occurrence is carried by its
`temporal_clause_names` row, and the bare route SHALL use that row wherever the
evaluator reads a clause name. The outputs of the bare route therefore spell positions and keys by what
the bytes carry, and the in-process routes spell them the same way:

- a clause is its `CheckedNodeId` plus an occurrence `{ role, ordinal }`;
- an atom is its index among the distinct atom nodes, numbered by first use in
  `formula_graph()` pre-order (FR-532 "Atom
  index");
- a position and an instance are the request's own values;
- a binder is its `NodeKey`, and its declared name only where the admitted binder
  node carries one;
- a location is the one the package's own location rows give, and none where the
  wire carries none (FR-390: QSL never invents one);
- a progress binding and a result identity use the package `name()` and the clause
  occurrence, and carry no source authority or source identity label.

### Capability inventory

Every capability of `EvidencePackage` and of `evaluate_protocol_slot` is available
from the bare package, or is listed here with the reason it needs source:

| Capability | From the bare package |
| --- | --- |
| `EvidencePackage::from_emission`, `from_read` | unchanged; they hold a source and keep the cross-check of FR-515 |
| `EvidencePackage::from_package` | new; FR-532 |
| `with_progress_capacity`, `rows`, `temporal_subject`, `profile`, `clock`, `clock_of`, `subject`, `evaluate_temporal_evidence` | available; the rows are the package's own |
| the construction refusals of `PackageMismatch` | `AmbiguousName` applies; `ClauseRow`, `ProfileSelections` and `Bindings` need an `Emission` and have no bare route (FR-532) |
| `evaluate_protocol_slot` and `evaluate_selected_protocol_slot` | available; FR-533 |
| `AdmittedEvaluationView::admit_protocol_observation` | available; FR-533 |
| `call`, `call_supplied`, `call_observing_applications` (call by `QualifiedName`) | unmet parity: a `QualifiedName` is a source declaration name, which the package does not hold (FR-515 `name()`); a call by the callee's node identity is the slot route's own dispatch |
| `evaluate` (a `CheckedExpression`) | unmet parity: its argument is a source-checked value a bytes-only consumer cannot hold; the slot call is its package form |
| `evaluate_holds`, `evaluate_holds_traced`, `evaluate_guard`, `evaluate_hyper_holds`, `evaluate_step_predicate`, `evaluate_clause`, `evaluate_clause_set`, `evaluate_value_invariants`, `check_separation`, `evaluate_frame`, `evaluate_collection_write` | unmet parity: each takes a source-checked clause, node or state clause; the evidence route takes the `holds` valuations as request facts (FR-516), and no consumer of the handoff evaluates the others |

### The identity file

The directory FR-517 writes carries `identities.json`, written once beside
`checked-package.json`. Its encoding follows `selection.json`: one JSON object,
pretty-printed with two-space indent, members in the order below, closed (an
unknown member is a refusal), UTF-8, ending in one newline, byte-identical for
equal inputs.

| Member | Content |
| --- | --- |
| `format` | the string `quire.checked-handoff.identities/1` |
| `declarations` | the unit's `SelectedExactDeclarationRow` values in the order the emitter holds them; each is an object `{ "structural_node", "selected_document", "form", "original_identity", "exact_identity" }` with `structural_node` the wire node id string, `form` one of `record_value_type`, `object_type`, `union`, `declared_unit`, and `exact_identity` a tagged object `{ "kind": "record" \| "object" \| "union" \| "declared_unit", "value": <value> }` whose `value` is a node reference for `record` and `union`, an `EffectiveId` for `object` and a `UnitId` for `declared_unit`, each as the value encodings below state |
| `fields` | the unit's `SelectedExactFieldRow` values in emitter order; each is `{ "record_node", "selected_document", "record_original_identity", "field_original_identity", "member_id" }` |

Value encodings. A **node reference** (a `NodeKey`) is the wire node id string the
package spells that node with (`CheckedNodeId`, as `structural_node` does); the
`NodeKey` of the pinned kernel is a structural value, not a digest, so no digest
spelling applies to it, and `read` recovers the key from the package's wire graph.
An **`EffectiveId`** is `{ "chain": [ { "package": <text>, "node": <text> }, ... ] }`,
the root effective type first and then each member in owner order. A **`MemberId`**
(`member_id`) is `{ "owner": { "kind": "node", "node": <node reference> } | { "kind":
"effective", "effective": <EffectiveId> }, "member": <nonempty text> }`. A **`UnitId`**
is `{ "domain": "declared" | "compound", "digest": <64 lowercase hexadecimal
digits> }`, the 32 digest bytes in order. Text is UTF-8 as written, and a number
is never used for an identity.

Trust boundary. Rows read from a consumer-parsed file grant no authority: IR
states that a row constructed by a caller does not make its Exact identity
authoritative, and the file is trusted as `checked-package.json` is, as an artifact
of the producer the consumer chose to read. `read` verifies only what the package
can answer: that each `structural_node`, `record_node` and node reference is a node
of the package, and that each declaration's `form` matches its identity (`Unjoined`
otherwise), and IR's `SelectedExactIdentities` projection validates the structural
join. It does not verify that QSL's model stage issued an identity; a row that
passes the joins but is forged can change which exact identity a node is read as,
which the evaluator then checks against the admitted nodes and types of the slot.

`ReadIdentityRows::read(bytes, package)` parses the file, joins the rows to the
package by IR's `SelectedExactIdentities` projection, and never derives a row from
the package bytes, a display name or source text. `IdentityFileRefusal` is closed:
`Malformed` (not JSON or not this shape, an unknown or repeated member),
`Format` (another `format` string, naming it), `Unjoined` (a row whose
`structural_node` the package does not hold, or whose form does not match its
identity), naming the first row. A refusal returns no rows and charges nothing.
quire-protocol's admission input list (its FR-043) names this file beside
`checked-package.json`.

### Refusals

- A call SHALL refuse with FR-516's cause table (temporal) or FR-390's catalog
  refusals (protocol), unchanged. This requirement adds no call cause of its own; the cause
  `aggregate-sample-kind` is FR-516's (window aggregate atoms), and it adds the
  `clause-unreadable` reasons `profile-unknown`, `clause-name` and `capture-type`
  and the supply refusals below, which FR-516's cause table lists.
- A slot call on a `BarePackage` that has no supply SHALL refuse
  `ViewFailure::Unsupported` with `MissingIdentityWitness` at the package locus
  (FR-390). A supply whose rows name no admitted identity or another package
  refuses `UnknownIdentityWitness` or `ForeignIdentityWitness`. There is no
  dependency refusal: `read_checked_package` refuses `MissingImport` for a lock
  that selects a dependency the read evidence does not supply, so a package it
  admitted retains every dependency its lock selects, and the type system leaves
  no package in the state a `DependencyUnheld` refusal would name. Each refusal
  above is evaluation-input admission before S6a, category
  unsupported, exit 21, `CallFailure::Admission` carrying the catalog pair
  `unknown_required_feature`/`unsupported-feature`, with nothing charged.
- A package that carries no `temporal_bindings` rows, or none for a clause
  occurrence, is not refused: the clause's clock row is absent and the call returns
  the result FR-516 gives a withdrawn clock row (`activation = Unavailable`).
  A package whose lock has no `profile_selections` row for a clause refuses
  `clause-unreadable` with reason `profile-selection` (FR-515); a row whose
  definition identity names no profile the evaluator evaluates refuses
  `clause-unreadable` with reason `profile-unknown`; one that names an
  infinite-trace profile returns the `not-finite-window-profile` temporal refusal
  dimension, as on the in-process route.
- The 18 temporal refusal dimensions of FR-516, the four `PackageMismatch`
  variants and FR-390's refusals are the whole vocabulary of the bare route.

### Charges

The bare route charges exactly the counters, at exactly the charge points, that
the in-process route charges for the same package and the same request (FR-523's
thirteen temporal counters; FR-255's ten value counters, attributed per binder as
FR-520-AC-11 states). Construction of a `BarePackage`,
supply admission and view creation charge nothing and accept no `Meter`.

### Relationship to the consumers

quire-protocol FR-045 evaluates compensation slots, and FR-046 assesses temporal
evidence, through this entry and pass it no source package and no emission.
FR-515 and FR-516 state the evidence package and the call; FR-520 states the
outcome map. Each carries a "Consumer entry" section that points here.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-531-AC-1 | In a test crate whose dependencies are `quire-contract-model`, `qsl-package` and `qsl-eval` and which names no `qsl_semantics::CheckedPackage`, no `Emission` and no `AdmittedPackageRead`, the package of **R**'s handoff directory (FR-517) read with `read_checked_package` gives a `BarePackage` by `BarePackage::new`, and from it a temporal evidence call (FR-532) and a protocol-slot call (FR-533) each return a result. | Test (TC-1331) |
| FR-531-AC-2 | `BarePackage` has no constructor from bytes, from a decoded graph or from a term, and no public signature of `BarePackage`, `BareSupply`, `EvidencePackage::from_package` or the `BarePackage` evaluation names `CheckedPackage`, `Emission` or `AdmittedPackageRead`; compile-fail probes of each construction fail, and an API-surface inspection lists the signatures. | Test (TC-1331) |
| FR-531-AC-3 | The evaluation code reached from `BarePackage` does not read a source graph: the same calls over a `BarePackage` whose package was read in a run where the source unit was never compiled give the results the calls give where it was (TC-1331 row 1 runs the calls in a child run that is given the handoff directory only). | Test (TC-1331) |
| FR-531-AC-4 | Over **R**'s three packages the bare route evaluates every temporal clause form (event position, fixed sample, timestamped event) and every protocol slot kind of FR-390's slot table that **R** holds; a node that is not a temporal clause refuses `clause-root-unresolved` and a slot node that is not a protocol node refuses as FR-390-AC-4 states, each with an empty charge log. | Test (TC-1331) |
| FR-531-AC-5 | Over a `BarePackage` with no supply, a protocol-slot call refuses `MissingIdentityWitness`; with rows of another package, `ForeignIdentityWitness`; with rows naming no admitted identity, `UnknownIdentityWitness`. Each is `CallFailure::Admission` with `unknown_required_feature`/`unsupported-feature`, category unsupported, exit 21, and the meter untouched. A temporal evidence call over the same `BarePackage` with no supply returns its result; and `read_checked_package` over bytes whose lock selects a dependency the evidence does not supply refuses `MissingImport`, so no `BarePackage` exists for it and no dependency refusal is defined. | Test (TC-1331) |
| FR-531-AC-6 | Over a package whose `temporal_bindings` rows are absent, the call returns `activation = Unavailable` with the `Missing` non-value for the clock row; over a lock with no `profile_selections` row for the clause, `clause-unreadable` with reason `profile-selection`; over a lock whose row names a definition no profile of the evaluator, `clause-unreadable` with reason `profile-unknown`; over a row naming an infinite-trace profile, the `not-finite-window-profile` temporal refusal. Each has the charges FR-516 gives its counterpart over a source. | Test (TC-1331) |
| FR-531-AC-7 | No result of the bare route holds, in its canonical bytes or its fields, a clause name, a source label or a source span: over **R**, the encoded `EvidenceResult` and `ProtocolSlotEvaluation` values do not contain the unit's `source:` labels or a byte region of the source map, and a clause name appears only as the `temporal_clause_names` row of its occurrence gives it, identically on every route. Identity is spelled as the "What the bytes do not carry" list states, and the progress binding of a result holds `package.name()`. | Test (TC-1331) |
| FR-531-AC-8 | The handoff directory of **R** holds `identities.json` with `format` `quire.checked-handoff.identities/1`; `ReadIdentityRows::read` over it and the package gives a slot call that evaluates; the file equals, byte for byte, the encoding above of the in-process unit's rows, and two writes of equal requests give equal files; omitting the file from a `BareSupply` makes the slot call refuse `MissingIdentityWitness` while the temporal call is unaffected. | Test (TC-1331) |
| FR-531-AC-12 | `ReadIdentityRows::read` over a file that is not JSON, one with an unknown or repeated member, one with another `format`, one with a row whose `structural_node` the package does not hold and one whose `form` does not match its identity returns `Malformed`, `Malformed`, `Format`, `Unjoined` and `Unjoined` respectively, naming the member, the format string or the first row, and returns no rows. | Test (TC-1331) |
| FR-531-AC-13 | The identity file of **R** encodes every identity as the "Value encodings" state: a node reference is the wire node id string of the package, an `EffectiveId` has the `chain` list root first, a `MemberId` has `owner` and `member`, and a `UnitId` has `domain` and 64 lowercase hexadecimal digits; `read` returns rows equal to the writer's, an uppercase or short digest returns `Malformed`, and a node reference the package does not hold returns `Unjoined`. A forged row that passes the joins is accepted by `read` and the slot call over it evaluates against the admitted nodes, so the test records that `read` makes no claim of authority. | Test (TC-1331) |
| FR-531-AC-9 | For each public method of `EvidencePackage` and each method of the `CheckedPackageEvaluation` trait, enumerated by a test that reads those two items' declarations in the crate source with `syn` and compares the names to the first column of the inventory table, the table has exactly one row: available from the bare package, or listed as not carried with a reason; a method added to either type without a row fails the test. | Test (TC-1331) |
| FR-531-AC-10 | The charge log of each call over the bare package equals the charge log of the same call over the in-process route (FR-532-AC-1, FR-533-AC-1); `BarePackage::new`, `with_supply` and view creation take no `Meter` and a call that refuses at supply admission leaves a caller's meter byte-identical. | Test (TC-1331) |
| FR-531-AC-11 | A search of FR-515 for the phrases `bytes-only` and `separate future capability` and of FR-515, FR-516 and FR-520 for the heading `Consumer entry` finds that FR-515 no longer states that an evaluator over `formula_graph()` with no source graph is out of scope, and FR-515, FR-516 and FR-520 each carry a "Consumer entry" section that names this requirement. | Inspection (TC-1331) |

## Dependencies

- **Upstream:** FR-337, FR-383 (the nodes and rows), FR-390 (the reader view and
  its identity rows; the reader-view criteria it states are the ones the bare slot
  call rests on), FR-515, FR-516, FR-517, FR-520; Contract IR `read_checked_package`
  and `SelectedExactIdentities`; QSL-1377 (selected-model identity-row production).
- **Downstream:** quire-protocol FR-045 and FR-046; FR-532 and FR-533.
