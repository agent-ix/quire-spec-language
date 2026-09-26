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
---
# FR-087: S-3a: one nominal typestate per stage output and the cross-package node key

## Description

ADR-013 §7 names slice **S-3** ("Typestate, clause and type": O-08, O-09
clause id, O-10, O-11, O-14, O-15, T-1, T-3), and no FR owns it (QSL-158).
QSL-158's own comments split S-3 along the line the scoping pass identified:
**S-3a** (this requirement) is the typestate half — T-1, T-3, O-15 — because
it is the whole of what QSL-6 (#242, ADR-011 M-4) waits on; **S-3b**
(FR-088) is the clause/name/type half — O-08, O-09 clause id, O-10, O-11,
O-14, C-26. This requirement covers S-3a only.

S-3a's gate (ADR-013 §7: "S-2, QC-10") is clear: S-2 landed as PR #260
(`97ec26e3`), and QC-10 landed under STD-2 (Done) — QSpec
[FR-322](https://github.com/agent-ix/quire-specification/blob/main/spec/objects/interfaces/FR-322-checked-package-artifact.md)
(`quire-specification`, not this repository; `spec/objects/` does not exist
here) defines `dependency_reference` as `PackageNodeKey{package, node}` at
its `:147-157`, and FR-322-AC-26 (its `:365`) fixes `node` as a
`WireNodeId`. Review finding FND-021
(`spec/reviews/stage-dag/integrity.md:200`, "PackageNodeKey has two shapes")
is already fixed in the ADR text: ADR-011:200 (I2) and ADR-011:353 (E3) both
read `PackageNodeKey{package: package_id, node: WireNodeId}`, agreeing with
ADR-013 T-3 (`:667`) and FR-322-AC-26. This requirement does not re-open
FND-021's review file; it restates the now-agreeing ADR text as its own
Inputs.

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
   FR-068 (M-5, QSL-139) in layer-3 `check`, and FR-068's own text
   acknowledges this contradicts ADR-011 §4's worked example, which names
   layer-4 `package` as the S4 type's owner; FR-068 resolved that
   contradiction by following §6.1's layer ordering instead, reasoning that
   §4's placement would open a `check` → `package` reverse edge, and opened
   QSL-167 to track "§4 versus §6.1 on `CheckedPackage`'s owning layer" as
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
   FR-087-AC-9. This closes the `CheckedPackage`-placement half of QSL-167;
   QSL-167's separate §6.2 refusal-row finding is untouched by this
   requirement.

3. **The new top-level `library` module this requirement builds is not
   built from nothing: two modules already exist and ADR-011's own
   module-move table assigns both to it.** `src/value/library.rs` already
   implements FR-307 ("reusable semantic libraries": `LibraryName`,
   `LibraryPackage`, `PackageId`, `ImportDeclaration`, `LibraryLock`, and
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

   **Owner ruling on QSL-158 (2026-09-21): how `resolve_libraries`/
   `LibraryLock`'s existing whole-graph resolution composes with the new
   per-dependency `VerifiedPackage`/`ImportView`/§4 binding.** Neither ADR
   names `LibraryLock` by that name; identifying it as the "library lock"
   both ADRs otherwise leave unnamed is this owner ruling's own
   contribution, consistent with: ADR-011 `:206` (I2's first rule, "an
   import is missing, or its identity is not listed in the library lock or
   pinned request"); ADR-011 `:253` (the E3 edge row, which lists "layer-3
   `library` import views (I2) for name resolution only, library lock" as
   E3's inputs alongside the import views); ADR-011 `:513` (the §4 binding's
   third condition, "that identity is listed in the consumer's library lock
   or pinned request"); and ADR-011 `:981` (row 8, the model-bound identity
   change scenario: "Consumers pinned to the old identity refuse at the I2
   binding (§4) until their lock names the new one"). This settles:

   a. `LibraryLock` relocates into `library` unchanged in shape and IS the
      library lock the §4 binding's third condition reads: it is the
      source of each dependency's expected `package_id` for that
      condition. No second lock type exists.
   b. `resolve_name` does not relocate into `library`. Name resolution
      against an imported dependency's exports is E3's own resolution over
      an `ImportView` (Behavior, "`ImportView` never resolves a name";
      ADR-011 `:200`; ADR-013 R-06), performed by the importing package's
      own check stage, not by `library`. `library`'s relocated code
      carries `resolve_libraries` and `LibraryLock` forward but not
      `resolve_name`: the checker resolves a name to a `PackageNodeKey` by
      looking it up against the `ImportView` `library` already handed it,
      never by calling back into `library`. E3's own resolution binds a
      qualified reference's `a::Name` qualifier by reading the relocated
      `LibraryLock`'s selections directly (ADR-011 `:253` lists "library
      lock" beside the import views as an E3 input) — `check` accordingly
      reads both `ImportView` and `LibraryLock` (read-only) from `library`,
      a permitted layer-3 edge under FR-068-AC-6's layer rule
      (FR-087-AC-9, TC-256).
   c. `library`'s existing `ExportIdentity{package: PackageId, node:
      NodeKey}` (`value/library.rs:101-108`) does not relocate in this
      shape: T-3's `PackageNodeKey{package: package_id, node: WireNodeId}`
      (Outputs) is the sole cross-package node reference `library` defines
      after relocation (FR-087-AC-5), and no relocated or new type in
      `library` names a `node: NodeKey` field.
   d. `package_identity`'s wire-to-`NodeKey` path does not relocate in that
      shape: no relocated type holds a `NodeKey` read from wire or preimage
      data, and the relocated `package_identity` reads a wire node
      reference as a `WireNodeId`, never a `NodeKey` (R-10, O-04). This
      covers every wire-fed `NodeKey` field `package_identity` defines
      today, not only the `node_key` function's own return type
      (`value/package_identity.rs:177-182`, built with `NodeKey::from_hex`
      over wire-read JSON): `PreimageDefect::AmbiguousDeclaration`'s
      `nodes: [NodeKey; 2]` field (`:89`) and
      `PreimageDefect::DeclarationNominalMismatch`'s `node: NodeKey` field
      (`:96`) are themselves built from the same wire-read preimage data
      `node_key` reads, and `ProjectedDeclarations`'s internal
      `BTreeMap<String, NodeKey>` (`:130`) is populated from that same
      preimage projection; all three relocate as `WireNodeId`-typed instead.
      No `NodeKey` is constructed from wire bytes, and no wire-fed value is
      held in a `NodeKey`-typed field, anywhere in `library`
      (FR-087-AC-11).
   e. `resolve_libraries`' existing whole-graph refusal set
      (`value/library.rs`'s `LibraryRefusal`, nine variants) classifies to
      exactly one of: an ADR-011 I2 graph rule (`:203-210`), the §4
      binding's condition 2 (digest/preimage admission, ruling (f)), the §4
      binding's condition 3 (identity pinned by the library lock, `:513`),
      or E3 name resolution (ruling (b)) — with one stated exception:
      - `PackageIdMismatch` (`:333-339`): raised by `verify_package`'s own
        recomputed-digest comparison — condition 2 itself (ruling (f)'s
        reused function).
      - `InvalidPreimage` (`:340-344`) and `UndeclaredExport` (`:345-349`,
        `.select(&package.exports)`): both raised by the same
        `verify_package` call, but only after condition 2's digest
        comparison has already completed — `InvalidPreimage` validates the
        preimage's own structure, `UndeclaredExport` checks the package's
        own declared exports against its own projected declarations.
        Neither is a prerequisite condition 2 needs to run; both are
        per-package admission checks grouped here only because the same
        reused function raises them, before any import-graph or name
        question is reached — not I2 or E3.
      - `InvalidQualifier` (`:411-414`): validates an import's `as`
        qualifier as an identifier — a check that exists only to support
        binding `a::Name` references during name resolution. It moves
        conceptually with `resolve_name` (ruling (b)): E3 name resolution,
        not an I2 graph rule.
      - `ConflictingDefinition` (`:425-434`) and `ImportCycle`
        (`:416-424`): two import paths claiming one library identity with
        different selections, and an import-graph cycle, respectively —
        I2's second and third rules exactly.
      - `StaleDependency{cause: RevisionMismatch}` (`:446-447`): raised
        when a supplied package's `package_id` *and* library identity
        already match the import, but its recorded `version` string does
        not — the identity itself is present; only the pinned-request
        metadata the consumer's lock records disagrees. This checks the
        same fact as condition 3 ("that identity is listed in the
        consumer's library lock or pinned request"), extended to the
        version the lock's entry records, not an I2 graph-structure
        question.
      - `StaleDependency{cause: ByteDigestMismatch}` (`:448-449`) and
        `MissingImport` (`:451`): the import's `package_id` is not present
        among the supplied packages under its identity at all — I2's first
        rule, "an import is missing, or its identity is not listed in the
        library lock or pinned request," directly.
      - **Exception: `DuplicatePackageId` (`:386-389`) fits none of the
        four.** It refuses only after both supplied packages have already,
        independently, passed `verify_package`'s digest check — so each
        one's `package_id` already equals the digest recomputed over its
        own `identity_preimage`. Two packages sharing one `package_id`
        therefore share byte-identical `identity_preimage` bytes; they can
        only still differ in the `LibraryPackage` fields the preimage
        excludes (`version`, `imports`, `exports`) — conflicting metadata
        over identical identity content, not two different contents
        colliding on one digest. This is the reverse of
        `ConflictingDefinition` (one identity, two competing selections
        from different import sites), not a restatement of it. ADR-011
        `:203-210`'s three I2 rules govern how import views relate to the
        identities they claim; none of the three addresses this
        supplied-pool precondition. `DuplicatePackageId` is a content-addressing sanity check the
        resolution algorithm's `by_id: BTreeMap<PackageId, _>` indexing
        needs to be sound, independent of and prior to any §4 admission or
        I2 graph question; it is retained as `resolve_libraries`' own
        refusal, stated as lying outside this criterion's taxonomy rather
        than forced into one of the four.

      This mapping is this requirement's own scope (FR-087-AC-12, TC-282),
      not a new refusal `resolve_libraries` gains.
   f. `verify_package` (`value/library.rs:331-349`, FR-307's existing
      `package_id`-recomputation-then-preimage-validation function) is
      reused, unchanged in shape, as the §4 binding's condition-2 digest
      check (Behavior, "The verified binding constructs `VerifiedPackage`,
      not checked typestate"); FR-087-AC-3 requires reuse of this existing
      computation, not a second, independently written digest
      implementation.

   TC-227's vectors are updated to these new shapes (`ExportIdentity`
   replaced by `PackageNodeKey`, `resolve_name` removed from `library`,
   `package_identity`'s wire node reference changed from `NodeKey` to
   `WireNodeId`) rather than carried forward byte-identical: relocating
   `LibraryLock`/`resolve_libraries` unchanged does not mean the module's
   every pre-existing type keeps its pre-existing shape, since two of those
   types (`ExportIdentity`, and `package_identity`'s wire-`NodeKey` path)
   are exactly what R-10/T-3 require this requirement to change. No
   production code calls `resolve_libraries` or constructs a `LibraryLock`
   today, so this shape change has no other caller to migrate.

## Inputs

- ADR-013 O-15 (typestate), T-1 (typestate encoding and names), T-3 (the
  cross-package node key), R-10 (checked typestate is minted only by the
  checker and the link step; wire bytes never become it).
- ADR-011 §2.1 (admitted types and owners), §4 ("Checking precedes lowering":
  the verified-binding and dependency-binding rules, and the private-
  constructor/public-accessor mechanism), §6.1 (the layer table: layer 3
  `library`, with `VerifiedPackage` and `ImportView`; layer 4 `package`).
- The current tree: `qsl-semantics/src/complete/package.rs:618-1268` defines
  `ResolvedSourcePackage` (complete-V1 lane C2); no *top-level* `library`
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
  - `LibraryName`, `LibraryPackage`, `PackageId`, `ImportDeclaration`,
    `LibraryLock`, `resolve_libraries` and the rest of `value::library`'s
    existing public surface (relocated, unchanged in shape, per item 3's
    owner ruling (a) and (e)), except `ExportIdentity` and `resolve_name`,
    which do not relocate in their current shape (item 3, owner ruling (b)
    and (c)); `value::package_identity`'s existing preimage-reading surface
    (relocated), except every `NodeKey`-typed field or return value fed by
    wire or preimage data — `node_key`'s return type, `PreimageDefect`'s
    `AmbiguousDeclaration.nodes`/`DeclarationNominalMismatch.node` fields,
    and `ProjectedDeclarations`'s internal map — which relocate typed
    `WireNodeId` instead (item 3, owner ruling (d)). `LibraryLock` and
    `LibraryRefusal` (the latter carrying `PreimageDefect`) are otherwise
    unchanged in shape.
  - `VerifiedPackage`: the v2-bytes package after the §4 verified binding
    holds (supported version; recomputed `package_id` equal to the declared
    one; that identity listed in the consumer's library lock or pinned
    request — the relocated `LibraryLock`, item 3 owner ruling (a)). Not
    checked typestate (R-10). Condition 2's digest recomputation reuses the
    relocated `verify_package` function unchanged (item 3, owner ruling
    (f)).
  - `ImportView`: converted from a `VerifiedPackage`, exposing the verified
    package's exported declarations as data for an importing package's own
    check stage to resolve by (O-04, R-06). Not checked typestate. Name
    resolution against it is E3's own, not `library`'s (item 3, owner
    ruling (b)); `check` reads `ImportView` and the relocated `LibraryLock`
    (read-only, for qualifier binding) from `library`, a permitted layer-3
    edge under FR-068-AC-6's layer rule (FR-087-AC-9, TC-256).
  - `PackageNodeKey{package: package_id, node: WireNodeId}` (T-3): the
    canonical cross-package node reference, pinning the verified content and
    naming a node without constructing a `NodeKey` from wire bytes. Equality
    is declared: both components compare lexically (ADR-013 §2). This
    replaces `value::library`'s existing `ExportIdentity{package, node:
    NodeKey}` as the module's sole cross-package node reference type (item
    3, owner ruling (c)).
  - `resolve_libraries`' existing whole-graph refusal set, classified
    across ADR-011 I2's package import graph rules, the §4 binding's
    conditions 2 and 3, and E3 name resolution, with `DuplicatePackageId`
    named as lying outside all four (item 3, owner ruling (e);
    FR-087-AC-12).
- `CheckedGraph` (S3 output, new type, private constructors in `check`).
  `PackageDeclarations::check` (`qsl-semantics/src/check/mod.rs:291`) SHALL return
  `CheckedGraph`, not `CheckedPackage`; `check` SHALL name no
  `CheckedPackage` type at all after this requirement's implementation.
- `CheckedPackage` (S4 in-process output), relocated to layer-4 `package`,
  per ADR-013 T-1's text and ADR-011 §4's mechanism: its fields and
  constructor private to `package`, its state reached elsewhere only
  through named accessors, and named by every caller at
  `qsl_package::CheckedPackage`, with no re-export in the root crate
  (QSL-182, ADR-011 §7.2; §4 as amended). Its layer-5 operations are methods of the `CheckedPackageEvaluation`
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
- Retirement of `ResolvedSourcePackage` (`qsl-semantics/src/complete/package.rs`)
  once both of its successors exist (Behavior, "`ResolvedSourcePackage`
  retires when both successors exist"; ruling on QSL-229). Its dependency
  half, the `import` selections, is resolved in layer-3 `library` against
  the I2 import views (`VerifiedPackage`, `ImportView`) that the M-4 I2
  reader produces (ADR-011 §6.2, §8; QSL-6). Its header-selection half, the
  `profile`, definition and `model` selections, is resolved at E3 against
  QSL's `DefinitionLock` catalog and the domain packages I1 admitted
  ([FR-110](FR-110-resolve-header-profile-selections-at-e3.md), ADR-011
  §2.4; QSL-234). The change that removes the type deletes it,
  `resolve_source_package` and `command::resolve_parsed_source`, and leaves
  no `pub use` alias or wrapper that keeps an old name reachable (QSL-269).
  Where the rest of `qsl_semantics::complete` goes is an open owner
  question (Behavior, disposition table).

## Behavior

### One nominal type per stage output; no generic state parameter

QSL SHALL define exactly one nominal type per stage output named in T-1:
unchecked `ParsedSource` (S2, pre-existing), checked `CheckedGraph` (S3,
new, in `check`), in-process checked `CheckedPackage` (S4, in `package`,
relocated from `check` by this requirement), packaged `EmittedPackage` (S4
wire, new), wire-admitted `VerifiedPackage` and `ImportView` (I2, both new,
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
dependency's own checked package, compiled from its digest-addressed source
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
value (`ParsedSource`) or a wire-admitted value (`VerifiedPackage`,
`ImportView`, or any `protocol_artifact` read) to `CheckedGraph` or
`CheckedPackage` SHALL exist. The replay executor's requirement for a
`CheckedPackage` (O-26) is satisfied by recompiling digest-addressed source
through S1 to S4, never by reading v2 bytes into checked typestate (R-10;
ADR-011 §4 "Checking precedes lowering").

### The verified binding constructs `VerifiedPackage`, not checked typestate

QSL SHALL construct a `VerifiedPackage` from v2 wire bytes only when all
three ADR-011 §4 conditions hold: the schema version is a supported v2
version; the FR-322 `package_id`, recomputed as the `quire.package.semantic/v2`
digest of the JCS bytes of the `identity_preimage` read, lexically equals
the declared `package_id`; and that identity is listed in the consumer's
library lock or pinned request. If any of the three conditions fails to
hold, QSL SHALL refuse with a named cause and yield nothing — no partial
`VerifiedPackage`, and no fallback to a digest of the file bytes, the lock
file, or the source. This binding and
`VerifiedPackage` itself SHALL be defined in `library`; the layer-4
`package` reader reads the bytes and calls `library` to verify them
(ADR-011 §4).

### `ImportView` never resolves a name

QSL `library` SHALL convert a `VerifiedPackage` into an `ImportView` that
exposes the verified package's exported declarations as opaque data, keyed
so an importing package's own check stage can resolve a reference to them
by `PackageNodeKey` (T-3) without `library` itself performing name
resolution or re-checking any declaration (ADR-013 T-2: "I2 re-checks no
declaration"). No name → identity lookup SHALL exist in `library` after the
check stage that produced the exporting package (R-06).

### E3 resolves an imported name in the importing package's checker

The importing package's own check stage SHALL resolve a qualified reference
`a::Name` by binding the qualifier `a` to the one import declaration of that
package whose `as` qualifier is `a`, and looking `Name` up among the exports
of the package the library lock selects for that import; the result is that
export's `PackageNodeKey`. An unqualified name resolves only to the
importing package's own declarations. An import written without an `as`
qualifier binds no name, and a library's name is not a qualifier. The
checker SHALL refuse a reference with `missing_declaration` /
`missing-name` when its qualifier is bound by no import, when the library
lock selects no package for the import that binds the qualifier, when the
selected package exports no declaration named `Name`, or when an unqualified name
matches no declaration of the importing package. The checker SHALL refuse
every use of a qualifier bound by two or more imports with
`ambiguous_declaration` / `ambiguous-name`, carrying one locus per import
that binds it.

### `PackageNodeKey` pins content and never mints a `NodeKey` from wire bytes

QSL `library` SHALL define `PackageNodeKey{package: package_id, node: WireNodeId}`
as the sole cross-package node reference (T-3). It SHALL NOT define or
accept a `PackageNodeKey` shaped `{package, node: NodeKey}`. A `NodeKey`
SHALL NOT be constructed from a `WireNodeId` anywhere: ADR-013 O-04 (`:173`)
states "In QSL only `check` calls it" (the `NodeKey` constructor), fed only
by a node-identity preimage `check` computes for its own package, never by
a `WireNodeId`. A `PackageNodeKey`'s `WireNodeId` resolves to a `NodeKey`
only by lookup against an already-minted `NodeKey` in a checked package
(ADR-013 O-04, `:175`: "It becomes a `NodeKey` only by lookup in a QSL
checked package"): at E4, against the dependency's own checked package,
compiled independently from its digest-addressed source, whose own S3
checking already minted the `NodeKey` being referenced; at E9 in the
`replay` facade, against the package recompiled from source, likewise
already checked. Neither E4 nor E9 calls the `NodeKey` constructor. Two
`PackageNodeKey`s SHALL compare equal iff both components compare
lexically equal (declared equality, ADR-013 §2); no digest or structural
comparison over the referenced node's content SHALL substitute.

### `ResolvedSourcePackage` retires when both successors exist

`complete::resolve_source_package` resolves a source's `import` selections
and its `profile`, definition and `model` selections, against
`DefinitionCatalog` and `ModelCatalog`, into one `ResolvedSourcePackage`.
It computes the definition dependency closure, applies `PackageLimits`, and
refuses with a `ResolutionCause` (a stale or substituted selection, a
missing selection, a definition cycle, a feature-set mismatch, a wrong
model selection, among others). ADR-011 gives each half its own successor
(ruling on QSL-229, 2026-09-24):

- **Dependency half.** QSL SHALL resolve a source's `import` selections in
  layer-3 `library` against the I2 import views (`VerifiedPackage`,
  `ImportView`) that the M-4 I2 reader produces (ADR-011 §6.2, the
  `complete::package` row; ADR-011 §8, lane C2 → I2 and `library`). Owner:
  QSL-6 (ADR-011 M-4).
- **Header-selection half.** QSL SHALL resolve a source's `profile`
  selections at E3 against QSL's `DefinitionLock` catalog, and each `model`
  declaration against the domain package I1 admitted for it
  ([FR-110](FR-110-resolve-header-profile-selections-at-e3.md); ADR-011
  §2.4, amended 2026-09-26; QSL-234). The lock's edition and definition
  selections come from the same catalog through the emitter (ADR-011 §2.4,
  amended 2026-09-24).

Each `resolve_source_package` capability goes one of two ways:

| Capability | Disposition |
| --- | --- |
| `import` resolution | Spine S4 source resolution against I2 import views (FR-099, ADR-015 D-1). |
| Unknown, stale-revision and stale-digest profile refusals (`unknown_profile`, `stale_dependency`) | FR-110, against the `DefinitionLock` catalog. |
| Conflicting selections of one profile identity (`ambiguous_declaration`/`conflicting-authority`) | FR-110: every selection but the exact `root` row refuses, so the non-matching one refuses `stale_dependency`. |
| Duplicate selection alias | FR-091-AC-22 (the assembler). |
| Model selection | I1 (FR-056) and `CheckedGraph::model_selections`. A `sha256:` compiled-model digest refuses `invalid_model_binding` at I1 (FR-056-CON-4). |
| An inadmissible parse | Spine `compile` refuses it at S1 (`CompileRefusal::Source`). |
| Definition closure over a caller-supplied `DefinitionCatalog`: dependency edges, `missing_import`, `definition-cycle` | Held (owner question on QSL-234). |
| Complete-V1 facet and 176-capability bundle (`CompleteBundle`, `feature-set-mismatch`, `unknown_required_feature`) | Held (owner question on QSL-234). |
| `PackageLimits` (definitions, dependency edges, depth, artifact bytes, single-artifact bytes) | Held (owner question on QSL-234). |
| Resolved-graph identity (`quire.complete.resolved-graph/2` `SemanticDigest`) | Held (owner question on QSL-234). |
| Compiled-model resolution (`ModelCatalog`, `ModelArtifact`) | Held (owner question on QSL-234). |

The held rows back QSpec FR-131-AC-1 to AC-3 and FR-339-AC-3 (QSpec
TC-180), which QSpec's `docs/v1-delivery-ticket-manifest.md` assigns to QSL
(V1-SRC-003 and V1-SRC-004; IT-011 adopts them). Their tests are the ones
tagged with those criteria in `tests/it/complete_package.rs` and
`complete::package_tests`. The spine's `DefinitionLock` catalog cannot take
them over as it stands: its rows carry no dependency edges, bytes or facets,
and it covers only the value facet of the nine FR-131 names. Which
component computes the FR-131 closure and bundle after
`resolve_source_package` retires is an open owner question on QSL-234.
Until it is ruled, the held rows, their inputs (`Definition`,
`DefinitionCatalog`, `ModelArtifact`, `ModelCatalog`, `ReaderAuthority`)
and their tests stay in place.

QSL SHALL remove `ResolvedSourcePackage`, `resolve_source_package` and
`command::resolve_parsed_source` in one change, made once FR-110 is
implemented (the dependency half already is: FR-099) and the held rows
have a ruled home. Each scenario of `tests/it/complete_package.rs` and
`complete::package_tests` is then backed against its successor for its row.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-087-CON-1 | This requirement does not implement the v2 emitter itself (`CheckedPackage` → `EmittedPackage` bytes, C-03) or the layer-4 `package` reader's byte-level parsing; those are ADR-011 T-8 (M-4, QSL-6/#242). This requirement's `package`-layer output is limited to the type definitions T-1 names and the private-constructor/public-accessor surface ADR-011 §4 requires, so that M-4 has a typestate to build against. | Design | Inspection |
| FR-087-CON-2 | This requirement does not implement O-08, O-09, O-10, O-11, O-14 or C-26; those are FR-088 (S-3b). A type this requirement defines (for example the checked type node C-26 converts) SHALL NOT be given clause-kind, qualified-name or type-descriptor behavior by this requirement. | Design | Inspection |
| FR-087-CON-3 | `PackageNodeKey`'s `node` component is a `WireNodeId`, never a `NodeKey`, at every point this requirement defines or reads it (R-10, O-04). A conversion function that constructs a `NodeKey` from a `PackageNodeKey`'s `node` field — at E4, at E9, or anywhere else — exceeds this requirement's scope and violates R-10; the only conforming behavior at E4 and E9 is a lookup against an already-minted `NodeKey` in a checked package, never a constructor call (ADR-013 O-04, `:173,175`). | Design | Test (TC-255) |
| FR-087-CON-4 | `ResolvedSourcePackage` is removed in one change, made once both of its successors exist: the spine's resolution of `import` selections against I2 import views (the dependency half, FR-099, ADR-011 M-4) and E3's resolution of `profile` and `model` selections against the `DefinitionLock` catalog and I1 (the header-selection half, FR-110), and once the disposition table's held rows have a ruled home. The removing change deletes `ResolvedSourcePackage`, `resolve_source_package` and `command::resolve_parsed_source`, and leaves no old name reachable: no feature flag, deprecation attribute or compatibility wrapper (no migration/fallback layer for prerelease software). | Process | Test (TC-246) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-087-AC-1 | Each of `CheckedGraph` (in `check`), `CheckedPackage` (in `package`), `EmittedPackage` (in `package`), `VerifiedPackage` and `ImportView` (both in `library`) has exactly one defining location, with every field and every constructor (the implicit struct-literal constructor, or an explicit `new`) private to that owning module, and every external read of the type's state goes through a named accessor. This is TC-171's own established claim (constructor/field privacy plus accessor-only reachability), extended to five types instead of three; TC-171 itself records why a `compile_fail` doctest cannot demonstrate it (a doctest links the crate externally, so it shows only crate-external inaccessibility, never that an in-crate sibling module — `value::expression` with respect to `check`'s types, or `check` with respect to `package`'s — cannot reach private state; and where a field is already module-private, not merely `pub(crate)`, a doctest's before/after comparison passes identically regardless of the module boundary this requirement draws, demonstrating nothing about it). This criterion is accordingly verified the same way TC-171 verifies its own claim: direct inspection of declared visibility and of the accessor surface, not a doctest. **Amended by QSL-181 (lead ruling R5, 2026-09-23).** `VerifiedPackage` and `ImportView` stay in layer-3 `library` with their fields and constructors private; `verify_binding` (the verified binding, `VerifiedPackage`'s only constructor) and the ADR-011 §4 condition-1 witness minter `SupportedV2Wire::attest_ir_admitted_v2` are `pub` so the layer-4 v2 reader can call them across the `qsl-semantics` crate boundary. The witness's field stays private (a `compile_fail` doctest in `library::witness`), so the minter is its only constructor, and arch-lint rule T12-E (FR-060) fails on any shipped call to the minter outside `qsl-package`'s `checked_v2` (QSL-182), the ADR-013 O-04 pattern T12-B applies to the `pub` kernel `NodeKey` constructor. | Inspection (TC-243) |
| FR-087-AC-2 | No conversion function exists from `ParsedSource` to `CheckedGraph` or `CheckedPackage` other than the S3 checker's own entry and the S4 link step's own entry; no conversion function exists from `VerifiedPackage`, `ImportView`, or any `protocol_artifact`-read value to `CheckedGraph` or `CheckedPackage`. A source scan of every function returning `CheckedGraph` or `CheckedPackage` confirms each is defined inside the S3 checker or the S4 link step (Inspection, primary evidence, since this rules out an in-crate bypass a doctest's external vantage cannot see); a `compile_fail` doctest per forbidden path (Behavior, "R-10"; TC-244), each naming a specific expected error code, additionally demonstrates the crate-external case. | Test (TC-244) |
| FR-087-AC-3 | `library` constructs a `VerifiedPackage` only when the three ADR-011 §4 verified-binding conditions hold, and refuses with a named cause and no partial output on: an unsupported schema version; a recomputed `package_id` that does not lexically equal the declared one; and an identity absent from the consumer's library lock or pinned request. A digest of the file bytes, the lock file, or the source is never accepted as a substitute for the recomputed `package_id`. | Test (TC-253) |
| FR-087-AC-4 | `library` converts a `VerifiedPackage` to an `ImportView` without resolving any name: `ImportView`'s exported-declaration data is keyed for `PackageNodeKey` lookup by the importing package's own checker, and no function in `library` takes a name (`QualifiedName` or bare string) and returns a declaration or node id after the exporting package's own check stage has run. | Test (TC-254) |
| FR-087-AC-5 | `PackageNodeKey` is defined exactly as `{package: package_id, node: WireNodeId}`; no second shape (`{package, node: NodeKey}`) exists anywhere in the crate. Two `PackageNodeKey` values compare equal iff both components compare lexically equal; a mutation test that swaps the equality implementation for a structural comparison over the referenced node's content fails this criterion's own adverse test. | Test (TC-245) |
| FR-087-AC-6 | No `NodeKey` constructor call anywhere in the crate is fed, directly or transitively, by a `PackageNodeKey`'s `WireNodeId` field, an `ImportView` entry, or any other wire-read value: `library`, `package`'s E4 dependency resolution, and `replay`'s E9 lookup each make zero `NodeKey`-constructor calls, resolving a `WireNodeId` to a `NodeKey` only by lookup against an already-minted `NodeKey` in an already-checked package (ADR-013 O-04, `:173,175`). This is scoped to what this requirement (S-3) owns: the crate-wide claim "only `check` calls the `NodeKey` constructor" is FR-060 T12-B's own allow-list, whose named debt list (FR-060 Behavior, "T12-B and T12-C: shipped code and debt lists") holds the real minting sites outside `check` this requirement does not touch or gate on — pre-existing debt this requirement neither fixes nor is blocked by (Remaining work: agent-ix/quire-spec-language#211). This criterion fails only if `library`, `package`'s E4 path, or `replay`'s E9 path gains a `NodeKey`-constructor call, or if any such call anywhere in the crate is fed by a `PackageNodeKey`'s `WireNodeId` field. This is a call-site and data-provenance policy (which module is permitted to mint, and what may feed a mint, not which shape a lookup returns), enforced by the FR-060 T-12 API-surface scan and a call-graph trace rather than by the type system, so it is verified by that scan and trace, not by a `compile_fail` doctest (a mismatched-type attempt at some other call site would fail identically regardless of location, proving nothing about it). | Test (TC-255) |
| FR-087-AC-7 | Once FR-110 is implemented (the dependency half is FR-099) and the held rows of Behavior's disposition table have a ruled home, `ResolvedSourcePackage`, `resolve_source_package` and `command::resolve_parsed_source` are absent from every crate of the workspace under every feature combination. Each scenario of `tests/it/complete_package.rs` and `complete::package_tests` is backed by a test against its row's successor (FR-099, FR-110, FR-091-AC-22, FR-056, or the ruled home of a held row), keeping its QSpec FR-131/FR-339 tags. A whole-workspace definition scan confirms the absence. | Test (TC-246) |
| FR-087-AC-8 | The canonical `EmittedPackage` (this requirement, layer-4 `package`) and `protocol_artifact::native::EmittedPackage` (SEAM-3, unrelated) remain two distinct types under two distinct module paths, with no `pub use` or re-export that would place both names into one import scope, and with no field, method, or shape shared between them (Description), for as long as the SEAM-3 type exists: ADR-011 §4 records that the two-public-types-named-`CheckedPackage` rule is met by *deleting* the native-v1 type with SEAM-1, not by permanent coexistence, and this requirement's own `CheckedPackage`/`checking::CheckedPackage<'a>` pair follows the same disposition (Description, item 2). The expiry condition for `EmittedPackage`'s two-module coexistence is SEAM-3's own deletion of `protocol_artifact::native::EmittedPackage` (ADR-011 §7.3), not an indefinite steady state this requirement asserts. | Inspection (TC-247) |
| FR-087-AC-9 | After this requirement's implementation, the `check`/`package` edge runs one way: `package` (layer 4) may import from `check` (layer 3), the layer-4 row's own direction (§6.1: "Depends on: 3, F, K"), and `check` imports nothing from `package` (the crate `qsl-package` since X-7) at either the `use`-line or the inline-path level (Cargo refuses the edge: `qsl-package` depends on `qsl-semantics`; this criterion reconfirms it). `check` defines no type, method, or field named `CheckedPackage`. `package`'s `CheckedPackage` is built by the S4 link step from a `CheckedGraph` and holds that `CheckedGraph` as a field (this package's own checked declarations) plus the S4-only checked dependency closure; it reaches check-owned state (`Node`, `Signature`, and the rest) through that field (`graph()` or named delegating accessors) and never re-declares, copies or re-derives a check-owned type in `package`. This design statement is kept because it is the S3-to-S4 typestate chain ADR-013 T-1 requires: S4's output is made from S3's output and nothing else, and every checked declaration has one owner. `check` imports from `library` only as layer 3's own order permits (`library` before `check` core); `check` reads `LibraryLock` only through its read-only accessors and never constructs or mutates one. `value::expression` (layer 5) imports `CheckedPackage` from `package` (`use qsl_package::CheckedPackage;`), the layer-5 row's own direction (§6.1: "Depends on: 4, 3, F, K"). **Amended by QSL-182 (X-7).** This sentence used to require the single closed re-export `pub use crate::checked_package::CheckedPackage;` in `value::expression`. Once `CheckedPackage` is in `qsl-package`, that line is a root-crate re-export of a moved item, which ADR-011 §7.2 and QSL-182's acceptance criteria forbid; they are the later and more specific rules, as QSL-181 ruled for FR-068-AC-10 (R3). The root crate re-exports no layer-crate item, and a `pub use` of one fails this criterion. This criterion demonstrates that the layering violation FR-068/QSL-167 recorded is closed, not relocated: a build in which `check` names `CheckedPackage`, imports from `package`, or in which `package`'s `CheckedPackage` holds anything other than a `CheckedGraph` for its own checked declarations, fails this criterion. **Amended by the layer-rule ruling (2026-09-22).** The number of `check` items `package` imports, and the number of `library` items `check` imports, are not bounded. A count of `package`'s `check` imports is met by a `package` that copies check-owned data into re-declared types, and fails a `package` that imports `Node` to name an accessor's return type, which is layer 4's permitted direction; the design statement above guards the property instead. Every `library` item is a permitted edge for `check`, because `library` precedes `check` core in §6.1's layer-3 order. That no `NodeKey` is minted from a `PackageNodeKey`'s `WireNodeId` or an `ImportView` entry is data provenance, held by FR-087-AC-6 and FR-060 T12-B. | Test (TC-256) |
| FR-087-AC-10 | `src/package/features.rs` and `src/package/view.rs` continue to import `CheckedPackage` from `crate::checking` (the lane-private `checking::CheckedPackage<'a>`), unchanged by this requirement's introduction of a canonical `CheckedPackage` elsewhere in `package`'s own module tree, for as long as `checking`'s lane exists. No file in `package` re-exports or glob-imports both the lane-private and the canonical `CheckedPackage` into one scope where a bare `CheckedPackage` reference would be ambiguous; a source scan of `features.rs` and `view.rs` confirms their `crate::checking::CheckedPackage` import is present and unrenamed while that lane exists, and a build with both names in one import scope (an ambiguity error) fails this criterion while the import is present. This two-name coexistence is not asserted as a permanent steady state: ADR-011 §8 (Q209-1) records that `checking`'s lane retires (lane A) or converges into its replacement (lane B/D) — "each other lane is deleted with its replacement" — and the expiry condition for `features.rs`/`view.rs`'s `crate::checking::CheckedPackage` import is that lane's own deletion, per its own convergence PR, not an indefinite state this criterion treats as final. After that deletion, this criterion no longer applies to those two files (there is only one `CheckedPackage` left to import). | Test (TC-247) |
| FR-087-AC-11 | `value::library` and `value::package_identity` no longer exist as modules under `value` after this requirement's implementation; `LibraryName`, `LibraryPackage`, `PackageId`, `ImportDeclaration`, `LibraryLock`, `resolve_libraries` and `package_identity`'s preimage-reading functions are reachable only from the new `library` module, unchanged in shape (Description, item 3, owner ruling (a), (e), (f)); `LibraryRefusal`'s own variant set is likewise unchanged, though two of its payloads are not: `PreimageDefect`, per the exception below, and `StaleDependency`'s `pin: StalePin`, which replaces `import: ImportDeclaration` so that a refusal from the §4 binding's condition 3 names the pinned lock entry and the selection the candidate presented rather than an import declaration that does not exist (`StalePin::Import` keeps `resolve_libraries`' import declaration); `value::library`'s `ExportIdentity{package, node: NodeKey}` and `resolve_name` do not relocate in their pre-move shape (owner ruling (b), (c)): `library` after relocation defines no type with a `node: NodeKey` field and no function resolving a name to a declaration or node id (`PackageNodeKey`, FR-087-AC-5, is the module's sole cross-package node reference). Every field or return value anywhere in `library` that is built from wire-read or preimage-read data is typed `WireNodeId`, never `NodeKey` (owner ruling (d)): `package_identity`'s `node_key`-equivalent function returns a `WireNodeId` and calls no `NodeKey` constructor; `PreimageDefect`'s `AmbiguousDeclaration.nodes` and `DeclarationNominalMismatch.node` fields are `WireNodeId`-typed; and `ProjectedDeclarations`'s internal map is keyed to `WireNodeId` values, not `NodeKey`. FR-307's acceptance criteria and TC-227's vectors pass against the relocated code, updated to these shapes. A whole-crate scan confirms no `NodeKey::from_hex`, `NodeKey::from_bytes`, `NodeKey::of` or other `NodeKey`-constructing call inside `library` or `package`, and no `NodeKey`-typed field, anywhere in `library`, that is populated from wire-read or preimage-read data — not only on a cross-package reference type, but on any type `library` defines, including `PreimageDefect` and `ProjectedDeclarations`. A definition of `value::library` or `value::package_identity` still present, an `ExportIdentity`-shaped type with a `NodeKey` field, a `resolve_name` function in `library`, a `NodeKey`-constructing call inside `library` or `package`, or a wire/preimage-fed `NodeKey`-typed field anywhere in `library`, each fails this criterion. ADR-015 D-2 and D-3 reshape two of these relocated types, as an exception to "unchanged in shape": `ImportDeclaration` holds its recorded digest as a `quire.package.semantic/v2` `DigestRecord`, and `LibraryName` is a non-empty string. | Test (TC-281) |
| FR-087-AC-12 | Every `LibraryRefusal` variant `resolve_libraries` (relocated into `library`) can return classifies to exactly one of: an ADR-011 I2 graph rule (`:203-210`), the §4 binding's condition 2, the §4 binding's condition 3, or E3 name resolution — with `DuplicatePackageId` named as the one stated exception, lying outside all four (Description, item 3, owner ruling (e), which gives the per-variant classification and DuplicatePackageId's own reasoning). Concretely: `PackageIdMismatch` classifies to condition 2 itself (the digest recomputation and comparison); `InvalidPreimage` and `UndeclaredExport` classify alongside condition 2 as per-package admission checks (raised by the same reused `verify_package`, FR-087-AC-3, but only after condition 2's own comparison has completed — neither is a prerequisite condition 2 needs to run); `StaleDependency{RevisionMismatch}` classifies to condition 3 (the identity is present; only the lock-recorded version disagrees); `InvalidQualifier` classifies to E3 name resolution (it moves conceptually with `resolve_name`, owner ruling (b)); `MissingImport` and `StaleDependency{ByteDigestMismatch}` classify to I2's first rule (missing or unlisted identity); `ConflictingDefinition` classifies to I2's second rule; `ImportCycle` classifies to I2's third rule; `DuplicatePackageId` is the stated exception (conflicting metadata over identical identity content, a supplied-pool precondition, not an I2/§4/E3 question). This criterion does not require `resolve_libraries` to gain a new refusal variant, and does not require force-fitting `DuplicatePackageId` into one of the four; it requires every variant to be classified, honestly, to one of the four or to the named exception, with no variant left unclassified or misclassified (for example, mapping `StaleDependency{RevisionMismatch}` to "missing or unlisted" rather than condition 3, or mapping `DuplicatePackageId` to I2's second rule, each fails this criterion). | Test (TC-282) |
| FR-087-AC-13 | The importing package's checker resolves imported names as Behavior, "E3 resolves an imported name in the importing package's checker", states. Given package `P` importing library `L@1` (which exports `R`) with no `as` qualifier, and declaring no `R` of its own, `P`'s checker refuses both `R` and `L::R` with `missing_declaration` / `missing-name`. Given `P` importing `A@1` and `B@1` both `as a`, each of two uses of `a::R` in `P` is refused `ambiguous_declaration` / `ambiguous-name`, and each refusal carries the loci of both imports. Given `P` importing `L@1` `as l`, `l::R` resolves to `PackageNodeKey{package: <L@1's package_id>, node: <R's WireNodeId>}`, and `l::Q`, which `L@1` does not export, is refused `missing_declaration` / `missing-name`. Given `P` importing `L@1` `as l`, checked against a library lock that holds no selection for `L`, `l::R` is refused `missing_declaration` / `missing-name`. | Test (TC-379) |
| FR-087-AC-14 | The E4 link step with dependencies (`CheckedPackage::link_with`) recomputes each imported package's `package_id` by emitting it and refuses one that differs from the `package_id` its import records with `DependencyIdentityMismatch` (`stale_dependency`), naming both; it records one selection per library identity over the direct imports and every dependency's own closure, and refuses two selections of one identity that differ in version or `package_id` with `invalid_package`/`conflicting-definition` (FR-307's diamond rule), naming both selections and both dependency paths. Two paths reaching one selection unify. A refusal yields no package. Binding an `Import`'s identity and version to the source's resolved import declarations, and FR-307's `revision-mismatch` check, are FR-099's (ADR-015 D-1). `CheckedPackage::dependencies` holds each direct import's checked package by `package_id`, and `CheckedPackage::dependency_selections` the closure in ascending UTF-8 byte order of identity. | Test (TC-253) |

## Dependencies

- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  §1 R-10, §3 O-15, §3.1 T-1 and T-3, §6 (lane-private table), §7 (the S-3
  slice row and its Gate).
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.1, §2.4 (lock evidence: where the header selections resolve), §4,
  §6.1, §6.2 (the `complete::package` row), §7.3 (M-4's row, which
  QSL-6/#242 implements against this requirement's typestate), §8 (Q209-1,
  Q209-2, lane C2).
- [US-005](../usecase/US-005-trust-checked-identity-across-packaging.md).
- [FR-060](FR-060-check-qsl-api-surface-boundary.md), whose T-12 API-surface
  check enforces the minting rule this requirement's `PackageNodeKey`
  depends on (only `check` calls the `NodeKey` constructor).
- [FR-068](FR-068-split-expression-checking-into-check-stage.md), which
  landed `check::CheckedPackage` and `PackageDeclarations::check`'s
  `CheckedPackage` return type (M-5), and opened QSL-167 for the ADR-011
  §4-versus-§6.1 conflict this requirement resolves per the QSL-158 owner
  ruling (Status): `PackageDeclarations::check` returns `CheckedGraph`
  after this requirement, and `check::CheckedPackage` (`qsl-semantics/src/check/mod.rs:127`)
  and its three checking methods (`qsl-semantics/src/check/mod.rs:601,625,648`) are
  relocated/retargeted as this requirement's Behavior and Outputs state.
- Linear QSL-6 (ADR-011 M-4: the I2 reader, successor to
  `ResolvedSourcePackage`'s dependency half) and QSL-234
  ([FR-110](FR-110-resolve-header-profile-selections-at-e3.md), successor
  to its header-selection half). FR-087-AC-7 and CON-4 wait on FR-110;
  QSL-269 owns the retirement (ruling on QSL-229).
- Linear QSL-158 (this requirement's owning ticket, the S-3a/S-3b split and
  the 2026-09-21 `CheckedPackage`-placement ruling) and QSL-167 (the ADR-011
  §4-versus-§6.1 defect, closed for its `CheckedPackage`-placement half by
  this ruling; its separate §6.2 refusal-row finding stays open).

## Status

Specified under QSL-158 (ADR-013 §7 S-3, split into S-3a/S-3b by the
2026-09-21 comment on that ticket). Partly implemented. Gate (ADR-013 §7:
"S-2, QC-10") is clear: S-2 landed as PR #260 (`97ec26e3`); QC-10 landed
under STD-2 (Done).

Landed: PR #304 added `CheckedGraph` (`check`), which
`PackageDeclarations::check` returns, and `CheckedPackage` and
`EmittedPackage` (layer-4 `package`). PR #299 relocated `value::library` and
`value::package_identity` into the top-level `library` module, where
`PackageNodeKey{package, node: WireNodeId}` is defined and wire-fed node
references are `WireNodeId`s, and added `LibraryRefusal::class()`. QSL-6
slice A1 (PR #340) added `VerifiedPackage`, `ImportView`
(`VerifiedPackage::into_import_view`, its only constructor) and the §4
verified binding, `library::verify_binding`. The binding
takes a condition-1 witness (`SupportedV2Wire`) that only the layer-4 v2
reader mints, in IR's `AdmittedV2` arm (since QSL-181 the minter and
`verify_binding` are `pub` for the crate boundary, and arch-lint rule T12-E
confines the minter's callers to that reader); it applies condition 2 through
`verify_package`, and condition 3 against a `PinnedRequest` (one selection
per identity, built from a `LibraryLock` or refused on a conflicting pin),
comparing both the `package_id` and the version (ruling (e)). The I2 reader
(`qsl-package`'s `checked_v2::read_checked_package_v2`) returns
`VerifiedPackage`. A binding refusal names the pinned selection and the one
the candidate presented, so `LibraryRefusal::StaleDependency` carries
`pin: StalePin` (an import declaration, or a pinned entry) in place of
`import: ImportDeclaration`; AC-11 lists this payload change.

Backed: FR-087-AC-3 (TC-253), FR-087-AC-4 (TC-254), FR-087-AC-5 (TC-245)
and FR-087-AC-12 (TC-282), and, below, AC-2 in full and AC-1, AC-6, AC-8,
AC-10 and AC-11 in part. TC-253's steps are backed as follows. Steps 1, 2, 4 and 8 run on
the wire path (`checked_v2` tests). Step 3 runs at the `library` level
(`library::binding_tests`), because on the wire path IR refuses a
non-recomputing `package_id` first and names it `stale_dependency`
(IR-238 item 3); `library` refuses it as `PackageIdMismatch`. Steps 5-7 also
run at the `library` level. Any wrong `package_id` refuses, so each of those
cases makes the substitute digest the value the candidate claims *and* the
pin selects; an implementation that trusted an agreeing claim and pin
instead of recomputing would admit it. A `syn` scan
(`tests/it/verified_binding_witness.rs`) fails if the condition-1 witness's
constructor is referenced anywhere but one call in the v2 reader's
`AdmittedV2` arm, including inside a macro; the witness's private field in
`library::witness` makes the compiler refuse any other construction.

FR-087-AC-4 (TC-254) is backed. `ImportView` holds its exports keyed by
`WireNodeId` and exposes them only as `(name, PackageNodeKey)` entries, in
node-id order; it has no method that takes a name. `library`'s admission
checks a package's export list against its declared names, and keeps only
the listed declarations, by membership alone
(`ProjectedDeclarations::undeclared`, which returns a name, and
`retain_exported`); neither maps a name to a node id. The checker builds
its own name index from the view's entries (`check::imports`) and resolves
a name there, refusing an unexported name `missing_declaration` /
`missing-name`. Steps 1-2 and 5 run on the wire path (`checked_v2` tests:
two exports whose node-id order is the reverse of their name order, and
the checker's index over them) and at the `library` level (two exports of
different node kinds, and a view holding only the listed export). Steps 3-4
are `qsl-semantics/tests/it/import_view_names.rs`, a `syn` scan of every `library` file.
It fails on any function taking a name and returning a `WireNodeId`,
`NodeKey`, `PackageNodeKey`, `ImportView`, a `library` type carrying one,
`Self::X` bound to one, or a generic built from one; on any name → node id
map (`BTreeMap`, `HashMap`, `IndexMap`, a reference to or an iterator over
one) as a return type, field or type alias; and on any `ImportView` method
taking a name. It covers trait default and required methods, traits
implemented for `ImportView`, generics bounded by `AsRef<str>`-like
traits, `str` aliases and renames, newtypes over a name, and signatures
inside macro tokens, each with a synthetic positive. It does not see a node
id returned as raw digest bytes, a hex `String` or a `usize` position; a
name passed as `&[u8]`; a closure without type annotations, including one
held in a `fn(&str) -> Option<WireNodeId>` variable; or a local variable's
type inside a function body.

For `VerifiedPackage` and `ImportView`, crate-external construction
is shown by `compile_fail` doctests: a struct literal of either type, and a
call to `verify_binding` with a hand-built candidate. The doctests show only
that a crate-external caller fails to compile; stable rustdoc does not check
a `compile_fail` block's error code.

QSL-158 backs AC-2 (TC-244) in full, and AC-1 (TC-243), AC-6 (TC-255),
AC-8 and AC-10 (TC-247) and AC-11 (TC-281) in part, with
`xtask::typestate_scan`. It is a `syn` scan of the shipped code of every
QSL crate, and `make ci` runs it. Shipped code excludes `#[cfg(test)]`
items and every file a `#[cfg(test)] mod` declares. The scan sees through
`use … as` renames and `type` aliases. Its own tests plant each evasion
it covers. The module doc lists what it does not see: macro bodies other
than for mints, out-parameters, and call graphs. Each tests.md row names
the steps its test backs.

- AC-1: each of the five stage-output types has one definition in the
  layer crates, in its owning file, with every field private and no type
  or const generic parameter. The only other namesakes are the two
  lane-private types AC-8 and AC-10 name. Accessor-only reads are not
  backed, because a private field stays readable by its module's child
  modules.
- AC-2, and AC-1's constructor half: only named constructors return an
  owned `CheckedGraph`, `CheckedPackage`, `EmittedPackage`,
  `VerifiedPackage` or `ImportView`. The named constructors are
  `PackageDeclarations::check`, `CheckedPackage::link`,
  `CheckedPackage::link_with`,
  `EmittedPackage::new`, `verify_binding` and
  `VerifiedPackage::into_import_view`, the two composing them
  (`qsl_package::read_import_view`, the I2 read of a library's own
  emission, and the spine's per-unit chain `Resolution::compile_unit`,
  ADR-015 D-1), `CheckedPackage::shared_graph` (a shared handle to the
  linked graph, ADR-015 D-5), plus the two lane-private producers. `compile_fail` doctests cover TC-244 rows 1 to 5. Rows 3 to 5
  (`VerifiedPackage`, `ImportView` and `protocol_artifact::AdmittedPackage`
  into checked typestate) fail with E0277. Stable rustdoc does not check
  that code, so each block is paired with one that must compile over the
  same names, and a renamed or moved item breaks the build.
- AC-6: no item in `library`, `qsl-package` or `qsl-replay` mints a
  `NodeKey`. Every mint is in `check`; T12-B's debt list is empty. No
  minting item names `WireNodeId`,
  `PackageNodeKey`, `ImportView` or `VerifiedPackage`. A wire value that
  reaches a mint through another function's call is not traced (TC-255
  steps 3 and 6).
- AC-8 and AC-10: the two `EmittedPackage`s are defined at their two
  paths with no field name in common. `src/package/features.rs` and
  `view.rs` import `crate::checking::CheckedPackage`. No root-crate file
  imports a canonical and a lane-private namesake together. Methods and
  trait impls are not compared.
- AC-11: `value::library` and `value::package_identity` are gone, no
  `ExportIdentity` exists, and `library` defines no `resolve_name` and no
  field or variant holding a `NodeKey`.

AC-7 (TC-246) is not delivered, and AC-13 (TC-379) is partly delivered:

- **AC-7 and CON-4.** Amended by the ruling on QSL-229 (2026-09-24) to
  follow ADR-011. They used to require every caller of
  `ResolvedSourcePackage` to move to `VerifiedPackage`/`ImportView`, but
  neither type resolves a definition or a compiled model. They now split
  `complete::resolve_source_package` into its two halves (Behavior,
  "`ResolvedSourcePackage` retires when both successors exist"). The
  dependency half exists: spine `compile`'s S4 source resolution binds each
  import against a library compiled from source and read into its
  `ImportView` (FR-099, ADR-015 D-1). The header-selection half is FR-110
  (amended 2026-09-26, QSL-234): E3 resolves header profiles against QSL's
  `DefinitionLock` catalog, which carries each row's digest from QSpec's
  `complete-value-lock.json`, and model declarations through I1. It needs
  no QSpec accessor (QSL-189 is Canceled, ADR-011 §2.4 amended
  2026-09-24). FR-110 is not implemented, so neither criterion is backed.
  Behavior's disposition table names each capability's successor; its
  held rows (the QSpec FR-131 closure and bundle) wait on an owner ruling
  on QSL-234. Until then `ResolvedSourcePackage`
  stays, reached through `command::resolve_parsed_source`
  (`src/command/source_package.rs`) and tested by
  `tests/it/complete_package.rs`. Remaining work: QSL-234 (FR-110 and the
  held-row ruling), then QSL-269 (the retirement).
- **AC-13.** E3's resolution rule is specified, and the QSpec schema defect
  that blocked it is fixed (QSpec STD-105, IR-287). E4 fills the dependency
  closure (`CheckedPackage::link_with`, AC-14), the emitter writes
  `dependency_selections`, and the I2 reader admits a non-empty one. The
  resolution against an `ImportView` (`check::imports::ImportedNames`)
  exists. Spine `compile`'s dependency input and the S4 source resolution
  (ADR-015 D-1, FR-099) and the typing of an imported name at E3
  (ADR-015 D-5) are implemented; TC-379 passes steps 1 and 3, and its steps
  2 and 4 are unreachable as written (see TC-379 Status).

AC-14 (TC-253) is backed by `qsl-package`'s
`e4_refuses_a_stale_dependency_and_a_conflicting_diamond`,
`a_diamond_selecting_one_package_unifies` and
`the_dependency_closure_is_written_in_the_lock_and_the_preimage`. Binding an
`Import`'s identity and version to the source's resolved import
declarations, and FR-307's `revision-mismatch` check, are FR-099's (ADR-015
D-1).

**Owner ruling on QSL-158 (2026-09-21): ADR-013 T-1 stands unamended.**
`CheckedPackage` is canonically layer-4 `package`; `check`'s S3 output is
`CheckedGraph`. The `check` → `package` reverse edge FR-068 avoided by
keeping `CheckedPackage` in `check` was an artifact of `CheckedGraph` not
yet existing, not a property the architecture requires: once this
requirement builds `CheckedGraph`, `check` produces it and names no
`CheckedPackage`, `package` constructs `CheckedPackage` from it (a
permitted 4-depends-on-3 edge), and `value::expression` names it through
one `pub use` (a permitted 5-depends-on-4 edge) — no reverse edge remains
(Description; Behavior, "`check` produces `CheckedGraph`; `package`
constructs `CheckedPackage`"; FR-087-AC-9). This closes the
`CheckedPackage`-placement half of QSL-167. QSL-167's other, unrelated
finding (a §6.2 refusal-row defect) is untouched by this requirement and
stays open under its own ticket.
