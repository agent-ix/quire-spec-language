---
id: FR-533
title: "Evaluate a protocol slot from the checked package's own nodes"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-400
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-520
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-531
    type: depends_on
  - target: ix://agent-ix/quire-protocol/FR-045
    type: references
---
# FR-533: Evaluate a protocol slot from the checked package's own nodes

## Description

FR-390 states that a package the reader returns for FR-383's nodes gives the same
`ProtocolSlotEvaluation` and the same charge log as the checked package that was
emitted, and that the reader view takes nothing from the source package. The
reader-backed view the code has today does not yet meet that: it is built from an
`AdmittedPackageRead`, which retains the source `CheckedPackage`, and it takes the
slot selection, the binders with their names, levels and types, the observation's
model selection and type environment, and the body it evaluates from that source;
the reader contributes a reference-slot projection and a node-presence check. A
consumer with only a `CheckedPackageV2` cannot reach any of it. This requirement
makes the protocol-slot call of `BarePackage` (FR-531) read the package's own nodes
for every one of those, and states the equivalence with the source route.

## Semantic authority and boundary

FR-390 owns the slots, the supply, the outputs and the reader view's projections
and refusals; FR-400 owns the checked-term read vocabulary; FR-520 owns the outcome
map. This requirement adds none of them. It states that the reader view of FR-390
is the evaluation path of the bare package, with the source package removed from
its inputs, and which source-side reads go away with it.

## Inputs

A `BarePackage` with its `BareSupply` (FR-531), whose identity rows and, for a package
with an equality application, equality rows come from the handoff directory; a `ProtocolSlot`, a
`ProtocolSupply` and a `Meter` (FR-390), unchanged. The observation of a supply
is admitted by the bare view's `admit_protocol_observation`, which takes the
caller's domain package documents, snapshots and limits as it does on the source
route.

## Outputs

`Result<ProtocolSlotEvaluation, CallFailure>` of FR-390, unchanged.

## Behavior

### Requirements

- The bare view SHALL read every fact of the table below from the package and
  SHALL consult no `CheckedPackage`.
- The bare route SHALL evaluate every slot of FR-390's table over every type the
  in-process route evaluates, including records with Collection members, plain or
  nested, and nested Option members.
- The bare route SHALL return the value, outcome, locations and charge log the
  in-process route returns for the same package, slot and supply.
- Where the wire does not yet carry a read the in-process route makes, the
  bare route SHALL refuse or report `None` as stated below, and the gap is an
  unmet dependency, not a scope reduction (FR-390).

### What the bare view reads from the package

| Read | From the package | Retired source read |
| --- | --- | --- |
| slot selection (node, member, index) and its refusals | the admitted `protocol` node's body members (FR-464) | `protocol_selection::select` over the source package |
| binders: node key, level, name, `ValueType` | the admitted binder nodes and the kernel `ValueType` projected from them (FR-390 "Typed projection admission") | the source frame of the checked body |
| the expression evaluated | the admitted term, read as FR-400 `NodeRead` through `TermHandle` | the source `CheckedBody` |
| the read location of a binder and of each visit | the `Location` of the admitted node and the `protocol_locations` rows (FR-390) | the source package's occurrence list |
| the observation's model selection and type environment | the package's `model_root` nodes and the caller's domain package documents | `graph().model_selections()` and `graph().scope().types()` |
| callee and dispatch candidates | the admitted callable and dispatch projection nodes, within the admitted closure | the source package's dispatch tables |
| the presence and shape of record members, and the value type of a slot's root | the wire projection; a Collection member (plain or nested) or a nested Option member has none yet ("Types the bytes cannot project") | the checked source record type (`scope().types().composite`) |

The view takes the slot root from the slot descriptor and the admitted nodes, and
the origin from the resolution of "Slot naming on the bytes route" (FR-390-AC-50),
and consults no `CheckedPackage`.

### The call

`BarePackage::evaluate_protocol_slot` and `evaluate_selected_protocol_slot` create
the view, select the slot, admit the supply and run the one S6a `Machine` over
the `NodeRead` projection, with the one `ProtocolClause` hook. A `ViewFailure` of
view creation is returned as `CallFailure::Admission` carrying the catalog pair
`unknown_required_feature`/`unsupported-feature`, with the reason and locus, before
any charge. No interpreter, family table or checking pass is added.

### Names and locations

A binder is named in refusals as FR-390's causes name it: by its `NodeKey`, and by
the declared name the admitted binder node carries where it carries one. A location
is the `Location` the package's own nodes and rows give, and `None` where the wire
carries none (`ProtocolSlotEvaluation::Unavailable.location`; `Evaluation.location`).
The in-process route is not changed: it keeps the location it takes from its source
(FR-390-AC-31 and AC-32). Where the wire carries no location for a read, the bare
route returns `None` and the in-process route returns the source location, which is
unmet parity with FR-390-AC-15's location equality; it closes when the package's
`protocol_locations` rows pair with the reader view's visits (see Dependencies). The
option was chosen over amending FR-390 to drop source locations because FR-390
requires equal locations on both routes and a bare-route-only `None` is the smallest
change that stays truthful about what the wire carries.

### Slot naming on the bytes route

A slot is identified on the bytes route by the triple (control node, slot kind,
ordinal), derived from the package's nodes alone. This is FR-390's
`ProtocolSlot { node, member, index }` read as follows: `node` is the key of the
`protocol` node that holds the slot; `member` is the slot kind (`activation.guard`,
`captures`, `registration_captures`, `activation_captures`, `trigger.guard`,
`retry.relation`, `recovery.predicate`, `constraint`, `condition`, `guard`,
`replication.until`, `ordering.key`, `visible`, `cases.guard`, `related.from`,
`related.to`); and `index` is the ordinal of the slot in that member's list, counted
from zero in the order the node's own body holds the list, which is ascending
node-id order for the `visible` set, ascending label order for `cases`, and the
emitted record order for `related` (FR-383). Neither a source span nor a site name
is part of the identity, and the descriptor of a bytes-route call carries neither.

The source route selects by the same triple. The source span of FR-383's
site-name registry (`related <start> <end>`, `decision <start>..<end>`) is an
alias of the slot under which the source route finds its `protocol_locations`
rows; the two routes name one slot by one triple, and the origin they pair rows
under is a consequence of the slot, not a second identity.

A site name with no span (every other slot kind) is built from the node as FR-383's
registry states, and the bytes route builds it that way. A span-named slot (a
`related` end, a `visible` entry or a case guard) has its origin resolved from
the package as follows, and the resolution is a function of the package bytes
alone:

1. If the slot's root is a leaf (a literal, a `Local` or another reference), the
   walk has no application, the slot has no `protocol_locations` row, and it needs
   no origin.
2. Otherwise the *candidates* are the origins of the control's span family that
   the control's path prefix and form name (`protocol <P> run <path> related
   <start> <end>` for an end, `protocol <P> run <path> decision <start>..<end>` for
   a visible entry or a case guard) and whose rows name exactly the applications of
   the slot's walk, each as often as the walk visits it (FR-383: the rows of an
   origin never name an application its walk does not visit, and number as many as
   the visits). An origin whose rows name an application the slot's walk does not
   visit is not a candidate.
3. When the slot has one candidate, that is its origin.
4. When the candidates are shared by several slots of the control with the same
   walk (slots that share a root), the *tied slots* are ordered by member in the
   order `visible`, `cases.guard`, `related` (source places the visible entries
   before the cases of a choice), and within `visible` and `cases.guard` by ascending
   `index`; the `related` ends are ordered by record, not by member then index: the
   two ends of record `i` are `related.from[i]` then `related.to[i]`, before the ends
   of record `i + 1`, so the order is `from[0]`, `to[0]`, `from[1]`, `to[1]`, ... (a `choice` or `repeat` holds no `related` end,
   and the other controls hold no `visible` entry or case guard, so the members of
   one control that can tie are the first two or the last two); the candidates are ordered by
   ascending numeric `(start, end)`, parsing both as decimal integers and not
   comparing the name's bytes. The tied slot of rank `k` takes the candidate of rank
   `k`. A candidate shared by `n` tied slots is therefore given to each of them
   once, and no slot has more than one origin. A tied slot whose rank is not below
   the number of candidates has none.
5. When the slot has no candidate although its root holds an application, view
   creation refuses `MissingLocationProjection` at the `Node` locus of the root
   (FR-390-AC-35), as when a row is missing.

No bytes-route selection returns several origins and none refuses as ambiguous.
The pairing of rule 4 equals the source route's whenever the node order of the
tied slots equals their source order. Cases are held in label order and `related`
records in their emitted record order (FR-383), neither of them source order, so two
cases whose guards are one equal expression and whose labels sort
against their source order are paired with each other's origin: the value, outcome
and charge log are unaffected (a location is not an input to evaluation), and the
`Location` values of the two guards are exchanged. That exchange is unmet parity
with FR-390-AC-15's location equality, recorded by FR-533-AC-13, and is closed
by the QSpec follow-up named in Dependencies.

In TC-1070 fixture L, `Decide` is a `choice` with one visible entry and two cases:
`case yes when { got.ready }` (written first) and `case no when { not got.ready }`
(written second). Its node-ordered slots are `visible[0]` (root `got.ready`),
`cases.guard[0]` (case `no`, root `not got.ready`) and `cases.guard[1]` (case `yes`,
root `got.ready`). The package holds three origins of the family,
`protocol Pipeline run Main Decide decision 2641..2650`, `... 2676..2685` and
`... 2735..2748`, and the rows of `got.ready`'s application under each, with the
row of the `not` application under `... 2735..2748` only. The origins are:

| Slot | Walk | Candidates | Origin |
| --- | --- | --- | --- |
| `Decide.cases.guard[0]` | `not`, `got.ready` | `... 2735..2748` (the only origin that names `not`) | `... 2735..2748` |
| `Decide.visible[0]` | `got.ready` | `... 2641..2650`, `... 2676..2685` | rank 0 of the tied slots: `... 2641..2650` |
| `Decide.cases.guard[1]` | `got.ready` | `... 2641..2650`, `... 2676..2685` | rank 1: `... 2676..2685` |

`Got.related.from[0]` and `Got.related.to[0]` have the reference `view` as root,
no row, and no origin. The `related` ends of fixture L are leaf references and the
model `M` has no Reference-typed field a non-leaf end could read, so no compiled
TC-1070 fixture holds tied `related` ends and the source route cannot be compared
for them; the rule is vectored on a package (FR-533-AC-14): if the ends of one
`related` record are one application with the origins `... related 99 100` and
`... related 100 105`, `related.from[0]` takes `... related 99 100` and
`related.to[0]` takes `... related 100 105`.

### Refusals and loci of the bare slot call

Every refusal of view creation and slot selection on the bytes route is returned as
`CallFailure::Admission` with the catalog pair `unknown_required_feature`/
`unsupported-feature`, before any charge, and its `CallAdmissionLocus` is:

| Refusal | Locus |
| --- | --- |
| `MissingIdentityWitness`, `ForeignIdentityWitness`, `UnknownIdentityWitness` (FR-531) | `CallAdmissionLocus::Package` |
| `MissingEqualityWitness`, `ForeignEqualityWitness`, `UnknownEqualityWitness` (FR-531 "The equality file") | `CallAdmissionLocus::Package` |
| `MissingTypeRepresentation`, `MissingLiteralProjection`, `MissingOperationProjection`, `MissingLocationProjection`, `MissingBindingProjection`, `MissingChargeSemantics` | `CallAdmissionLocus::Node` with the key FR-390 names |
| a slot node, member or index the package does not hold (FR-390 selection rules 1 and 2) | `CallAdmissionLocus::Node` for a node, `CallAdmissionLocus::Member { name, index }` for a member or index, as the source route reports them |

`CallAdmissionLocus::Package` carries no node key and no location, as FR-390's
`Package` locus of `UnsupportedView` carries none. The bytes route never reports
`Node` or `Binder` for a witness refusal.

### Equality slots

A slot whose walk holds an `==` or `!=` application reads that application's
admitted equality decision through IR's `CheckedPackageV2::admitted_equality`. The
reader projection reads one for every equality identity it admits, not only the
native kinds: `quire.op.string`, `uuid`, `timestamp`, `boolean`, `integer`,
`rational`, `decimal`, `quantity` and `structural` `eq` and `ne` (an integer `=` in
a retry relation or recovery predicate included), and an equality application with
no row is an internal fault of the source route. `admitted_equality` takes three row
sets the producer issued at checking time: the equality application rows (the
original application key and authored `Location` of the emitted application and
occurrence), the equality fact rows (the typed operand, resolved unit, dimension
declaration, dimension term, conversion edge, Text or Plan type and enum member
facts the decision rests on) and the selected source-unit rows.

The package already carries `checked_term_data.equality_decisions` (QSpec FR-322),
and the keys, occurrences and source-map entries of the rows can be recovered from
the package's wire graph. What the rows add is the producer's witness: the authored
`Location`, the checker-held `value_type` and the producer-selected fact, which
`admitted_equality` takes and, by its own statement, never reconstructs, selecting
no fact from an unrelated declaration. A bytes-only consumer therefore needs that
witness beside the package, as it needs the identity rows (FR-531 "The identity
file"): the handoff carries it, in `equality.json` (FR-517, FR-531 "The equality
file"), by the same mechanism as `identities.json`, and the bare route reads
equality decisions only through it. The bare route derives no witness from the
package in its place and does not guess a decision. A package whose walk holds no
equality application needs no equality file, and the slots that hold none do not
read it. Whether Contract IR can derive the witness from the package alone is an
open question for the IR lane; should it do so, the member is removed with that
change, not kept beside it.

### Types the bytes cannot project

A slot whose binder, literal or callee type has no kernel representation in the
admitted closure refuses `ViewFailure::Unsupported` with `MissingTypeRepresentation`
(FR-390-AC-17) on the bare route. The bare route SHALL evaluate slots over records
whose members are Collections, plain or nested, and over records whose members are
nested Options, as the in-process route does. Recursive record types and plain
Option members already project and are not part of this gap; a plain Collection
member is not claimed to project, because Contract IR's record-field projection
strips one optional layer and refuses a Collection or Option member that remains
(`UnsupportedExactType`), and IR-1324 is measuring the plain Collection case. The
code comment at the record-field projection says the wire projector does not yet
recurse through Option and Collection members, and the code reads presence and the
record's field shape from the checked source record instead. The bare route has no
source record, so until the projector carries those member types it refuses
`MissingTypeRepresentation`, and no bare result substitutes a guessed type. This is
unmet parity with an unmet dependency, Contract IR-1324 (project Collection members,
plain or nested, and nested Option members of selected records) (FR-390: an
unsupported projection of a type QSL emits and evaluates is an unmet dependency,
not a scope reduction).

### Equivalence

For one package, one slot, one supply and one meter limit set, the bare route and
the source route (`CheckedPackage` and the reader-backed view over an
`AdmittedPackageRead`) return the same `ProtocolSlotEvaluation` or the same
refusal, including value, outcome category, exit status, catalog code and cause,
unavailable binder, location and ordered losses, and the same charge log:
every counter of FR-255 at every charge point, including the explicit-dereference
distinction of FR-390-AC-19 and the one-visit-per-application rule for shared
terms. The per-binder attribution and `accounting_version` are carried by both as
FR-520-AC-11 states.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-533-AC-1 | For every slot of FR-390's table that **R** holds (the activation guard, the captures of each stage, the trigger guard, the retry relation, the recovery predicate of `Partial` and `Full`, and the constraints), over the supplies of TC-1070 and TC-1315, the result and the ordered charge log over the `BarePackage` equal those over the in-process `CheckedPackage` and over the reader-backed view of an `AdmittedPackageRead`, with value, category, exit status, catalog code and cause, unavailable binder, location and losses compared, location only where the wire carries one (a read with no wire location is the unmet parity of AC-8 and is not compared). The `BareSupply` of the bare route carries the equality rows read from the handoff directory's `equality.json`, which every slot with an equality application (the integer `=` of `Refund`'s retry relation and recovery predicate included) needs. | Test (TC-1333) |
| FR-533-AC-2 | For each outcome row of FR-520's map (true and false guard, retry relation and recovery predicate; an unavailable binder; a wrongly typed value; an unsupported construct; a meter one below measured usage), the bare route produces the successor outcome, and the exact-usage and one-below vectors of FR-390-AC-9 hold for **R**'s `retry.relation` over the `BarePackage`. The `BareSupply` of the bare route carries the equality rows read from the handoff directory's `equality.json`, which every slot with an equality application (the integer `=` of `Refund`'s retry relation and recovery predicate included) needs. | Test (TC-1333) |
| FR-533-AC-3 | A `BarePackage` whose package was read in a run in which the source unit was never compiled evaluates AC-1's slots with the results of AC-1; and over a package read from **R**'s bytes, a `BarePackage` evaluation of slot A returns the same result when the in-process `CheckedPackage` held beside it in the test is replaced by the checked package of another unit that has the same protocol node ids but a different expression at slot A, because the bare route reads no `CheckedPackage` (the in-process route's result changes with that replacement, the bare route's does not). | Test (TC-1333) |
| FR-533-AC-4 | A call-graph search from `qsl_eval::bare::BarePackage::evaluate_protocol_slot` finds that no function reachable from it takes or reads a `CheckedPackage`, `CheckedBody`, `AdmittedPackageRead::source` or a source `Graph`, that `protocol_selection::select` over a source is not called on the bare route, and that the bare route's `NodeKind` match has no wildcard arm. | Inspection (TC-1333) |
| FR-533-AC-5 | A bare observation admission over the domain package documents of **R**'s model admits the snapshot that the source route admits, with equal `AdmissionUsage`; a model selection the package does not hold refuses `invalid_model_binding`/`wrong-model-selection` as FR-390-AC-21 states, with equal records on both routes. | Test (TC-1333) |
| FR-533-AC-6 | Over a package whose `model_root` nodes select a model other than the supply's, the bare slot call refuses before any charge with the record the source route gives for the same mismatch. | Test (TC-1333) |
| FR-533-AC-7 | The bare route evaluates a slot whose binder type is a record with a plain Collection member, a record with a nested Collection member (a collection of options or of collections) and a record with a nested Option member (an Option of a collection or of an option) to the value and charge log of the source route; a recursive record and a record with a plain Option member evaluate on the bare route already. Until the wire projector carries those member types, the bare route refuses each of the three cases with `ViewFailure::Unsupported` and `MissingTypeRepresentation` at the `Node` locus of the binder, carrying `unknown_required_feature`/`unsupported-feature`, no partial value and no charge, while the source route evaluates the slot; the test records the unmet parity, and where Contract IR's projection is measured to carry a plain Collection member the first case moves from refusal to evaluation. No value of a different type is substituted. | Test (TC-1333) |
| FR-533-AC-8 | A binder unavailable and read returns `Unavailable` naming the binder and the read, with the location the package's rows give (FR-390-AC-32), equal to the in-process route's; where the wire carries no leaf location the bare result is `None` while the in-process result keeps its source location, and the test records that difference as the unmet parity of "Names and locations". | Test (TC-1333) |
| FR-533-AC-9 | Three slot calls over one `Meter` on the bare route leave a charge log that, per FR-520-AC-7, equals the sum on fresh meters for `work_units` and `result_units` and the maximum for the other eight counters, and equals the log of the same three calls on the source route. | Test (TC-1333) |
| FR-533-AC-10 | `BarePackage::new`, `with_supply`, view creation, slot selection and supply admission charge no counter and take no `Meter`; a refusal at any of them leaves a caller's meter byte-identical. | Test (TC-1333) |
| FR-533-AC-11 | Over the package of TC-1070 fixture L, the bare route selects `Decide.visible[0]`, `Decide.cases.guard[0]` and `Decide.cases.guard[1]` by the triple (control node, member, index) alone, with a descriptor that carries no span and no site name; the origins resolve to `protocol Pipeline run Main Decide decision 2641..2650`, `... 2735..2748` and `... 2676..2685` respectively, as the table of "Slot naming on the bytes route" gives, each slot has exactly one origin, and no selection returns two origins or refuses as ambiguous. | Test (TC-1333) |
| FR-533-AC-12 | For every slot of TC-1070 fixtures E, E-C and L, the source route and the bytes route select the same triple, the same root node, the same binders (name, level, key, `ValueType`) and the same origin; `Got.related.from[0]` and `Got.related.to[0]` of fixture L have no origin on either route. In a copy of fixture L's package whose origins `... decision 2641..2650`, `... 2676..2685` and `... 2735..2748` are renamed, rows with them, to `... decision 999..1005`, `... decision 1000..1004` and `... decision 1001..1002`, `Decide.visible[0]` takes `999..1005`, `Decide.cases.guard[1]` takes `1000..1004` and `Decide.cases.guard[0]` takes `1001..1002`, because the candidates are ordered numerically by `(start, end)`; ordered by the UTF-8 bytes of the names (`1000..1004` before `999..1005`) `Decide.visible[0]` would take `1000..1004` and `Decide.cases.guard[1]` would take `999..1005`, so the test fails if the bytes order is used. | Test (TC-1333) |
| FR-533-AC-13 | Unmet parity, exact condition: two tied slots of one member (two `cases.guard` slots of a `choice`, or two ends of `related` records) have one root node and one walk, and their node order is the opposite of their source order (cases are held in ascending label order, so with the labels sorting against source positions the lower index is the later case in source; `related` records are held in the emitted record order). Then the bytes route gives `cases.guard[i]` the candidate origin of rank `i` and `cases.guard[j]` that of rank `j`, which are each other's source-route origins, and the two slots' `Location` values are exchanged relative to the source route; no other slot kind and no slot with a distinct root expression is affected. The value, outcome, exit status, catalog code, cause and charge log of both slots equal the source route's. In a copy of fixture L's unit whose cases `yes` and `no` both have the guard `got.ready`, the labels sort `no` before `yes` while `yes` is written first, the test asserts the equal values and charge logs and the exchanged `Location` values and records the exchange as this unmet parity; with the cases written `no` first, it asserts equal `Location` values. | Test (TC-1333) |
| FR-533-AC-14 | In a copy of fixture L's package whose rows of the `decision 2641..2650` origin are removed, `Decide.visible[0]` (rank 0 of two tied slots, one candidate) takes `... 2676..2685` and view creation refuses `MissingLocationProjection` at the `Node` locus of the `got.ready` application for `Decide.cases.guard[1]` (rank 1, no candidate), carrying no partial view and no charge. In a package copy where the two ends of one `related` record of fixture L's `Got` are one application with rows under the origins `... related 99 100` and `... related 100 105`, `related.from[0]` takes `99 100` and `related.to[0]` takes `100 105` (numeric order of `(start, end)`); ordered by the UTF-8 bytes of the names `related.from[0]` would take `100 105`, so the test fails if the bytes order is used. | Test (TC-1333) |
| FR-533-AC-15 | Over a `BarePackage` with no supply, with identity rows of another package and with identity rows that name no admitted identity, the slot call refuses `MissingIdentityWitness`, `ForeignIdentityWitness` and `UnknownIdentityWitness`, each as `CallFailure::Admission` with `unknown_required_feature`/`unsupported-feature` and `locus == CallAdmissionLocus::Package`; the same three refusals for the equality rows (FR-531) report `Package`; a slot over a missing node reports `Node`, over a missing member `Member { name, index }`, and `MissingTypeRepresentation` reports `Node` with the binder's key; no witness refusal reports `Node` or `Binder`; the meter is byte-identical after each. | Test (TC-1333) |
| FR-533-AC-16 | The bare route, given the identity rows and the equality rows read from the handoff directory, with no `Emission` and no `CheckedPackage` in the test crate, evaluates to the in-process route's value and charge log: the integer equality of `Refund`'s retry relation and recovery predicate in TC-1070 fixture E; the String, UUID and Timestamp equality slot of the fixture TC-1289 defines; and the quantity equality slot of the fixture TC-1290 defines. Over a bare package whose equality rows are omitted each of those slots refuses `MissingEqualityWitness`, while `Refund`'s `trigger.guard` (no `==` or `!=` application) evaluates. | Test (TC-1333) |
| FR-533-AC-17 | An inspection of the bare route finds no function that derives an equality application row, equality fact row or source-unit row from a node, a name or a source, that reads `Emission::equality_producer_identity_rows`, `equality_fact_witness_rows` or `selected_source_unit_rows`, or that selects a span-named origin by its name text other than by the rule of "Slot naming on the bytes route". | Inspection (TC-1333) |

## Dependencies

- **Upstream:** FR-390 and its open reader-view criteria (AC-13, AC-15, AC-16,
  AC-18 to AC-22, AC-24 and the location and projection criteria that the reader view
  must meet from the package alone), FR-383, FR-400, FR-520, FR-531; QSL-1377
  (identity rows, Done).
- **Delivery gates AC-1 needs:** QSL-1625 (reader roots that are computed
  expressions, not only a field read or a plain reference, and the pairing of
  `protocol_locations` rows with visits) and QSL-1626 (compound `UnitId` in the
  reader projection). Until they land the bare route faults or refuses for those
  slots, which is unmet parity, not a scope reduction.
- **Contract IR:** IR-1324 (the exact projection carries Collection members, plain or
  nested, and nested Option members of selected records; owner luna:IR:codex2) is the unmet-parity
  dependency of FR-533-AC-7. The direct dependencies a package retains from its
  read evidence are read through `CheckedPackageV2::direct_dependencies()`
  (FR-531), so a package that has dependencies is no unmet parity.
- **Downstream:** quire-protocol FR-045.
- **quire-eval and Contract IR (bytes-route admission and equality):** the `Package`
  locus is a variant of `quire_eval::CallAdmissionLocus`, which has `Node`,
  `Population`, `Member` and `Binder` today; FR-390 requires the package-level locus
  for the identity-witness refusals, so the QSL code adds the variant through the
  first-party dependency on `quire-eval` (a separate repository, owner the
  `quire-eval` lane), or asks that lane to, and the bytes route reports `Package`
  once it exists. Until it exists FR-533-AC-15 is unmet. The
  equality rows of "Equality slots" are read with IR's `admitted_equality`; the
  writer (FR-517) and the reader (FR-531) are QSL's, and no IR change is needed
  for the file. Whether IR can derive the witness from the package alone is an open question for the IR lane; if it does, the file is removed.
- **Pairing of equal case guards (QSpec follow-up):** the exchange of FR-533-AC-13
  closes only when the package carries, for each case guard, which source case it
  belongs to. The origin name a slot is located under belongs to the site-name
  registry of QSpec FR-322, which writes a case guard's name from its source span
  and carries no case label or ordinal; QSL cannot add a form to that registry. The
  fix is a QSpec follow-up (a registry form or a row member that names the case, for
  example by label), after which the rule of "Slot naming on the bytes route" uses it
  in place of rank for tied case guards. Until then FR-533-AC-13 stays unmet parity
  and no feature is cut.
- **Observation admission:** the type environment of FR-533's observation admission
  (the composites and object types an observation and a supply are admitted against)
  is built from the package's `model_root` nodes, the selected record views and the
  caller's domain package documents. Whether that suffices to give the keys on which
  `admits` and `conforms` agree with the source route is being measured; FR-533-AC-5
  and the supply admission of FR-390-AC-21 on the bare route are gated on that
  measurement, and no rule is given here before it.
- **Enum equality fixture:** FR-533-AC-16 covers no enum equality slot, because no
  fixture of the TC-1070, TC-1289 or TC-1290 family defines an enum-typed slot, and
  the equality file encodes `enum_member` facts that only such a slot exercises.
  Enum equality on the bare route is unmet verification until a fixture with an enum
  equality slot is defined, with its source text, in TC-1333; the encoding of
  FR-531-AC-17 covers the `enum_member` fact tag meanwhile.
