---
id: ADR-015
title: "Compile and replay against dependencies"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-307
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-323
    type: depends_on
---
# ADR-015: Compile and replay against dependencies

## Status

Accepted (2026-09-25). Amends ADR-011 §2.1 (E3), §4 (dependency binding)
and §5 (spine `compile`), and ADR-013 O-02, O-04, O-26 and QC-27, as each
decision below states. QSpec states the wire and identity parts in FR-307,
FR-322 and FR-323 (agent-ix/quire-specification); this
record states QSL's side.

## Context

ADR-011 §4 says E4 and the layer-6 `replay` facade check every dependency
of a package against the `package_id` its import records, and that an
ordinary compile gets each dependency's source from "the S4 source
resolution". The E4 closure exists (`CheckedPackage::link_with`,
FR-087-AC-14). Five questions were still open, so E3 refused every
`import`:

1. How spine `compile`, the CLI and `replay` are given dependency sources,
   and the four FR-001 labels that name them. The labels fix each
   dependency's `SourceOwner` and so its `package_id`.
2. How an import names its library. An import names the library by
   identity alone; the library's content is bound by the `package_id` the
   compile recomputes, and ADR-013 O-02 forbids a `PackageId` built from
   caller hex.
3. What a library identity is. `library::LibraryName` accepts identifier
   segments only; FR-322 admits any non-empty string, and QSpec's vectors
   use `test/geometry`.
4. How `replay` knows which of its sources is the proved package and which
   is which dependency. FR-098 required exactly one source. With a stale
   dependency, no source recompiles to the recorded id, so nothing says
   which source was meant to be that dependency.
5. How E3 types a use of an imported name, such as `l::f(x)`, when an
   `ImportView` carries names only; and how a reference into a dependency
   enters a node's identity (ADR-013 QC-27, FR-087-AC-13, TC-379).

## Decision

### D-1 The dependency input and the S4 source resolution

A compile takes a **dependency input**: a set of **supplied libraries**
(QSpec FR-307), at most one per library identity. A supplied library is
`{identity, source}`, where `identity` is a `LibraryName` (D-3) and
`source` a source unit: its
FR-001 `SourceIdentity` (two labels), a display path and its bytes. It
carries no `package_id`: a library's `package_id` is the one its own
compile yields (ADR-013 O-02). Building the input refuses, naming both
offending libraries or the empty field:

- a second library supplied under an identity already supplied, and a
  library whose source has the authority and identity of the unit's or of
  another library's source (one owner per compile, ADR-013 O-04), with
  `invalid_package`/`conflicting-definition`;
- an empty identity with `invalid_identifier`
  (`HostCause::SelectionIdentity`).

Spine `compile` takes the dependency input beside FR-056's package input.
Each supplier names its libraries, as FR-001 has every caller name its
sources:

- a library caller builds the dependency input itself;
- the CLI's `1-draft` native-compile/1 request carries it in a `libraries`
  member, one `{identity, source}` object per library, where
  `source` is the same source selection a model or the program uses
  (file, `sha256:` source digest and the two labels) (FR-027);
- `replay` builds it from the request's package reference (D-4).

The **S4 source resolution** is the spine step between S2 and E3 that turns
the unit's imports into what E3 and E4 admit. It runs beside I1 and its own
refusals report stage `intake`. It visits each `import` of the unit, in
source order, and of each library it compiles, depth first, and applies
these steps to each import in this order:

1. **Cycle.** When the import's identity names a library whose compile is
   in progress, the compile refuses `invalid_package`/`definition-cycle`,
   naming the identity path. This check runs before any other step.
2. **Reuse.** An import of an identity an earlier import in the closure
   named reuses that import's library once its compile has completed, and
   skips steps 3 to 5. One library is supplied per identity, so every
   import of an identity selects the same library.
3. **Selection.** The supplied library of the import's identity is
   selected. None refuses `missing_import`/`missing-selection` at the
   import's identity string.
4. **Compile.** The library's source compiles through S1 to S4 by this same
   resolution, against the same dependency input and package input and
   under the same stage limits. Each library compile is charged the full S1
   to S4 limits as its own unit, and the number of library compiles is at
   most the number of supplied libraries.
5. **View.** The library's `package_id` is recomputed by emitting its
   checked package and is the library's selection in the lock. The emitted
   v2 bytes are read through the I2 reader, with a pinned request holding
   that one selection (identity, version, recomputed `package_id`), into a
   `VerifiedPackage` and then its `ImportView` (ADR-011 §4 verified
   binding).

Two kinds of refusal are **closure-level** and are reported unwrapped by
the top-level compile, wherever in the closure they are found:

- the dependency-input refusals, with no source region, naming both
  libraries or the empty field;
- the cycle refusal (step 1), located at the identity string of the import
  that closes the cycle, in the source that declares it.

Every other refusal raised while resolving or compiling a library is
wrapped as `CompileRefusal::Dependency { path, refusal }`: `path` is the
`LibraryName` path from the unit to that library, and `refusal` the
library's own refusal, reporting its own stage and located in the
library's source. A refusal at one of the unit's own imports (step 3) is the unit's own and is not wrapped.

E3 receives, per import, the `ImportView` and the library's `CheckedGraph`
(D-5). E4 receives, per import, the library's `CheckedPackage` and links
through `CheckedPackage::link_with`, which records the closure and applies
FR-307's diamond rule. A supplied library that no import reaches is not
compiled and is not recorded.

With the dependency input in place, E3 refuses an `import` only by the
causes above; it no longer refuses every import.

### D-2 An import names its library by identity alone

The complete-V1 import is `import "L" [as a];`. A `version` or `digest`
token after the identity string is a syntax error at S1. The library is
selected by identity, and its content is bound by the `package_id` the
compile recomputes from the library's checked package (ADR-013 O-02,
`PackageId::of_preimage`): that recomputed `PackageId` is the lock's
selection, and every `PackageId` in a selection, a closure or a
`PackageNodeKey` is a recomputed one. `ImportDeclaration` and E4's `Import`
hold the identity and no recorded digest.

### D-3 A library identity is a non-empty string

A library identity is the FR-322 `DependencySelection.identity`: a
non-empty string with no segment structure, equal to another only when
their UTF-8 bytes are equal and ordered by UTF-8 bytes (QSpec FR-307,
FR-322). `library::LibraryName` wraps that string. Its constructor refuses
only the empty string, and it has no segment accessor. `verify_binding`,
`PinnedRequest`, `ImportDeclaration`, the E4 closure and the dependency
input key libraries by it. The S1 parser's identity bound (at most 512
bytes) is a source limit, not part of the identity.

### D-4 Replay pairs each dependency's sources with its identity

The in-process replay request's package reference is
`{package_id, sources, dependencies}`, QSpec FR-323's reference without
the wire's `contract_version`, which the in-process request does not
carry (ADR-013 O-26). `sources` is the
proved package's own lock `sources`. `dependencies` holds one entry per
entry of the proved package's `dependency_selections`, in order, each
`{identity, version, package_id, sources}`, where `sources` is that
dependency's own lock `sources`. CG copies them from the proved package
and the dependency packages the proving run admitted, and invents none
(ADR-013 C-12).

`replay` refuses with the first of these rules, each rule applied over all
entries, in entry order, before the next rule, except that rules 6 and 7 apply
before rule 5. It uses QSpec FR-323's codes
for the rules FR-323 states; the one-source rule and the dependency-input
rule (rules 2 and 3) are QSL's own preconditions on its request, and both
run before the recompile:

1. The entries are in strictly ascending UTF-8 byte order of `identity`, a
   repeated identity included, else `ReplayRefusal::DependencySelections`
   (`invalid_package`/`invalid-value` at `/package/dependencies`). This
   runs before any dependency input is built or source compiled.
2. The proved package's `sources`, and each entry's `sources`, name exactly
   one `quire.source.bytes/v1` source, else the existing `NotASource` or
   `SourceCount`.
3. `replay` builds the dependency input from the entries: the identity from
   the entry, the two labels from its source reference, the
   reference's identity as the path, and the bytes from the byte
   provision, as FR-001 states for the proved source. The FR-071 reader
   bound bounds the number of entries. A dependency-input refusal, such as
   two entries whose sources share one authority and identity, refuses
   `ReplayRefusal::DependencyInput` carrying it
   (`invalid_package`/`conflicting-definition`).
4. `replay` recompiles the proved source through spine `compile` against
   that dependency input (D-1), and carries its refusal as
   `ReplayRefusal::Recompile`. A removed entry refuses there as
   `missing_import`/`missing-selection` at the import it supplied.
   When that import is in a library, not in the proved unit, the refusal
   arrives wrapped in `CompileRefusal::Dependency` with the library's path
   (D-1).
5. After rules 6 and 7, the recompiled `package_id` equals the request's
   (ADR-013 O-26), else the existing `PackageIdMismatch`. An edited
   dependency source changes the proved package's `package_id` too, so rules
   6 and 7 run first and name the stale dependency.
6. Every entry's identity is held by the recompiled package's
   `dependency_selections`, else `ReplayRefusal::DependencySelections`
   (`invalid_package`/`invalid-value` at `/package/dependencies`).
7. Every entry's `package_id` equals the closure's selection of its
   identity, else `ReplayRefusal::DependencyIdentityMismatch { identity,
   requested, recompiled }` (`stale_dependency`/`byte-digest-mismatch`),
   where `requested` is the entry's `DigestRecord` and `recompiled` the
   `PackageId`, named as `PackageIdMismatch` names its fields.

The entry, not the recompile, says which source was meant to be which
dependency, so a stale dependency is named by its identity even though
nothing recompiles to its recorded id. Every refusal yields no verdict.

### D-5 E3 types an imported name from the dependency's checked graph

E3 resolves `a::Name` through the import's `ImportView`, as FR-087-AC-13
states, to `PackageNodeKey{package, node: WireNodeId}`. It then types the
use from the dependency's `CheckedGraph`, which D-1 compiled from source
and handed to E3 beside the view: it finds the node whose `NodeKey` bytes
equal the `WireNodeId`, by lookup among the keys that graph holds and
never by minting a `NodeKey` (ADR-013 O-04, R-10; ADR-011 FB-13), and reads
the checked declaration there. A call `l::f(x)` checks as a call of that
function: its arguments against the function's checked parameter types, and
its result type is the function's checked result type. An `ImportView`
stays name data only. No type is read from wire bytes (ADR-011 FB-03).

An imported name E3 accepts names a `function` declaration whose parameter
and result types are package-independent: no node in the transitive
closure of the signature type nodes' `dependencies` carries an FR-322
`declaration` or a `ModelOwner`. Such types (builtin, bounded-domain and
anonymous structural types over them) have the same node id in every
package (ADR-013 O-04, OQ-G), so the importing graph holds each under the id
the dependency gives it. The importing graph holds exactly the type nodes
its own nodes reference, such as a call's `result_type` and the types of its
own arguments, and no other node of the imported signature. Such a node
gets the occurrences FR-093 gives any node: a `type` occurrence where the importing unit writes the type, and
otherwise one `generated` occurrence, as for a type node that no region
denotes. A use of an imported name that names any other declaration, or a
function one of whose reachable signature type nodes carries a
`declaration` or a `ModelOwner` (a `Set<R>`, a tuple holding `R`, a
quantity over a declared unit, or `Reference<M::T>` for a model type
`M::T`), refuses `ill_typed`/`operator-ineligible` at the use
(QSpec FR-322). An imported name stands only as a callee.

A call site discharges an imported function's preconditions exactly as
it discharges a local function's. A complete-V1 `function` declares no
precondition clause (QSpec shared grammar `function`), so its parameter
types are its only precondition. E3 checks each argument of `l::f(x)`
against `f`'s checked parameter type with the conversions and the QSpec
FR-146 obligations that a call of a local function with `f`'s signature
gets, and discharges each obligation from the importing unit's own facts.
The callee's own body obligations (FR-146 totality, termination and
partial operations) are discharged from its parameter types when D-1
compiles the library from source. A library that leaves one unproved
refuses inside `CompileRefusal::Dependency` and yields no view, so E3
never re-checks those obligations and never assumes a fact from them at
the call site.

A reference to an imported function lowers to QSpec FR-322's
`dependency_reference` term `{term: "dependency_reference", package, node}`,
with the view's `package_id` and the node's `WireNodeId`, in the node body
and in its node-identity preimage alike. So the call `l::f(x)` is a
`quire.op.function.call` application whose callee argument is that term and
whose `result_type` is `f`'s result type node. The referenced dependency's
`package_id` and node id enter the referencing node's id; the term is never
listed in the node's `dependencies` (FR-322-AC-36, FR-322-AC-37). This
answers ADR-013 QC-27's open question: an application's join does not count
a `dependency_reference`. A node-identity preimage never holds the id of the
node's own package (ADR-013 QC-18). A `dependency_reference` holds a
dependency's `package_id`, fixed before the importing unit compiles, and
D-1's cycle refusal keeps the dependency graph acyclic, so the preimage
stays acyclic.

### Amendments

- ADR-011 §2.1 E3: E3's admitted inputs gain, per import, the
  dependency's `CheckedGraph` for typing imported declarations (D-5). The
  import views stay name data only. E4's dependency packages are compiled
  from source by D-1.
- FR-087-AC-6: E3's lookup of an import view's `WireNodeId` among the keys
  of the dependency's checked graph, in `check`, is a lookup, not a mint
  (D-5).
- FR-087-AC-11: `ImportDeclaration` and `LibraryName` keep their module and
  change shape as D-2 and D-3 state.
- ADR-011 §4, dependency binding, source 1: "the S4 source resolution" is
  D-1.
- ADR-011 §5, spine `compile`: it takes the dependency input (D-1), and E3
  refuses an `import` only by D-1's causes.
- ADR-013 O-02: every `PackageId` is recomputed, never built from an
  import (D-2).
- ADR-013 O-04: a `WireNodeId` from an import view also becomes a `NodeKey`
  by lookup at E3, in the dependency's checked graph compiled from source
  (D-5).
- ADR-013 O-26: the request's package reference carries
  `dependencies` (D-4).
- ADR-013 QC-18: a preimage holds no id of its own package; a
  `dependency_reference` holds a dependency's, acyclic by D-1 (D-5).
- ADR-013 QC-27: answered by D-5.

## Consequences

- A package can import libraries on the spine, and its `package_id` binds
  each dependency's `package_id` twice: through `dependency_selections` and
  through every node that references into it.
- A compile reads each library's source every time; nothing reads a
  dependency's v2 bytes as authority. The v2 bytes a compile emits for a
  library are only read back to build its import view.
- `replay` can name a stale dependency by identity.
- The CLI request gains an optional member; a request with no `libraries`
  compiles as before.
- IR's `quire-contract-model` reader admits the `dependency_reference` term
  in the v2 node body and in the application-node preimage, and resolves it
  against the admitted dependency packages (QSpec FR-322-AC-36). CG's replay
  adapter fills the request's `dependencies` (ADR-013 C-12, QSpec
  FR-323-AC-7).

## References

- Linear QSL-264 (call-site preconditions of an imported function, D-5).
