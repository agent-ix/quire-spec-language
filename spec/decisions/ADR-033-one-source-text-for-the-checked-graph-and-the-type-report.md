---
id: ADR-033
title: "One source text produces both the checked graph and the type report"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-035
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-049
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

Status: Proposed. It becomes Accepted on merge. The planner has ruled for the
option below and closed the decisions in Decision.

## Recommendation

**Adopt Option A+D: the complete-V1 grammar (`qsl-cst`, mapped by `qsl-forms`)
is the one source grammar, and the type report is produced by the S3 binding
phase and family checkers of ADR-011, from the same parse that feeds the
checker.** One `Source` is read once. One parse of it yields the `CheckedGraph`
and the `TypeReport`. The checked unit compares one digest on every side. The
composed linker and checker are not re-read over the new forms: each composed
family is deleted when that family's type phase moves into S3, as ADR-011
rules, and the label-only match is deleted when the producer lands.

Reasons:

1. ADR-011 rules the shape. Its M-6e row says: "SEAM-2: each composed family is
   deleted in the PR that lands its S3 family checker and S4 emission. The
   composed checker is deleted, not kept (owner ruling)." Its lane table
   (section 8, row "B composed") says: "B1 → S1 and S2. B2 to B4 → S3 binding
   phase. ... B6 and B7 → S3 family checkers." Type admission in S3, from the
   S1 and S2 output, is that ruling applied to the type report.
2. The dependency direction forbids the opposite choice. The root crate depends
   on `qsl-cst`, `qsl-forms` and `qsl-semantics`, so the layer crates cannot
   be re-based on the root crate's syntax.
3. The digest comparison is true by construction and still a real check. Both
   sides are built from one `Source`, and a graph or report from other bytes is
   refused.
4. It deletes the interim label match instead of keeping a second route.

The cost is the type solver: its binding and typing are written as S3 phases
over S2 forms, family by family. The slice is large; it is measured below.

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
- **Callers of the graph path.** The production path is the spine
  (`qsl-replay/src/spine/lifecycle.rs`): `parse_with_cancel`, `build_unit`,
  `assemble` and `check` over one `Source`, returning the public
  `spine::CheckedUnit` (the checked package, the unit's source and its
  libraries). Other non-test callers are `qsl-package` (emit) and the
  `population-fixtures` writer. `qsl-bench` and tests in `qsl-eval`,
  `qsl-forms`, `qsl-semantics`, `qsl-replay` and the root crate call
  `build_unit` or `assemble` directly.
- **Callers of the type path.** The `handoff-writer` producer
  (`src/protocol_artifact/handoff/writer.rs`) and the root
  `tests/it/composed_*.rs` files. Nothing outside tests calls both paths on one
  source, and the spine does not run the type path.
- **Two `CheckedUnit` types.** `spine::CheckedUnit` holds a checked package
  and its sources. The root crate's `checking::source_inventory::CheckedUnit`
  holds a graph, a type report and a formal identity, built by a label-only
  match; `qsl_inventory` lives beside it.
- **The grammars differ.** The shared header is `language "ix:native" edition
  "1-draft";`. After it, complete-V1 reads `profile <alias> = "<identity>";`: a
  definition resolves by identity alone, with no version or digest in the
  selection (`qsl-foundation` `selection`). The composed grammar reads `profile
  <alias> = "<identity>" version "<v>" digest "<d>";` and resolves the
  definition by identity, revision and digest. `import` and `model`
  declarations read the same in both. So `qsl_cst::parse` returns a recovered
  `ParsedSource` with an `invalid_syntax` diagnostic at `version` for the
  composed binding source, and `build_unit` refuses it
  (`FormsCause::RecoveringCst`).
- **The families.** At the time of this decision, S2 had dispatch entries for `function`, `type`, `record`,
  `tuple`, `enum`, `predicate`, `dimension`, `unit`, `invariant`, `pre`,
  `post`, `body`, `protocol` and `temporal` (`qsl-forms/src/dispatch.rs`). It
  had none for a `relation` or `hyper` clause, which `qsl-cst` parsed and S2
  refused with `NoDispatchEntry`. A `temporal` clause passes S2; S3 refuses it
  with `UnimplementedTemporalClause` (`unsupported_construct`) until its
  checker lands. The composed syntax has four declaration kinds (predicate,
  state, temporal, protocol); `relation` and `hyper` exist only in `qsl-cst`.
- **The digests cannot agree today.** The graph test fixture and the composed
  binding fixture are two different texts, so the bytes and the digests
  differ. The label-only `CheckedUnit::new` therefore compares authority and
  identity only.

## Options

| Option | Grammar that wins | Both stages from one text | Fits ADR-011 | Verdict |
| --- | --- | --- | --- | --- |
| A+D | complete-V1; type facts produced by the S3 binding phase and family checkers | yes | yes: B1 to S1/S2, B2 to B4 to S3, B6/B7 to family checkers, composed deleted per family | recommended |
| B | composed (root crate) | yes: S2 and S3 re-based on composed syntax | no; reverses the crate direction | rejected |
| C | none: one front end feeds two parsers or an adapter | no: a text the second parser cannot read, or a second tree | no; a dual grammar | rejected |
| A-only | complete-V1; composed linker and checker kept and re-read over S2 forms | yes | no: ADR-011 says the composed checker is "deleted, not kept" | rejected |

### Option A+D: complete-V1 grammar, type facts from S3

- *Wins:* complete-V1.
- *Graph path:* unchanged. The spine front end already reads one `Source`
  through `parse_with_cancel`, `build_unit`, `assemble` and `check`.
- *Type path:* binding and typing become S3 phases over the S2 forms of the
  same `ParsedUnit`. They resolve a profile alias by identity alone, as the
  graph side does, through `DefinitionLock::pinned` and the spine's
  `LockEvidence`. The type phase takes no caller definition inventory. The
  composed binder's per-profile revision and digest matching
  (`linking/composed/definitions.rs`) is deleted, not ported.
- *Order:* ADR-011 M-6e deletes one composed family per implementing PR. A
  family moves when its binding and typing run in S3 over S2 forms; the same
  PR deletes that family's composed linker, checker and emission. The last
  family's PR deletes the remainder, including `parse_native`,
  `parse_native_source`, `src/parser/composed*` and `src/syntax/composed*`.
  The historical `0-draft` parser in `src/parser.rs` (`parse`, `parse_source`)
  is not touched: ADR-011 keeps it with native `compile` until M-6c.
- *Files, over the whole sequence:* `src/linking/composed/**` and
  `src/checking/composed/**` (about 13,000 lines in total, deleted per family);
  `src/syntax/composed*`; the composed arms of `src/parser.rs` and
  `src/parser/composed*`; `src/protocol_artifact/handoff/writer.rs` (its
  namespace comes from the producer once its families have moved);
  `src/checking/source_inventory.rs`; the `parse_native*` re-exports in
  `src/lib.rs`.
- *Migration cost:* high. Composed `profile X = ".." version ".." digest ".."`
  lines appear in 18 source and test files (including format-string
  generators): `src/linking/composed/arena.rs`, `src/parser.rs`,
  `src/protocol_artifact/handoff/writer.rs`, twelve `tests/it` files,
  `tests/support/composed_types`, `tests/support/protocol_artifact` and
  `tests/fixtures/composed-choreography.native`; and in 40 files under
  `artifacts/compiled-protocol-v1` and `artifacts/compiled-protocol-v2`. A
  `model M = ".." version ".." digest ".."` line is complete-V1 and is not
  composed; only `profile` and `import` are identity-only. Each surviving
  fixture takes the identity-only profile spelling and a complete-V1 profile
  binding. A test that asserts the composed grammar's own shape
  (`composed_syntax`, `parser_differential`) is deleted with the parser it
  tests.
- *Fixtures and goldens:* the handoff snapshots under `artifacts/` and the
  `native_*_emission` goldens belong to the protocol-artifact lane that is
  deleted; they are not re-digested. Model digests, checked node identities
  and `package_id` do not depend on the profile line's spelling.
- *What the checked unit compares:* the single source digest. The selected
  formal source, the graph's `RawSourceRef` and the type report unit's
  `Source::reference()` are equal as whole references (authority, identity,
  digest).
- *The interim label match:* `UnitLabel`, `UnitLabelMismatch` and the
  label-only `CheckedUnit::new` are deleted when the producer lands.
- *One `CheckedUnit`:* the spine's `spine::CheckedUnit` is the one type. It
  carries the type report, the formal identity and the digest comparison. The
  type report is one-unit by construction, so a report that covers two units
  cannot be built. The inventory's own `CheckedUnit` is deleted; `qsl_inventory`
  lives in `qsl_replay` and reads `spine::CheckedUnit`, no longer in the root
  crate's `checking::source_inventory`.

### Option A-only: keep the composed linker and checker over S2 forms

Rejected. It re-reads `linking::composed` and `checking::composed` (about
13,000 lines) over `qsl_forms::ParsedUnit`, which keeps the composed checker
that ADR-011 says is "deleted, not kept (owner ruling)", and writes the
re-reading only to delete it at M-6e.

### Option B: the composed grammar is the single grammar

- *Wins:* the root crate's composed grammar.
- *Call paths:* `qsl-forms`, `qsl-semantics`, `qsl-package`, `qsl-eval`,
  `qsl-replay` and `qsl-route` would read the root crate's arena syntax. They
  cannot depend on the root crate, which depends on them (ADR-011 section 6.1
  keeps layer 1 a leaf), so the syntax would have to move down first.
- *Migration cost:* highest. It re-implements every S2 form and every S3
  family checker on the composed arena, then deletes the layer-1 CST, which
  ADR-011 makes the lossless authoring layer.
- *Rejected:* it reverses ADR-011 and the crate direction.

### Option C: one front end that feeds both parsers

- *Shape:* a function that takes one text and runs `qsl_cst::parse` for the
  graph and `parse_native_source` for the types, or that lowers a `ParsedUnit`
  to a `ComposedUnit` for the type phase.
- *Why it fails:* a text that satisfies both parsers must be written in the
  intersection of two grammars, which excludes each grammar's profile line
  from the other. A lowering from `ParsedUnit` to `ComposedUnit` is a bridge:
  a second tree kept only to feed the code ADR-011 deletes. Pre-release, a
  second grammar or a bridge is not built.
- *Rejected.*

## Decision

Option A+D is adopted: the complete-V1 grammar is the single source grammar,
the type report is produced by the S3 binding phase and family checkers of
ADR-011 from the same parse that feeds the checker, the composed linker and
checker are deleted family by family as each family's type phase moves (not
re-read over S2 forms), the checked unit compares whole `RawSourceRef`s, and no
compatibility layer, bridge or second grammar exists. FR-362 is the normative
text. These points follow from it and are decided.

- **Moved and unmoved families.** The producer admits a family only once its
  type phase runs in S3 over S2 forms. It refuses a construct of a family
  whose type phase has not moved with `unsupported_construct`, at the
  construct. A unit is therefore never read by two parsers. S2
  builds a form for a `relation` clause and a `hyper` clause over
  `behaviours`, S3 checks them, and a checked clause is refused only as
  `NotEmitted`, as a checked `temporal` clause is, until an S4 emission writes
  it. A `hyper` clause over a finite trace domain and a `relation` over names
  that are not model operations keep the finite reading of QSpec FR-191.
- **Profile selection.** A source selects a profile by identity alone
  (FR-110). A profile line that states `version` or `digest` after the
  identity is not the single grammar and is refused. The type phase resolves
  profiles through `DefinitionLock::pinned` and the spine's `LockEvidence`;
  there is no caller definition inventory parameter, and the source never
  carries a revision or digest.
- **Refusal and exhaustion.** A unit the S3 type phase refuses is an error
  outcome of `check` and yields no partial inventory rows. The exhaustion stops
  `TypeWorkExhausted` and `TypeAdmissionUnfinished` are a different case: they
  stay `Incomplete` with the retained partial rows and the typed cause
  (IR FR-052-AC-10). The inventory's identity binds the whole `RawSourceRef`;
  the row identity encoding belongs to the owning FR.
- **One checked unit.** `spine::CheckedUnit` is extended; the inventory's own
  `CheckedUnit` is deleted.
- **Composed requirements.** FR-035, FR-036, FR-040 and FR-049 stay normative
  for the families that have not moved. The PR that moves a family retires or
  amends the parts of those requirements, and of their test cases, that name
  that family; retiring the rest of the composed lane is open work tracked as
  QSL-868. This record
  amends none of them.

The implementing slice is sequenced after the supplied-budget and union slices.

## Consequences

- One `Source` and one digest name the unit on every side of the checked unit.
- The composed profile spelling and the composed parser disappear family by
  family; the remaining retirement is open work tracked as QSL-868.
- The interim label match has no successor and no flag.
- The owner-issued source inventory cannot be built from a graph and a type
  report that do not describe the same bytes.
- FR-035, FR-036, FR-040 and FR-049 keep describing the composed lane until
  each family's PR retires its part, so the two bodies of text apply to
  disjoint families at every point.
  The composed lane (`src/linking/composed`, `src/checking/composed`,
  `src/parser/composed*`, `src/syntax/composed*`) is still present; its
  retirement is open work tracked as QSL-868.

## Alternatives Considered

- **Compare the digest of the graph's source with the type report's and
  accept that fixtures use two texts.** Rejected: the comparison would fail for
  every real source, since the two stages cannot read one text.
- **A shared text with a pre-pass that strips the composed profile suffix.**
  Rejected as Option C: a second reading of the text in a grammar the
  complete-V1 parser does not define.
- **Keep the label match and add a digest check beside it.** Rejected: a
  label-only route stays reachable, and the guarantee is the weaker one.
- **A second `CheckedUnit` beside the spine's.** Rejected: two public types
  for one concept.

## References

- ADR-011 (stage DAG; SEAM-2; M-6e) and ADR-013 (source references).
- FR-362 (the requirement this record proposes), FR-035, FR-036, FR-040,
  FR-049, FR-087, FR-091, FR-110.
