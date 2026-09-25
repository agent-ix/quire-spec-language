---
id: FR-099
title: "Compile a complete-V1 unit against supplied libraries"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-014
    type: implements
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-307
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-099: Compile a complete-V1 unit against supplied libraries

## Description

When a complete-V1 unit declares an `import`, spine `compile`
(`qsl_replay::spine::compile`) SHALL resolve it against the dependency
input its caller supplies, compile each imported library from its source
through S1 to S4, bind each import to the library's recomputed
`package_id`, type every use of an imported name from the library's checked
graph, and emit the importing package with the library closure and its
cross-package references (ADR-015 D-1, D-2, D-3, D-5; QSpec FR-307, FR-322).

## Inputs

- The unit's source (FR-001 `SourceIdentity`, path, bytes), FR-056's
  package input and the stage limits, as today.
- The dependency input: supplied libraries, each `{identity, version,
  source}`, where `identity` is a non-empty library identity string,
  `version` a version string and `source` a source unit with its four
  FR-001 labels, path and bytes. At most one library per identity.

## Outputs

- A checked package whose lock and identity preimage carry the resolved
  closure as FR-322 `dependency_selections`, and whose nodes reference
  imported declarations by `dependency_reference`; or a refusal with no
  package.

## Behavior

- S1 SHALL read an import's `digest` as exactly 64 lowercase hexadecimal
  characters into a `quire.package.semantic/v2` `DigestRecord`, and SHALL
  refuse any other spelling, a `sha256:` prefix included, with
  `invalid_digest` at the digest string. The recorded digest SHALL never
  become a `PackageId`; each check compares a recomputed `PackageId` with it
  lexically (ADR-013 O-02, O-18).
- A library identity SHALL be any non-empty string, compared and ordered by
  its UTF-8 bytes, with no segment structure (`LibraryName`).
- The dependency input SHALL refuse a second library supplied under an
  identity it already holds with `invalid_package`/`conflicting-definition`.
- For each import, in source order, the S4 source resolution SHALL select the
  library supplied under the import's identity, refusing
  `missing_import`/`missing-selection` at the import when none is, and
  `stale_dependency`/`revision-mismatch` at the import when its version
  differs from the import's.
- The resolution SHALL compile each selected library's source through S1 to
  S4 against the same dependency input, package input and stage limits, once
  per library within one compile. A library reached again while its own
  compile is in progress SHALL refuse `invalid_package`/`definition-cycle`
  naming the identity path. A library's own refusal SHALL refuse the compile,
  named by the library's identity and located in the library's source.
- The resolution SHALL recompute each library's `package_id` by emitting its
  checked package and SHALL refuse `DependencyIdentityMismatch`
  (`stale_dependency`/`byte-digest-mismatch`) at the import when it differs
  from the import's recorded digest, naming the identity, the recorded
  digest and the recomputed `package_id`.
- The resolution SHALL read each library's emitted bytes through the I2
  reader, pinned to that one selection, into its `ImportView` (ADR-011 §4).
- E3 SHALL resolve `a::Name` through the import's `ImportView` (FR-087-AC-13)
  and SHALL type the use from the library's `CheckedGraph`, finding the
  declaration by lookup of the resolved `WireNodeId` among that graph's keys
  and never by minting a `NodeKey`. A call `l::f(x)` SHALL check its
  arguments against `f`'s checked parameter types, and its result type
  SHALL be `f`'s checked result type.
- E4 SHALL link each import's checked library package through
  `CheckedPackage::link_with` (FR-087-AC-14). A supplied library no import
  reaches SHALL be neither compiled nor recorded.
- The lowering SHALL write a reference to an imported declaration as the
  FR-322 `dependency_reference` term `{package, node}` (the view's
  `package_id` and the declaration's `WireNodeId`) in the node body and in
  its node-identity preimage, and SHALL NOT list it in the node's
  `dependencies`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-099-AC-1 | A unit declaring `import "test/geometry" version "1" digest "<d>" as g;`, and referencing nothing of it, compiled with `test/geometry` version `1` supplied from source that compiles to `package_id` `d`, emits a package whose lock and identity preimage each hold the one `DependencySelection` `{test/geometry, 1, d}`, and QSL's I2 read admits it. The same unit with `test/geometry` and an unimported `test/other` supplied emits identical bytes. | Test (TC-446) |
| FR-099-AC-2 | The import with `digest "sha256:<d>"` refuses at S1 with `invalid_digest` at the digest string. The library supplied from a source whose one declaration is changed refuses `DependencyIdentityMismatch` (`stale_dependency`/`byte-digest-mismatch`) at the import, naming `test/geometry`, `d` and the recompiled `package_id`, with no package. | Test (TC-446) |
| FR-099-AC-3 | With no library supplied as `test/geometry` the import refuses `missing_import`/`missing-selection` at the import; with `test/geometry` supplied at version `2` it refuses `stale_dependency`/`revision-mismatch`; with two libraries supplied as `test/geometry` the dependency input refuses `invalid_package`/`conflicting-definition`; with `test/a` and `test/b` supplied, each importing the other, the compile refuses `invalid_package`/`definition-cycle` naming both identities. None yields a package. | Test (TC-446) |
| FR-099-AC-4 | `LibraryName` admits `test/geometry`, `a.b` and `L` and refuses only the empty string; two supplied libraries `test/b` and `test/a` import in either order into a closure listed `test/a` then `test/b`. | Test (TC-446) |
| FR-099-AC-5 | With `test/geometry` exporting `function f using v(x: Int[0, 9]): Boolean pure { x < 5 }`, a unit importing it `as g` checks `function p using v(y: Int[0, 9]): Boolean pure { g::f(y) }`, and `g::f(true)` refuses `ill_typed` at the argument. The emitted `p` body is a `quire.op.function.call` application whose callee is `{term: "dependency_reference", package: <test/geometry's package_id>, node: <f's node id>}`, whose `result_type` is the Boolean type node, and whose node `dependencies` do not list `f`. | Test (TC-446) |
| FR-099-AC-6 | Recompiling that unit with `test/geometry` supplied from a source whose `f` body changes to `x < 6`, and the import's digest updated to the new `package_id`, gives `p`'s call node a different node id and the package a different `package_id`. | Test (TC-446) |

## Dependencies

- ADR-015 D-1 to D-3 and D-5; ADR-011 §2.1 E3 and E4, §4; ADR-013 O-02,
  O-04, O-18.
- [FR-087](FR-087-typestate-and-cross-package-node-key.md): `ImportView`,
  `PackageNodeKey` and E3 name resolution (AC-13), and the E4 link step
  with dependencies (AC-14).
- [FR-027](FR-027-export-compiled-native-package.md): the CLI supplies the
  dependency input in native-compile/1's `libraries`.
- QSpec FR-307 (import binding, supplied libraries, diamond and cycle
  rules), FR-322 (`dependency_selections`, `dependency_reference`,
  FR-322-AC-36).
- IR's v2 reader admits the `dependency_reference` term for the emitted
  package of AC-5 and AC-6 to read back through IR.

## Status

Specified (QSL-255 part b). No test backs it yet.
