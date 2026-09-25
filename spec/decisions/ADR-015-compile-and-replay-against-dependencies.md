---
id: ADR-015
title: "Compile and replay against dependencies (QSL-255)"
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
# ADR-015: Compile and replay against dependencies (QSL-255)

## Status

Accepted (2026-09-25). Amends ADR-011 §2.1 (E3), §4 (dependency binding)
and §5 (spine `compile`), and ADR-013 O-02, O-04, O-26 and QC-27, as each
decision below states. QSpec states the wire and identity parts in FR-307,
FR-322 and FR-323 (agent-ix/quire-specification, the QSL-255 change); this
record states QSL's side.

## Context

ADR-011 §4 says E4 and the layer-6 `replay` facade check every dependency
of a package against the `package_id` its import records, and that an
ordinary compile gets each dependency's source from "the S4 source
resolution". The E4 closure exists (`CheckedPackage::link_with`,
FR-087-AC-14). Five questions were still open, so E3 refuses every
`import`:

1. How spine `compile`, the CLI and `replay` are given dependency sources,
   and the four FR-001 labels that name them. The labels fix each
   dependency's `SourceOwner` and so its `package_id`.
2. How an import's digest is spelled. QSL's parser read it as a
   `sha256:`-prefixed `DefinitionRef` digest; FR-307 makes it the bare
   64-hex `package_id`; ADR-013 O-02 forbids a `PackageId` built from
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
`{identity, version, source}`, where `identity` is a `LibraryName` (D-3),
`version` the library's version string, and `source` a source unit: its
FR-001 `SourceIdentity` (four labels), a display path and its bytes. It
carries no `package_id`: a library's `package_id` is the one its own
compile yields (ADR-013 O-02). Building the input refuses, naming both
offending libraries or the empty field:

- a second library supplied under an identity already supplied, and a
  library whose source has the authority and identity of the unit's or of
  another library's source (one owner per compile, ADR-013 O-04), with
  `invalid_package`/`conflicting-definition`;
- an empty identity or version with `invalid_identifier`
  (`HostCause::SelectionIdentity` or `SelectionVersion`).

Spine `compile` takes the dependency input beside FR-056's package input.
Each supplier names its libraries, as FR-001 has every caller name its
sources:

- a library caller builds the dependency input itself;
- the CLI's `1-draft` native-compile/1 request carries it in a `libraries`
  member, one `{identity, version, source}` object per library, where
  `source` is the same source selection a model or the program uses
  (file, `sha256:` source digest and the four labels) (FR-027);
- `replay` builds it from the request's package reference (D-4).

The **S4 source resolution** is the spine step between S2 and E3 that turns
the unit's imports into what E3 and E4 admit. It runs beside I1 and its own
refusals report stage `intake`. For each `import` of the unit, in source
order, and of each library it compiles, depth first:

0. When an earlier import in the closure names the same identity with a
   different version or digest, the compile refuses
   `invalid_package`/`conflicting-definition`, naming both dependency paths
   (QSpec FR-307's diamond rule). An import equal to an earlier one reuses
   its library.
1. The supplied library of the import's identity is selected. None refuses
   `missing_import`/`missing-selection` at the import. A version other than
   the import's refuses `stale_dependency`/`revision-mismatch` at the
   import.
2. The library's source compiles through S1 to S4 by this same resolution,
   against the same dependency input and package input and under the same
   stage limits. Within one compile, each library compiles at most once and
   serves every import of its identity. Each library compile is charged the
   full S1 to S4 limits as its own unit, and the number of library compiles
   is at most the number of supplied libraries. A library reached again
   while its own compile is in progress refuses
   `invalid_package`/`definition-cycle`, naming the identity path. A
   library whose compile refuses makes the compile refuse with
   `CompileRefusal::Dependency { path, refusal }`: `path` is the
   `LibraryName` path from the unit to the library, and `refusal` the
   library's own refusal, reporting its own stage and located in the
   library's source.
3. The library's `package_id` is recomputed by emitting its checked
   package. When it differs from the import's recorded `d` (D-2), the
   compile refuses `DependencyIdentityMismatch`
   (`stale_dependency`/`byte-digest-mismatch`) at the import, naming the
   identity, the recorded digest and the recomputed `package_id`
   (ADR-011 §4).
4. The emitted v2 bytes are read through the I2 reader, with a pinned
   request holding that one selection (identity, version, recomputed
   `package_id`), into a `VerifiedPackage` and then its `ImportView`
   (ADR-011 §4 verified binding).

E3 receives, per import, the `ImportView` and the library's `CheckedGraph`
(D-5). E4 receives, per import, the library's `CheckedPackage` and links
through `CheckedPackage::link_with`, which records the closure and applies
FR-307's diamond rule. A supplied library that no import reaches is not
compiled and is not recorded.

With the dependency input in place, E3 refuses an `import` only by the
causes above; it no longer refuses every import.

### D-2 The import digest is the bare `package_id`, held as a claim

The complete-V1 `import "L" version "v" digest "d"` spells `d` as exactly
64 lowercase hexadecimal characters, the FR-322 `PackageId` `digest`
(QSpec FR-307). S1 reads `d` into a `DigestRecord` in the
`quire.package.semantic/v2` domain (`qsl_foundation::digest`), the same
type the replay request's `package_id` uses. Any other spelling, a
`sha256:` prefix included, refuses at S1 with `invalid-digest`
(`HostCause::SelectionDigest`) at the digest string. The identity and
version keep the parser's existing selection bounds.

The recorded digest is a claim. It is never a `PackageId`: a `PackageId` is
only ever computed from a checked package (ADR-013 O-02,
`PackageId::of_preimage`). Each check compares a recomputed `PackageId` with
the recorded `DigestRecord` lexically (ADR-013 O-18). `ImportDeclaration`
and E4's `Import` hold the recorded `DigestRecord`; every `PackageId` in a
selection, a closure or a `PackageNodeKey` is a recomputed one.

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

The replay request's package reference is QSpec FR-323's
`{package_id, contract_version, sources, dependencies}`. `sources` is the
proved package's own lock `sources`. `dependencies` holds one entry per
entry of the proved package's `dependency_selections`, in order, each
`{identity, version, package_id, sources}`, where `sources` is that
dependency's own lock `sources`. CG copies them from the proved package
and the dependency packages the proving run admitted, and invents none
(ADR-013 C-12).

`replay`:

1. requires the proved package's `sources`, and each entry's `sources`, to
   name exactly one `quire.source.bytes/v1` source, with the existing
   refusals (`NotASource`, `SourceCount`) for a definition document or
   another count;
2. builds the dependency input from the entries: identity and version from
   the entry, the four labels from its source reference, the reference's
   identity as the path, and the bytes from the byte provision, as FR-001
   states for the proved source. The FR-071 reader bound bounds the number
   of entries;
3. recompiles the proved source through spine `compile` against that
   dependency input (D-1), and carries its refusal as
   `ReplayRefusal::Recompile`. A stale dependency's source refuses there as
   `DependencyIdentityMismatch` at the import that records it, naming the
   identity. A removed entry refuses there as `missing_import`/
   `missing-selection` at the import it supplied, and an entry with a
   changed version as `stale_dependency`/`revision-mismatch`;
4. requires the recompiled `package_id` to equal the request's
   (ADR-013 O-26);
5. checks the entries against the recompiled package's
   `dependency_selections`, in this order: an entry whose identity the
   closure does not hold refuses `ReplayRefusal::DependencySelections`
   (`invalid_package`/`invalid-value` at `/package/dependencies`); an
   entry whose `package_id` differs from the closure's selection of its
   identity refuses `ReplayRefusal::DependencyIdentityMismatch { identity,
   recorded, recompiled }` (`stale_dependency`/`byte-digest-mismatch`),
   where `recorded` is the entry's `DigestRecord` and `recompiled` the
   `PackageId`; entries out of the closure's order refuse
   `ReplayRefusal::DependencySelections`.

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
and result types carry no FR-322 `declaration`: builtin, bounded-domain and
anonymous structural types. Their node ids are the same in every package
(ADR-013 O-04, OQ-G), so the importing graph holds each such type under the
id the dependency gives it, with occurrences of its own. A use of an
imported name that names any other declaration, or a function whose
signature names a type carrying a `declaration`, refuses
`ill_typed`/`operator-ineligible` at the use (QSpec FR-322).

A reference to an imported function lowers to QSpec FR-322's
`dependency_reference` term `{term: "dependency_reference", package, node}`,
with the view's `package_id` and the node's `WireNodeId`, in the node body
and in its node-identity preimage alike. So the call `l::f(x)` is a
`quire.op.function.call` application whose callee argument is that term and
whose `result_type` is `f`'s result type node. The referenced dependency's
`package_id` and node id enter the referencing node's id; the term is never
listed in the node's `dependencies` (FR-322-AC-36, FR-322-AC-37). This
answers ADR-013 QC-27's open question: an application's join does not count
a `dependency_reference`.

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
- ADR-013 O-02: an import's recorded digest is a `DigestRecord` claim,
  never a `PackageId` (D-2).
- ADR-013 O-04: a `WireNodeId` from an import view also becomes a `NodeKey`
  by lookup at E3, in the dependency's checked graph compiled from source
  (D-5).
- ADR-013 O-26: the request's package reference carries
  `dependencies` (D-4).
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
