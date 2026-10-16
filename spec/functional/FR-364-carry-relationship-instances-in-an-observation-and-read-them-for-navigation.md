---
id: FR-364
title: "Carry relationship instances in an observation and read them for runtime navigation"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-085
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: depends_on
  - target: ix://agent-ix/quire-semantic-value/FR-119
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: references
  - target: ix://agent-ix/quire-specification/FR-152
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-006
    type: depends_on
---
# FR-364: Carry relationship instances in an observation and read them for runtime navigation

## Description

When an observation is admitted, the model layer SHALL hold the links between
objects that a declared navigation relationship has in that state as
relationship instances. It SHALL refuse an instance set that is not a
well-formed set of links of the declared relationship. When a clause navigates
a relationship end, the evaluator SHALL take its targets from a read-only view
of those instances alone, and SHALL decide the refusals and charges
quire-specification [FR-152](ix://agent-ix/quire-specification/FR-152)'s
"Navigation" section states. When it must tell a missing object that is
dangling from one it cannot decide, it SHALL read a population manifest that
the observation's producer issued from an exhausted traversal of the admitted
package's population declarations, and SHALL decide `dangling` only from a
closed manifest.

This requirement is QSL's own contract for the runtime half of FR-152's
"Navigation" section, which [FR-085](FR-085-resolve-relationship-end-references.md)
leaves out of its scope. FR-152 says what navigation returns and charges; it
does not say how an observation carries the instances navigation reads. This
requirement says that, and [FR-106](FR-106-admit-snapshots-and-invocations.md)
binds it to the snapshot document.

## Inputs

- A navigation relationship of the admitted domain package
  ([FR-085](FR-085-resolve-relationship-end-references.md)): both ends name
  object types, each end has a role and a multiplicity `{lower, upper,
  ordered, unique}`, and the relationship has a direction.
- The object references of the observation
  ([FR-084](FR-084-admit-closed-populations-and-resolve-lookup.md)): each is a
  reference key of a universe, the object's most-specific effective type and
  its object identity.
- For an extent to admit: the relationship's declaration key, a completeness
  flag and a list of instances, each a source reference and a target
  reference.
- For `resolve`: the population manifest of the observation (below), bound
  with the observation and the type environment to one source and selection.
- For a navigation: the resolved site (the relationship, the end named, the
  traversal direction and that end's multiplicity), the receiver reference,
  and the stage limits.

## Outputs

- Admission: an admitted extent, or one refusal.
- Navigation: a `Reference<U>`, an `Option<Reference<U>>`, a
  `Set<Reference<U>>[l,u]` or a `Bag<Reference<U>>[l,u]`, or one refusal or
  incomplete result.

## Behavior

### A relationship instance is one link of one relationship

A relationship instance is the triple (relationship, source reference, target
reference). The relationship is named by its declaration key. The source
object is an object of the relationship's source end type or of a type that
conforms to it, and the target object likewise for the target end. An
instance is not a field value and not an object: it has no identity but its
triple. The relationship's direction decides which traversals a clause may
write, never what an instance is, so the same triple serves a traversal from
the source and a traversal from the target.

### An extent is the instances of one relationship in one observation

The extent of a relationship in an observation is the multiset of its
instances and a completeness flag. An observation holds at most one extent per
relationship, in its object environment beside its population bindings. A
population binding carries no instance, because an instance links objects of
two populations or more. A relationship with no extent is unlisted.

An unlisted extent is not complete and holds no instance that can be read. An
empty extent is a complete extent with no instance: it states that the
relationship has no link in the state. A complete extent states that these are
all of the relationship's links; an incomplete extent holds the links known
and states nothing about the rest. The three are different and none is
converted into another.

An extent has no order. A document's or a caller's listing order is never
observable: every read is in canonical reference-key order.

### Extent admission is the same for every way of building an extent

Whether an extent is read from a snapshot (FR-106) or supplied by a
model-layer caller, admission SHALL decide conditions 1 and 2 for each extent
and conditions 3 and 4 for each instance of it, extent by extent and instance
by instance in the order supplied, and stop at the first failing condition:

1. If the relationship names no navigation relationship of the admitted
   domain package (an unknown key, a connection, an allocation or any other
   declaration), then admission SHALL refuse `invalid_runtime_input`/
   `wrong-role-mapping`, retaining the supplied relationship.
2. If the observation already holds an extent for the relationship, then
   admission SHALL refuse `invalid_runtime_input`/`conflicting-identity`,
   retaining the relationship.
3. If the reference at the source end has a type that does not conform to the
   relationship's source end type, or the reference at the target end has a
   type that does not conform to the target end type, then admission SHALL
   refuse `invalid_runtime_input`/`wrong-value-kind`, retaining the
   relationship, the end (`source` or `target`), the reference's type and the
   declared end type. A type that is no object type of the admitted model does
   not conform. Conformance is [FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md)'s,
   reflexive. A full reference carries its type, so condition 3 checks every
   end of a model-layer extent, whether the observation holds its object or
   not: existence and type conformance are separate, and a missing object
   affects only `resolve` and never the type check. Only an end of a snapshot
   document whose object the snapshot does not list has no type to check
   (the document names a population and a key, and the type component of the
   reference comes from the object); condition 3 skips that end, and the
   closure rule below decides it.
4. If an instance repeats an earlier instance of the same extent (equal
   relationship, source reference and target reference), then admission SHALL
   refuse `invalid_runtime_input`/`conflicting-identity` at the later
   instance, unless both of the relationship's ends declare `unique: false`.
   Then the repeats are separate instances, each one occurrence of the
   link. Repeating a link through an end that declares `unique: true` would
   put a reference in that end's set twice.

Admission SHALL NOT compare the number of instances at an object with an end's
`lower` or `upper` bound. An observation is evidence of a state, and a state
may break a declared multiplicity; the breach is a refusal of the navigation
that reads it (below), exactly as an out-of-range integer in a post snapshot
is evidence. No condition above refuses a count.

### Dangling ends are decided where the observation is closed

An instance end naming an object that the observation does not hold is a
dangling end. A snapshot cannot even form its reference key, because the
document names a population and an object key and the type component of the
key comes from the object. FR-106 therefore refuses a dangling end of a
document at its closure check, with the rule it applies to a `reference` value:
`dangling_reference`/`absent-target-in-complete-population` when the end's
population is complete. When the population is not complete, an end naming a
key it does not list returns the incomplete outcome (`incomplete_population`/
`incomplete-scope` naming the population), because no object gives the end its
type component; an end naming a listed key admits. An extent a
model-layer caller supplies carries full references and is admitted without a
closure check, so the navigation below decides, with `resolve`, an end of such
an extent whose object the observation does not hold. A snapshot-built extent
never reaches that decision: every stored end is a held object. Neither place
changes the other's rule.

### The population manifest states which populations exist and which are complete

An observation's population manifest is a state, `closed` or `partial`, and
rows, one per population declaration of the admitted domain package
([FR-084](FR-084-admit-closed-populations-and-resolve-lookup.md)), in ascending
declaration-key order. A row is (population, covered types, complete). The
covered types are the declaration's declared member types, a non-empty set.
`complete` is `true` only when the observation states that the population holds
every object of its member types: a population the model layer binds (FR-084
binds a closed population) and a snapshot population listed `"complete":
true`. A declared population the observation does not bind or list has a row
with `complete` `false`, because an absent population states nothing about its
members. The manifest therefore names every population the observation could
hold, and no applicable population can be omitted from it.

A row **covers** a type `T` exactly when `T` conforms to at least one of the
row's covered types, reflexively
([FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md),
[FR-084](FR-084-admit-closed-populations-and-resolve-lookup.md)): a declared
member type covers itself and every type that conforms to it, and a declared
child type does not cover its parent. A population holds an object only when
the object's type conforms to a declared member type, so every population that
could hold an object of type `T` covers `T`.

**The manifest is closed only by an exhausted traversal.** The producer of the
observation owns the traversal: FR-106 admission for a snapshot-built
observation and the model layer's observation builder for a model-layer one.
Its evidence is an authenticated population enumerator that yields one row per
population declaration of the admitted package and reads the observation's own
bindings or listing for each row's `complete`, and nothing a caller supplies. The
manifest is `closed` exactly when the enumerator reports normal exhaustion
within its row limit. An enumerator that stops for any other reason (the row
limit, a failure) leaves the manifest `partial`, with the rows read and the
stop reason. No operation accepts a `closed` flag, builds a closed manifest
from supplied rows, converts a partial manifest into a closed one or closes a
manifest before the enumerator is exhausted. The semantic-value store builds
the manifest from the enumerator and keeps its state private
([FR-119](ix://agent-ix/quire-semantic-value/FR-119)).

### `resolve` decides a missing object by the manifest alone

For a reference `x` of type `T` whose object the observation does not hold,
`resolve(x)` SHALL return, stopping at the first rule that applies:

1. `undecidable` when the manifest is `partial`;
2. `undecidable` when `T` is no object type of the admitted model;
3. `undecidable` when no row covers `T`: a population the manifest does not
   cover `T` with may hold `x`, and no completeness is guessed, so there is no
   vacuous `dangling`;
4. `undecidable` when any row that covers `T` is not complete, even if another
   covering row is complete;
5. otherwise `dangling`: the manifest is closed, at least one row covers `T`
   and every covering row is complete, so every population that could hold `x`
   is complete and none holds it.

A held object is `present` before any of these rules, whether or not the
manifest is closed. The rules read no population member list and no field.

### One bundle binds the manifest, the observation and the type environment

The producer issues one bundle of the manifest, the observation's object
closure and the type environment, all bound to one key: the checked unit's
source reference as a whole (authority, identity and the `quire.source.bytes/v1`
digest, [FR-362](FR-362-bind-a-checked-unit-to-its-source-by-digest.md)) and the
selection (the selected clause and, for a snapshot-built observation, the
digest of each selected document). Keys are equal only when every component is
equal: equal labels with a different digest are different keys. `resolve` is
read only through the bundle. A constructor that takes the selected key and
the three parts separately SHALL refuse the first part, in the order
`manifest`, `closure`, `types`, whose key differs from the selected key with
`invalid_runtime_input`/`conflicting-identity`, retaining the part, the
selected key and the differing one. A manifest from another source or
selection therefore never classifies a missing reference.

### What the model layer passes to the semantic-value store

The model layer passes: the navigation relationships, each by its declaration
key with both end types (from the authenticated declarations) and both `unique`
flags, and no multiplicity bound and no identity the store minted; the extents
in the order supplied; and the population enumerator. The store admits the
extents under conditions 1 to 4, builds the manifest, and answers `complete`,
`targets`, `sources` and `resolve` with the read contract below. It decides no
charge, reads no document and authenticates no source: the bundle's key is the
model layer's.

### The read contract: what the evaluator may rely on

Runtime navigation reads an observation's relationship instances through
exactly these reads, and no other:

| Read | Result |
| --- | --- |
| `complete(R)` | `true` exactly when the observation holds a complete extent of `R`; `false` for an unlisted or incomplete extent |
| `targets(R, r)` | the target reference of every instance of `R` whose source reference equals `r`, one per instance, in ascending reference-key order; repeats of one reference are adjacent |
| `sources(R, r)` | the source reference of every instance of `R` whose target reference equals `r`, in the same order |
| `resolve(x)` | `present` when the observation holds the object `x` names, whatever the manifest says; otherwise `dangling` or `undecidable` by the manifest, as the next section states |

Reference equality is equality of the whole reference key (universe, type,
object identity), never of the object identity alone or of a display name.

Every read is pure and total: it charges nothing (charges belong to the
evaluator), reads no field value, no population member list, no path, clock or
global, and returns the same result for the same observation. No read mutates
the observation: an extent is fixed when admitted, and a navigation leaves the
observation equal to what it was. `targets` and `sources` of an unlisted or
incomplete extent return what the extent holds; the evaluator reads
`complete(R)` first and does not use them when it is `false`.

### Runtime navigation reads only the extent of its relationship

For a navigation `r.name` whose site resolved to relationship `R`, with the
destination end's multiplicity `{l, u}`, a traversal from the source reads `targets(R, r)`
and a traversal from the target reads `sources(R, r)`. A field of the receiver
is never read as a link and a link is never read as a field: a name matching
both is refused at checking, not here. The evaluator SHALL then, stopping at
the first outcome:

1. charge `model.navigate`;
2. if `complete(R)` is `false`, return `incomplete_population`/
   `incomplete-scope`, retaining `R` and the receiver, with no further
   charge;
3. take the selected references in ascending reference-key order;
4. for a single-valued end (`[0,1]` or `[1,1]`), decide with no charge each
   selected reference's `resolve`, in order, then the count bound, and return;
5. for a multi-valued end, decide each selected reference's `resolve` with no
   charge and then charge one `collection.visit` for it, in order; after the
   last charge `collection.bound`, decide the count bound with no charge, and
   charge `collection.result-retain`.

A `resolve` of `dangling` is `dangling_reference`/
`absent-target-in-complete-population`, retaining the reference and the
relationship; of `undecidable`, `incomplete_population`/`incomplete-scope`,
retaining the reference and the relationship. The first selected reference
that is not `present` decides the outcome, whichever kind it is: an
`undecidable` reference before a `dangling` one gives the incomplete outcome,
and the reverse gives the refusal. Both come before the count bound, so a
missing target never yields a cardinality refusal. A count below `l` is `cardinality_out_of_bound`/
`below-minimum` and a count above a finite `u` is `cardinality_out_of_bound`/
`above-maximum`, each retaining the collection type, the inclusive bound, the
count and the expression locus; the count is the number of selected
instances. A refusal makes no later charge, so a multi-valued bound refusal
charges no `collection.result-retain` and returns no value. A denied charge is
the incomplete outcome of the charge point, with no value.

The result is the multiplicity-determined value of FR-152: `[1,1]` the one
`Reference<U>`; `[0,1]` the `Option<Reference<U>>`, `none` for count 0 and the
present reference for count 1; any other finite `{l, u}` the `Set<Reference<U>>[l,u]`
when `unique` is `true` and the `Bag<Reference<U>>[l,u]` when it is `false`, in
ascending reference-key order. The static form, direction, ambiguity and
`unbounded` or `ordered` refusals are decided at checking and are not decided
here. The role `name` is the one nearest the receiver, as FR-363 and QSpec
FR-322 resolve it: the source end's role names the forward traversal from the
source type, the target end's role the inverse traversal from the target
type, and `{l, u}` and `U` belong to the end the role does not name, as in
FR-363; a navigation site reaches the evaluator already resolved.

### Where each refusal fires

| Condition | Fires at | Code and cause |
| --- | --- | --- |
| Unknown or non-navigation relationship | extent admission, condition 1 | `invalid_runtime_input`/`wrong-role-mapping` |
| Two extents of one relationship | extent admission, condition 2 | `invalid_runtime_input`/`conflicting-identity` |
| End object of a type that does not conform to the end type | extent admission, condition 3 | `invalid_runtime_input`/`wrong-value-kind` |
| Repeated instance where an end is unique | extent admission, condition 4 | `invalid_runtime_input`/`conflicting-identity` |
| Dangling end in a snapshot | snapshot admission, closure check | `dangling_reference`/`absent-target-in-complete-population` |
| Required population of an instance not complete | snapshot admission, completeness check | `incomplete_population`/`incomplete-scope` |
| End reference of a model-layer extent, absent, of a type that does not conform | extent admission, condition 3 | `invalid_runtime_input`/`wrong-value-kind` |
| Part of a bundle from another source or selection | bundle construction | `invalid_runtime_input`/`conflicting-identity` |
| Dangling end of a model-layer extent | navigation, before the count bound | `dangling_reference`/`absent-target-in-complete-population` |
| End of a model-layer extent that `resolve` cannot decide | navigation, before the count bound | `incomplete_population`/`incomplete-scope` |
| Extent unlisted or incomplete | navigation, after `model.navigate` | `incomplete_population`/`incomplete-scope` |
| Count below `lower` or above `upper` | navigation, after the dangling decisions | `cardinality_out_of_bound`/`below-minimum` or `above-maximum` |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-364-AC-1 | Extent admission refuses each of its four conditions alone with its code and cause: a relationship naming no navigation relationship (an unknown key, and separately a connection) `wrong-role-mapping`; a second extent of one relationship `conflicting-identity`; a `Tank` at the source end of a relationship whose source end is `Sys`, and separately at its target end, `wrong-value-kind` naming the end; a repeated instance of a relationship whose ends are unique `conflicting-identity` at the later instance. A repeated instance of a relationship whose two ends both declare `unique: false` admits as two instances; with only one of the two ends non-unique it refuses. | Test (TC-1033) |
| FR-364-AC-2 | Admission compares no count with a bound: an extent giving one object four targets through an end `{0,3}`, an object with no instance through an end `{1,1}`, and an extent with no instance at all each admit. | Test (TC-1033) |
| FR-364-AC-3 | `targets` and `sources` return references in ascending reference-key order whatever order the extent was built in, `sources` returns the references of the opposite end of the same instances `targets` returns, a repeated instance of a non-unique relationship returns its reference twice and adjacently, and two references with equal object identity in different universes are not equal. | Test (TC-1033) |
| FR-364-AC-4 | An unlisted extent and an incomplete extent give `complete(R)` `false`, a complete extent with no instance gives `true` with empty `targets`; the three are never read as one another. Reading leaves the observation equal to what it was, and two evaluations of one navigation give equal results. | Test (TC-1033) |
| FR-364-AC-5 | Navigating a multi-valued end `{0,3}` with two targets returns the `Set<Reference<U>>[0,3]` in reference-key order after `model.navigate`, two `collection.visit`, `collection.bound` and `collection.result-retain`, and a work-unit limit equal to that schedule completes identically while one unit fewer is incomplete at `collection.result-retain`. A non-unique end holding one target twice returns the `Bag<Reference<U>>[0,3]` with that target twice. | Test (TC-1034) |
| FR-364-AC-6 | A count above a finite `upper` refuses `cardinality_out_of_bound`/`above-maximum` after `model.navigate`, one `collection.visit` per selected reference and `collection.bound`, with no `collection.result-retain` and no value; a count below `lower` refuses `below-minimum` at the same point; a single-valued end with count 0 through `[1,1]` refuses `below-minimum` and with count 2 refuses `above-maximum`, each after only `model.navigate`. | Test (TC-1034) |
| FR-364-AC-7 | A single-valued end returns its `Reference<U>`, and for `[0,1]` the present `Option` or `none`, after `model.navigate` alone; a traversal from the target end returns the `sources` of the same extent. | Test (TC-1034) |
| FR-364-AC-8 | A dangling target of a model-layer extent refuses `dangling_reference`/`absent-target-in-complete-population` before the count bound: with four selected targets through `{0,3}` the fourth absent, the refusal comes after `model.navigate` and three `collection.visit` and no cardinality refusal is returned. A target absent from a population not held complete gives `incomplete_population`/`incomplete-scope`. | Test (TC-1034) |
| FR-364-AC-9 | Navigating through an unlisted extent and through an incomplete extent each give `incomplete_population`/`incomplete-scope` after `model.navigate` and no later charge, while the complete empty extent returns the empty collection or `none`. | Test (TC-1034) |
| FR-364-AC-10 | The manifest has one row per declared population in ascending declaration-key order, with the declared member types as covered types: a bound population and a population listed complete are complete rows, and a declared population the observation does not bind or list, and one listed incomplete, are incomplete rows. An enumerator that reports normal exhaustion, including exactly at its row limit, gives a closed manifest, and one stopped by the row limit or by a failure gives a partial manifest that holds the rows read. No operation closes a manifest from supplied rows, from a partial manifest or on request. | Test (TC-1038) |
| FR-364-AC-11 | `resolve` of a missing reference gives `dangling` only from a closed manifest in which every row covering its type is complete and at least one row covers it. It gives `undecidable` when a covering row is incomplete (even beside a complete covering row), when no row covers the type (a type no declared population names, and a parent type whose declared children are covered), and under a partial manifest. A held object is `present` under any manifest. | Test (TC-1038) |
| FR-364-AC-12 | Navigation over a model-layer extent whose target is missing and undecidable gives `incomplete_population`/`incomplete-scope` after `model.navigate` and the `collection.visit` of the targets before it, and no cardinality refusal even when the count is above `upper`; the first selected target that is not present decides between `dangling` and `undecidable`. | Test (TC-1038) |
| FR-364-AC-13 | Condition 3 checks the type of every end of a model-layer extent whether the object is held or not: a missing `Tank` at the `Pump` end refuses `wrong-value-kind` naming the end, `Tank` and `Pump`; a missing `Pump` at that end admits and is decided by `resolve`; a reference whose type is no object type of the model refuses `wrong-value-kind`. A snapshot document end naming an unlisted key skips condition 3 and is decided by the closure rule. | Test (TC-1038) |
| FR-364-AC-14 | A bundle whose manifest, closure or type environment carries a key other than the others' (another source digest under the same authority and identity, another selected clause, another selected document digest) is refused `invalid_runtime_input`/`conflicting-identity` naming the part, the selected key and the differing one; a bundle of one key resolves a missing reference by its manifest. | Test (TC-1038) |

## Dependencies

- **Upstream:** [FR-084](FR-084-admit-closed-populations-and-resolve-lookup.md)
  supplies the reference key and the population closure this requirement
  resolves against; [FR-085](FR-085-resolve-relationship-end-references.md)
  supplies the navigation relationship, its ends and its multiplicity;
  [FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md) supplies
  conformance. quire-specification FR-152 owns the navigation result types,
  the traversal rule, the refusals and the `model.navigate` schedule of
  `quire.value.accounting/v1`; AD-006 owns the decision to keep populations in
  the model view.
- **Downstream:** [FR-106](FR-106-admit-snapshots-and-invocations.md) binds
  extent admission to the snapshot document and decides the dangling and
  completeness rules for it. The checker's typing of a relationship end, its
  direction, ambiguity and `unbounded` or `ordered` refusals, and the
  `quire.op.model.navigate` lowering are FR-363's.
- [FR-362](FR-362-bind-a-checked-unit-to-its-source-by-digest.md) supplies the
  source reference the bundle key compares as a whole. quire-semantic-value
  [FR-119](ix://agent-ix/quire-semantic-value/FR-119) is the store this
  requirement's reads and manifest are built in.
- The object environment holds the extents. Whether the extent store lives in
  the semantic-value object closure or in the model layer's environment beside
  it is an implementation placement; this requirement fixes only the reads
  above.
