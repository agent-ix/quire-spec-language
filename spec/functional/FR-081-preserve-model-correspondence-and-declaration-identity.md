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
- The selected `quire.model.complete/v2` definition and its rule manifest
  `quire.model.complete.rules/v2`.
- `ModelNormalizationLimitsV1` of `quire.value.accounting/v2`, with unchanged
  public object/member names and the replacement model-identity charge semantics.

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

The binder SHALL use the selected definition's closed key/block/range,
literal/prefix and declaration-sequence grammar and its unique normal form.
Its compressed preimages SHALL expand to every exact semantic fact, input key,
rule and ordinal in phase/input/rule order with identical-fact multiplicity,
with each distinct inheritance path
retained. The binder SHALL eagerly compute ALL effective identities, hidden
entries included, followed by every universe identity and the complete
digest-ordered view identity. Shared helper commitments are not extra facts
or effective declarations. This requirement delegates exact preimage grammar
and factoring/tie-break rules to FR-150 and the selected model definition;
it does not create a second identity writer.

The retained view SHALL provide every declaration's compressed preimage and
reachable exact key/block records as well as immutable originals/origins.
The strict consumer boundary SHALL verify admitted selection, semantic
normalization, canonical overlap/expansion, every eager digest and the complete
inventory. A root-only view or requested-member proof is not that transport.
The checked graph's structural NodeKey-to-DeclarationKey correspondence is
not the full model effective view and SHALL NOT substitute for it.

This initial norm draft SHALL NOT be adopted with the old wire/schema/vector
bytes or old selected closure. The pre-release replacement requires one
coherent selection, rule manifest, accounting, producer edge protocol, strict
reader and caller-minted identity-domain cohort before any code cutover.
Optional operation `redefines` is supported by the producer document schema
and retained by the released selected-document IR reader; an older locked
reader is not adoption of that capability. Before code cutover actual producer
edges and origins SHALL survive selected-document admission, proper-ancestor
normalization, full provenance/correspondence re-export and independent
reconstruction under the same selected strict-reader contract. Kind, inherited
target, cycle/conflict and signature/contract checks remain at their owning
stages; absent, changed, malformed, dangling and ambiguous-edge controls remain.
Schema support or fairness name resolution alone cannot establish this full
round-trip, complete eager inventory or a bounded sole-writer export. Matched
wire/schema/vectors/domains and admitted reader/transport costs remain
prerequisites, not an unconditional replacement-readiness claim.
The binder SHALL NOT introduce dual readers, old-digest transport, digest
relabelling, compatibility shims or public API aliases for this replacement.

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

The binder SHALL follow replacement accounting's declaration-first phase-5
schedule, with separately admitted metadata scans, normal-form work, descriptor
lookups/validation/pricing and each dependency's pre-encoding hash charge.
It SHALL NOT compute type/helper hashes in phases 1–4, encode a node to discover
its price, reserve bytes posthoc or use another canonical encoder. Its exact
analytical template price SHALL equal the sole writer's encoded length before
admitting the digest. A denied event performs none of its named work; a denied
hash performs zero node encoding/hashing and no dependent work. Previously
admitted preparation remains charged. Selection-local shared hits retain
their lookup charges, never bypass the operation meter or create ambient state.
Its delegated logical event interpreter includes every closed-template root,
field/array/scalar read, original/owner/header slot, dependency reuse lookup and
binding proof, range/provenance validation, retained-slot price and actual hash.
Pricing's v charge covers its retained-slot reads without optional additional
field events or text rescans. Metadata first use is the admitted selection-local
key rank/field occurrence (or selection identity field), not equal text or
physical interning. Different key ranks with equal package text are scanned
separately; reuse of one occurrence retains the charged lookup without rescanning.
All owner/root/universe/complete-sequence/dictionary orderings and prerequisite
indexes use accounting's prescribed projections, bottom-up stable comparisons,
logical moves and admissions. No physical search/sort choice changes the
public event sequence or first denial. Normal form retains BOTH equal-input
subset/redefine facts: for admitted subtype field `s` subsetting and redefining
proper ancestor field `t`, `[s,t]` redefine precedes `[s,t]` subset by exact
rule-identity bytes. A swapped-rule, dropped-occurrence or deduplicated candidate
cannot pass canonical expansion; valid overlap alone cannot cause refusal.
Existing canonical-expansion and exact-denial acceptance includes these adverse
cases, metadata hits/misses and ordering prerequisites under different layouts,
with byte-identical identities/traces and zero named/dependent work at denial.
Phase-4 conflict order uses owning original keys as the accounting authority
states, without changing the meaning of an effective member key. All existing
logical counters, one-meter stage transfers and failure suppression remain
unchanged; a later conformance refusal cannot replace an earlier hash denial.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-081-CON-1 | The model binder SHALL NOT read `title`, `displayName` or any other presentation field of an admitted declaration for any identity, key, digest, ordering or resolution purpose. | Design | Test (TC-216) |
| FR-081-CON-2 | The model binder's public entry point SHALL take the `DomainPackage` and limits as explicit parameters. | Design | Inspection |
| FR-081-CON-3 | The model binder SHALL declare no static, `OnceCell`, `thread_local` or other ambient state carrying declarations between calls. | Design | Inspection |

## Acceptance Criteria

Existing procedures TC-213, TC-214, TC-215, TC-216, TC-217, TC-237 and TC-238
retain their verifying relationships and criterion traces; method-only cells
do not delete or replace those procedures. Their existing bytes alone do not
qualify the replacement closure or the new adverse accounting cases.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-081-AC-1 | Given a domain package admitted by FR-056, every original declaration key the binder's correspondence names is byte-identical to the key FR-056 admitted for it; no key in the correspondence is absent from, or added beyond, the admitted set. | Test |
| FR-081-AC-2 | Two effective declarations derived from distinct original declaration keys are never equal, even when their declared shape (fields, types, multiplicities) is identical; each effective identity differs because its preimage's `original` component differs. | Test |
| FR-081-AC-3 | Given a three-level chain where a base type declares a member and two of its descendants each independently redefine it, so that both redefinitions reach the most specific type's effective member, the more specific (descendant) redefiner's entry is the one a name resolves to, and the less specific redefiner's entry remains present in the correspondence with its own original key, derivation facts and effective identity readable rather than being deleted. | Test |
| FR-081-AC-4 | Given two admitted domain packages that are byte-identical except for a `displayName`/title field on one declaration, every original key, effective identity and correspondence ordering the binder produces is byte-identical between the two runs. | Test |
| FR-081-AC-5 | Given the same `DomainPackage` value and the same limits, two independent binder invocations — including invocations in two separate process runs — produce byte-identical original keys, effective identities and correspondence ordering, with no observable interaction between the runs. | Test |
| FR-081-AC-6 | Given two declared redefinitions of one member reaching the same effective type where neither redefining owner is a proper descendant of the other, normalization refuses derivation-conflict naming both redefiners and their owning types, and the correspondence exposes no effective member for either redefinition — not the winning one FR-081-AC-3's descendant case would pick, because no descendant exists here. | Test |
| FR-081-AC-7 | Given the same `DomainPackage` value with its IR nodes presented in two different orders, normalization from each produces byte-identical original keys, effective identities and correspondence ordering. | Test |
| FR-081-AC-8 | Given two object types, one field, one operation and one scalar admitted by FR-056, the retained original inventory contains exactly their admitted keys and origins. Effective-entry/fact origins are a subset; the scalar with no selected qualify rule creates no qualify fact, effective entry or extra declaration/hash charge. | Test |
| FR-081-AC-9 | Expanding the canonical compressed provenance yields every ordered fact/input/rule/ordinal, including all hidden redefinitions and diamond paths; altered overlap, count, tail, ordinal meaning or alternative factoring cannot be admitted as the canonical correspondence. | Test |
| FR-081-AC-10 | Every effective entry has its eager digest and retained preimage; complete view/universe computation includes them all. Missing provenance records or a root-only/requested-member proof transport cannot pass complete-view admission. | Test |
| FR-081-AC-11 | Analytical closed-template prices agree with the sole writer for actual admitted strings; one-less or zero denial at preparation/hash charge points performs none of the denied work, including zero encoding/hashing of a denied node, while shared hits retain their prescribed lookup charges without rehashing. | Test |
| FR-081-AC-12 | An old or mixed selection/schema/reader/accounting/domain cohort is not admitted as the replacement through digest relabelling, a compatibility reader or an API alias. | Inspection |

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
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-03 and O-05 record QSL's ownership of original declaration identity and
  effective declaration identity; this requirement states their behavioral
  contract, not their Rust module placement.
