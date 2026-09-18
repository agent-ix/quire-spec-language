# Complete model definition

Definition identity: `quire.model.complete/v1`; revision: `1-draft.1`.
Language: `ix:native` / `1-draft`. This definition is the complete-V1 model,
inheritance, conformance, dispatch, systems-model and closed-environment root.
Implementation and qualification remain separately reported.

## Always-selected definitions

Every package that selects this root also selects the complete-value root and
its always-selected closure through
[`complete-value-lock.json`](complete-value-lock.json), and:

| Identity | Revision | Definition artifact |
| --- | --- | --- |
| `quire.state.graph/v1` | `1-draft.3` | [Finite graphs](state-graph.md) |
| `quire.model.complete.rules/v1` | `1-draft.1` | [Exact rule closure](model-complete-rules.json) |
| `quire.model.effective-declaration.schema/v1` | `1-draft.1` | [Effective identity schema](../../checked-package-v2/model-effective-declaration.schema.json) |
| `quire.model.effective-declaration.vectors/v1` | `1-draft.1` | [Effective identity vectors](../../checked-package-v2/model-effective-declaration-vectors.json) |

The accounting role is `quire.value.accounting/v1`, already selected by the
complete-value root. Its model and graph charge points and its
`ModelNormalizationLimitsV1` object are the only accounting for this root. The
QSL `quire.state.evaluation-work/1` counter set is not selected: a package
selecting this root is evaluated and checked only under
`quire.value.accounting/v1`, and no caller limit is clamped to a default.
Diagnostics are interpreted by `quire.native.diagnostics/v1` at revision
`1-draft.3` or later; that catalog is selected by the response, not by this
root.

This root supersedes, for a package that selects it, the "No overload
resolution or dynamic dispatch is introduced" sentence of the shared grammar and
the FS03 state-contract statement that inheritance, redefinition and dynamic
dispatch lie outside the state profile. Every other FS03 rule is retained.

## Producer dependency

This root consumes Filament producer interface `1.3.0`, the additive minor
requested by agent-ix/filament-core-data#143 under FCD FR-126. It is a named
dependency: this definition does not edit or implement the producer. The
member names below are the consumer's exact reading. A `1.3.0` document whose
spelling or member kind differs is `refused { code: unknown_wire, cause:
unsupported-wire }` until this definition is revised.

Every record named below carries a producer key: the correspondence
`ProducerObject.authority` (FCD FR-116), `identity`, the namespaced `revision`
(FCD FR-113, namespace `filament-core-data/producer-object-revision-1`) and a
`digest` selection whose domain is exactly `filament-canonical-json-1`. It also
carries its `locus`. The consumer never mints, renames, merges or reconstructs
a producer key.

| Capability | Record or inline member | Members read | Exact meaning |
| --- | --- | --- | --- |
| `generalization` | separate generalization record | `specificTypeIdentity`, `generalTypeIdentity` | The specific object type export directly conforms to the general object type export. |
| `subsetting` | separate subsetting record | `owningTypeIdentity`, `subsettingFeatureIdentity`, `subsettedFeatureIdentity` | In the owning type, the values of the subsetting feature are a subset of the values of the subsetted feature. |
| `redefinition` | separate redefinition record | `owningTypeIdentity`, `redefiningFeatureIdentity`, `redefinedFeatureIdentity` | In the owning type and its descendants, the redefining feature replaces the one inherited redefined feature. |
| `operation-signature` | inline `signature` on an `operation` member export | `receiverTypeIdentity`; ordered `parameters`, each `{parameterIdentity, typeIdentity, multiplicity}`; `result`, either `{typeIdentity, multiplicity}` or JSON `null` for no result; `effect`, exactly `{fieldWrites, creates, deletes}`, each an array of export identities | The receiver is parameter 0. The effect set is the operation's authored frame: the field exports it may write, and the object types it may create and delete. |
| `interface-signature` | inline `interfaceFeatures` on an `object` type export | ordered array of `{featureIdentity, featureKind}`, `featureKind` exactly `field` or `operation` | The object type is an Interface with exactly these ordered features. An object type export without this member is not an Interface. |
| `port-direction` | inline `direction` on an endpoint record (FCD FR-114) | exactly `in`, `out` or `inout` | The flow direction of the endpoint used as a Port. It is never inferred from a role or from relationship direction (FCD FR-128). |
| `typed-multiplicity` | inline member `multiplicity`, one JSON object, on: a field member export; a component record; an endpoint record; each of a relationship record's two end objects; each `parameters` entry of an operation `signature`; and a non-null `result` of an operation `signature` | exactly the four members `lower`, a JSON integer in `0..=u64::MAX`; `upper`, such an integer or exactly the string `unbounded`; `ordered`, a JSON boolean; and `unique`, a JSON boolean | Inclusive bounds. `unbounded` is greater than every finite value. `ordered` and `unique` are members of that same `multiplicity` object, never separate records or members of the carrying record. `lower > upper` is `refused { code: invalid_model_binding, cause: unpreserved-model-meaning }`. |
| `part-signature` | two inline members of a component record: `typeIdentity`, a type export identity string, and that record's `multiplicity` object (the `typed-multiplicity` shape) | the declared part type export and its typed multiplicity | The component is a Part of its `owningTypeIdentity` composite with that declared type. It is not a separate record. |
| `subtype-closure` | bundle header member `generalizationClosure` | exactly `closed` or `open` | `closed` states that the bundle's generalization records are every generalization among the selected model's object types. |
| `subsetting-closure` | bundle header member `subsettingClosure` | exactly `closed` or `open` | `closed` states that the bundle's subsetting records are every subsetting among the selected model's features. |
| `redefinition-closure` | bundle header member `redefinitionClosure` | exactly `closed` or `open` | `closed` states that the bundle's redefinition records are every redefinition among the selected model's features. |

Three requirements go beyond the seven items listed in
agent-ix/filament-core-data#143, and the producer request must carry them
before a `1.3.0` bundle can bind: the `part-signature` inline members of a
component record; the `ordered` and `unique` members of every `multiplicity`
object; and the `subsettingClosure` and `redefinitionClosure` header members.

A `1.3.0` bundle whose required record or inline member is absent returns
`refused { code: invalid_model_binding, cause: unsupplied-producer-record }`
with payload `{capability, required: "1.3.0", supplied: "1.3.0", item}`, where
`item` is the producer key of the record that lacks it (the FR-117 bundle key
for an absent header member). Absence of a separate generalization, subsetting
or redefinition record is not detectable. It means no such generalization only
under `generalizationClosure: closed`, no such subsetting only under
`subsettingClosure: closed`, and no such redefinition only under
`redefinitionClosure: closed`; each header member speaks only for its own
record kind. Under `generalizationClosure: open`, every query that needs the
subtype set is incomplete, as FR-151 and FR-153 state. Under
`subsettingClosure: open`, rule `quire.model.normalize.subset/v1` refuses
`refused { code: invalid_model_binding, cause: unclosed-subsettings }`, and
under `redefinitionClosure: open`, rule `quire.model.normalize.redefine/v1`
refuses `refused { code: invalid_model_binding, cause: unclosed-redefinitions }`;
each retains the ModelSelection and the header value, is decided from the
header before phase 4's first charge, and leaves no effective view.

`part-signature` is inline on components, so a component lacking it is decoded
and reported by rule `quire.model.systems.kind-mapping/v1`, not by decoding.

### Producer interface 1.2.0 input

A package selecting this root and bound to a producer interface `1.2.0` bundle
receives no effective view and no derived substitute: a `1.2.0` bundle is never
read as having no generalizations, no redefinitions, no signatures or
single-valued multiplicities. After decoding, the checker returns one
`refused { code: invalid_model_binding, cause: unsupplied-producer-record }`
with payload `{capability, required: "1.3.0", supplied: "1.2.0", item}` for
each row below and each item, ordered by capability in table order, then by the
record kinds in the order the row lists them, then by item producer key. Every such refusal is reported, and no later phase runs.

| Capability | One refusal per item |
| --- | --- |
| `subtype-closure` | the bundle (its FCD FR-117 key) |
| `subsetting-closure` | the bundle (its FCD FR-117 key) |
| `redefinition-closure` | the bundle (its FCD FR-117 key) |
| `generalization` | object type export |
| `redefinition` | field or operation member export of an object type |
| `subsetting` | field member export of an object type |
| `operation-signature` | operation member export |
| `interface-signature` | object type export |
| `port-direction` | endpoint record |
| `typed-multiplicity` | field member export, component record, endpoint record, then relationship record |
| `part-signature` | component record |

A document declaring any other `interfaceVersion` is `refused { code:
unknown_wire, cause: unsupported-wire }` before any charge. Packages that do not
select this root keep their FS03 `quire.state.*` meaning over `1.2.0` bundles.

## Model selection and correspondence bundle

A ModelSelection key (FR-321) is `(authority, export, contract_version)`:
`authority` is the correspondence `ProducerObject.authority`; `export` is the
admitted FCD FR-117 static bundle key `(identity, revision, digest)`; and
`contract_version` is `(interfaceVersion, wireSchema)` (FCD FR-126). A
correspondence bundle is exactly one admitted FR-117 static bundle joined to one
such ModelSelection key. Two exports whose ModelSelection keys differ belong to
different correspondence bundles.

## Identity domains

An original declaration identity is the producer key above. QSL derives the
following identities in its own domains. Their exact preimage shapes are
[`model-effective-declaration.schema.json`](../../checked-package-v2/model-effective-declaration.schema.json)
and their known vectors are
[`model-effective-declaration-vectors.json`](../../checked-package-v2/model-effective-declaration-vectors.json).
Each identity is SHA-256 over the RFC 8785 JCS bytes of its schema-valid
preimage, written as `{domain, digest}` with a lowercase 64-hex digest. Each
domain string differs from every producer digest domain, from
`quire.checked-semantic-node/v1` and from every other QSL domain, so no QSL
identity can equal a producer identity or an I04 node key, even with equal
digest bytes.

| Domain | Preimage | Identifies |
| --- | --- | --- |
| `quire.model.effective-declaration/v1` | `{version, owner_effective_type, original, derivation}` | One effective declaration: an effective type (`owner_effective_type` is `null`) or an effective member of an effective type. |
| `quire.model.effective-view/v1` | `{version, model_selection, rules, declarations}` | One complete effective view of one ModelSelection under one rule manifest. |
| `quire.model.object-universe/v1` | `{version, model_selection, root_types}` | The model-level object universe export shared by a connected generalization component. |

An effective declaration identity binds only producer keys and derivation
facts; it does not bind the ModelSelection. Equal original records under two
ModelSelections therefore yield equal effective declaration identities (the
vectors `n01-type-A` and `n02-type-A`), while their effective views and object
universes differ. Every use that must separate selections pairs the effective
declaration identity with the ModelSelection key or with an object universe
identity, which binds the selection.

The effective member key is `(owner effective type identity, original
declaration producer key)`. Paths that reach the same original declaration
collapse to one effective member that retains a derivation fact for every path.
Two different original declarations never collapse, whatever their names or
shapes.

`derivation` is the ordered array of the declaration's own facts. Each fact is
`{ordinal, rule, inputs}`: `rule` is `{identity, revision}` from the manifest's
`normalization_rules`; `inputs` is the ordered array of producer keys the fact
consumed (generalization, subsetting or redefinition record keys along a path,
from the specific end, followed by the original declaration key); and
`ordinal` is the fact's zero-based decimal-string position in the array. Facts
are ordered by phase, then by `inputs` compared element by element, each
producer key compared by `authority`, `identity`, `revision.namespace`,
`revision.value`, `digest.domain` and `digest.sha256` as UTF-8 bytes, a proper
prefix first.

The effective view's `declarations` is the array of `{effective_id, preimage}`
for every effective declaration, ascending by the `effective_id` digest hex.
Replaying normalization from the same original records and rule manifest
reproduces the effective view byte for byte; replay equality is equality of
the `quire.model.effective-view/v1` identity. An I04 checked package carries it
as a `correspondence` node with semantic form `model_correspondence`; this
amendment adds no member to the V2 wire schema.

### Object universe

The object universe of an object type `T` is the connected component of `T` in
the admitted generalization graph restricted to object types. Its identity is
`quire.model.object-universe/v1` over the ModelSelection key and the ascending
array of effective type identities in that component for which no
generalization record names the type as its specific type. A population is a membership view inside a universe,
never a universe. FCD `modelIdentity` names the ModelSelection; FCD
`populationIdentity` names the population binding and never contributes to a
universe identity. Every type that conforms to `T` has `T`'s universe.

### Reference identity key

A `Reference<T>` value's FR-204 triple is `(universe, type, object)`:

- `universe` is the object universe identity of the referenced object's type;
- `type` is the effective type identity of the object's most-specific type, the
  population member's `typeIdentity` (FCD FR-121) mapped through the effective
  view, never the static type `T`;
- `object` is the exact UTF-8 bytes of the member's `objectIdentity`, with no
  parsing, trimming or normalization (FR-035).

The canonical key compares `universe`, then `type`, then `object`, each by its
FR-204 key bytes, as FR-144 states: the key bytes of an identity component are
its UTF-8 domain string immediately followed by its 32 digest bytes, compared
as one byte string, and the key bytes of `object` are its exact UTF-8 bytes.
Every comparison is unsigned bytewise lexicographic, with a proper prefix
first. The vectors file publishes key-order
vectors. `Reference<T>` is a static type only: an upcast never changes the
triple.

## Normalization rules

Each manifest `normalization_rules` entry names a rule identity, revision,
`stage` (`normalization`, `conformance`, `dispatch`, `systems` or
`environment`), `phase` (an integer for the `normalization` stage, otherwise
`null`) and the artifact that defines it. Normalization applies the manifest's `normalization_rules` in phase order. No
rule declares an override relation. Every refusal lists each contributing
producer key and derivation path in effective-member order.

| Phase | Rule | Derivation |
| --- | --- | --- |
| 1 | `quire.model.normalize.decode/v1` | Decode each record under the admitted `interfaceVersion`. No derivation fact. |
| 2 | `quire.model.normalize.qualify/v1` | One fact per original declaration and owner, with inputs `[original]`: an object type export (owner `null`); a field or operation member export, owned by its owning type; a component record, owned by its `owningTypeIdentity`; an endpoint record, owned by the owning type of its `owningComponentIdentity` component; and a relationship record, owned by each distinct end-owner type of its two ends, source end first. |
| 3 | `quire.model.normalize.inherit/v1` | For each type `T` and each generalization path `T -> ... -> A`: one fact on `T` with inputs `[path records..., A]`; then, for each member `m` whose original owner is `A`, one fact on the effective member `(T, m)` with inputs `[path records..., m]`. |
| 4 | `quire.model.normalize.subset/v1` | For each subsetting record reaching `T` (owning type `T` or an ancestor of `T`): one fact on `(T, subsetting feature)` with inputs `[path records..., record, subsetted feature]`. |
| 4 | `quire.model.normalize.redefine/v1` | For each redefinition record reaching `T`: one fact on `(T, redefining feature)` and one on `(T, redefined feature)`, each with inputs `[path records..., record, redefined feature]`. The redefined member stays in the view and is hidden from name resolution, lookup and dispatch candidacy. When several non-conflicting records redefine one member reaching `T`, only the redefining feature of the most derived redefining owner is exposed; every other redefining feature of that member keeps its facts and is hidden in `T` the same way. |
| 5 | `quire.model.normalize.canonicalize/v1` | Compute every effective identity and the effective view. No derivation fact; ordering never changes a producer identity or resolves ambiguity. |

The *element owner* of a producer element is: the type itself for an object
type export; the owning type for a field or operation member export; the
`owningTypeIdentity` for a component; and, for an endpoint, the element owner
of its `owningComponentIdentity` component. The *end-owner type* of a
relationship end is the element owner of the element its `typeIdentity` names.
A relationship end `e` is an end of effective type `T` exactly when `T` has the
effective member for the relationship derived from the end-owner type of the
opposite end; `r.name` over `Reference<T>` navigates to `e` when `name` is `e`'s
role. The effective declaration of a Connection or Allocation, as returned by a
binder request, is the effective member owned by the end-owner type of its
source end.

A phase 3 extension that would revisit a type already on the current path
closes a cycle. Each cycle is `refused { code: invalid_model_binding, cause:
specialization-cycle }` exactly once, keyed by the set of its generalization
records: it is reported at the first extension, in `normalize.cycle-check`
charge order, that closes it, and a later extension closing a cycle with the
same record set is charged but not reported again. The refusal lists the
cycle's records in cycle order (path order from the revisited type), rotated
to start at the least record in producer key order. A redefinition whose target is
not a member inherited by its owning type, or a redefining feature named by two
redefinition records with different targets, is `refused { code:
invalid_model_binding, cause: redefinition-target }` listing the redefinition
records. Two phase 4 redefinitions of one member reaching `T` conflict unless
one redefining owner is a proper descendant of every other redefining owner; a
conflict is `refused { code: invalid_model_binding, cause: derivation-conflict }`
with both derivation paths, and no effective member is exposed for either.

## Systems-model kind mapping

Rule `quire.model.systems.kind-mapping/v1` maps producer records to exactly one
systems kind. Kinds are disjoint.

| Kind | Producer record | Owner and type |
| --- | --- | --- |
| Part | FCD FR-114 component record with `part-signature` | owning composite = the effective type of `owningTypeIdentity`; declared type = `typeIdentity`; typed multiplicity |
| Port | FCD FR-114 endpoint record with `port-direction`, whose `owningComponentIdentity` is a Part | owning Part; interface type = the endpoint `typeIdentity`, which must be an Interface; endpoint typed multiplicity |
| Interface | `object` type export with `interface-signature` | ordered feature signatures |
| Connection | FCD FR-115 relationship record whose two ends both name endpoint records that are Ports and whose `semantics.category` is not exactly the bytes `allocation` | effective member owned by the source end-owner type; source and target Port identities |
| Allocation | FCD FR-115 relationship record whose `semantics.category` is exactly the bytes `allocation` | effective member owned by the source end-owner type; source element = the identity named by the source end's `typeIdentity`, which must be a Part (component), a Port (endpoint) or an operation member export; target element = the identity named by the target end's `typeIdentity`, which must be a Part |

The rule maps component records, then endpoint records, then relationship
records, each group ascending by producer key, so an endpoint's kind follows
its component's and a relationship's follows its ends'. It reports, for each
record, every applicable refusal: a component lacking `part-signature` is
`refused { code: invalid_model_binding, cause: unsupplied-producer-record }`
with capability `part-signature` and maps to no kind; an endpoint with
`port-direction` whose owning component is not a Part is `refused { code:
invalid_model_binding, cause: wrong-export }` with required kind `Part` and
actual kind `none` or the component's kind, and maps to no kind; and a
relationship whose category is not `allocation`, and one of whose ends names an
endpoint that is not a Port, is `wrong-export` with required kind `Port` and
actual kind `none`, once per such end, source end first, and maps to no kind. A
relationship whose ends both name type exports is a navigation relationship
with no systems kind. Rule `quire.model.systems.allocation/v1` then refuses an
Allocation whose target element is not a Part with `wrong-export`, required
kind `Part` and actual kind `none` or the element's kind.

`semantics.category` is producer-defined text (FCD FR-115). The exact byte
match on `allocation` is this consumer's mapping decision, recorded by rule
`quire.model.systems.allocation/v1`, as is reading an allocation end's
`typeIdentity` as the allocated element's producer identity; no other category spelling, case variant
or role name is read as an allocation. Direction is read only from FCD FR-128
relationship `semantics.direction` and from `port-direction`, never from a
role.

## Qualification catalog and package selection

The canonical lock is [`complete-model-lock.json`](complete-model-lock.json).
Its `requires` entry pins the complete-value lock by raw-byte digest. Its
`qualification_catalog` closes every artifact this root selects by role,
identity, revision, digest domain and raw-byte digest. Its `package_selection`
places every catalog role in `always`; this root has no conditional or
alternative profile. A package selection is checked with the same closed
refusal codes and order as the complete-value lock.

Every selected artifact's exact bytes and `quire.definition.bytes/v1` digest are
retained in the checked package. Any rule-byte change requires a successor
manifest revision and changes package identity.

## Selected rules

The links below are readable projections only. Their normative identities and
digests are the entries in `model-complete-rules.json`.

| Rule |
| --- |
| [Model, graph and state architecture](../../../spec/assurance/AD-006-model-graph-state-contracts.md) |
| [Finite graph identity and reachability](../../../spec/functional/foundation/FR-043-evaluate-finite-graph-relations.md) |
| [Records, tuples and object references](../../../spec/functional/type-model/FR-143-evaluate-record-tuple-recursive-values.md) |
| [Collection algebra and reference keys](../../../spec/functional/type-model/FR-144-preserve-complete-collection-algebra.md) |
| [Normalization provenance](../../../spec/functional/type-model/FR-150-preserve-model-normalization-provenance.md) |
| [Conformance, redefinition and dispatch](../../../spec/functional/type-model/FR-151-resolve-conformance-redefinition-dispatch.md) |
| [Systems-model binding and navigation](../../../spec/functional/type-model/FR-152-bind-systems-model-structures.md) |
| [Closed model environments](../../../spec/functional/type-model/FR-153-query-closed-model-environments.md) |
| [Declared object identity](../../../spec/objects/foundation/FR-204-declared-object-identity.md) |
| [Model-selection artifact](../../../spec/objects/interfaces/FR-321-model-selection-artifact.md) |
| [I03 model selection](../../../spec/objects/interfaces/interface-003-model-selection.md) |
