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
pub struct BarePackage<'p> { /* private; neither Copy nor Clone */ }

impl<'p> BarePackage<'p> {
    pub fn new(package: &'p CheckedPackageV2) -> Self;
    pub fn with_supply(self, supply: BareSupply<'p>, budget: &mut NativeJoinBudget)
        -> Result<Self, ExactTypeProjectionError>;
    pub fn package(&self) -> &'p CheckedPackageV2;
}

pub struct BareSupply<'p> {
    pub identities: &'p ReadIdentityRows,
    pub equality: Option<&'p ReadEqualityRows>,
}

impl ReadIdentityRows {
    pub fn read(identities_json: &[u8], package: &CheckedPackageV2)
        -> Result<Self, IdentityFileRefusal>;
}

impl ReadEqualityRows {
    pub fn read(equality_json: &[u8], package: &CheckedPackageV2)
        -> Result<Self, EqualityFileRefusal>;
}
```

- `package` is the `CheckedPackageV2` that `read_checked_package` returned. A
  caller cannot build a `BarePackage` from bytes, from a decoded graph or from a
  term; `BarePackage` has no other constructor.
- `ReadIdentityRows` is the QSL-owned read-evidence wrapper of FR-390: the selected
  declaration/field identity rows and final source-record rows paired with the
  package they were produced for. A
  consumer obtains it with `ReadIdentityRows::read(bytes, package)` from the
  `identities.json` of the handoff directory and the package (below), and by no
  other route. Only the protocol-slot call reads it (FR-533); the temporal
  evidence call reads nothing but the package (FR-532).
- `ReadEqualityRows` is the equality counterpart: the producer's equality application
  rows, equality fact rows and selected source-unit rows (IR's
  `SelectedEqualityApplicationRow`, `SelectedEqualityFactRow` and
  `SelectedSourceUnitRow`) paired with the package they were produced for. A consumer
  obtains it with `ReadEqualityRows::read(bytes, package)` from the `equality.json` of
  the handoff directory and by no other route. Only a protocol-slot call whose walk
  holds an `==` or `!=` application reads it (FR-533 "Equality slots"); `equality` is
  `None` for a supply without it.
- The dependency closure is not a supply. `BarePackage` derives the admitted
  closure from the direct dependencies the reader retained in the package, which
  the consumer passed to `read_checked_package` as read evidence; a consumer
  builds no `AdmittedPackages`, which has no public constructor. QSL reads the
  retained dependencies through the read access Contract IR gives to the packages
  a `CheckedPackageV2` holds. That access is `CheckedPackageV2::direct_dependencies()`,
  a public read-only iterator over the retained direct dependencies in
  lock-selection order; `BarePackage::dependencies()` reads the closure from it,
  each dependency once, nearest first.
- The domain package documents (FR-056's `PackageInput`) stay a caller input of
  observation admission, as for the in-process route; they are bytes the caller
  holds, not source.

`NativeJoinBudget` and `ExactTypeProjectionError` are Contract IR's (FR-358).
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
checked formula, and no clause name on the clause node itself (FR-515 reads `name()` from the
`temporal_clause_names` row). The declared name of each clause occurrence is carried by its
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
| `with_progress_capacity`, `with_max_window_values`, `rows`, `temporal_subject`, `profile`, `clock`, `clock_of`, `subject`, `evaluate_temporal_evidence` | available; the rows are the package's own |
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
| `format` | the string `quire.checked-handoff.identities/3` |
| `declarations` | the unit's `SelectedExactDeclarationRow` values in the order the emitter holds them; each is an object `{ "structural_node", "selected_document", "form", "original_identity", "exact_identity" }` with `structural_node` the wire node id string, `form` one of `record_value_type`, `object_type`, `union`, `declared_unit`, and `exact_identity` a tagged object `{ "kind": "record" \| "object" \| "union" \| "declared_unit", "value": <value> }` whose `value` is a node reference for `record` and `union`, an `EffectiveId` for `object` and a `UnitId` for `declared_unit`, each as the value encodings below state |
| `fields` | the unit's `SelectedExactFieldRow` values in emitter order; each is `{ "record_node", "selected_document", "record_original_identity", "field_original_identity", "member_id" }` |
| `source_records` | every final emission's source-owned record row, in final row order: `{ "structural_node", "source_authority", "source_identity", "qualified_declaration", "record_key", "members" }`; each ordered member is `{ "original_member_identity", "admitted_name", "presence", "type_node", "value_type" }` |

| `source_unions` | every final emission's `SelectedSourceUnionRow`, in producer order: `{ "structural_node", "source_authority", "source_identity", "qualified_declaration", "type_handle", "settled_key", "members" }`; each ordered member is `{ "declaration", "identifier", "payload_nodes", "payload" }` |

All five members are required. The reader SHALL reject `/1`, `/2`, an absent member,
an unknown or repeated member and a malformed row; it SHALL NOT supply an empty
default, read a legacy variant or silently omit a row that cannot be represented.
The writer SHALL emit an empty `source_records` array only when the admitted
producer set is actually empty. The format reader
does not certify whole-witness completeness from an empty array: a request for a
record whose row was omitted reaches native Exact projection and returns its
typed `UnavailableIdentityWitness`. The public bare view maps that native failure
to its existing `MissingTypeRepresentation` at the requested admitted node,
without a structural-key success fallback; this carrier adds no public native
diagnostic-payload variant.

The writer SHALL acquire `source_unions` from the same final `Emission`, retaining
the original `type_handle`, `settled_key`, each sealed member's `declaration` and
`identifier`, and the ordered final `payload_nodes` and original Exact `payload`
types. The three key fields use the original opaque key encoding below; payload
types use its original-value grammar and depth limits. The reader SHALL join
these rows through native `with_source_union_rows`, rejecting foreign, stale or
conflicting rows. `source_unions` is required even when empty; an empty list
does not witness an authored union. No display name or graph-derived key
substitutes for an absent producer row.
Selected model population catalogs are read from the retained native
`CheckedPackageV2` using the Contract IR capability delivered by IR-1358; the
identity carrier does not repeat that catalog. QSL-1762's selected object
closure uses the same declaration/field and source-record families and does
not add a population catalog to this carrier.

The writer SHALL acquire source-record rows from the same final `Emission` as
`checked-package.json`, retaining its final structural and member type IDs,
source owner, original declaration segments, original record key, original member
identities, final admitted names, presence and exact value types in order. A
rewritten monitor-root row therefore keeps the producer's original record key
while carrying the final renamed bindings. The writer SHALL NOT substitute an
emitted structural node key for an original opaque record key.

The new rows distinguish **graph references** (`structural_node`, `type_node`) from **original opaque values**. `record_key` is the shared
`qsl_foundation::node_wire` shape `{ "package", "path", "ordinal" }`, whose path
contains text or nested node-key objects. It preserves the producer key as a
value, never as a graph index or a key inferred from a name. A source member's
`value_type` uses the equality file's complete value-type grammar, except every
NodeKey inside it (composite, enum variant owner, declared or compound unit) is
this original-value shape, rather than a graph reference. Integers and rational
components remain canonical decimal text; presence is `required` or `optional`.

Each original key SHALL contain at most 32 nested node-key objects, counting the
outer key. The writer SHALL check this limit iteratively before recursive JSON
work and the reader SHALL refuse a key above it as `Malformed` at
`source_records`. The bridges use the shared iterative node text codec and SHALL
NOT construct or recursively discard an unbounded intermediate JSON value.

The bytes reader SHALL retain these original producer values and join each source
record through Contract IR's `with_source_record_rows` before exposing its Exact
record type. A healthy carried row SHALL use `source_record_view`; the legacy
`UnavailableIdentityWitness` structural-key substitution is not success for a
carried source record. Stale, foreign or conflicting supplied record rows return
a typed native join refusal with no partial file read result. A missing row is
refused by native projection when that record is requested. The carrier SHALL
NOT introduce a QSL mirror validator or claim that record-row joining qualifies
whole-witness validate/rebind owned by IR-1360.

Value encodings. A **node reference** (a `NodeKey`) is the wire node id string the
package spells that node with (`CheckedNodeId`, as `structural_node` does); the
`NodeKey` of the pinned kernel is a structural value, not a digest, so no digest
spelling applies to it, and `read` recovers the key from the package's wire graph.
An **`EffectiveId`** is `{ "chain": [ { "package": <text>, "node": <text> }, ... ] }`,
the root effective type first and then each member in owner order. A **`MemberId`**
(`member_id`) is `{ "owner": { "kind": "node", "node": <node reference> } | { "kind":
"effective", "effective": <EffectiveId> }, "member": <nonempty text> }`. A **`UnitId`**
of a `declared_unit` row is `{ "kind": "declared", "node": <node reference> }`, the
declared unit's node. The pinned `UnitId` is `Declared(NodeKey)` or
`Compound(list of (NodeKey, Integer))`, with no digest, so no digest spelling exists.
A compound unit is the evaluator's own product over declared root units and is no
selected declaration: the `declared_unit` form takes only a declared unit, `read`
refuses any other `kind` as `Malformed`, and the file carries no compound unit. A
slot over a quantity of a compound unit needs the reader projection of compound
`UnitId` (QSL-1626), which is the existing unmet-parity dependency of FR-533, not a
gap of the file. Text is UTF-8 as written, and a number is never used for an
identity.

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
identity), naming the first row, and `Native` (a typed IR row-join refusal,
retaining its code and native structural locus). Thus tagging a record node as a
declared unit is a native mismatched-witness refusal even when the tag and key
encoding are individually well-shaped. A refusal returns no rows and charges no
scalar evaluation meter.
quire-protocol's admission input list (its FR-043) names this file beside
`checked-package.json`.

### The equality file

The directory FR-517 writes carries `equality.json`, written once beside
`identities.json`, with its encoding rules: one JSON object, pretty-printed with
two-space indent, members in the order below, closed (an unknown or repeated member
is a refusal), UTF-8, ending in one newline, byte-identical for equal inputs. The
rows are the producer's witness, issued at checking time (FR-533 "Equality slots");
the writer builds each row from the in-process unit's rows, and joins every node and
key member to the wire node id the emitted package spells that node with
(`WireGraph::of_package_bytes` over the emitted bytes, as the inventory file does for
its `node` member), because an in-process `CheckedNodeId` or `NodeKey` carries
structural path segments and the wire form carries operand indices. A node the
package does not name is the writer's `Error::Unnameable`.

The file's value-type vocabulary covers every `quire_exact::ValueType` variant (the
tags listed under "Value encodings" below, `carried_quantity`, `map` and
`bounded_quantity` included), so the writer names the value type of any equality
fact. FR-517-AC-14's `Full` recover compares a receipt sum with a captured target
that FR-517-AC-13 types as a carried quantity; whether the checker records that
operand type as `Quantity` or as `CarriedQuantity` is not established by FR-505 or
FR-517, and the vocabulary admits both.

| Member | Content |
| --- | --- |
| `format` | the string `quire.checked-handoff.equality/1` |
| `applications` | the unit's equality application rows in emitter order; each is `{ "application", "occurrence", "source_map_entry", "node_key", "location" }` in that member order |
| `facts` | the unit's equality fact rows in emitter order; each is `{ "application", "checked_application", "occurrence", "source_map_entry", "structural_key", "structural_node", "owner", "value_type", "fact" }` in that member order |
| `source_units` | the unit's selected source-unit rows in emitter order; each is `{ "structural_node", "source_authority", "source_identity", "qualified_declaration", "unit_id" }` in that member order |

Leaf encodings. Each is a JSON value whose members appear in the order given, with
no other member; a reader refuses an unknown or repeated member as `Malformed`.
Integers are JSON integers (`u64` or `usize` values, never a float or a string);
text is UTF-8 as written.

- A **node id** (the package's `CheckedNodeId`, used for `application`,
  `checked_application`, `structural_node`, and every `*_node` member) is the JSON
  value `serde` gives that type, the form `checked-package.json` spells a graph
  node's `node_id` with: `{ "package": <text>, "path": [ <segment>, ... ],
  "ordinal": <integer> }`, a segment being text (a name) or `{ "node": <integer> }`
  (a graph-local operand index). This is what "The identity file" above calls the
  wire node id; the identity file uses the same value.
- A **node reference** (a `NodeKey`: `node_key`, `structural_key`, `base`, `target`,
  `source_declaration`, `declaration`, `member`, and a `node` member) is the node id
  of the package node that key names, as "The identity file" states for a node
  reference; `read` recovers the key from the package's wire graph.
- An **occurrence** (`CheckedOccurrence`) is `{ "role": <role>, "ordinal":
  <integer> }`, `<role>` one of `declaration`, `type`, `expression`, `anchor`,
  `claim`, `generated`.
- A **source map entry** (`CheckedSourceMapEntry`) is `{ "node_id": <node id>,
  "role": <role>, "ordinal": <integer>, "regions": [ { "source": { "authority":
  <text>, "identity": <text> }, "start": <integer>, "end": <integer> }, ... ] }`,
  `regions` in the order the entry holds them and never empty.
- An **owner** (`CheckedNodeOwner`) is `null` or a tagged object: `{ "kind":
  "model_root", "identity": <text> }`, `{ "kind": "source", "authority": <text>,
  "identity": <text> }` or `{ "kind": "model", "identity": <text>, "node": <text> }`.
- A **location** (`quire_exact::Location`) is `{ "node": <node reference>,
  "occurrence": { "role": <text>, "ordinal": <integer> } }`, `role` the text of the
  location's origin role.
- A **checked rational** (`CheckedRational`) is `{ "numerator": <text>,
  "denominator": <text> }`, both canonical decimal integer strings: an optional `-`
  for a negative numerator, no `+`, no leading zero, `0` for zero, and a positive
  denominator.
- A **unit id** is `{ "kind": "declared", "node": <node reference> }`, or in a
  fact row `{ "kind": "compound", "terms": [ { "node": <node reference>,
  "exponent": <canonical decimal integer string> }, ... ] }` with the terms in the
  compound's own order. A `source_units` row takes the declared form only, as
  "The identity file" does for `declared_unit`.
- A **value type** (`quire_exact::ValueType`) is `null` or a tagged object `{
  "kind": <tag> [, members] }` with the tag and members: `boolean`, `integer`,
  `string`, `uuid`, `timestamp` (no member); `int` `{ "lower", "upper" }`;
  `rational` `{ "numerator": <interval>, "denominator": <interval> }`; `decimal`
  `{ "lower", "upper", "min_scale", "max_scale", "rounding" }`; `float` `{ "width",
  "rounding" }`; `quantity` `{ "unit": <unit id> }`; `text` `{ "min", "max",
  "profile" }`; `enum` `{ "ordered": <boolean>, "variants": [ { "owner": <node
  reference>, "case": <text> }, ... ] }`; `option` `{ "element": <value type> }`;
  `composite` `{ "node": <node reference> }`; `collection` `{ "collection": <kind>,
  "element": <value type>, "bound": null | { "minimum", "maximum" } }`; `reference`
  `{ "effective": <effective id> }`; `population` `{ "maximum": null | <integer> }`; `carried_quantity` `{ "unit":
  <unit id>, "carrier": <carrier> }`; `bounded_quantity` `{ "unit": <unit id>, "min":
  null | <quantity>, "max": null | <quantity> }`; `map` `{ "values": <value type>,
  "bound": null | { "minimum", "maximum" } }` (a map's keys are Text, so the member
  names no key type). A *quantity* is `{ "magnitude": <checked rational>, "unit":
  <unit id> }`. A *carrier* is a tagged object `{ "kind": "decimal", "lower", "upper",
  "min_scale", "max_scale", "rounding" }`, `{ "kind": "integer", "lower", "upper",
  "rounding" }` or `{ "kind": "rational", "numerator": <interval>, "denominator":
  <interval> }`, with the members and spellings of `decimal`, `int` and `rational`
  above. For example a `map` of `bounded_quantity` values over the unit node `["U"]` of package
  `p`, with `min` null, is, as a JSON value with members in the order shown (the
  bound members are JSON integers, the magnitude components canonical decimal
  strings): `{ "kind": "map", "values": { "kind": "bounded_quantity", "unit": {
  "kind": "declared", "node": { "package": "p", "path": ["U"], "ordinal": 0 } },
  "min": null, "max": { "magnitude": { "numerator": "5", "denominator": "1" },
  "unit": { "kind": "declared", "node": { "package": "p", "path": ["U"], "ordinal":
  0 } } } }, "bound": { "minimum": 0, "maximum": 4 } }`.
  An *interval* is `{ "lower": <text>, "upper": <text> }` and `lower`, `upper` of
  `int` and `decimal` are canonical decimal integer strings; `rounding` is one of
  `exact`, `toward_zero`, `toward_positive`, `toward_negative`, `nearest_even`,
  `nearest_away`; `width` one of `binary32`, `binary64`; `profile` one of
  `unicode_scalars`, `nfc`, `nfd`, `nfkc`, `nfkd`, `binary_utf8`; `<kind>` one of
  `sequence`, `set`, `bag`, `ordered_set`; an *effective id* is as "The identity
  file" encodes it.
- A **fact** is `{ "kind": <tag>, ... }` with the members of the same-named IR fact
  in the order IR declares them: `operand_type` `{ "position", "role" }`; `unit` `{
  "unit_id", "quantity_type": <node id>, "operand_position", "role" }`;
  `dimension_declaration` `{ "unit_id", "operand_position", "role", "ordinal" }`;
  `dimension_term` `{ "unit_id", "operand_position", "ordinal", "base": <node
  reference>, "base_node": <node id>, "exponent": <text, as the row holds it> }`;
  `conversion_edge` `{ "unit_id", "operand_position", "ordinal", "target": <node
  reference>, "target_node": <node id>, "scale": <checked rational>, "offset":
  <checked rational> }`; `text_plan_type` `{ "position", "role" }`; `enum_member` `{
  "source_declaration": <node reference>, "source_declaration_node": <node id>,
  "declaration": <node reference>, "declaration_node": <node id>, "member": <node
  reference>, "member_node": <node id>, "case": <text>, "ordinal", "ordered":
  <boolean> }`. A `role` of `operand_type`, `unit` and `text_plan_type` is `source`
  or `conversion_target`; that of `dimension_declaration` is `unit_dimension`,
  `conversion_dimension`, `compound_base` or `base_dimension`.

Vector. A unit with one equality application `E` (package `p`, node path `["E"]`)
and one source-unit row for the declared unit `U` writes this file, the rows built
from `E` at occurrence `expression` 0 with one source region of source
`{ "authority": "a", "identity": "s" }` bytes 4 to 9, the operand-type fact of
position 0 role `source` with value type `string`, and `U` owned by source `a`/`s`
at declaration `["U"]`. The bytes are exactly these lines, each ending in a line
feed, written as `serde_json`'s pretty printer writes it (two spaces per level, every
object and array member on its own line, an empty array as `[]`), with no trailing
space. The source unit `U` of the vector is owned by source `a`/`s` at declaration
`["U"]`:

```
{
  "format": "quire.checked-handoff.equality/1",
  "applications": [
    {
      "application": {
        "package": "p",
        "path": [
          "E"
        ],
        "ordinal": 0
      },
      "occurrence": {
        "role": "expression",
        "ordinal": 0
      },
      "source_map_entry": {
        "node_id": {
          "package": "p",
          "path": [
            "E"
          ],
          "ordinal": 0
        },
        "role": "expression",
        "ordinal": 0,
        "regions": [
          {
            "source": {
              "authority": "a",
              "identity": "s"
            },
            "start": 4,
            "end": 9
          }
        ]
      },
      "node_key": {
        "package": "p",
        "path": [
          "E"
        ],
        "ordinal": 0
      },
      "location": {
        "node": {
          "package": "p",
          "path": [
            "E"
          ],
          "ordinal": 0
        },
        "occurrence": {
          "role": "expression",
          "ordinal": 0
        }
      }
    }
  ],
  "facts": [
    {
      "application": {
        "package": "p",
        "path": [
          "E"
        ],
        "ordinal": 0
      },
      "checked_application": {
        "package": "p",
        "path": [
          "E"
        ],
        "ordinal": 0
      },
      "occurrence": {
        "role": "expression",
        "ordinal": 0
      },
      "source_map_entry": {
        "node_id": {
          "package": "p",
          "path": [
            "E"
          ],
          "ordinal": 0
        },
        "role": "expression",
        "ordinal": 0,
        "regions": [
          {
            "source": {
              "authority": "a",
              "identity": "s"
            },
            "start": 4,
            "end": 9
          }
        ]
      },
      "structural_key": {
        "package": "p",
        "path": [
          "E"
        ],
        "ordinal": 0
      },
      "structural_node": {
        "package": "p",
        "path": [
          "E"
        ],
        "ordinal": 0
      },
      "owner": {
        "kind": "source",
        "authority": "a",
        "identity": "s"
      },
      "value_type": {
        "kind": "string"
      },
      "fact": {
        "kind": "operand_type",
        "position": 0,
        "role": "source"
      }
    }
  ],
  "source_units": [
    {
      "structural_node": {
        "package": "p",
        "path": [
          "U"
        ],
        "ordinal": 0
      },
      "source_authority": "a",
      "source_identity": "s",
      "qualified_declaration": [
        "U"
      ],
      "unit_id": {
        "kind": "declared",
        "node": {
          "package": "p",
          "path": [
            "U"
          ],
          "ordinal": 0
        }
      }
    }
  ]
}
```

`ReadEqualityRows::read(bytes, package)` parses the file and joins every wire node id
string to a node of the package; it derives no row from the package bytes, a display
name or source text, and grants no authority beyond what IR's `admitted_equality`
validates when a decision is read. `EqualityFileRefusal` is closed: `Malformed` (not
JSON or not this shape, an unknown or repeated member or row member, an unknown fact
tag, a non-integer number), `Format` (another `format` string, naming it) and
`Unjoined` (a row whose node id string the package does not hold, or whose
application is not an equality application of the package), naming the first row. A
refusal returns no rows and charges nothing. quire-protocol's admission input list
names this file beside `identities.json`.

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
- A slot call whose walk reaches an `==` or `!=` application (FR-533 "Equality
  slots"), over a supply with no
  equality rows, refuses `ViewFailure::Unsupported` with `MissingEqualityWitness`;
  with rows of another package, `ForeignEqualityWitness` (decided before any row is
  joined, by comparing the rows' package name with the package's name; rows of a
  different package that has the same name are not detected); with rows of the package that hold no row for an equality application the
  walk reaches, or that Contract IR does not admit, `UnknownEqualityWitness`. Each is
  evaluation-input admission like the identity-witness refusals above, at the
  `Package` locus, which the bytes route reports as `CallAdmissionLocus::Package`
  (FR-533 "Refusals and loci of the bare slot call"). A slot call whose walk holds no
  equality application does not read the equality rows and is not refused for their
  absence.
- A package that carries no `temporal_bindings` rows, or none for a clause
  occurrence, is not refused: the clause's clock row is absent and the call returns
  the result FR-516 gives a withdrawn clock row (`activation = Unavailable`).
  A package whose lock has no `profile_selections` row for a clause refuses
  `clause-unreadable` with reason `profile-selection` (FR-515); a row whose
  definition identity names no profile the evaluator evaluates refuses
  `clause-unreadable` with reason `profile-unknown`; one that names an
  infinite-trace profile returns the `not-finite-window-profile` temporal refusal
  dimension, as on the in-process route.
- The 18 temporal refusal dimensions of FR-516, the five `PackageMismatch`
  variants and FR-390's refusals are the whole vocabulary of the bare route.

### The native join budget

Joining the supply's identity rows to the package is native work and native
logical storage, which Contract IR accounts for in one caller-owned
`NativeJoinBudget` (Contract IR FR-358). Its work and storage are not the
`Meter`'s counters and are not an input-byte count. QSL states the budget as
follows.

- **One finite budget per pass, built by the pass entry.** The entry of a pass
  (the read's evaluation view, `BarePackage::with_supply`, the temporal
  evidence constructors that hold a source, the handoff writer) builds or is
  given one `NativeJoinBudget` of finite work and storage and passes that same
  value to every join of the pass. No helper below an entry builds, defaults,
  resets or enlarges a budget, and no entry has an unbounded form. The helpers
  `selected_exact_identities_from_rows`, `SelectedModels::exact_identities`,
  `CheckedGraph::selected_exact_identities` and `ReadIdentityRows::selected`
  take `&mut NativeJoinBudget` directly, because `qsl-semantics` is below
  `qsl-package` and cannot name the package limits.
- **Where the ceilings come from.** `V2ReadLimits` (FR-255's `i2.*` row owner)
  gains the required member `storage`, the logical-storage ceiling, and has no
  `Default`; `V2ReadLimits::bounded(storage)` states it and takes the other
  members from Contract IR's bounded reader limits. A read's view builds its
  budget once from the read's `work` and `storage`. The storage ceiling is a
  count of logical ownership units, never derived from `artifact_bytes`, and
  is the `i2.native_join_storage` setting (FR-255); `V2ReadLimits.work` is
  `i2.work_units`. The table default 16777216 applies only where the outer
  configuration reads the `i2.*` settings table: the compile entry's
  `SpineLimits` supplies it, and a request's `native_join` limits and a test
  fixture state their finite values explicitly. Nothing below an entry
  supplies a budget.
- **One join, then borrowing.** `with_supply` joins the supply once, moves the
  joined identities into the `BarePackage`, and every later selection, world,
  projection and temporal call borrows them. A `BarePackage` is therefore
  neither `Copy` nor `Clone`, and no call re-joins or copies the identities.
- **Exact projection shares the admission budget.** The source evaluation view
  and supplied bare package retain the owner `ExactTypeProjection` answers
  needed by their protocol world and slots under that same native budget.
  Unsupported owner answers remain available to the reading stage; a denied
  native charge refuses admission. Subsequent world, slot and capture reads
  borrow these answers and never create a replacement allowance. A bytes-only
  temporal evidence entry without a supply takes the caller's native budget
  explicitly for its capture projections, as do native report admission and
  evaluation entries. These native charges remain separate from the scalar
  evaluator's `Meter`.
- **Witness defects keep their stage.** A missing, foreign or structurally
  unknown identity witness is not refused by `with_supply`; it is held and
  refused, with the order FR-390 states, by the call that reads the identities
  (FR-531-AC-5). `with_supply` returns an error only for a denied native charge.
- **Denial is its own typed refusal.** A denied work or storage charge returns
  Contract IR's `ExactTypeProjectionError` with code `ExhaustedWork` or
  `ExhaustedStorage`, its locus and its attempted charge, and no
  `BarePackage`. It is not an `UnknownIdentityWitness`, not an
  `InternalFault` and not an empty set of identities. A denial surfaced through
  the stage-failure or diagnostic path is a `stage_limit_exceeded`-class refusal
  naming `i2.work_units` or `i2.native_join_storage`; `ExhaustedWork` and
  `ExhaustedStorage` stay the library form. The same denial from a
  view's join is `ViewFailure::Exhausted`, from an evidence constructor
  `PackageMismatch::Exhausted`, and from the handoff writer
  `Error::NativeJoinExhausted`. `qsl-eval`'s `CallFailure` has no arm for it:
  the denial happens at the pass entry, before any call that returns one.
  A view or writer that owns the original ceilings retains the native error
  together with a `NativeJoinLimitError`: its `limit` is the diagnostic
  `LimitExceeded` with the original configured bound and the cumulative
  attempted count (consumed plus the first unpaid charge), and its catalog
  fields include the setting. The native error retains its structural locus.

### Charges

The bare route charges exactly the counters, at exactly the charge points, that
the in-process route charges for the same package and the same request (FR-523's
thirteen temporal counters; FR-255's ten value counters, attributed per binder as
FR-520-AC-11 states). Construction of a `BarePackage`,
supply admission and view creation charge nothing to the `Meter` and accept
none; supply admission and view creation charge only the caller's
`NativeJoinBudget`.

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
| FR-531-AC-5 | Over a `BarePackage` with no supply, a protocol-slot call refuses `MissingIdentityWitness`; with rows of another package, `ForeignIdentityWitness`; with rows of the package naming no admitted identity, `UnknownIdentityWitness`; rows of another package are `ForeignIdentityWitness` also when they name no admitted identity. Each is `CallFailure::Admission` with `unknown_required_feature`/`unsupported-feature`, category unsupported, exit 21, and the meter untouched. A temporal evidence call over the same `BarePackage` with no supply returns its result; and `read_checked_package` over bytes whose lock selects a dependency the evidence does not supply refuses `MissingImport`, so no `BarePackage` exists for it and no dependency refusal is defined. | Test (TC-1331) |
| FR-531-AC-6 | Over a package whose `temporal_bindings` rows are absent, the call returns `activation = Unavailable` with the `Missing` non-value for the clock row; over a lock with no `profile_selections` row for the clause, `clause-unreadable` with reason `profile-selection`; over a lock whose row names a definition no profile of the evaluator, `clause-unreadable` with reason `profile-unknown`; over a row naming an infinite-trace profile, the `not-finite-window-profile` temporal refusal. Each has the charges FR-516 gives its counterpart over a source. | Test (TC-1331) |
| FR-531-AC-7 | No result of the bare route holds, in its canonical bytes or its fields, a clause name, a source label or a source span: over **R**, the encoded `EvidenceResult` and `ProtocolSlotEvaluation` values do not contain the unit's `source:` labels or a byte region of the source map, and a clause name appears only as the `temporal_clause_names` row of its occurrence gives it, identically on every route. Identity is spelled as the "What the bytes do not carry" list states, and the progress binding of a result holds `package.name()`. | Test (TC-1331) |
| FR-531-AC-8 | The handoff directory of **R** holds required `identities.json` with `format` `quire.checked-handoff.identities/3`; `ReadIdentityRows::read` over it and the package gives a slot call that evaluates; the file equals, byte for byte, the encoding above of the in-process unit's rows, and two writes of equal requests give equal files; omitting the file from a `BareSupply` makes the slot call refuse `MissingIdentityWitness` while the temporal call is unaffected. | Test (TC-1331) |
| FR-531-AC-12 | `ReadIdentityRows::read` over a file that is not JSON, one with an unknown or repeated member, one with another `format`, one with a row whose `structural_node` the package does not hold and one whose `form` does not match its identity returns `Malformed`, `Malformed`, `Format`, `Unjoined` and `Unjoined` respectively, naming the member, the format string or the first row, and returns no rows. | Test (TC-1331) |
| FR-531-AC-13 | The identity file of **R** encodes every identity as the "Value encodings" state: a node reference is the wire node id string of the package, an `EffectiveId` has the `chain` list root first, a `MemberId` has `owner` and `member`, and a declared `UnitId` has `kind` `declared` and a `node` reference; `read` returns rows equal to the writer's, a unit with `kind` other than `declared` returns `Malformed`, a declared unit whose node reference the package does not hold returns `Unjoined`, and a `declared_unit` row whose `exact_identity` has another `kind` returns `Unjoined`. A forged row that passes the joins is accepted by `read` and the slot call over it evaluates against the admitted nodes, so the test records that `read` makes no claim of authority. | Test (TC-1331) |
| FR-531-AC-9 | For each public method of `EvidencePackage` and each method of the `CheckedPackageEvaluation` trait, enumerated by a test that reads those two items' declarations in the crate source with `syn` and compares the names to the first column of the inventory table, the table has exactly one row: available from the bare package, or listed as not carried with a reason; a method added to either type without a row fails the test. | Test (TC-1331) |
| FR-531-AC-10 | The charge log of each call over the bare package equals the charge log of the same call over the in-process route (FR-532-AC-1, FR-533-AC-1); `BarePackage::new`, `with_supply` and view creation take no `Meter` and a call that refuses at supply admission leaves a caller's meter byte-identical. | Test (TC-1331) |
| FR-531-AC-11 | A search of FR-515 for the phrases `bytes-only` and `separate future capability` and of FR-515, FR-516 and FR-520 for the heading `Consumer entry` finds that FR-515 no longer states that an evaluator over `formula_graph()` with no source graph is out of scope, and FR-515, FR-516 and FR-520 each carry a "Consumer entry" section that names this requirement. | Inspection (TC-1331) |
| FR-531-AC-14 | The handoff directory written for TC-1070 fixture E holds `equality.json` with `format` `quire.checked-handoff.equality/1`; `ReadEqualityRows::read` over it and the package gives a slot call on `Refund`'s retry relation (an integer `=` application) that evaluates to the in-process route's value and charge log; the file equals, byte for byte, the encoding above of the in-process unit's rows, and two writes of equal requests give equal files. | Test (TC-1331) |
| FR-531-AC-15 | `ReadEqualityRows::read` over a file that is not JSON, one with an unknown or repeated member, one with an unknown fact tag, one with another `format`, one with a row whose node id string the package does not hold and one whose application is not an equality application returns `Malformed`, `Malformed`, `Malformed`, `Format`, `Unjoined` and `Unjoined` respectively, naming the member, the tag, the format string or the first row, and returns no rows. | Test (TC-1331) |
| FR-531-AC-16 | A slot call on TC-1070 fixture E's `Refund` retry relation refuses `MissingEqualityWitness` over a supply with `equality: None`, `ForeignEqualityWitness` with rows of another package and `UnknownEqualityWitness` with rows of the package from which the row of the retry relation's application is removed, each as `CallFailure::Admission` with `unknown_required_feature`/`unsupported-feature`, category unsupported, exit 21, `CallAdmissionLocus::Package` and the meter untouched; `Refund`'s `trigger.guard`, whose walk holds no equality application, over the same supply evaluates. | Test (TC-1331) |
| FR-531-AC-17 | The encoding of the rows of "The equality file" vector equals the vector's bytes exactly; `ReadEqualityRows::read` over those bytes and a package that holds the node `["E"]` and `["U"]` returns rows equal to the input rows; for a row of every fact tag, every value-type tag, every owner kind and a compound unit id in a fact row, writing then reading returns equal rows and a second write is byte-identical; a checked rational whose numerator is spelled `+1`, `01` or `-0`, or whose denominator is `0` or `-2`, returns `Malformed`; a compound unit id in a `source_units` row returns `Malformed`. | Test (TC-1331) |
| FR-531-AC-18 | For a row whose fact value type is each of a `carried_quantity` over a decimal carrier, over an integer carrier and over a rational carrier, a `bounded_quantity` with both bounds and with `min` and `max` null, and a `map` with a bound and without one (including the example of "Value encodings"), `ReadEqualityRows::to_bytes` followed by `ReadEqualityRows::read` returns equal rows, a second write is byte-identical, and the value-type subtree the writer emits for the example, parsed as JSON, equals the example's JSON value with its members in the stated order (members compared in order, not as a set); a `bounded_quantity` whose bound is in another unit than `unit`, and a `carried_quantity` carrier tagged other than `decimal`, `integer` or `rational`, return `Malformed`. | Test (TC-1331) |
| FR-531-AC-19 | An authentic final emission with a source record round-trips its original opaque record key, source owner, final structural/type IDs, ordered original member identities, final admitted names, presence and exact value types, including nested opaque identities. The bytes-only read joins it through native IR and `source_record_view` equals the original row; no healthy source record succeeds by structural-key substitution. | Test (TC-1331) |
| FR-531-AC-20 | A file with `/1`, absent `source_records`, an unknown/repeated member, malformed or over-depth opaque key/type, foreign structural/type node, stale admitted name or conflicting record row refuses with the named typed format/shape/native-join condition and no partial identity read. With a supplied empty record-row list, an actual request for that record preserves native `UnavailableIdentityWitness`, never succeeding by structural-key substitution. Whole-witness completeness is not asserted by this carrier read. | Test (TC-1331) |
| FR-531-AC-21 | A `BarePackage::with_supply` over a fixture whose identity join is measured on a fresh budget at work **W** and storage **S** returns the package on `NativeJoinBudget::new(W, S)`, and returns `ExhaustedWork` on `new(W - 1, S)` and `ExhaustedStorage` on `new(W, S - 1)`, each with IR's locus and attempted charge, no package, the work denial naming `i2.work_units` and the storage denial naming `i2.native_join_storage`, and the budget's consumed counters equal to those IR reports. The three results differ in kind from `UnknownIdentityWitness`, `InternalFault` and an empty identity set. | Test (TC-1331) |
| FR-531-AC-22 | Over a supply with rows of another package, with rows IR does not join, and with no supply, `with_supply` over a budget that admits the join returns a package whose slot call refuses `ForeignIdentityWitness`, `UnknownIdentityWitness` and `MissingIdentityWitness`, as FR-531-AC-5 states, and the same rows over a budget one unit short of the unknown rows' join return the budget denial and not `UnknownIdentityWitness`. | Test (TC-1331) |
| FR-531-AC-23 | Two joins on one budget are charged cumulatively: after a successful `with_supply` the budget's consumed counters equal the join's measured work and storage, a second `with_supply` on the same budget is refused at the first charge that crosses the remaining allowance, and a selection, a world and a projection call made on the first package consume no further native work or storage. | Test (TC-1331) |
| FR-531-AC-24 | `V2ReadLimits` has a required `storage` member and no `Default`; a read's evaluation view over limits whose `storage` is exactly the measured join storage succeeds and over one unit less returns `ViewFailure::Exhausted` with `ExhaustedStorage`, whose refusal names `i2.native_join_storage`; `EvidencePackage::from_read` and `from_emission` over a budget one unit short of the source join return `PackageMismatch::Exhausted`; the handoff writer over `native_join` one unit short returns `Error::NativeJoinExhausted` and writes no directory. | Test (TC-1331) |
| FR-531-AC-25 | An inspection of the public signatures of `selected_exact_identities_from_rows`, `SelectedModels::exact_identities`, `CheckedGraph::selected_exact_identities`, `ReadIdentityRows::selected`, `BarePackage::with_supply` and the evidence constructors finds each takes a `NativeJoinBudget` by mutable reference, finds no `NativeJoinBudget::new` and no `u64::MAX` allowance in a library path below an entry, and finds that `BarePackage` implements neither `Copy` nor `Clone`. | Inspection (TC-1331) |

| FR-531-AC-26 | An authentic final emission's source-union rows round-trip original type handles, settled keys, sealed member identities, final structural and payload node IDs and ordered Exact payload types; the read rows equal the emitter rows, native `source_union_rows` retains them and rewriting is byte-identical. | Test (TC-1331) |
| FR-531-AC-27 | `/2`, absent `source_unions`, malformed member identity, stale member name, foreign source owner and conflicting union rows return typed format, malformed or native-join refusals without a partial read. An empty union list never fabricates a producer witness from the graph. | Test (TC-1331) |

## Dependencies

- **Upstream:** FR-337, FR-383 (the nodes and rows), FR-390 (the reader view and
  its identity rows; the reader-view criteria it states are the ones the bare slot
  call rests on), FR-515, FR-516, FR-517, FR-520; Contract IR `read_checked_package`
  and `SelectedExactIdentities`; QSL-1377 (selected-model identity-row production).
- **Downstream:** quire-protocol FR-045 and FR-046; FR-532 and FR-533. quire-protocol
  FR-043's admission input list follows the two files.
- **Writer and reader encoders for the three added tags:** the code ticket for the
  `carried_quantity`, `bounded_quantity` and `map` encoders follows in the B2a work;
  until it lands FR-531-AC-18 is unmet and the writer refuses the three types
  `Error::Unnameable`.
- **Wire forms the writer adds:** the equality file's leaf encodings are pinned in
  "The equality file". `CheckedNodeId`, `CheckedOccurrence`, `CheckedSourceMapEntry`,
  `CheckedNodeOwner` and `CheckedRational` have `serde` forms in Contract IR today and
  the file uses them. `quire_exact::Location`, `ValueType` (with its interval,
  decimal, float, text, enum and collection parts), `UnitId`, `NodeKey` and
  `EffectiveId` have none; the QSL writer and `ReadEqualityRows::read` encode and
  decode them by hand to the forms above, from those types' public accessors and
  constructors, and no change to Contract IR or quire-exact is required for the file.
