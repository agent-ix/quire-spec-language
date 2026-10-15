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
"Navigation" section states.

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
model-layer caller, admission SHALL decide these conditions in this order and
stop at the first:

1. If the relationship names no navigation relationship of the admitted
   domain package (an unknown key, a connection, an allocation or any other
   declaration), then admission SHALL refuse `invalid_runtime_input`/
   `wrong-role-mapping`, retaining the supplied relationship.
2. If the observation already holds an extent for the relationship, then
   admission SHALL refuse `invalid_runtime_input`/`conflicting-identity`,
   retaining the relationship.
3. If the object at the source end has a type that does not conform to the
   relationship's source end type, or the object at the target end has a type
   that does not conform to the target end type, then admission SHALL refuse
   `invalid_runtime_input`/`wrong-value-kind`, retaining the relationship, the
   end (`source` or `target`), the object's type and the declared end type.
   Conformance is [FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md)'s,
   reflexive. An end whose object the observation does not hold has no type
   to check; condition 3 skips it, and the closure rule below decides it.
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
population is complete, and the incomplete outcome when it is not. An extent a
model-layer caller supplies carries full references and is admitted without a
closure check, so the navigation below decides a dangling end of such an
extent. Neither place changes the other's rule.

### The read contract: what the evaluator may rely on

Runtime navigation reads an observation's relationship instances through
exactly these reads, and no other:

| Read | Result |
| --- | --- |
| `complete(R)` | `true` exactly when the observation holds a complete extent of `R`; `false` for an unlisted or incomplete extent |
| `targets(R, r)` | the target reference of every instance of `R` whose source reference equals `r`, one per instance, in ascending reference-key order; repeats of one reference are adjacent |
| `sources(R, r)` | the source reference of every instance of `R` whose target reference equals `r`, in the same order |
| `resolve(x)` | `present` when the observation holds the object `x` names; `dangling` when it does not and the population of `x` is held complete; `undecidable` when it does not and the population is not held complete |

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
relationship; of `undecidable`, `incomplete_population`/`incomplete-scope`.
Both come before the count bound, so a dangling target never yields a
cardinality refusal. A count below `l` is `cardinality_out_of_bound`/
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
type, and `{l, u}` and `U` belong to the end the role does not name. QSpec
FR-152's examples name the far end; this requirement follows FR-363; a navigation site reaches the evaluator already resolved.

### Where each refusal fires

| Condition | Fires at | Code and cause |
| --- | --- | --- |
| Unknown or non-navigation relationship | extent admission, condition 1 | `invalid_runtime_input`/`wrong-role-mapping` |
| Two extents of one relationship | extent admission, condition 2 | `invalid_runtime_input`/`conflicting-identity` |
| End object of a type that does not conform to the end type | extent admission, condition 3 | `invalid_runtime_input`/`wrong-value-kind` |
| Repeated instance where an end is unique | extent admission, condition 4 | `invalid_runtime_input`/`conflicting-identity` |
| Dangling end in a snapshot | snapshot admission, closure check | `dangling_reference`/`absent-target-in-complete-population` |
| Required population of an instance not complete | snapshot admission, completeness check | `incomplete_population`/`incomplete-scope` |
| Dangling end of a model-layer extent | navigation, before the count bound | `dangling_reference`/`absent-target-in-complete-population` |
| Extent unlisted or incomplete | navigation, after `model.navigate` | `incomplete_population`/`incomplete-scope` |
| Count below `lower` or above `upper` | navigation, after the dangling decisions | `cardinality_out_of_bound`/`below-minimum` or `above-maximum` |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-364-AC-1 | Extent admission refuses each of its four conditions alone with its code and cause: a relationship naming no navigation relationship (an unknown key, and separately a connection) `wrong-role-mapping`; a second extent of one relationship `conflicting-identity`; a `Tank` at the source end of a relationship whose source end is `Sys`, and separately at its target end, `wrong-value-kind` naming the end; a repeated instance of a relationship whose ends are unique `conflicting-identity` at the later instance. A repeated instance of a relationship whose two ends both declare `unique: false` admits as two instances; with only one of the two ends non-unique it refuses. | Test (TC-1033) |
| FR-364-AC-2 | Admission compares no count with a bound: an extent giving one object four targets through an end `{0,3}`, an object with no instance through an end `{1,1}`, and an extent with no instance at all each admit. | Test (TC-1033) |
| FR-364-AC-3 | `targets` and `sources` return references in ascending reference-key order whatever order the extent was built in, `sources` returns the references of the opposite end of the same instances `targets` returns, a repeated instance of a non-unique relationship returns its reference twice and adjacently, and two references with equal object identity in different universes are not equal. | Test (TC-1033) |
| FR-364-AC-4 | An unlisted extent and an incomplete extent give `complete(R)` `false`, a complete extent with no instance gives `true` with empty `targets`; the three are never read as one another. Reading leaves the observation equal to what it was, and two evaluations of one navigation give equal results. | Test (TC-1033) |
| FR-364-AC-5 | Navigating a multi-valued end `{0,3}` with two targets (and, for a non-unique end, a repeated target twice) returns the `Set<Reference<U>>[0,3]` in reference-key order after `model.navigate`, two `collection.visit`, `collection.bound` and `collection.result-retain`, and a work-unit limit equal to that schedule completes identically while one unit fewer is incomplete at `collection.result-retain`. | Test (TC-1034) |
| FR-364-AC-6 | A count above a finite `upper` refuses `cardinality_out_of_bound`/`above-maximum` after `model.navigate`, one `collection.visit` per selected reference and `collection.bound`, with no `collection.result-retain` and no value; a count below `lower` refuses `below-minimum` at the same point; a single-valued end with count 0 through `[1,1]` refuses `below-minimum` and with count 2 refuses `above-maximum`, each after only `model.navigate`. | Test (TC-1034) |
| FR-364-AC-7 | A single-valued end returns its `Reference<U>`, and for `[0,1]` the present `Option` or `none`, after `model.navigate` alone; a traversal from the target end returns the `sources` of the same extent. | Test (TC-1034) |
| FR-364-AC-8 | A dangling target of a model-layer extent refuses `dangling_reference`/`absent-target-in-complete-population` before the count bound: with four selected targets through `{0,3}` the fourth absent, the refusal comes after `model.navigate` and three `collection.visit` and no cardinality refusal is returned. A target absent from a population not held complete gives `incomplete_population`/`incomplete-scope`. | Test (TC-1034) |
| FR-364-AC-9 | Navigating through an unlisted extent and through an incomplete extent each give `incomplete_population`/`incomplete-scope` after `model.navigate` and no later charge, while the complete empty extent returns the empty collection or `none`. | Test (TC-1034) |

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
  `quire.op.model.navigate` lowering are FR-363's (open pull request #867).
- The object environment holds the extents. Whether the extent store lives in
  the semantic-value object closure or in the model layer's environment beside
  it is an implementation placement; this requirement fixes only the reads
  above.
