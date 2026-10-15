---
id: ADR-033
title: "One source text produces both the checked graph and the type report"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: relates_to
---
# ADR-033: One source text produces both the checked graph and the type report

Status: Proposed. It becomes Accepted on merge. The planner has ruled for Option A and closed the two decisions below.

## Recommendation

**Adopt Option A: the complete-V1 grammar (`qsl-cst`, mapped by `qsl-forms`) is
the one source grammar, and the type phase reads that grammar's output.** One
`Source` is read once. One parse of it feeds the assembler and checker, which
produce the `CheckedGraph`, and the binding and type phase, which produces the
`TypeReport`. The checked unit then compares one digest on every side. The
root crate's composed grammar and its parser are deleted in the same slice that
lands the producer.

Reasons:

1. ADR-011 already rules it. SEAM-2 (`syntax::composed`, `parser::composed`,
   `linking::composed`, `checking::composed`) is deleted by owner ruling, and
   B1 converges on S1 and S2. Option A is that ruling applied to the type
   phase. Options B and C contradict it.
2. The dependency direction forbids B. The root crate depends on `qsl-cst`,
   `qsl-forms` and `qsl-semantics`, so the layer crates cannot be re-based on
   the root crate's syntax.
3. A makes the digest comparison true by construction and still a real check.
   Both sides are built from one `Source`, so the digests agree for the
   producer's own output, and a graph or report from other bytes is refused.
4. It deletes the interim label match instead of keeping a second route.

The cost of A is the type phase: it is re-read over S2 forms, which touches
every consumer of the composed namespace (see the Option A cost below). The
slice is large. The rest of this record gives the measurements and the
alternatives.

## Context

Measured on main when this was written.

- **The graph path.** `qsl_cst::parse` reads the complete-V1 grammar.
  `qsl_forms::build_unit` maps the CST to a `ParsedUnit`.
  `PackageDeclarations::assemble` takes that unit and a `RawSourceRef`, and
  `check` returns the `CheckedGraph`, which carries the `RawSourceRef`
  (authority, identity, digest).
- **The type path.** `parse_native_source` (root crate, `src/parser.rs`) reads
  the composed grammar into a `ComposedUnit`. `linking::admit_namespace`
  checks it against an explicit `SourceInventory` whose entries carry a
  digest. `binding::bind` builds the `binding::Report`.
  `checking::composed::admit_types` returns the `TypeReport`. Each namespace
  unit holds its own `Source` and `Source::reference()`.
- **Callers.** The graph path is called by `qsl-package` (emit), the
  `population-fixtures` writer and tests in `qsl-eval`, `qsl-forms`,
  `qsl-semantics`, `qsl-replay` and the root crate. The type path is called by
  the `handoff-writer` producer (`src/protocol_artifact/handoff/writer.rs`)
  and by the root `tests/it/composed_*.rs` files. Nothing outside tests calls
  both on one source.
- **The grammars differ.** The shared header is `language "ix:native" edition
  "1-draft";`. After it, complete-V1 reads `profile <alias> = "<identity>";`: a
  definition resolves by identity alone, with no version or digest in the
  selection (`qsl-foundation` `selection`). The composed grammar reads `profile
  <alias> = "<identity>" version "<v>" digest "<d>";` and resolves the
  definition by identity, revision and digest. `import` and `model`
  declarations read the same in both (`model <alias> = "<id>" version "<v>"
  digest "<d>";`). So the composed binding source is refused by `build_unit`
  with `FormsRefusal` (`RecoveringCst`), expecting `;` after the identity.
- **The digests cannot agree today.** The graph test fixture and the composed
  binding fixture are two different texts, so the bytes and the digests
  differ. The label-only `CheckedUnit::new` therefore compares authority and
  identity only.

## Options

| Option | Grammar that wins | Both stages from one text | Fits ADR-011 | Verdict |
| --- | --- | --- | --- | --- |
| A | complete-V1 (`qsl-cst`, `qsl-forms`) | yes: type phase reads S2 forms | yes (SEAM-2 deleted) | recommended |
| B | composed (root crate) | yes: S2 and S3 re-based on composed syntax | no; reverses the crate direction | rejected |
| C | none: one front end feeds two parsers or an adapter | no: a text the second parser cannot read, or a second tree | no; a dual grammar | rejected |
| D | complete-V1 | yes: type facts become output of the S3 checker | yes, as the end state | A's destination, not a separate slice |

### Option A: complete-V1 is the single grammar

- *Wins:* complete-V1.
- *Call path 1 (graph):* unchanged except that `qsl_cst::parse` and
  `build_unit` now serve the producer. The graph is built from the producer's
  `Source`.
- *Call path 2 (types):* `linking::composed` (inventory, arena, scopes,
  definitions, binding) and `checking::composed` (solver, sources, proofs)
  read `qsl_forms::ParsedUnit` instead of `ComposedUnit`. `admit_namespace`
  stops parsing: it takes the producer's parsed units and still checks
  them against the expected inventory. `binding::bind` resolves a profile
  alias by identity alone, like the graph side, and the composed binder's
  per-profile revision and digest matching (`definitions.rs`) is deleted.
- *Files:* `src/parser.rs` and `src/parser/composed*`, `src/syntax/composed*`
  (deleted); `src/linking/composed/**`, `src/checking/composed/**` (re-read
  over S2 forms; about 15,000 lines); `src/protocol_artifact/handoff/writer.rs`
  (builds its namespace from the producer); `src/checking/source_inventory.rs`
  (see below); the root `lib.rs` re-exports `parse_native*`.
- *Migration cost:* high. About 29 files under `tests/`, `src/` and `examples/`
  spell a composed profile line (`git grep 'profile .*= .*version'`), among
  them `tests/support/composed_types/mod.rs`, the `composed_*` and `native_*`
  test files, `tests/fixtures/composed-choreography.native`,
  `examples/protocol-handoff` and the handoff recipe inputs. Each profile
  line becomes the identity-only spelling. A test that asserts the composed
  grammar's own shape (`composed_syntax`, `parser_differential`) is deleted with
  the parser it tests.
- *Fixtures, goldens, identities:* a source's bytes change wherever the profile
  line changes, so every source digest in a golden or expected inventory
  changes with it. A compiled-protocol artifact's source references change
  for the same reason. Model digests, checked node identities and
  `package_id` do not depend on the profile line's spelling and do not change;
  the author confirms this with the existing identity goldens before merging.
- *What the checked unit compares:* the single source digest. The selected
  formal source, the graph's `RawSourceRef` and the type report unit's
  `Source::reference()` are equal as whole references (authority, identity,
  digest).
- *The interim label match:* `UnitLabel`, `UnitLabelMismatch` and the
  label-only `CheckedUnit::new` are deleted in the same slice, and the unit
  is built by the producer or by a constructor that compares digests.

### Option B: the composed grammar is the single grammar

- *Wins:* the root crate's composed grammar.
- *Call paths:* `qsl-forms`, `qsl-semantics`, `qsl-package`, `qsl-eval`,
  `qsl-replay` and `qsl-route` would read the root crate's arena syntax. They
  cannot depend on the root crate, which depends on them (ADR-011 section 6.1
  keeps layer 1 a leaf), so the syntax would have to move down first.
- *Migration cost:* highest. It re-implements every S2 form and every S3
  family checker on the composed arena, then deletes the layer-1 CST, which
  ADR-011 makes the lossless authoring layer.
- *Fixtures:* every S1, S2 and S3 test and golden changes.
- *Rejected:* it reverses ADR-011's SEAM-2 ruling and the crate direction.

### Option C: one front end that feeds both parsers

- *Shape:* a function that takes one text and runs `qsl_cst::parse` for the
  graph and `parse_native_source` for the types, or that lowers a `ParsedUnit`
  to a `ComposedUnit` for the type phase.
- *Why it fails:* a text that satisfies both parsers must be written in the
  intersection of two grammars, which today excludes the composed profile line
  and the complete-V1 profile line each from the other. A lowering from
  `ParsedUnit` to `ComposedUnit` is a bridge: a second tree kept only to feed
  the code ADR-011 deletes. Pre-release, a second grammar or a bridge is not
  built; the one path is chosen and the other deleted in the same slice.
- *Rejected.*

### Option D: the S3 checker also produces the type facts

- *Shape:* ADR-011 maps B2 to B4 into the S3 binding phase. Taken to the end,
  the pending obligations and type dispositions become output of
  `PackageDeclarations::check`, the `TypeReport` stops being a separate stage,
  and `CheckedUnit` reduces to the graph and its source.
- *Cost:* the largest rewrite of the type solver, done family by family under
  ADR-011 M-6e.
- *Relation to A:* A reads S2 forms and so moves toward D. A does not need D
  to finish the single-source guarantee. If the type phase is written as a
  new S3 phase over S2 forms from the start, A and D are the same work for the
  families the inventory covers.

## Decision

Option A is adopted: the complete-V1 grammar is the single source grammar, the
composed lane is deleted, the checked unit compares whole `RawSourceRef`s, and
no compatibility layer, bridge or second grammar exists. FR-362 is the normative
text. Two points follow from it and are decided.

- **Moved and unmoved families.** ADR-011 deletes the composed lane one family
  at a time. The producer admits a family only once its type phase reads S2
  forms. It refuses a construct of a family whose type phase has not moved with
  `unsupported_construct`, at the construct. A unit is therefore never read by
  two parsers.
- **Profile selection.** A source selects a profile by identity alone (FR-110).
  A profile line that states `version` or `digest` after the identity is not
  the single grammar and is refused. Any pin of a definition's revision and
  digest belongs to the caller's definition inventory, which is already an
  explicit input of the binding phase.

The implementing slice is sequenced after the supplied-budget and union slices.

## Consequences

- One `Source` and one digest name the unit on every side of the checked unit.
- The composed profile spelling and the composed parser disappear.
- The interim label match has no successor and no flag.
- The owner-issued source inventory cannot be built from a graph and a type
  report that do not describe the same bytes.

## Alternatives Considered

- **Compare the digest of the graph's source with the type report's and
  accept that fixtures use two texts.** Rejected: the comparison would fail for
  every real source, since the two stages cannot read one text.
- **A shared text with a pre-pass that strips the composed profile suffix.**
  Rejected as Option C: a second reading of the text in a grammar the
  complete-V1 parser does not define.
- **Keep the label match and add a digest check beside it.** Rejected: a
  label-only route stays reachable, and the guarantee is the weaker one.

## References

- ADR-011 (stage DAG; SEAM-2; M-6e) and ADR-013 (source references).
- FR-362 (the requirement this record proposes), FR-087, FR-091, FR-110.
