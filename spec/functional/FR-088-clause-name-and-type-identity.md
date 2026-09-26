---
id: FR-088
title: "S-3b: frame and clause identity, clause kind, qualified names, and type descriptors"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: traces_to
---
# FR-088: S-3b: frame and clause identity, clause kind, qualified names, and type descriptors

## Description

ADR-013 §7 slice **S-3** has no owning FR (QSL-158). QSL-158's comments
split S-3 into S-3a (FR-087: T-1, T-3, O-15) and **S-3b** (this
requirement): O-08 (frame identity), O-09's clause half (obligation
identity is CG's own conformance work, out of this requirement's scope),
O-10 (clause kind), O-11 (qualified names), O-14 (type descriptors) and
C-26 (checked type node → kernel `ValueType`). S-3b does not gate QSL-6
(#242, ADR-011 M-4), so it can overlap that work; it depends on the S-3a
types this requirement's cited types are checked-graph members of
(`CheckedGraph`, FR-087), but does not depend on `PackageNodeKey` or
`library`.

This requirement's gate is ADR-013 §7's "S-2, QC-10" for S-3 as a whole
(clear: S-2 landed as PR #260 `97ec26e3`; QC-10 landed under STD-2, Done —
see FR-087's Description for the verification detail) plus S-3a's
`CheckedGraph` type (FR-087), since every object this requirement owns is a
member of that checked graph.

Three of this requirement's five owned items carry an ADR-013 instruction to
turn prose into tests, not to restate the prose as the acceptance criterion:

- O-15 (FR-087's, cited here only because the forbidden-construction
  evidence rule also governs this requirement's own types): "`compile_fail`
  tests for every forbidden construction."
- O-10: "Wire-string totality tests in both directions; mutation tests on
  each mapping."
- C-26: "test per type-node form, including a sum."

Acceptance Criteria FR-088-AC-4 (O-10) and FR-088-AC-9/AC-10 (C-26) enumerate
the concrete forms these three instructions cover, rather than repeating the
ADR's summary sentence as if it were itself testable.

## Inputs

- ADR-013 O-08 (frame identity), O-09 (clause half only; the obligation half
  is CG conformance work, ADR-013 §7 lists no ticket for it), O-10 (clause
  kind), O-11 (qualified names), O-14 (type descriptors), C-26 (checked type
  node → kernel `ValueType`), O-04 (checked node identity, pre-existing:
  S-1/S-2), O-06 (member identity, pre-existing: S-2), O-07 (source
  occurrence identity, ADR-013 §3 O-07: occurrence key disambiguates two
  structurally identical clauses; the full occurrence-key-keyed source map
  is S-4's, this requirement uses only the key shape).
- ADR-013 §6 lane-private table: `syntax::ClauseKind` (native-v1, canonical
  owner O-10); native-v1's use of `ir::SymbolName` (canonical owner O-11);
  `checking::types::NativeType` and native-v1's use of `ir::ValueType`
  (canonical owner O-14).
- ADR-013 §8 QC-15 (the kernel `Value`/`ValueType` component types
  this requirement's sum shape depends on, already added to the kernel row
  by S-1/S-2: `VariantId`, an opaque digest newtype with one public
  constructor from a digest, no dependency on `check` or `model`).
- ADR-013 O-14, T-6, C-30, QC-22 and the OQ-B, OQ-D and OQ-F rulings: an
  enum variant's `VariantId` is its QSpec FR-141 enum member node key, an
  enum value carries its `VariantId` and its rank in the FR-141 canonical
  member list, and the rank is its QSpec FR-144 canonical key; a quantity
  type's `UnitId` is a two-domain digest record over the QSpec FR-142 unit
  identities.
- The layer-3 `check` core module ADR-011 §6.1 places the clause-kind enum
  in (`CheckContext`, family checker trait, shared checked types,
  `FamilyOutcome`, `FamilyResult`, `EvalOutcome`).

## Outputs

- One closed checked clause-kind enum, defined once in the layer-3 `check`
  core (O-10). `syntax::ClauseKind` is untouched and gains no new variant
  (it stays lane-private).
- Frame identity (O-08): the checked node id of the `state` node with
  `semantic_form: "frame"`, plus the resolution of its FR-340
  `modifies`/`creates`/`deletes` node-id sets to their `DeclarationKey`s
  through the model correspondence (O-04). This requirement implements the
  identity and the resolution step only; FR-340's frame semantics
  themselves are #210's. Amended by FR-105 (QSL-273): a `modifies` entry
  naming a field is the pair (declaring `object_type` node, field name),
  not a node of its own (ADR-013 O-06, FR-094); it resolves by the object
  type node's `DeclarationKey` and the member name.
- Clause identity (O-09, clause half): the checked node id of the `claim`,
  `temporal`, `protocol` or `state`/`state_clause` node (the last added by
  FR-105, QSL-273), with the O-07 occurrence key available to
  disambiguate two structurally identical clauses at different source
  occurrences. This requirement does not build `KaniObligationIdentity` or
  any CG-side type.
- Qualified names (O-11): the `QualifiedName` type (a non-empty sequence of
  identifiers), and the checker's own name → node id resolution function.
  No name → identity lookup function is exposed for use after the check
  stage (R-06); the one documented exception (the `replay` facade's E9
  lookup of the executor's `QualifiedName`, OQ-5) is S-3a/`replay`'s own
  scope, not built here, though this requirement's `QualifiedName` type is
  what that lookup resolves against.
- Type descriptors (O-14): package types as checked graph nodes
  (`scalar_type`, `composite_type`, `bounded_domain`), identified by node
  id; `ValueTypeRef{Native(NativeValueType), Package(DeclarationKey)}` for
  model field types; the sum-type checked node form, whose variants are
  declared members (O-06) and whose kernel counterpart is the QC-15 sum
  shape (opaque `VariantId`s, no `NodeKey`).
- C-26: the total conversion function, checked type node → kernel
  `ValueType`, owned by the QSL checker, covering every checked type-node
  form including the sum form.

## Behavior

### One closed clause-kind enum; each downstream layer maps to it, never from it

QSL SHALL define exactly one clause-kind enum in the layer-3 `check` core.
Every conversion out of it (QSL → v2 wire strings; v2 → IR `ClauseKind`; IR
→ RT observation kind; IR → CG obligation kind) is each layer's own owned
mapping (ADR-013 O-10), not built by this requirement, but this
requirement's own QSL → v2 direction SHALL be total with no `_` arm: every
variant of the checked clause-kind enum maps to exactly one v2 `node_tag`/
`semantic_form` pair or clause operation identity, and no v2 string this
enum's wire spelling admits maps to zero or more than one variant.

Amended by FR-105 (QSL-273). The enum gains `Invariant`, `Precondition` and
`Postcondition`. All three spell the pair (`state`, `state_clause`) and the
clause operation `quire.op.state.clause` (STD-111); their application's kind
member tells them apart, so for these three the injective spelling is the
triple (pair, operation identity, kind member), and a decoder reads the kind
member after the operation identity. `StateTransition` spells
(`state`, `transition`), its QSpec form, not (`state`, `frame`). The pair
(`state`, `frame`) decodes to no clause kind: a frame node's body is QSpec
FR-340's frame term, which holds no clause application, so a frame is never
read as a clause. Frame identity (O-08) is unchanged by this: a frame is
identified by its own `state`/`frame` node id, whatever operation anchor
references it.

### Two identities resolve through the model correspondence, never by search

Frame identity's `modifies`/`creates`/`deletes` sets (O-08) and any other
lookup from a checked node id to its `DeclarationKey` in this requirement's
scope SHALL resolve only through the model correspondence: the S3 checker
records it on `CheckedGraph`, its own stage output (FR-087 T-1), and the S4
link step carries it, unchanged, into the `CheckedPackage` it builds from
that `CheckedGraph` — so a layer-4 or layer-5 consumer reads the
correspondence from the `CheckedPackage` it holds (ADR-013 O-04's own text:
"Consumers read the correspondence from the `CheckedPackage`"), without
`check` itself ever naming `CheckedPackage` (FR-087-AC-9). No consumer of
this requirement's types SHALL search a collection whose order no
declaration defines to find a matching node (R-05).

### Clause identity is the node id; the occurrence key, not the node id alone, disambiguates

QSL SHALL identify a clause by the checked node id of its `claim`,
`temporal` or `protocol` node. For two occurrences of a structurally
identical clause (same node id, since node ids are content-addressed,
ADR-013 O-04), QSL SHALL distinguish them only by their O-07 occurrence key
(node id, role, ordinal), and SHALL NOT distinguish them by display text,
source order, or collection iteration order (R-05). This requirement
exposes the occurrence key as an input to the identity, not as a field
CG's `KaniObligationIdentity` digest computes independently a second time.

### Names resolve to node ids only inside the checker; no later lookup exists

QSL SHALL resolve a `QualifiedName` to a node id only inside the check
stage. This requirement SHALL NOT expose a name-resolution function whose
signature would let a post-check module (a stage after S3, a backend
repository, or a CLI command) look up a node by name (R-06). QSL SHALL
treat a `QualifiedName` only as a declared component of an identity
preimage (used, for example, in `DeclarationKey`'s or the checked node's own
preimage where a domain package's declared name participates), and SHALL
NOT treat it as an identity in its own right.

### Package types are checked nodes; the kernel shape is a separate, evaluation-only representation

QSL SHALL identify a package type (`scalar_type`, `composite_type`,
`bounded_domain`) by its checked node id (record, tuple and union identity
follow FR-143-AC-6). The kernel `ValueType` (already extended with the sum
shape by S-1/S-2's QC-15 work) is the type's evaluation-time shape, produced
from the checked node by C-26; it is not itself identity-bearing the way
the checked node id is (ADR-013 O-14: "Equality | normalized (node id) ...
Semantic (structural) for kernel `ValueType` during evaluation").

### C-26 is total, and a sum keeps its node id and its variant identities

QSL's checker SHALL convert every checked type-node form to the kernel
`ValueType` with no `_` arm: a source checked type node with no matching
arm is a compile error, not a runtime refusal. For a sum-type checked node,
the conversion SHALL preserve both the node's own id (unchanged, since the
node id is never re-minted by a conversion) and each variant's `VariantId`
(computed from the declaring sum and its member, QC-15); the resulting
kernel sum shape SHALL carry no `NodeKey`, and a sum value produced from it
SHALL carry its `VariantId`, never a variant index in its place (ADR-013
O-14 "Sum types").

### An enum value carries its FR-141 member key and its canonical rank

For an enum, each variant's `VariantId` SHALL be its QSpec FR-141 enum
member node key, whose preimage names the enum's node key and the case
(ADR-013 O-14, C-30). The kernel enum shape SHALL carry the FR-141
canonical member list, and an enum value SHALL carry its `VariantId` and
its rank, the variant's index in that list. Admission SHALL refuse a value
whose rank disagrees with its `VariantId`'s index in the shape. Identity and
equality of an enum value SHALL use its `VariantId` only, and its canonical
key SHALL be its rank, which orders values as the QSpec FR-144 enumeration
key row states. C-30 SHALL admit only an enum member node key as a
`VariantId`.

### A quantity type carries a two-domain UnitId

For a quantity type, C-26 SHALL produce a `UnitId` that is the declared
unit's QSpec FR-142 node key under the `quire.checked-semantic-node/v1`
label, and `semantic_value` SHALL produce the `quire.value.compound-unit/v1`
digest under its own label for a compound unit (ADR-013 T-6, OQ-B). A
declared unit and a compound unit SHALL never be equal. C-30 SHALL admit
only a unit node key as a declared-arm `UnitId`.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-088-CON-1 | This requirement builds the clause identity (the checked node id and its occurrence-key disambiguation) only. It does not build `KaniObligationIdentity`, the CG obligation-kind mapping, or any CG-owned type; those are CG conformance work with no #213 ticket (ADR-013 §7). | Design | Inspection |
| FR-088-CON-2 | This requirement does not implement FR-340's frame semantics (the meaning of `modifies`/`creates`/`deletes`); it implements only the identity and the resolution of those sets' node ids to `DeclarationKey`s. Frame semantics are #210's. | Design | Inspection |
| FR-088-CON-3 | This requirement does not build the occurrence-key-keyed source map itself (that is S-4, ADR-013 O-07/O-12); it uses only the occurrence-key shape (node id, role, ordinal) as an input to clause identity. | Design | Inspection |
| FR-088-CON-4 | This requirement's `QualifiedName` type and the checker's name-resolution function do not implement the `replay` facade's E9 lookup (OQ-5); that lookup is `replay`'s own module (layer 6), built separately, though it resolves against this requirement's `QualifiedName` type. | Design | Inspection |
| FR-088-CON-5 | This requirement does not modify `syntax::ClauseKind`, `checking::types::NativeType`, native-v1's use of `ir::ValueType`, or native-v1's use of `ir::SymbolName`: all four are lane-private (ADR-013 §6) and gain no new variant, field or consumer under R-09. | Design | Test (TC-257) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-088-AC-1 | Exactly one checked clause-kind enum is defined in the layer-3 `check` core; `syntax::ClauseKind` is unchanged and gains no variant. A whole-crate definition scan confirms one canonical enum and zero variants added to the lane-private one. | Test (TC-257) |
| FR-088-AC-2 | Frame identity resolves each `modifies`/`creates`/`deletes` `NodeKey` to its `DeclarationKey` only by reading the model correspondence the S3 checker recorded on `CheckedGraph` and the S4 link step carried, unchanged, into the `CheckedPackage` a layer-4/layer-5 consumer holds (ADR-013 O-04: "Consumers read the correspondence from the `CheckedPackage`"); an adverse test that removes an entry from the correspondence and re-derives the same frame from source (rather than reusing the correspondence) demonstrates the resolution is not re-derived by search. | Test (TC-248) |
| FR-088-AC-3 | Given two occurrences of a structurally identical clause (same checked node id) at two distinct source positions, the two clauses' occurrence keys (node id, role, ordinal) differ, and a consumer keying on (node id, occurrence key) together, not on node id alone, distinguishes them; an adverse test that changes display text, diagnostic text, or the collection order the clauses are iterated in leaves both clauses' identities (node id, occurrence key) unchanged (R-05). | Test (TC-249) |
| FR-088-AC-4 | Every checked clause-kind variant maps to exactly one v2 `node_tag`/`semantic_form`/clause-operation-identity spelling (for `Invariant`, `Precondition` and `Postcondition`, that spelling plus the `quire.op.state.clause` kind member, FR-105), every v2 spelling this enum's wire vocabulary defines maps back to exactly one variant, and (`state`, `frame`) maps back to none: a wire-string totality test iterates every variant forward and every wire string backward and finds no gap and no ambiguity. A mutation test on the forward mapping function (each mutant flips one variant's target wire string, or deletes one match arm) fails the totality test for every mutant, with no mutant allow-listed (ADR-013 §4 preamble; ADR-012 §5.3 evidence convention). | Test (TC-250) |
| FR-088-AC-5 | No function exists whose signature accepts a `QualifiedName` or a bare string and returns a node id or a declaration, callable from outside the check stage; a source and call-graph scan over every crate module confirms every call site of the checker's name-resolution function lies inside the check stage, with the sole documented exception being the `replay` facade's own E9 lookup (a separate module, not built by this requirement) never called from `check` itself. | Test (TC-251) |
| FR-088-AC-6 | `QualifiedName` is used only as a declared component of an identity preimage (for example inside `DeclarationKey`'s or a checked node's own preimage where a name participates); no equality or hashing implementation on any identity type treats a `QualifiedName` as the sole identity-bearing field where a node id or digest is available instead. | Inspection (TC-258) |
| FR-088-AC-7 | A package type's identity is its checked node id: node keys are content keys scoped only by owner (ADR-013 O-04, OQ-G). Two type declarations with the same structure under the same qualified name in two packages of the same owner share one node id. Two with the same structure and qualified name in packages with different source owners get distinct node ids, and two with different qualified names get distinct node ids. A builtin or anonymous type node, such as `Int[0, 9]`, has no owner and shares one id across packages and owners. A reference into another package is distinguished by its `PackageNodeKey` (ADR-013 T-3). Within one package, two source *occurrences* of a reference to the same declared type (ADR-013 O-07's own term: "each source occurrence of a node is keyed by (node id, role, ordinal)" — for example the same type named as the field type of two different fields) resolve to the one node id the single declaration was minted with; this is not a claim about declaring a type twice; declaring a structurally identical type a second time under a colliding name is a name-binding refusal (a duplicate declaration), a different criterion, not a second occurrence of the first node id, and is out of this criterion's scope. This repository's own precedent for a duplicate name/binding — `checking::composed::proofs::CorrespondenceError::DuplicateDeclaration`, `src/checking/composed/proofs/correspondence.rs:96` — refuses rather than admits with a shared id. | Test (TC-259) |
| FR-088-AC-8 | `ValueTypeRef` is exactly the two-member union `{Native(NativeValueType), Package(DeclarationKey)}`; no third variant exists, and no model field type is represented by a bare `NodeKey` or a raw string type name. | Inspection (TC-260) |
| FR-088-AC-9 | C-26's conversion function is total: for every checked type-node form (`scalar_type`, `composite_type`, `bounded_domain`, and the sum form), a test constructs a checked node of that form and confirms the function returns a kernel `ValueType` with no panic, no `_`-arm fallback, and no lossy substitution; a checked type-node form added without a corresponding match arm fails to compile (`clippy::wildcard_enum_match_arm`, following ADR-011 §5's convention for other exhaustive maps). | Test (TC-252) |
| FR-088-AC-10 | For the sum form specifically, the source checked node's own id is not re-minted by C-26's conversion (it is the same content-addressed id the node carried before conversion, read from `CheckedGraph`/`CheckedPackage`, not a value the kernel shape carries); C-26's output kernel `ValueType` carries, per variant, a `VariantId` computed from the declaring sum and that member, and the kernel shape carries no `NodeKey` anywhere in its variant representation. A sum value built from this conversion carries its `VariantId`, and an adverse test confirms no code path reads or stores a bare variant index in place of the `VariantId`. | Test (TC-252) |
| FR-088-AC-11 | For an enum `E` declared with cases `b`, `a`, `c` in that order, each variant's `VariantId` equals the node key over `{version: quire.enum-member-node/v1, declaration_node_id: E's node key, case}`. When `E` is ordered, the variants' ranks are `b` 0, `a` 1, `c` 2, and a set of all three values visits `b`, `a`, `c`. When `E` is unordered, its canonical member list is `a`, `b`, `c`, the ranks are `a` 0, `b` 1, `c` 2, and the set visits `a`, `b`, `c`. A value that pairs `E::a`'s `VariantId` with rank 2 is refused at admission. Renaming the declaration `E` to `F` changes every `VariantId` and changes no rank and no visiting order. C-30 refuses `E`'s own declaration node key as a `VariantId`. | Test (TC-409) |
| FR-088-AC-12 | For declared units `m` and `km` (a scaled unit of `m`) and the compound unit `m^1`, the three `UnitId`s are pairwise unequal. `m`'s `UnitId` carries `m`'s node key under the `quire.checked-semantic-node/v1` label. The compound `UnitId`s of every entry in QSpec's compound-unit vectors equal the vector digests under the `quire.value.compound-unit/v1` label. C-30 refuses a dimension node key as a declared-arm `UnitId`. | Test (TC-411) |

## Dependencies

- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  §1 R-05, R-06, §3 O-04, O-06, O-07, O-08, O-09, O-10, O-11, O-14, §4 C-26,
  §6 (lane-private table), §7 (the S-3 slice row).
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.1 (admitted types), §6.1 (layer-3 `check` core position).
- [FR-087](FR-087-typestate-and-cross-package-node-key.md): S-3a's
  `CheckedGraph` is the type this requirement's checked nodes are members
  of, and is where the S3 checker records the model correspondence this
  requirement's frame-identity resolution (FR-088-AC-2) reads, through the
  `CheckedPackage` the S4 link step carries it into (ADR-013 O-04); this
  requirement does not depend on FR-087's `PackageNodeKey` or `library`
  module.
- [US-005](../usecase/US-005-trust-checked-identity-across-packaging.md).
- Linear QSL-158 (this requirement's owning ticket, the S-3a/S-3b split).
  FR-088-AC-11 and AC-12 are QSL-131's, amended into this requirement in
  place.
- ADR-013 §8 OQ-B, OQ-D and OQ-F; QSpec FR-141, FR-142 and FR-144
  (`ix://agent-ix/quire-specification/FR-141`, `FR-142`, `FR-144`).

## Status

Specified under QSL-158 (ADR-013 §7 S-3, split into S-3a/S-3b by the
2026-09-21 comment on that ticket). S-3b implemented by #300: AC-1, AC-2,
AC-3, AC-4, AC-9 and AC-10 are backed by real `check()`-driven tests
(TC-257, TC-248, TC-249, TC-250, TC-252). AC-7's within-package half
is backed on main with a declared type's node id taken from the
caller-supplied `CompositeDeclaration` key (TC-259 step 4); FR-092-AC-12
(QSL-211) makes that id the FR-092 key `check` mints, and TC-259 step 4
asserts the caller key until QSL-156 A4b adopts it. Its owner-scoped and builtin or anonymous
cross-package cases (TC-259 steps 1 to 3) and recompilation (step 5) are
not implemented. Remaining
work: QSL-156. AC-5 is enforced for its
`QualifiedName` half only (TC-251); the "or a bare string" half is
investigated and documented as a gap, not enforced (see
`tests/it/name_resolution_confinement.rs`'s own module doc). QSL-158 backs AC-6
(TC-258) and AC-8 (TC-260) in part, with `xtask::typestate_scan`. For AC-6:
in the layer crates, no struct or variant has a `QualifiedName` as its only
field, and no map field or map-returning function outside `check` is keyed
by one. A hand-written `PartialEq` or `Hash`, a name beside a filler field,
a local map, and the root crate's lane-private `QualifiedName` types are
not covered. For AC-8: `ValueTypeRef` is defined once, as exactly
`Native(NativeValueType)` and `Package(DeclarationKey)`. No `model` field
whose name contains `type` is a `NodeKey` or a string, and the three
value-type records carry a `ValueTypeRef`. A field type under another name
is not covered, and TC-260 step 4 is not backed. AC-12 (TC-411)
is implemented by QSL-131 V4. AC-11 (TC-409) is not implemented. Remaining
work: QSL-131.
