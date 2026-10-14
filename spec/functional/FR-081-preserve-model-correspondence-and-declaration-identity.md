---
id: FR-081
title: "Preserve immutable model correspondence and original/effective declaration identity"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: references
  - target: ix://agent-ix/quire-specification/FR-150
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-006
    type: depends_on
  - target: ix://agent-ix/quire-specification/interface_004
    type: references
---
# FR-081: Preserve immutable model correspondence and original/effective declaration identity

## Description

When the model binder normalizes the original declarations
[FR-056](FR-056-admit-domain-package-model-declarations.md) admits from one
domain package, it SHALL derive one immutable correspondence between each
original declaration and the effective declarations, where selected rules
derive them. Every original
declaration key SHALL survive unchanged in that correspondence. Every derived
declaration SHALL carry a normalized identity computed only from the
correspondence's own preimage. No identity, key or ordering the binder
produces SHALL depend on a display name, a registration order or any state
held outside the one `DomainPackage` value and limits the caller supplies.

This requirement is QSL's own binding of quire-specification
[FR-150](ix://agent-ix/quire-specification/FR-150) ("Preserve original and
effective model declarations") and [AD-006](ix://agent-ix/quire-specification/AD-006)'s
model-view decisions into the compiler's actual types and behavior; FR-150
states the normative rule, this requirement states QSL's binder contract for
it.

## Inputs

- The original declarations [FR-056](FR-056-admit-domain-package-model-declarations.md)
  builds, each keyed by `(domain package identity, IR node identity,
  sha256-jcs)`.
- The declared `supertypes`, `subsets` and `redefines` edges each original
  declaration carries as an inline property (never a second producer record).
- The selected `quire.model.complete/v1` definition and its rule manifest
  `quire.model.complete.rules/v1`.
- `ModelNormalizationLimitsV1` of `quire.value.accounting/v1`.

The selection identities above stay as they are: no consumer's selection
changes, so FR-056 and every requirement or test that selects them keep their
text. The identity domains `quire.model.effective-declaration`,
`quire.model.effective-view` and `quire.model.object-universe` move to `/v2`,
and the inherit, subset and redefine rule identities move to `/v2`; the
definition, manifest and accounting definition are edited in place to carry
them.

## Outputs

An immutable original-to-effective view — one `EffectiveId`-keyed entry per
resolved effective declaration, each retaining its original declaration key
and an ordered derivation-fact trail, together with the complete retained
original declaration inventory — or a typed refusal or incomplete
result.

## Behavior

### Original identity never moves

The model binder SHALL treat each original declaration's key as quire-specification
[FR-150](ix://agent-ix/quire-specification/FR-150) fixes it: never minted,
renamed or merged, and never carrying an edge the domain package does not
declare. This requirement does not restate FR-150's keying rule; a
supertype, subset or redefine edge exists in the correspondence only when the
admitted declaration itself carries it.

### Effective identity is a normalized digest over the correspondence, never over display text

The correspondence owner SHALL retain the exact intake inventory and actual
FCD origins, separately from its effective-entry set. Phase-2 qualification
applies only to the kinds selected by QSpec `model-complete.md`; a scalar,
clause or population without a qualify rule remains an original without a
fabricated effective entry or fact. Every effective entry and fact input SHALL
refer to a key from this inventory. Retention introduces no additional
effective-declaration or hash charge and no effective preimage member.

Each effective declaration's identity is the `quire.model.effective-declaration/v2`
digest quire-specification [FR-150](ix://agent-ix/quire-specification/FR-150)
defines, over exactly `{version, owner_effective_type, original, derivation}`;
this requirement does not restate that preimage. The model binder SHALL NOT
read a `title`, `displayName` or any other presentation field when computing
an identity, a key, a digest, an ordering or a resolution decision. Two
admitted declarations for which the selected normalization rules derive
effective entries, with equal declared shape but distinct original declaration
keys, SHALL yield distinct effective declarations; the binder SHALL NOT
collapse those derived declarations by an identity-equivalence rule. An
original whose kind has no selected derivation rule remains in the exact
original inventory without acquiring an effective identity from this guarantee.

A variant type is an effective type and each of its cases an effective member of
it, each with one `qualify` fact `[declaration]` and no other (FR-056
"Effective identity of a variant type"). A case's `original` is its case node's
declaration key, so its identity does not depend on the order of the
`variants` list.

### An identity chains through its parents' identities

An effective declaration's preimage SHALL bind exactly two things: its own
content, and the effective identity of each of its direct parents. A direct
parent of an effective type is each of its direct supertypes. A direct parent
of an effective member `(T, m)` is the effective member `(P, m)` of each direct
supertype `P` of `T` that has one. The preimage SHALL NOT contain an ancestor
path of more than one edge. A parent identity is a fixed-width digest field,
so it adds a constant number of bytes however deep its ancestry is.

The retained derivation facts are the one-edge facts: one qualify fact, one
inherit fact per direct parent (carrying that parent's identity), and the
phase-4 subset and redefine facts of the declaration's own and its
redefiners' original declarations, each once. A diamond stores one inherit
fact per direct edge, never one per path. Hidden declarations (a redefined or
losing member) are computed, retained and ordered by digest exactly as visible
ones; hiding is not part of any preimage.

The full path-form derivation, every distinct inheritance path included, is
the deterministic expansion of the retained facts through the parent
identities the view holds. A reader expands it on demand under its own limits;
the binder SHALL NOT encode or hash an expanded path. The expansion of a
declaration equals the path-form facts in phase, input and rule order, with
multiplicity retained, where a fact's path is as follows. An inherit fact
expands to one fact per distinct path from the declaration's type to the
owner, in ascending-key depth-first order. A redefine fact on `(T, r)` or
`(T, t)` expands to exactly one fact per redefiner `r` reaching `T`, carrying
the least path from `T` to `r`'s owner (the first in that order), whether one
or several paths reach the owner; a subset fact on `(T, s)` expands to one
fact per subsets target, carrying the least path from `T` to `s`'s owner. So
the expansion reproduces the path-form fact array exactly where one path
reaches each owner, and for a diamond it lists every path in the inherit facts
and the least path in the phase-4 facts. Because each preimage binds its parents' identities,
equal preimages have equal expansions, so two effective declarations that
differ in owner, original key or expanded facts have different preimages; and
one effective declaration has exactly one preimage.

The binder SHALL compute every effective identity eagerly, hidden declarations
included, then every universe identity and the complete digest-ordered view
identity, hashing parents before children. A parent link of the wrong kind, or
naming a declaration whose original or owner does not match the fact's inputs,
SHALL be refused by the strict reader. Every published identity vector is
replaced; a `/v1` identity digest is not an identity of the replacement, and the binder SHALL NOT
add a dual reader, digest relabelling, compatibility shim or public API alias.
The exact preimage grammar belongs to
[FR-150](ix://agent-ix/quire-specification/FR-150) and the selected
definition; this requirement does not create a second identity writer.

Total bytes admitted to the identity hash calls of a package with `V` effective
types and `E` direct supertype edges and no members are linear in `V + E`
(about `834 V + 574 E` bytes for the fixture of TC-1024), not in the
number of ancestor paths or their length.

### Losing redefinitions are retained, not deleted

When two or more declared redefinitions of one member reach the same
effective type and one redefining owner is a proper descendant of every
other, the binder SHALL resolve that descendant's redefinition as effective
and SHALL retain every other reaching redefinition's entry in the
correspondence, marked as not the member a name resolves to. A losing
redefinition's original key, its
derivation facts and its own effective identity SHALL remain readable from
the correspondence for provenance and diagnostics.

### A conflicting redefinition exposes no effective member for either path

When two or more declared redefinitions of one member reach the same
effective type and no single redefining owner is a proper descendant of
every other — the "Losing redefinitions are retained" rule above does not
apply because no descendant exists to resolve the conflict — the binder
SHALL refuse normalization for that member with a derivation-conflict cause
naming the conflicting type, the member and every competing redefiner, and
SHALL expose no effective member for either reaching redefinition, winning or
losing. This is quire-specification [FR-150](ix://agent-ix/quire-specification/FR-150)-AC-5's
"conflicting derivations report both rule paths and expose no chosen
effective member," bound to this compiler's refusal type.

### Correspondence ordering never depends on IR node order

Replaying normalization over the same `DomainPackage` value with its IR
nodes presented in a different order SHALL produce a byte-identical
original-to-effective view: the same original keys, the same effective
identities and the same correspondence ordering. This is quire-specification
FR-150-AC-4's order-independence guarantee, bound to this compiler's
correspondence type.

### The binder holds no ambient registry

The model binder SHALL be a pure function of the one `DomainPackage` value
and the one limits value its caller supplies. It SHALL hold no static,
thread-local or process-global table of declarations, and no binder run
SHALL observe or be observed by another run's declarations. Given the same
`DomainPackage` and the same limits, every call SHALL produce byte-identical
original keys, effective identities and correspondence ordering, whether the
calls occur in the same process or in independent processes.

### Charging and incompleteness

The model binder SHALL charge normalization work under
`ModelNormalizationLimitsV1` before exposing a derived fact. If a charge is
denied, the binder SHALL return an incomplete result naming the exhausted
charge point and SHALL expose no effective declaration for the run.

Each direct supertype edge SHALL be charged once, as one `normalize.fact` and
one `normalize.cycle-check` unit, however many paths reach its target; the
binder SHALL NOT charge an ancestor path's length for an acyclic package.
`derivation_facts` counts the retained facts, not the path-form facts of their
expansion. A preimage's length is counted from its parts, a parent digest
being fixed width, and is charged to `hashed_bytes` before the preimage is
encoded or hashed, with one exception: an effective type's identity is hashed
in phase 3, before its phase-5 `normalize.hash`, because phase 4's charge order
uses it. That preimage holds only the type's own key and its direct edges,
each already admitted by a `normalize.fact` and a `normalize.cycle-check`
charge. Every member, universe and view preimage is charged before it is
encoded.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-081-CON-1 | The model binder SHALL NOT read `title`, `displayName` or any other presentation field of an admitted declaration for any identity, key, digest, ordering or resolution purpose. | Design | Test (TC-216) |
| FR-081-CON-2 | The model binder's public entry point SHALL take the `DomainPackage` and limits as explicit parameters. | Design | Inspection |
| FR-081-CON-3 | The model binder SHALL declare no static, `OnceCell`, `thread_local` or other ambient state carrying declarations between calls. | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-081-AC-1 | Given a domain package admitted by FR-056, every original declaration key the binder's correspondence names is byte-identical to the key FR-056 admitted for it; no key in the correspondence is absent from, or added beyond, the admitted set. | Test (TC-213) |
| FR-081-AC-2 | Two effective declarations derived from distinct original declaration keys are never equal, even when their declared shape (fields, types, multiplicities) is identical; each effective identity differs because its preimage's `original` component differs. | Test (TC-214) |
| FR-081-AC-3 | Given a three-level chain where a base type declares a member and two of its descendants each independently redefine it, so that both redefinitions reach the most specific type's effective member, the more specific (descendant) redefiner's entry is the one a name resolves to, and the less specific redefiner's entry remains present in the correspondence with its own original key, derivation facts and effective identity readable rather than being deleted. | Test (TC-215) |
| FR-081-AC-4 | Given two admitted domain packages that are byte-identical except for a `displayName`/title field on one declaration, every original key, effective identity and correspondence ordering the binder produces is byte-identical between the two runs. | Test (TC-216) |
| FR-081-AC-5 | Given the same `DomainPackage` value and the same limits, two independent binder invocations — including invocations in two separate process runs — produce byte-identical original keys, effective identities and correspondence ordering, with no observable interaction between the runs. | Test (TC-217) |
| FR-081-AC-6 | Given two declared redefinitions of one member reaching the same effective type where neither redefining owner is a proper descendant of the other, normalization refuses derivation-conflict naming both redefiners and their owning types, and the correspondence exposes no effective member for either redefinition — not the winning one FR-081-AC-3's descendant case would pick, because no descendant exists here. | Test (TC-237) |
| FR-081-AC-7 | Given the same `DomainPackage` value with its IR nodes presented in two different orders, normalization from each produces byte-identical original keys, effective identities and correspondence ordering. | Test (TC-238) |
| FR-081-AC-8 | Given two object types, one field, one operation and one scalar admitted by FR-056, the retained original inventory contains exactly their admitted keys and origins. Effective-entry/fact origins are a subset; the scalar with no selected qualify rule creates no qualify fact, effective entry or extra declaration/hash charge. | Test |
| FR-081-AC-9 | Expanding the retained facts of every effective declaration through its parent identities yields the path-form facts of an independent enumeration over the original declarations, in phase, input and rule order with multiplicity, for a chain, a diamond and a hidden redefinition; a link naming a declaration of the wrong kind, owner or original is refused. | Test (TC-1022) |
| FR-081-AC-10 | Changing one ancestor's original declaration key, one parent link, or one phase-4 fact changes the identity of every descendant that reaches it, and no two distinct effective declarations of a fixture set share an identity. | Test (TC-1023) |
| FR-081-AC-11 | Hashed bytes, `derivation_facts` and `work_units` of a chain, a width-2 diamond ladder and a dense multi-parent shape equal the closed formulas of the test case; a diamond ladder whose path count is exponential is admitted at the default limits. | Test (TC-1024) |
| FR-081-AC-12 | For a deterministic single-inheritance chain of 10,000 edges (10,001 effective types, package encoding at most 524,288 bytes), normalization at the NFR-012 default limits eagerly computes every effective identity and the complete view with 20,001 retained facts, at most 16,777,216 hashed bytes and at most 100,000 work units. | Test (TC-1025) |
| FR-081-AC-13 | The FR-081-AC-12 case, run in an isolated process, has a peak resident set of at most 21,474,836,480 bytes and a wall time of at most 30 seconds in the release profile on the reference host Terra, each measured from package input admission through retention of the complete result. | Test (TC-1025) |
| FR-081-AC-16 | Given a variant type with two cases, the effective view holds one effective type and two effective members of it, each derived from the key of its own declaration with one `qualify` fact; permuting the cases changes no effective identity or position. | Test (TC-1003) |

## Dependencies

- **Upstream:** [FR-056](FR-056-admit-domain-package-model-declarations.md)
  supplies the original declarations this requirement normalizes;
  quire-specification FR-150 and AD-006 own the normative rule and model-view
  decisions this requirement binds; quire-specification interface_004 (I04)
  is the immutable checked-package boundary the resulting correspondence is
  carried across, as a `correspondence` node.
- **Downstream:** [FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md),
  [FR-083](FR-083-resolve-unique-most-specific-dispatch.md) and
  [FR-084](FR-084-admit-closed-populations-and-resolve-lookup.md) resolve
  conformance, dispatch and population membership over the correspondence
  this requirement produces.
- [TC-1022](../test-cases/TC-1022-model-identity-links-expand-to-exact-provenance.md),
  [TC-1023](../test-cases/TC-1023-model-identity-chain-is-unique-and-binds-ancestry.md),
  [TC-1024](../test-cases/TC-1024-model-identity-cost-is-linear.md) and
  [TC-1025](../test-cases/TC-1025-full-size-model-identity-case.md) verify the
  chain-link identity, its uniqueness and its cost.
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-03 and O-05 record QSL's ownership of original declaration identity and
  effective declaration identity; this requirement states their behavioral
  contract, not their Rust module placement.
