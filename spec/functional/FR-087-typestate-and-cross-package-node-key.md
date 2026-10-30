---
id: FR-087
title: "S-3a: one nominal typestate per stage output and the cross-package node key"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-111
    type: traces_to
---
# FR-087: S-3a: one nominal typestate per stage output and the cross-package node key

## Description

ADR-013 §7 names slice **S-3** ("Typestate, clause and type": O-08, O-09
clause id, O-10, O-11, O-14, O-15, T-1, T-3), and no FR owns it.
S-3 splits along the line the scoping pass identified:
**S-3a** (this requirement) is the typestate half — T-1, T-3, O-15 — because
it is the whole of what #242 (ADR-011 M-4) waits on; **S-3b**
(FR-088) is the clause/name/type half — O-08, O-09 clause id, O-10, O-11,
O-14, C-26. This requirement covers S-3a only.

S-3a's gate (ADR-013 §7: "S-2, QC-10") is clear: S-2 landed as PR #260, and QC-10 landed under STD-2 (Done). QSpec
[FR-322](https://github.com/agent-ix/quire-specification/blob/main/spec/objects/interfaces/FR-322-checked-package-artifact.md)
(`quire-specification`, not this repository) defines `dependency_reference`
as a structural node id `{package, path, ordinal}` whose `package` is the
dependency's `LibraryName`, agreeing with ADR-011 (I2, E3) and ADR-013 T-3.

**Two names already exist in `src/` for names ADR-013 T-1 assigns, and this
requirement gives each a disposition rather than discovering the collision
during implementation.**

1. `protocol_artifact::native::EmittedPackage` (`src/protocol_artifact/native/mod.rs:68,71,104`)
   is the SEAM-3 native-v1 protocol-artifact emitter's struct (`quire.compiled-protocol/1,2,3`,
   ADR-010 B8). ADR-011 §7.3's module-move table assigns all of `protocol_artifact`
   to "SEAM-3 protocol wires," disposed of by the PRs landing the state and
   temporal replacement evaluators (#120, #121, #164, #188, #189; M-6c) and
   the checked-graph S4 emission and IR v2 admission work (#218,
   agent-ix/quire-contract-ir#141; M-6d), with "the last lane PR" deleting
   any remainder. That disposition already exists in ADR-011 and needs no
   new decision here: this requirement's canonical `EmittedPackage` (T-1,
   Outputs) is defined in layer-4 `package`, a different module from
   `protocol_artifact::native`, gains no re-export that would place both
   names in one import scope, and is unrelated in shape and purpose to the
   native-v1 struct. Neither type is renamed by this requirement; they
   coexist under their own module paths until SEAM-3's own PRs delete the
   native-v1 one.
2. `CheckedPackage` exists twice today, and **neither occupies the module
   ADR-013 T-1 names.** `checking::CheckedPackage<'a>` (`src/checking/`,
   used at `src/package/features.rs:9,148` and `src/package/view.rs:13,97,136,431`)
   is lane-private (ADR-013 §6: "Checked typestate | `checking::CheckedPackage<'a>` |
   O-15") and is untouched by this requirement: it retires with its lane
   when that lane converges (ADR-011 §8, Q209-1), and is not a third
   candidate for `CheckedPackage`'s canonical home. The second —
   `check::CheckedPackage` (`qsl-semantics/src/check/mod.rs:127`,
   `PackageDeclarations::check` at `:291` returning it) — landed under
   FR-068 (M-5) in layer-3 `check`, and FR-068's own text
   acknowledges this contradicts ADR-011 §4's worked example, which names
   layer-4 `package` as the S4 type's owner; FR-068 resolved that
   contradiction by following §6.1's layer ordering instead, reasoning that
   §4's placement would open a `check` → `package` reverse edge, and recorded
   "§4 versus §6.1 on `CheckedPackage`'s owning layer" as
   an ADR-011 defect.

   **The collision is not only cross-module: two submodules `package`
   already owns import the lane-private type by its bare name.**
   `src/package/features.rs:9` and `src/package/view.rs:13` both
   `use crate::checking::{CheckedPackage, ...}` today, inside the very
   module tree this requirement makes define its own, canonical
   `CheckedPackage`. This requirement does not rename, alias, or delete
   `checking::CheckedPackage<'a>` — that disposition is its lane's own
   convergence (ADR-011 §8, Q209-1), unaffected by this requirement — and
   it does not add a re-export or glob import anywhere in `package` that
   would place the lane-private and the canonical `CheckedPackage` into one
   import scope; `features.rs` and `view.rs` keep their existing
   `crate::checking::CheckedPackage` import unchanged (see FR-087-AC-10).

   **Ruled (owner ruling on QSL-158, 2026-09-21): ADR-013 T-1 stands
   unamended, and `CheckedPackage` is canonically layer-4 `package`.** The
   reverse-edge objection FR-068 raised does not survive measurement: it
   rests on `PackageDeclarations::check` (`qsl-semantics/src/check/mod.rs:291`) returning
   `CheckedPackage` today only because **`check` has no S3 output type of
   its own** — `CheckedGraph` does not exist anywhere in `src/` (confirmed:
   a whole-repository search for `struct CheckedGraph` returns nothing).
   M-5 had to return the one stage-output type that existed, so it kept
   `CheckedPackage` local rather than opening an edge back into a
   not-yet-built `package`. That is an artifact of `CheckedGraph`'s absence,
   not a property the architecture requires. T-1 already names the shape
   that removes the edge: S3's own output is `CheckedGraph`, and S4's is
   `CheckedPackage`. This requirement is what builds `CheckedGraph`, so it
   is where the reconciliation lands — see Outputs and Behavior, "`check`
   produces `CheckedGraph`; `package` constructs `CheckedPackage`," and
   FR-087-AC-9. This closes the `CheckedPackage`-placement half of that defect;
   the separate §6.2 refusal-row finding is untouched by this
   requirement.

3. **The new top-level `library` module this requirement builds is not
   built from nothing: two modules already exist and ADR-011's own
   module-move table assigns both to it.** `src/value/library.rs` already
   implements FR-307 ("reusable semantic libraries": `LibraryName`,
   `LibraryPackage`, `ImportDeclaration`, `LibraryLock`, and
   `resolve_libraries`, covering qualified imports, transitive-closure
   locking, diamond unification and cycle refusal), re-exported flatly at
   `value::*` (`src/value/mod.rs:154`) and verified today by TC-227
   (`tests/library_resolution.rs`). `src/value/package_identity.rs` is "a
   structural preimage reader with no wire I/O" that `library` already
   imports (`value/library.rs:25`). ADR-011's module table (`:744`) reads:
   "`value::library`, `value::package_identity`, `value::model_query` |
   3 `library`, 3 `library`, 3 `model`" — both `value::library` and
   `value::package_identity` are ADR-011's own relocation target for this
   requirement's `library` module, not a naming coincidence next to it.
   ADR-011 `:187` further lists `value::library` among the modules that
   *today* implement part of the S3 check stage, and FR-060's T12-B
   (`spec/functional/FR-060-check-qsl-api-surface-boundary.md:82`) names
   `value::library` among its *allowed* callers of the kernel `NodeKey`
   constructor, not a real current one: FR-060's own Status section
   (`:169-183`) records T12-B's actual real call sites as
   `value::enumeration`, `value::unit` and `value::model_query`, none of
   them `value::library`. `library`'s own current source does call a
   `NodeKey`-shaped constructor by a different path, though:
   `value/package_identity.rs`'s `node_key` function (`:177-182`) builds a
   `NodeKey` with `NodeKey::from_hex` from a wire-read JSON preimage — a
   call T12-B's rule does not scan for (its patterns are `NodeKey::of(`,
   `NodeKey::from_bytes(` and `node_key_of(`, not `NodeKey::from_hex(`) and
   that ADR-013 O-04/R-10 forbid once this code sits in a layer-3 module
   ordered ahead of `check`, per this requirement's own R-10 boundary
   (Behavior, "R-10: checked typestate is never built from wire bytes";
   FR-087-AC-6, FR-087-AC-11). This requirement's Outputs accordingly
   include relocating `value::library` and `value::package_identity` into
   the new top-level `library` module — not copying them, and not leaving
   `value::library` behind as a second, un-relocated home for the same
   code (FR-087-AC-11).

   **Current shape.** A library is identified by its `LibraryName` alone;
   `LibraryLock::selections()` returns the selected `Vec<LibraryName>`, one
   per imported library identity. No content or version is recorded or
   compared, and no second selection type exists. `resolve_name` does not
   relocate into `library`: name resolution against an imported
   dependency's exports is E3's own resolution over an `ImportView`
   (Behavior, "`ImportView` never resolves a name"; ADR-013 R-06),
   performed by the importing package's own check stage, which reads
   `ImportView` and `LibraryLock` (read-only) from `library`, a permitted
   layer-3 edge under FR-068-AC-6's layer rule (FR-087-AC-9, TC-256).
   `ImportView::of(&LibraryPackage)` validates the package's identity
   preimage structurally (`verify_package`) and holds each export as a
   `(name, NodeKey)` entry, where the key is the structural `node_id`
   (`{package, path, ordinal}`) of the preimage node that declares the
   name. `library` defines no type that resolves a name to a node.
   `resolve_libraries`' refusal set (`LibraryRefusal`) is classified across
   ADR-011 I2's package import graph rules, the preimage admission
   `ImportView::of` performs, and E3 name resolution (FR-087-AC-12,
   TC-282); that classification is this requirement's own scope, not a new
   refusal `resolve_libraries` gains.

## Inputs

- ADR-013 O-15 (typestate), T-1 (typestate encoding and names), T-3 (the
  cross-package node key), R-10 (checked typestate is minted only by the
  checker and the link step; wire bytes never become it).
- ADR-011 §2.1 (admitted types and owners), §4 ("Checking precedes lowering":
  the verified-binding and dependency-binding rules, and the private-
  constructor/public-accessor mechanism), §6.1 (the layer table: layer 3
  `library`, with `ImportView`; layer 4 `package`).
- The current tree: `ResolvedSourcePackage` (complete-V1 lane C2, formerly
  `qsl-semantics/src/complete/package.rs`) is retired; no *top-level* `library`
  module exists yet (`src/lib.rs` has no `mod library;`), but
  `src/value/library.rs` (FR-307, re-exported at `value::*`,
  `src/value/mod.rs:154`) and `src/value/package_identity.rs` already exist
  and are ADR-011's own relocation target for it (`:744`; Description, item
  3); `qsl-semantics/src/check/mod.rs:127` defines `CheckedPackage` (landed under
  FR-068/M-5); `src/checking/` defines the lane-private
  `checking::CheckedPackage<'a>`; `src/protocol_artifact/native/mod.rs:68`
  defines the unrelated SEAM-3 `EmittedPackage`.

## Outputs

- A top-level `library` module (`qsl-semantics/src/library/`, declared in `src/lib.rs`) at
  the layer-3 position ADR-011 §6.1 assigns it (after `model`, before the
  `check` core in the intra-layer-3 order). This module is the relocation
  target ADR-011's module table (`:744`) names for `value::library`
  (FR-307) and `value::package_identity`, not a module built independently
  of them (Description, item 3): both relocate into it, carrying FR-307's
  existing acceptance criteria and TC-227's vectors forward, updated to the
  shapes item 3's owner ruling requires (below); `value::library` is not
  left behind as a second home for the same code. `value::library`'s
  current `NodeKey::from_hex` call inside `package_identity`'s `node_key`
  function (ADR-011 `:187`; Description, item 3) does not survive the move:
  after relocation, `library` calls no `NodeKey` constructor and mints no
  `NodeKey` from a `WireNodeId` (FR-087-AC-6 and FR-087-AC-11 govern this
  directly once the code is inside `library`). The module holds:
  - `LibraryName`, `LibraryPackage`, `ImportDeclaration`, `LibraryLock`,
    `resolve_libraries` and the rest of `value::library`'s public surface
    (relocated), except `ExportIdentity` and `resolve_name`, which do not
    relocate; `value::package_identity`'s preimage-reading surface
    (relocated), which reads each node id as the structural `NodeKey` the
    preimage names. A package is identified by its `LibraryName` alone.
  - `ImportView`: built only by `ImportView::of(&LibraryPackage)`, which
    validates the package's identity preimage structurally and exposes the
    package's exported declarations as `(name, NodeKey)` data for an
    importing package's own check stage to resolve by (O-04, R-06). Not
    checked typestate. Name resolution against it is E3's own, not
    `library`'s; `check` reads `ImportView` and the relocated `LibraryLock`
    (read-only, for qualifier binding) from `library`, a permitted layer-3
    edge under FR-068-AC-6's layer rule (FR-087-AC-9, TC-256).
  - The canonical cross-package node reference (T-3) is the structural
    `NodeKey`, whose `package` component names the package. Equality is
    structural: package, path and ordinal (ADR-013 §2).
  - `resolve_libraries`' whole-graph refusal set, classified across
    ADR-011 I2's package import graph rules, the preimage admission
    `ImportView::of` performs, and E3 name resolution (FR-087-AC-12).
- `CheckedGraph` (S3 output, new type, private constructors in `check`).
  `PackageDeclarations::check` (`qsl-semantics/src/check/mod.rs:291`) SHALL return
  `CheckedGraph`, not `CheckedPackage`; `check` SHALL name no
  `CheckedPackage` type at all after this requirement's implementation.
- `CheckedPackage` (S4 in-process output), relocated to layer-4 `package`,
  per ADR-013 T-1's text and ADR-011 §4's mechanism: its fields and
  constructor private to `package`, its state reached elsewhere only
  through named accessors, and named by every caller at
  `qsl_package::CheckedPackage`, with no re-export in the root crate
  (ADR-011 §7.2; §4 as amended). Its layer-5 operations are methods of the `CheckedPackageEvaluation`
  extension trait, which `value::expression` defines and implements for it
  (ADR-011 §4); a caller brings the trait into scope, and the trait keeps
  the method names, `CheckedPackage::call` among them (AD-016 Owner
  decision 6). Layer-4 `package` is the crate `qsl-package` (X-7, ADR-011
  §6.2, `package` row), so every `package` path this requirement names for
  the canonical `CheckedPackage` resolves under `qsl_package`. `package` constructs it from the `CheckedGraph` the
  S4 link step consumes, holding that `CheckedGraph` as a field (this
  package's own checked declarations) alongside the S4-only checked
  dependency closure; every accessor `CheckedPackage` exposes for a
  check-owned type (`Node`, `Signature`, and the rest of today's
  `check::CheckedPackage` accessor surface) delegates through the
  `CheckedGraph` field's own accessor methods rather than `package`
  re-declaring or copying those types itself (FR-087-AC-9).
- `EmittedPackage` (S4 wire output: the v2 bytes with their `package_id`),
  new, defined in layer-4 `package`, distinct from
  `protocol_artifact::native::EmittedPackage` (Description).
- Retirement of `ResolvedSourcePackage` (formerly `qsl-semantics/src/complete/package.rs`)
  once both of its successors exist (Behavior, "`ResolvedSourcePackage`
  retires when both successors exist"; ruling on QSL-229). Its dependency
  half, the `import` selections, is resolved in layer-3 `library` against
  the I2 import views (`ImportView`) that the M-4 I2
  reader produces (ADR-011 §6.2, §8). Its header-selection half, the
  `profile`, definition and `model` selections, is resolved at E3 against
  the QSpec definition prose and the domain packages I1 admitted
  ([FR-110](FR-110-resolve-header-profile-selections-at-e3.md), ADR-011
  §2.4). The change that removes the type deletes it,
  `resolve_source_package` and `command::resolve_parsed_source`, and leaves
  no `pub use` alias or wrapper that keeps an old name reachable.
  The closure and bundle move to `library::bundle`
  ([FR-111](FR-111-link-a-complete-v1-definition-bundle.md)).

## Behavior

### One nominal type per stage output; no generic state parameter

QSL SHALL define exactly one nominal type per stage output named in T-1:
unchecked `ParsedSource` (S2, pre-existing), checked `CheckedGraph` (S3,
new, in `check`), in-process checked `CheckedPackage` (S4, in `package`,
relocated from `check` by this requirement), packaged `EmittedPackage` (S4
wire, new), wire-admitted `ImportView` (I2, new,
in `library`). No type
SHALL be generic over a stage-state parameter (for example
`Package<State>`), so no `impl` block can accept two states through one
type. Each type's constructors SHALL be private to its stage module: a
`compile_fail` test on each public constructor path is the ADR-013 §1
evidence rule for R-10 and the typestate half of O-15.

### `check` produces `CheckedGraph`; `package` constructs `CheckedPackage`

`PackageDeclarations::check` (`qsl-semantics/src/check/mod.rs:291`) SHALL return
`CheckedGraph`. The checking methods FR-068 placed on `check::CheckedPackage`
(`check_expression`, `check_postcondition_expression`,
`check_clause_expression`, `qsl-semantics/src/check/mod.rs:601,625,648`) are S3 checking
behavior — they type and admit expressions against the package's declared
functions and dispatch tables, work that ADR-013 T-1 assigns to S3, before
S4 linking — and SHALL be retargeted to take and return `CheckedGraph` in
place of `CheckedPackage`. `check` SHALL define no `CheckedPackage` type,
method, or field after this requirement's implementation.

Layer-4 `package` SHALL define `CheckedPackage`, constructed by the S4 link
step from a `CheckedGraph` and, per E4, the checked dependency closure (each
dependency's own checked package, compiled from its identity-addressed source
and verified against the identity the checking package's I2 resolution
named). `CheckedPackage`'s constructor and fields SHALL be private to
`package`; `CheckedPackage::call` and `CheckedPackage::evaluate` (layer 5,
methods of `value::expression`'s `CheckedPackageEvaluation` trait, unaffected
by this requirement) SHALL reach its state only through `package`'s named
accessors, per ADR-011 §4's mechanism.

This closes the edge ADR-011 §6.1 forbids: `check` (layer 3) SHALL import
nothing from `package` (layer 4), since `check` no longer names
`CheckedPackage` at all. `package` importing `CheckedGraph` from `check` is
the layer-4 row's own permitted direction ("Depends on: 3, F, K"), and
`value::expression` importing `CheckedPackage` from `package` is the
layer-5 row's own permitted direction ("Depends on: 4, 3, F, K"). No
reverse edge exists at any point in this chain.

### R-10: checked typestate is never built from wire bytes

QSL SHALL construct `CheckedGraph` and `CheckedPackage` only through the S3
checker and the S4 link step respectively. No conversion from an unchecked
value (`ParsedSource`) or a wire-admitted value (`ImportView`, or any `protocol_artifact` read) to `CheckedGraph` or
`CheckedPackage` SHALL exist. The replay executor's requirement for a
`CheckedPackage` (O-26) is satisfied by recompiling identity-addressed source
through S1 to S4, never by reading v2 bytes into checked typestate (R-10;
ADR-011 §4 "Checking precedes lowering").

### `ImportView::of` is the only constructor of an import view

QSL SHALL construct an `ImportView` only through
`ImportView::of(&LibraryPackage)`, defined in `library`. `of` validates the
package's identity preimage structurally and refuses with a named cause
and yields nothing when the preimage is malformed or an export names no
declaration; no fallback to the file bytes or the source exists. A package
is identified by its `LibraryName` alone. The layer-4 `package` reader reads
the bytes into a `LibraryPackage` and calls `library` (ADR-011 §4).

### `ImportView` never resolves a name

QSL `library` SHALL build an `ImportView` from a `LibraryPackage` that
exposes the package's exported declarations as data, keyed
so an importing package's own check stage can resolve a reference to them
by structural `NodeKey` (T-3) without `library` itself performing name
resolution or re-checking any declaration (ADR-013 T-2: "I2 re-checks no
declaration"). No name → identity lookup SHALL exist in `library` after the
check stage that produced the exporting package (R-06).

### E3 resolves an imported name in the importing package's checker

The importing package's own check stage SHALL resolve a qualified reference
`a::Name` by binding the qualifier `a` to the one import declaration of that
package whose `as` qualifier is `a`, and looking `Name` up among the exports
of the package the resolution selects for that import; the result is that
export's structural `NodeKey`. An unqualified name resolves only to the
importing package's own declarations. An import written without an `as`
qualifier binds no name, and a library's name is not a qualifier. The
checker SHALL refuse a reference with `missing_declaration` /
`missing-name` when its qualifier is bound by no import, when the
selected package exports no declaration named `Name`, or when an unqualified name
matches no declaration of the importing package. Two cases never reach E3.
A qualifier bound by two imports is a duplicate alias, which the assembler
refuses `ambiguous_declaration` / `ambiguous-name` before E3 runs, naming
the alias and both import spans (FR-091). An import for which no library is
supplied is refused `missing_import` / `missing-selection` at stage
`intake` by the S4 source resolution, before E3 runs (ADR-015 D-1,
FR-099-AC-3).

### A structural `NodeKey` names its package

The canonical cross-package node reference is the structural `NodeKey`
(`{package, path, ordinal}`), whose `package` component names the package
(T-3). `library` defines no second reference shape. A `NodeKey` is
constructed in `check` for its own package's nodes (ADR-013 O-04, "In QSL
only `check` calls it"); a key read from a dependency's preimage is
resolved by lookup against an already-checked package: at E4, against the
dependency's own checked package, compiled independently from its
identity-addressed source; at E9 in the `replay` facade, against the package
recompiled from source. Two keys compare equal iff package, path and ordinal
compare equal (ADR-013 §2).

### `ResolvedSourcePackage` retires when both successors exist

`complete::resolve_source_package` resolves a source's `import` selections
and its `profile`, definition and `model` selections, against
`DefinitionCatalog` and `ModelCatalog`, into one `ResolvedSourcePackage`.
It computes the definition dependency closure, applies `PackageLimits`, and
refuses with a `ResolutionCause` (a substituted selection, a
missing selection, a definition cycle, a feature-set mismatch, a wrong
model selection, among others). ADR-011 gives each half its own successor
(ruling on QSL-229, 2026-09-24):

- **Dependency half.** QSL SHALL resolve a source's `import` selections in
  layer-3 `library` against the I2 import views (`ImportView`) that the M-4 I2 reader produces (ADR-011 §6.2, the
  `complete::package` row; ADR-011 §8, lane C2 → I2 and `library`). Owner:
  ADR-011 M-4.
- **Header-selection half.** QSL SHALL resolve a source's `profile`
  selections at E3 against the QSpec definition prose, and each `model`
  declaration against the domain package I1 admitted for it
  ([FR-110](FR-110-resolve-header-profile-selections-at-e3.md); ADR-011
  §2.4, amended 2026-09-26). The edition and each law's `DefinitionRef`
  come from the same definition prose through the emitter (ADR-011 §2.4,
  amended 2026-09-24).

Each `resolve_source_package` capability goes one of two ways:

| Capability | Disposition |
| --- | --- |
| `import` resolution | Spine S4 source resolution against I2 import views (FR-099, ADR-015 D-1). |
| Unknown profile refusal (`unknown_profile`) | FR-110, against the QSpec definition prose. |
| Conflicting selections of one profile identity (`ambiguous_declaration`/`conflicting-authority`) | FR-110: every selection but the exact `root` row refuses. |
| Duplicate selection alias | FR-091-AC-22 (the assembler). |
| Model selection | I1 (FR-056), which admits each selected package and records it by its `model_root` node. A `model` declaration carries no version (FR-056). |
| An inadmissible parse | Spine `compile` refuses it at S1 (`CompileRefusal::Source`). |
| Definition closure over a caller-supplied `DefinitionCatalog`: root refusal (`unknown_profile`), dependency edges, `missing_import`, `conflicting-authority`, `definition-cycle` | Moves to `library::bundle` ([FR-111](FR-111-link-a-complete-v1-definition-bundle.md)). |
| Complete-V1 facet and 176-capability bundle (`CompleteBundle`, `feature-set-mismatch`, `unknown_required_feature`) | Moves to `library::bundle` (FR-111). |
| `PackageLimits` (definitions, dependency edges, artifact bytes, single-artifact bytes) | Moves to `library::bundle`; every field still applies, to the catalog and to the closure (FR-111). |
| Resolved-graph identity (`quire.complete.resolved-graph/2`) | Moves to `library::bundle` as the bundle identity (FR-111). |
| Source authority and selection spans on a refusal (`SourceAuthority`, `PackageRefusal`'s span) | Deleted. A bundle refusal names its root's index (FR-111); a spine refusal names its region (FR-110, ADR-011 §2.3). |
| Compiled-model resolution (`ModelCatalog`, `ModelArtifact`, `ModelConflict`, the model `PackageError` variants) | Deleted. Model selections resolve only through I1 (a `model` declaration has no version form, FR-056). The FR-131 half of its test, a definition root naming a compiled model's identity, moves to FR-111-AC-2 (`unsupported-selection`). |

`library::bundle` is layer-3 `library`, the owner ADR-011 §6.2 gives
`complete::package`. It keeps QSpec FR-131-AC-1 to AC-3 and FR-339-AC-3
(V1-SRC-003 and V1-SRC-004, which QSpec's delivery manifest assigns to QSL)
backed: the tests tagged with them move with the code (ruling on QSL-234,
2026-09-26). Those tags are written
`QSpec-FR-131-AC-n` and `QSpec-FR-339-AC-3`; the repo-wide rule is in
[AGENTS.md](../../AGENTS.md#trace-tag-ids). It builds no checked
package, no v2 bytes and no backend artifact.

QSL SHALL delete `ResolvedSourcePackage`, `resolve_source_package`,
`SourceAuthority`, the compiled-model items and
`command::resolve_parsed_source`, and move the rest of
`complete::package` to `library::bundle`, in one change, made
once FR-110 is implemented (the dependency half already is: FR-099). In
that change each scenario of `tests/it/complete_package.rs` and
`complete::package_tests` moves to its row's successor with its tags.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-087-CON-1 | This requirement does not implement the v2 emitter itself (`CheckedPackage` → `EmittedPackage` bytes, C-03) or the layer-4 `package` reader's byte-level parsing; those are ADR-011 T-8 (M-4, #242). This requirement's `package`-layer output is limited to the type definitions T-1 names and the private-constructor/public-accessor surface ADR-011 §4 requires, so that M-4 has a typestate to build against. | Design | Inspection |
| FR-087-CON-2 | This requirement does not implement O-08, O-09, O-10, O-11, O-14 or C-26; those are FR-088 (S-3b). A type this requirement defines (for example the checked type node C-26 converts) SHALL NOT be given clause-kind, qualified-name or type-descriptor behavior by this requirement. | Design | Inspection |
| FR-087-CON-3 | **RETIRED (`PackageNodeKey` and `WireNodeId` are deleted).** The cross-package node reference is the structural `NodeKey`, whose `package` component names the package; no `NodeKey` is constructed outside `check` (ADR-013 O-04). | Design | Test (TC-255), retired |
| FR-087-CON-4 | `ResolvedSourcePackage` is removed in one change, made once both of its successors exist: the spine's resolution of `import` selections against I2 import views (the dependency half, FR-099, ADR-011 M-4) and E3's resolution of `profile` and `model` selections against the QSpec definition prose and I1 (the header-selection half, FR-110). The same change moves the closure and bundle to `library::bundle` (FR-111). It leaves no old name reachable: no feature flag, deprecation attribute or compatibility wrapper (no migration/fallback layer for prerelease software). | Process | Test (TC-246) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-087-AC-1 | Each of `CheckedGraph` (in `check`), `CheckedPackage` (in `package`), `EmittedPackage` (in `package`), `ImportView` (in `library`) has exactly one defining location, with every field and every constructor (the implicit struct-literal constructor, or an explicit `new`) private to that owning module, and every external read of the type's state goes through a named accessor. This is TC-171's own established claim (constructor/field privacy plus accessor-only reachability), extended to four types instead of three; TC-171 itself records why a `compile_fail` doctest cannot demonstrate it (a doctest links the crate externally, so it shows only crate-external inaccessibility, never that an in-crate sibling module — `value::expression` with respect to `check`'s types, or `check` with respect to `package`'s — cannot reach private state; and where a field is already module-private, not merely `pub(crate)`, a doctest's before/after comparison passes identically regardless of the module boundary this requirement draws, demonstrating nothing about it). This criterion is accordingly verified the same way TC-171 verifies its own claim: direct inspection of declared visibility and of the accessor surface, not a doctest. **Amended.** `ImportView` stays in layer-3 `library` with its fields private; `ImportView::of(&LibraryPackage)` is its only constructor, and a package is identified by its `LibraryName` alone. | Inspection (TC-243) |
| FR-087-AC-2 | No conversion function exists from `ParsedSource` to `CheckedGraph` or `CheckedPackage` other than the S3 checker's own entry and the S4 link step's own entry; no conversion function exists from `ImportView` or any `protocol_artifact`-read value to `CheckedGraph` or `CheckedPackage`. A source scan of every function returning `CheckedGraph` or `CheckedPackage` confirms each is defined inside the S3 checker or the S4 link step (Inspection, primary evidence, since this rules out an in-crate bypass a doctest's external vantage cannot see); a `compile_fail` doctest per forbidden path (Behavior, "R-10"; TC-244), each naming a specific expected error code, additionally demonstrates the crate-external case. | Test (TC-244) |
| FR-087-AC-3 | **RETIRED (the verified binding is deleted).** `ImportView::of(&LibraryPackage)` validates the identity preimage structurally and refuses with a named cause (`InvalidPreimage`, `UndeclaredExport`) and no partial output; a package is identified by its `LibraryName` alone. | Test (TC-253), retired |
| FR-087-AC-4 | `ImportView::of(&LibraryPackage)` builds an `ImportView` without resolving any name: `ImportView`'s exported-declaration data is keyed for structural `NodeKey` lookup by the importing package's own checker, and no function in `library` takes a name (`QualifiedName` or bare string) and returns a declaration or node id after the exporting package's own check stage has run. | Test (TC-254) |
| FR-087-AC-5 | **RETIRED (`PackageNodeKey` is deleted).** A cross-package node reference is the structural `NodeKey`, whose `package` component names the package; two keys compare equal iff package, path and ordinal are equal. | Test (TC-245), retired |
| FR-087-AC-6 | No `NodeKey` constructor call anywhere in the crate is fed, directly or transitively, by an `ImportView` entry, or any other wire-read value: `library`, `package`'s E4 dependency resolution, and `replay`'s E9 lookup each make zero `NodeKey`-constructor calls, resolving a wire-read node id to a `NodeKey` only by lookup against an already-minted `NodeKey` in an already-checked package (ADR-013 O-04, `:173,175`). This is scoped to what this requirement (S-3) owns: the crate-wide claim "only `check` calls the `NodeKey` constructor" is FR-060 T12-B's own allow-list, whose named debt list (FR-060 Behavior, "T12-B and T12-C: shipped code and debt lists") holds the real minting sites outside `check` this requirement does not touch or gate on — pre-existing debt this requirement neither fixes nor is blocked by (Remaining work: agent-ix/quire-spec-language#211). This criterion fails only if `library`, `package`'s E4 path, or `replay`'s E9 path gains a `NodeKey`-constructor call, or if any such call anywhere in the crate is fed by an `ImportView` entry. This is a call-site and data-provenance policy (which module is permitted to mint, and what may feed a mint, not which shape a lookup returns), enforced by the FR-060 T-12 API-surface scan and a call-graph trace rather than by the type system, so it is verified by that scan and trace, not by a `compile_fail` doctest (a mismatched-type attempt at some other call site would fail identically regardless of location, proving nothing about it). | Test (TC-255) |
| FR-087-AC-7 | Once FR-110 is implemented (the dependency half is FR-099), `ResolvedSourcePackage`, `resolve_source_package`, `SourceAuthority`, `ModelCatalog`, `ModelArtifact` and `command::resolve_parsed_source` are absent from every crate of the workspace under every feature combination, and `qsl_semantics::complete` no longer exists: its closure, bundle, limits and identity are `library::bundle` (FR-111). Each scenario of `tests/it/complete_package.rs` and `complete::package_tests` has a test against its row's successor in Behavior's disposition table, keeping its QSpec FR-131/FR-339 tags, written `QSpec-FR-131-AC-n` and `QSpec-FR-339-AC-n`. A whole-workspace definition scan confirms the absence. | Test (TC-246) |
| FR-087-AC-8 | The canonical `EmittedPackage` (this requirement, layer-4 `package`) and `protocol_artifact::native::EmittedPackage` (SEAM-3, unrelated) remain two distinct types under two distinct module paths, with no `pub use` or re-export that would place both names into one import scope, and with no field, method, or shape shared between them (Description), for as long as the SEAM-3 type exists: ADR-011 §4 records that the two-public-types-named-`CheckedPackage` rule is met by *deleting* the native-v1 type with SEAM-1, not by permanent coexistence, and this requirement's own `CheckedPackage`/`checking::CheckedPackage<'a>` pair follows the same disposition (Description, item 2). The expiry condition for `EmittedPackage`'s two-module coexistence is SEAM-3's own deletion of `protocol_artifact::native::EmittedPackage` (ADR-011 §7.3), not an indefinite steady state this requirement asserts. | Inspection (TC-247) |
| FR-087-AC-9 | After this requirement's implementation, the `check`/`package` edge runs one way: `package` (layer 4) may import from `check` (layer 3), the layer-4 row's own direction (§6.1: "Depends on: 3, F, K"), and `check` imports nothing from `package` (the crate `qsl-package` since X-7) at either the `use`-line or the inline-path level (Cargo refuses the edge: `qsl-package` depends on `qsl-semantics`; this criterion reconfirms it). `check` defines no type, method, or field named `CheckedPackage`. `package`'s `CheckedPackage` is built by the S4 link step from a `CheckedGraph` and holds that `CheckedGraph` as a field (this package's own checked declarations) plus the S4-only checked dependency closure; it reaches check-owned state (`Node`, `Signature`, and the rest) through that field (`graph()` or named delegating accessors) and never re-declares, copies or re-derives a check-owned type in `package`. This design statement is kept because it is the S3-to-S4 typestate chain ADR-013 T-1 requires: S4's output is made from S3's output and nothing else, and every checked declaration has one owner. `check` imports from `library` only as layer 3's own order permits (`library` before `check` core); `check` reads `LibraryLock` only through its read-only accessors and never constructs or mutates one. `value::expression` (layer 5) imports `CheckedPackage` from `package` (`use qsl_package::CheckedPackage;`), the layer-5 row's own direction (§6.1: "Depends on: 4, 3, F, K"). **Amended (X-7).** This sentence used to require the single closed re-export `pub use crate::checked_package::CheckedPackage;` in `value::expression`. Once `CheckedPackage` is in `qsl-package`, that line is a root-crate re-export of a moved item, which ADR-011 §7.2 and the X-7 extraction's acceptance criteria forbid; they are the later and more specific rules, as lead ruling R3 ruled for FR-068-AC-10. The root crate re-exports no layer-crate item, and a `pub use` of one fails this criterion. This criterion demonstrates that the layering violation FR-068 recorded is closed, not relocated: a build in which `check` names `CheckedPackage`, imports from `package`, or in which `package`'s `CheckedPackage` holds anything other than a `CheckedGraph` for its own checked declarations, fails this criterion. **Amended by the layer-rule ruling (2026-09-22).** The number of `check` items `package` imports, and the number of `library` items `check` imports, are not bounded. A count of `package`'s `check` imports is met by a `package` that copies check-owned data into re-declared types, and fails a `package` that imports `Node` to name an accessor's return type, which is layer 4's permitted direction; the design statement above guards the property instead. Every `library` item is a permitted edge for `check`, because `library` precedes `check` core in §6.1's layer-3 order. That no `NodeKey` is minted from an `ImportView` entry is data provenance, held by FR-087-AC-6 and FR-060 T12-B. | Test (TC-256) |
| FR-087-AC-10 | `src/package/features.rs` and `src/package/view.rs` continue to import `CheckedPackage` from `crate::checking` (the lane-private `checking::CheckedPackage<'a>`), unchanged by this requirement's introduction of a canonical `CheckedPackage` elsewhere in `package`'s own module tree, for as long as `checking`'s lane exists. No file in `package` re-exports or glob-imports both the lane-private and the canonical `CheckedPackage` into one scope where a bare `CheckedPackage` reference would be ambiguous; a source scan of `features.rs` and `view.rs` confirms their `crate::checking::CheckedPackage` import is present and unrenamed while that lane exists, and a build with both names in one import scope (an ambiguity error) fails this criterion while the import is present. This two-name coexistence is not asserted as a permanent steady state: ADR-011 §8 (Q209-1) records that `checking`'s lane retires (lane A) or converges into its replacement (lane B/D) — "each other lane is deleted with its replacement" — and the expiry condition for `features.rs`/`view.rs`'s `crate::checking::CheckedPackage` import is that lane's own deletion, per its own convergence PR, not an indefinite state this criterion treats as final. After that deletion, this criterion no longer applies to those two files (there is only one `CheckedPackage` left to import). | Test (TC-247) |
| FR-087-AC-11 | **RETIRED (the `PackageId` selection and `WireNodeId`-typed `library` shapes are deleted).** `value::library` and `value::package_identity` no longer exist as modules under `value`; `LibraryName`, `LibraryPackage`, `ImportDeclaration`, `LibraryLock`, `resolve_libraries` and `package_identity`'s preimage-reading functions are reachable only from the top-level `library` module. A package is identified by its `LibraryName` alone, `LibraryLock::selections()` returns `Vec<LibraryName>`, and `library` defines no function resolving a name to a declaration or node id. | Test (TC-281), retired |
| FR-087-AC-12 | Every `LibraryRefusal` variant `resolve_libraries` (in `library`) can return classifies to exactly one of: an ADR-011 I2 graph rule, the preimage admission `ImportView::of` performs, or E3 name resolution. Concretely: `InvalidPreimage` and `UndeclaredExport` classify to preimage admission as per-package checks (raised by `verify_package`); `InvalidQualifier` classifies to E3 name resolution; `MissingImport` classifies to I2's first rule (missing or unlisted identity); `ConflictingDefinition` (two supplied packages of one library identity) classifies to I2's second rule; `ImportCycle` classifies to I2's third rule. This criterion does not require `resolve_libraries` to gain a new refusal variant; it requires every variant to be classified to one of the three, with none left unclassified or misclassified. | Test (TC-282) |
| FR-087-AC-13 | The importing package's checker resolves imported names as Behavior, "E3 resolves an imported name in the importing package's checker", states. Given package `P` importing library `L@1` (which exports `R`) with no `as` qualifier, and declaring no `R` of its own, `P`'s checker refuses both `R` and `L::R` with `missing_declaration` / `missing-name`. Given `P` importing `A@1` and `B@1` both `as a`, the assembler refuses the second alias `a` with `ambiguous_declaration` / `ambiguous-name`, naming both import spans, and E3 does not run. Given `P` importing `L@1` `as l`, `l::R` resolves to R's exported structural `NodeKey` in `L@1`'s package, and `l::Q`, which `L@1` does not export, is refused `missing_declaration` / `missing-name`. Given `P` importing `L@1` `as l` with no library supplied as `L`, the import is refused `missing_import` / `missing-selection` at stage `intake`, and E3 does not run. | Test (TC-379) |
| FR-087-AC-14 | The E4 link step with dependencies (`CheckedPackage::link_with`) records each imported package's `package_id` as selected (an import names its library by identity alone); the closure holds one `{identity, package_id}` entry per library identity over the direct imports and every dependency's own closure, and the step refuses two packages of one identity whose `package_id`s differ with `invalid_package`/`conflicting-definition` (FR-307's diamond rule), naming both packages and both dependency paths. Two paths reaching one package unify. A refusal yields no package. Binding an `Import`'s identity to the source's resolved import declarations is FR-099's (ADR-015 D-1). `CheckedPackage::dependencies` holds each direct import's checked package by `package_id`, and `CheckedPackage::dependency_selections` the closure in ascending UTF-8 byte order of identity. | Test (TC-253) |

The `library`/`ImportView` name-resolution scan does not see a node
id returned as raw bytes, a hex `String` or a `usize` position; a
name passed as `&[u8]`; a closure without type annotations, including one
held in a `fn(&str) -> Option<WireNodeId>` variable; or a local variable's
type inside a function body.

For `ImportView`, crate-external construction
is shown by a `compile_fail` doctest: a struct literal of the type. The doctests show only
that a crate-external caller fails to compile; stable rustdoc does not check
a `compile_fail` block's error code.

`xtask::typestate_scan` scans shipped code, excluding `#[cfg(test)]`
items and every file a `#[cfg(test)] mod` declares. The scan sees through
`use … as` renames and `type` aliases. The module doc lists what it does
not see: macro bodies other than for mints, out-parameters, and call graphs.

Accessor-only reads are not
backed, because a private field stays readable by its module's child
modules. A wire value that reaches a mint through another function's call
is not traced (TC-255 steps 3 and 6). Methods and trait impls are not
compared for AC-8 and AC-10.

## Dependencies

- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  §1 R-10, §3 O-15, §3.1 T-1 and T-3, §6 (lane-private table), §7 (the S-3
  slice row and its Gate).
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.1, §2.4 (package selections: where the header selections resolve), §4,
  §6.1, §6.2 (the `complete::package` row), §7.3 (M-4's row, which
  #242 implements against this requirement's typestate), §8 (Q209-1,
  Q209-2, lane C2).
- [US-005](../usecase/US-005-trust-checked-identity-across-packaging.md).
- [FR-060](FR-060-check-qsl-api-surface-boundary.md), whose T-12 API-surface
  check enforces the minting rule this requirement's structural
  `NodeKey` references depend on (only `check` calls the `NodeKey` constructor).
- [FR-068](FR-068-split-expression-checking-into-check-stage.md), which
  landed `check::CheckedPackage` and `PackageDeclarations::check`'s
  `CheckedPackage` return type (M-5), and recorded the ADR-011
  §4-versus-§6.1 conflict this requirement resolves per the QSL-158 owner
  ruling (Status): `PackageDeclarations::check` returns `CheckedGraph`
  after this requirement, and `check::CheckedPackage` (`qsl-semantics/src/check/mod.rs:127`)
  and its three checking methods (`qsl-semantics/src/check/mod.rs:601,625,648`) are
  relocated/retargeted as this requirement's Behavior and Outputs state.
- ADR-011 M-4's I2 reader (successor to `ResolvedSourcePackage`'s
  dependency half) and [FR-110](FR-110-resolve-header-profile-selections-at-e3.md)
  (successor to its header-selection half), per the ruling on QSL-229.
- The owner ruling on QSL-158 (2026-09-21) on `CheckedPackage`'s placement,
  which closes the `CheckedPackage`-placement half of the ADR-011
  §4-versus-§6.1 defect; its separate §6.2 refusal-row finding stays open.

## References

- Linear QSL-263 (AC-13's duplicate-alias and missing-library cases assert the stage that refuses them).
