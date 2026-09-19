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

A package that selects this root has dynamic dispatch, inheritance and
redefinition as this definition states. Every other FS03 state-contract rule
applies unchanged.

## Model input

The model is declared in the spec's own artifacts (AD-006). One artifact
declares one domain object, such as `Order`, and its module object type, such
as `entity`, fixes the artifact's shape. quire-rs extracts the declarations.
The FCD extraction-frontend lifts the extracted declarations of one spec
package into one semantic IR document, the **domain package**. It is
serialized with RFC 8785 JCS and fingerprinted with `sha256-jcs`.

A package imports a domain package with a `model` declaration:

```text
model Orders = "acme/orders" version "1" digest "<64 lowercase hex digits>";
```

The import is a ModelSelection (FR-321):
`{identity, version, digest_domain: "sha256-jcs", digest}`. It is part of the
package identity. QSL model intake admits the domain package and builds the
Quire model declarations from its IR nodes. Quire owns the meaning of every
declaration. Clauses name a declaration by qualified name: the model alias,
then the segments of its IR node identity after the package identity, such as
`Orders::Order` for IR node `ix://acme/orders/Order`.

### Intake

Rule `quire.model.normalize.intake/v1` admits one domain package. The package
input is a map from `sha256-jcs` digest to package bytes. Intake looks up the
bytes by the selected digest, recomputes their digest, and then reads the
package's own identity and version. It runs these checks in order. Each failure ends intake with no declaration.

| Check | Refusal |
| --- | --- |
| The selection's digest domain is exactly `sha256-jcs` | `stale_dependency`/`digest-domain-mismatch` with the expected and actual domain |
| The package input supplies bytes under the selected digest | `missing_import`/`missing-selection` with the ModelSelection |
| SHA-256 over the package's JCS bytes equals the selected digest | `stale_dependency`/`byte-digest-mismatch` with the expected and actual digest |
| The package's own identity and version equal the selection | `invalid_model_binding`/`wrong-model-selection` with both |

Intake then reads each IR node and builds one original declaration per node.
Nodes are read ascending by declaration key. A node that is not a valid IR
node of its kind, or a type definition or population node whose artifact id
is not an object id (defined under the declaration identity rule below), is `invalid_model_binding`/`malformed-declaration`. The
refusal retains the IR node identity, the source artifact and the source span
that the node carries. A supertype, field type, subsets, redefines, relationship-end,
frame or population member reference that names no node of the package is
`missing_declaration`/`missing-name` with the same artifact and span. Intake
reports every such refusal in node order, and then no later phase runs.
Within one node, every failing check reports in FR-154's table order, the object
id check first. A reference that names a refused node is not reported again as
`missing_declaration`/`missing-name`.

A source span is `{artifact, start, end}`. `artifact` is the artifact id, such
as `Order`. `start` and `end` are zero-based byte offsets in the artifact
file. The span records where a declaration came from. It appears in
diagnostics and provenance and is never a key member; the artifact id enters a
key only as the type's identity segment below.

A model-declaring artifact is an artifact whose type is an object type of any
spec-objects module (spec-objects-architecture, spec-objects-business, …), and
its artifact id is an object id: ASCII letters, digits and
underscores, starting with a letter (`^[A-Za-z][A-Za-z0-9_]*$`). An object id
is always a `member-name`, so a clause names every declaration by qualified
name, such as `M::sys_pump`. Artifact-type ids such as `FR-100` declare no
model node.

A declaration's identity is its artifact id. Every type definition is declared
by its own artifact: an object type, value type, variant type, population,
interface, part, port, connection or allocation declared by the artifact with
id `Order` in domain package `acme/orders` has IR node identity
`ix://acme/orders/Order`, and a port declared by the artifact with id
`pump_out` has `ix://acme/orders/pump_out`. A member's identity appends its
member name to its owner's identity: field `total` of `Order` is
`ix://acme/orders/Order/total`. Fields, relationships, operations and clauses
are the only members. The owner of a part or port and the ends of a connection
or allocation are references, not members, and never part of an identity. The
artifact title is the declaration's
`displayName`, used for presentation only; it is never part of an identity and
never resolves a qualified name. Retitling an artifact keeps every declaration
key. Changing an artifact's id declares a different type with a different
declaration key, and renaming a member declares a different member.

### Construct meanings

The domain package is a semantic IR 2.0.0 document. Each type definition's
`kind` is `{module, name}` and names one entry of the document's embedded
`constructs` table. Intake binds each type definition by that entry's `meaning`,
a Quire meaning id of [FR-208](../../../spec/objects/foundation/FR-208-quire-meaning-vocabulary.md).
A population node names its construct the same way. The kind name and module
select the construct entry only and never select a meaning. A type definition
or population node whose `kind` names no construct entry, or whose construct
has no `meaning` or a `meaning` outside FR-208, is
`invalid_model_binding`/`malformed-declaration` with its IR node identity,
artifact and span.

### Declaration kinds

| Declaration | IR node | Owner |
| --- | --- | --- |
| Object type | `TypeDefinition` whose construct meaning is `quire.meaning.model.object-type/v1` | none |
| Value type | `TypeDefinition` whose construct meaning is `quire.meaning.model.value-type/v1` | none |
| Variant type | `TypeDefinition` whose construct meaning is `quire.meaning.model.variant-type/v1` | none |
| Record value type, event type, state machine, process, persistence interface, namespace | the `TypeDefinition` whose construct meaning is `quire.meaning.model.record-value-type/v1`, `quire.meaning.model.event-type/v1`, `quire.meaning.model.state-machine/v1`, `quire.meaning.model.process/v1`, `quire.meaning.model.persistence-interface/v1` or `quire.meaning.model.namespace/v1`, valid as FR-208 defines that meaning | none |
| Field | `TypeDefinition.fields[]` entry | its type |
| Relationship | `TypeDefinition.relationships[]` entry | its type |
| Operation | `Operation` | its receiver type |
| Invariant | `Clause` with `language: quire` | its type or operation |
| Population | Population node whose construct meaning is `quire.meaning.model.population/v1` | none |
| Part, Port, Interface, Connection, Allocation | the `TypeDefinition` whose construct meaning is that kind's `quire.meaning.systems.*` id | none; the `owner` and end references place it in the effective view as below |

A clause in any other language is kept with its span. It is never evaluated
and never a proof obligation.

## Model feature rules

Rule `quire.model.features/v1` checks each declaration after intake. Each row
is checked for every declaration of its kind, ascending by declaration key.
Every refusal names the IR node identity, the artifact and the span.

| Feature | Rule | Refusal |
| --- | --- | --- |
| Identity | A type definition's identity, including a population's and a systems construct's, is `ix://<package identity>/<artifact id>`. A member's identity is its owner's identity, `/` and the member name. The artifact title is `displayName`, for presentation only, and the span is the declaration's origin; neither is part of any identity. Two nodes with one identity are not allowed. | `invalid_model_binding`/`conflicting-binding` |
| Supertypes | Every `supertypes[]` entry of an object type names an object type of the package, and every entry of a record value type, event type, state machine or process names a node of its own meaning (FR-208); an entry naming a node of another meaning is refused. The supertype graph is acyclic; phase 3 checks it. A persistence interface or namespace with a non-empty `supertypes` is refused and its entries are not checked further. | `missing_declaration`/`missing-name`; `invalid_model_binding`/`malformed-declaration`; `unsupported_construct`/`declaration-form`; `invalid_model_binding`/`specialization-cycle` in phase 3 |
| Abstract | An abstract type has no direct instances. A population member whose most-specific type is abstract is refused. Dispatch covers only concrete subtypes. | `invalid_runtime_input`/`abstract-instance` |
| Fields | `typeRef` names a native value type, or a type of the package; a `typeRef` naming an event type, state machine, process, persistence interface or namespace (FR-208) is refused. `multiplicity` has `lower <= upper`. | `missing_declaration`/`missing-name`; `invalid_model_binding`/`malformed-declaration`; `invalid_model_binding`/`unpreserved-model-meaning` |
| Presence | `presence` is exactly `required` or `optional`. A `required` field is present in every object. An `optional` field may be absent. Absent and empty stay distinct: lower bound `0` makes an empty collection legal and never makes a field optional. | `invalid_model_binding`/`malformed-declaration`; `invalid_runtime_input`/`missing-member` |
| Subsets | Every `subsets[]` entry names a field of the owning type or of a supertype. | `missing_declaration`/`missing-name` |
| Redefines | `redefines` names a field or operation of the package. Phase 4 checks that the target is inherited by the owning type and that no two members of one type redefine one target. | `missing_declaration`/`missing-name`; `invalid_model_binding`/`redefinition-target` in phase 4 |
| Relationships | Each end names an object type or a process (FR-208) of the package, and a relationship may be owned by an object type or a process, with object-type relationship meaning; an end naming a node of any other business model meaning (FR-208), and a relationship owned by one, is refused. Each end has a role and a multiplicity with `lower <= upper`. `direction` is exactly `source-to-target`, `target-to-source`, `bidirectional` or `undirected`. | `missing_declaration`/`missing-name`; `invalid_model_binding`/`malformed-declaration`; `invalid_model_binding`/`unpreserved-model-meaning` |
| Frames | `Operation.frame` has exactly `modifies`, `creates` and `deletes`. Each `modifies` entry names a field or relationship. Each `creates` or `deletes` entry names an object type or a process (FR-208); an entry naming a node of any other meaning is refused. | `missing_declaration`/`missing-name`; `invalid_model_binding`/`malformed-declaration` |
| Populations | Each member type names an object type or a process (FR-208); a member type naming a node of any other business model meaning (FR-208), such as a namespace, is refused. `extent` is exactly `closed` or `open`. | `missing_declaration`/`missing-name`; `invalid_model_binding`/`malformed-declaration` |
| Business model meanings | Each record value type, event type, state machine, process, persistence interface and namespace node meets its FR-208 rules: required members present, identity fields absent or present as its meaning states, `occurrenceField` resolving to `Timestamp`, transition, step, `persists` and `members` references naming nodes of the stated meaning, guard clauses in `quire`, unique state, transition and step identities, and no unsupported form. A process has an object universe of runs; no node of the other five meanings has one. Invariants, `Pre:` and `Post:` contracts and generalization have the meanings FR-208 states. | `missing_declaration`/`missing-name`; `invalid_model_binding`/`malformed-declaration`; `invalid_model_binding`/`conflicting-binding`; `unsupported_construct`/`declaration-form`; `wrong_snapshot`/`forbidden-pre-read`; `invalid_model_binding`/`derivation-conflict` in phase 3 |
| Systems kinds | Each part, port, interface, connection and allocation node meets the systems-model kind rules below. | as below |

Multiplicity is `{lower, upper, ordered, unique}`. `lower` is an integer in
`0..=u64::MAX`. `upper` is such an integer or `unbounded`, which is greater
than every finite value. `ordered` and `unique` are booleans.

## Identity domains

An original declaration key is `{package, node, digest_domain: "sha256-jcs"}`:
the domain package identity, the IR node identity and the fixed digest domain.
QSL derives the following identities in its own domains. Their exact preimage
shapes are
[`model-effective-declaration.schema.json`](../../checked-package-v2/model-effective-declaration.schema.json)
and their known vectors are
[`model-effective-declaration-vectors.json`](../../checked-package-v2/model-effective-declaration-vectors.json).
Each identity is SHA-256 over the RFC 8785 JCS bytes of its schema-valid
preimage, written as `{domain, digest}` with a lowercase 64-hex digest. Each
domain string differs from `sha256-jcs`, from `quire.checked-semantic-node/v1`
and from every other QSL domain, so no QSL identity can equal a domain package
digest or an I04 node key, even with equal digest bytes.

| Domain | Preimage | Identifies |
| --- | --- | --- |
| `quire.model.effective-declaration/v1` | `{version, owner_effective_type, original, derivation}` | One effective declaration: an effective type (`owner_effective_type` is `null`) or an effective member of an effective type. |
| `quire.model.effective-view/v1` | `{version, model_selection, rules, declarations}` | One complete effective view of one ModelSelection under one rule manifest. |
| `quire.model.object-universe/v1` | `{version, model_selection, root_types}` | The object universe shared by a connected generalization component. |

A declaration key carries no digest. An effective declaration identity binds
declaration keys and derivation facts only. Two versions of one domain package
with equal nodes therefore yield equal effective declaration identities, while
their effective views and object universes differ: the vectors `n01v2-view`
and `n01v2-universe` select F1's nodes as version `2`, carry declarations
byte-equal to `n01-view`'s, and differ from `n01-view` and `n01-universe`.
Equal nodes in two different packages under one identity also share effective
declaration identities, as `n01-type-A` and `n02-type-A` show. Every use that must separate selections pairs the effective
declaration identity with the ModelSelection or with an object universe
identity, which binds the selection.

Declaration keys compare by `package`, then `node`, as UTF-8 bytes, a proper
prefix first.

The effective member key is `(owner effective type identity, original
declaration key)`. Paths that reach the same original declaration collapse to
one effective member that retains a derivation fact for every path. Two
different original declarations never collapse, whatever their names or
shapes.

`derivation` is the ordered array of the declaration's own facts. Each fact is
`{ordinal, rule, inputs}`: `rule` is `{identity, revision}` from the manifest's
`normalization_rules`; `inputs` is the ordered array of declaration keys the
fact consumed; and `ordinal` is the fact's zero-based decimal-string position
in the array. Facts are ordered by phase, then by `inputs` compared element by
element in declaration key order, a proper prefix first.

The effective view's `declarations` is the array of `{effective_id, preimage}`
for every effective declaration, ascending by the `effective_id` digest hex.
Replaying normalization from the same domain package and rule manifest
reproduces the effective view byte for byte; replay equality is equality of
the `quire.model.effective-view/v1` identity. An I04 checked package carries it
as a `correspondence` node with semantic form `model_correspondence`.

### Object universe

The object universe of an object type `T` is the connected component of `T` in
the supertype graph restricted to object types. Its identity is
`quire.model.object-universe/v1` over the ModelSelection and the ascending
array of effective type identities in that component that have no supertype.
A population is a membership view inside a universe, never a universe. Its
declaration key never contributes to a universe identity. Every type that
conforms to `T` has `T`'s universe.

### Reference identity key

A `Reference<T>` value's FR-204 triple is `(universe, type, object)`:

- `universe` is the object universe identity of the referenced object's type;
- `type` is the effective type identity of the object's most-specific type,
  the runtime member's declaration key mapped through the effective view,
  never the static type `T`;
- `object` is the exact UTF-8 bytes of the member's object identity, with no
  parsing, trimming or normalization (FR-035).

The canonical key compares `universe`, then `type`, then `object`, each by its
FR-204 key bytes, as FR-144 states: the key bytes of an identity component are
its UTF-8 domain string immediately followed by its 32 digest bytes, compared
as one byte string, and the key bytes of `object` are its exact UTF-8 bytes.
Every comparison is unsigned bytewise lexicographic, with a proper prefix
first. The vectors file publishes key-order vectors. `Reference<T>` is a static
type only: an upcast never changes the triple.

## Normalization rules

Each manifest `normalization_rules` entry names a rule identity, revision,
`stage` (`normalization`, `conformance`, `dispatch`, `systems` or
`environment`), `phase` (an integer for the `normalization` stage, otherwise
`null`) and the artifact that defines it. Normalization applies the manifest's
`normalization_rules` in phase order. No rule declares an override relation.
Every refusal lists each contributing declaration key and derivation path in
effective-member order.

| Phase | Rule | Derivation |
| --- | --- | --- |
| 1 | `quire.model.normalize.intake/v1` | Admit the domain package and build one original declaration per IR node, then apply `quire.model.features/v1`. No derivation fact. |
| 2 | `quire.model.normalize.qualify/v1` | One fact per original declaration and owner, with inputs `[declaration]`: an object type or an interface (owner `null`); a field, relationship or operation, owned by its type or interface; a part, owned by its owning type; a port, owned by the owning type of its part; and a connection or allocation, owned by the end-owner type of each end, source end first. |
| 3 | `quire.model.normalize.inherit/v1` | For each type `T` and each supertype path `T -> P1 -> ... -> A`: one fact on `T` with inputs `[P1, ..., A]`; then, for each member `m` whose owner is `A`, one fact on the effective member `(T, m)` with inputs `[P1, ..., A, m]`. |
| 4 | `quire.model.normalize.subset/v1` | For each field `s` whose `subsets[]` names `t`, where the owner of `s` is `T` or a supertype of `T`: one fact on `(T, s)` with inputs `[path types..., s, t]`. |
| 4 | `quire.model.normalize.redefine/v1` | For each member `r` whose `redefines` names `t`, where the owner of `r` is `T` or a supertype of `T`: one fact on `(T, r)` and one on `(T, t)`, each with inputs `[path types..., r, t]`. The redefined member stays in the view and is hidden from name resolution, lookup and dispatch candidacy. When several non-conflicting members redefine one member reaching `T`, only the redefining member of the most derived owner is exposed; every other redefining member keeps its facts and is hidden in `T` the same way. |
| 5 | `quire.model.normalize.canonicalize/v1` | Compute every effective identity and the effective view. No derivation fact; ordering never changes a declaration key or resolves ambiguity. |

`path types` is the list of supertypes from the specific type `T` to the owner,
excluding `T`. It is empty when `T` is the owner.

The *element owner* of a declaration is: the type itself for an object type or
an interface; the owning type for a field, relationship or operation; the type
a part's `owner` names; and, for a port, the element owner of the part its
`owner` names. The *end-owner type* of
a connection or allocation end is the element owner of the declaration that
the end names. A relationship end `e` is an end of effective type `T` exactly
when `T` has the effective member for the relationship; `r.name` over
`Reference<T>` navigates to `e` when `name` is `e`'s role. The effective
declaration of a Connection or Allocation, as returned by a binder request, is
the effective member owned by the end-owner type of its source end. Every
systems effective member is keyed by `(owner effective type, construct
declaration key)`; a clause names the construct by its artifact id, never
through its owner.

A phase 3 extension that would revisit a type already on the current path
closes a cycle. Each cycle is `refused { code: invalid_model_binding, cause:
specialization-cycle }` exactly once, keyed by the set of its supertype edges:
it is reported at the first extension, in `normalize.cycle-check` charge order,
that closes it, and a later extension closing a cycle with the same edge set is
charged but not reported again. The refusal lists the cycle's type keys in
cycle order (path order from the revisited type), rotated to start at the least
key. A redefinition whose target is not a member inherited by its owning type,
or two members of one type that redefine one target, is
`refused { code: invalid_model_binding, cause: redefinition-target }` listing
every such redefining member. Two phase 4 redefinitions of one member reaching `T`
conflict unless one redefining owner is a proper descendant of every other
redefining owner; a conflict is `refused { code: invalid_model_binding, cause:
derivation-conflict }` with both derivation paths, and no effective member is
exposed for either.

Phase 3 also derives the business model data of FR-208. A state machine's
effective states and transitions are the union of its own with those of every
ancestor; a state name, or a transition's `from`, `to` and `trigger`, declared
by two distinct owners reaching one type is `refused { code:
invalid_model_binding, cause: derivation-conflict }` with both derivation
paths, and a transition whose `from`, `to`, `trigger` or `guard` names no
effective state, operation or clause of its machine is `refused { code:
invalid_model_binding, cause: malformed-declaration }`. An event type whose
ancestor's occurrence field is neither its own occurrence field nor a field
that its occurrence field redefines is `refused { code: invalid_model_binding,
cause: derivation-conflict }` with both paths. States, transitions, occurrence
fields and steps add no `normalize.inherit` member fact, because they are not
members; a process's steps are not inherited.

## Systems-model kind mapping

Rule `quire.model.systems.kind-mapping/v1` maps each IR node whose construct
meaning is a systems meaning to exactly one systems kind by that meaning id,
never by its kind name or module. A node whose construct meaning is not a
systems meaning has no systems kind, even when its kind name is `port`. Kinds
are disjoint.

| Kind | Construct meaning | Owner and type |
| --- | --- | --- |
| Part | `quire.meaning.systems.part/v1` | owning composite = the effective owning type; declared type; typed multiplicity |
| Port | `quire.meaning.systems.port/v1`, whose owner is a Part | owning Part; direction `in`, `out` or `inout`; interface type, which must be an Interface; typed multiplicity |
| Interface | `quire.meaning.systems.interface/v1` | ordered feature signatures, each a field or an operation |
| Connection | `quire.meaning.systems.connection/v1`, whose two ends both name Ports | effective member owned by the source end-owner type; source and target Port keys; declared `direction` |
| Allocation | `quire.meaning.systems.allocation/v1` | effective member owned by the source end-owner type; source element, which must be a Part, a Port or an operation; target element, which must be a Part |

The rule maps interfaces, then parts, then ports, then connections, then
allocations, each group ascending by declaration key, so a port's kind follows
its part's and its interface's, and a connection's follows its ends'. It reports, for each node, every applicable
refusal: a port whose owner is not a Part is `refused { code:
invalid_model_binding, cause: wrong-export }` with required kind `Part` and
actual kind `none` or the owner's kind, and maps to no kind; a port whose
interface type is not an Interface is `wrong-export` with required kind
`Interface`; and a connection one of whose ends names a node that is not a Port
is `wrong-export` with required kind `Port` and actual kind `none`, once per
such end, source end first, and maps to no kind. A relationship between object
types is a navigation relationship with no systems kind. Rule
`quire.model.systems.allocation/v1` then refuses an Allocation whose target
element is not a Part with `wrong-export`, required kind `Part` and actual
kind `none` or the element's kind.

Direction is read only from a connection's or relationship's declared
`direction` and from a port's direction, never from a role.

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
| [Domain package model intake](../../../spec/functional/type-model/FR-154-admit-domain-package-model.md) |
| [Declared object identity](../../../spec/objects/foundation/FR-204-declared-object-identity.md) |
| [Quire meaning id vocabulary](../../../spec/objects/foundation/FR-208-quire-meaning-vocabulary.md) |
| [Model-selection artifact](../../../spec/objects/interfaces/FR-321-model-selection-artifact.md) |
| [I03 model selection](../../../spec/objects/interfaces/interface_003-model-selection.md) |
