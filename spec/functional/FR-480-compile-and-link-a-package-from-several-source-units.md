---
id: FR-480
title: "Compile and link one checked package from several source units"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/US-014
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-033
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-323
    type: depends_on
---
# FR-480: Compile and link one checked package from several source units

## Description

QSL SHALL compile one checked package from an ordered, non-empty list of
complete-V1 source units, the package's **unit list**, and link their
declarations into one package namespace, as QSpec FR-151 and FR-322 define a
package of linked source units. Each unit is read once and parsed once with
the one grammar (ADR-033, FR-362). Its declarations are owned by that unit's
`SourceOwner`. A declaration of any unit is visible by name in every unit,
and the unit's `model` and `import` aliases stay local to it. A unit list of
one source is the one-unit compile every other requirement describes.

## Use case

An author splits a specification into a file of types and operations and a
file of their contracts. Both files compile into one checked package, a
contract in the second file names the first file's operation by name, and
the package means the same whichever order the two files are listed in.

## Inputs

- The unit list: one `Source` per unit (authority and identity, FR-001, with the
  exact bytes), in **request
  order**, the order in which the caller lists them.
- The domain package documents, dependency input (FR-099) and lock evidence
  the units' selections need, as for one unit.
- FR-278's library operations take the unit list as follows: `parse` runs
  once per unit and returns that unit's `ParsedSource`; `select` takes the
  model selections of every unit; `check` takes every unit's
  `ParsedSource`, in request order, with the `AdmittedModels`, dependency
  input and lock evidence. FR-027's native-compile/1 `1-draft` request names
  the unit list as its `program`: one source selection, or a non-empty array
  of source selections in request order (FR-027 states the command's
  edition, intake and refusal rules for each unit).

## Outputs

- One `CheckedPackage` whose `quire.checked-package/v2` lock lists every
  unit in `lock.sources`, with one checked unit (FR-362) per source unit.
- On refusal: one refusal with the stage, code, cause and locus this
  requirement or the one-unit rules give. Every locus names its unit's
  `RawSourceRef` (FR-096). No partial package is produced.

## Behavior

### The unit list

- The compiler SHALL refuse an empty unit list with
  `invalid_package`/`invalid-value`, and two units with one (authority,
  identity) with `invalid_package`/`conflicting-definition`, naming both
  list positions. Both refusals happen before any unit is parsed.
- The compiler SHALL parse each unit once through S1 and S2 under that
  unit's own FR-001 labels and limits, in request order. A unit's S1 or S2
  refusal refuses the package, and the first such refusal in request order
  is reported.
- **Source order** is request order, then the byte offset within a unit. It
  is QSpec FR-142's source order. It orders refusal reports, FR-091's "in
  source order" lists and FR-142's turn list. It never orders the admitted
  package: see "Ownership, identity and emission".

### Headers

- Every unit SHALL declare the same `language` and `edition`, and the same
  profile selections, as definition identities in the same order. A unit
  whose header differs from the first unit's SHALL be refused with
  `invalid_package`/`conflicting-definition` at that unit's header, naming
  both headers. Profile aliases are local to their unit, so two units may
  spell one profile's alias differently.
- This check runs in the spine compile (FR-278's `check` over the unit
  list), after every unit has parsed. Through FR-027's command, the per-unit
  edition check of FR-027 runs first, before any unit reaches the spine, so a
  command request that mixes `1-draft` and `0-draft` units refuses
  `unknown_edition` there and never reaches this check.
- FR-110's lock rows are therefore one set for the package: each unit gives
  the same `definition_selections`.

### Selections and imports

- A `model` or `import` alias SHALL resolve only within the unit that
  declares it (QSpec FR-151-AC-16). Two units may bind one alias to two
  model identities, or two aliases to one identity, and alias uniqueness
  applies within each unit.
- `model_selections` SHALL hold one entry per selected domain package
  identity over every unit (FR-110). Two units that select one identity
  under two versions SHALL be refused with
  `invalid_package`/`conflicting-definition`, naming both selections.
- `dependency_selections` SHALL hold one entry per directly imported
  library identity over every unit, with FR-099's closure and identity rules
  unchanged. Two units that import one identity resolved to two
  `package_id`s are refused as FR-099 refuses two closures that disagree.

### Names

- The native declaration names of every unit form one package namespace:
  types, enums, unions, aliases, functions, predicates, operations,
  `operation-body` declarations, clauses, dimensions, units and every other
  declaration a unit's top level holds. A declaration SHALL be visible by
  its bare name in every unit, whether its unit comes before or after the
  referring unit in request order.
- A name declared in two units SHALL be refused as two declarations of it in
  one unit are, `ambiguous_declaration`/`ambiguous-name` (FR-091; QSpec
  FR-151 "native declaration names"), naming both declarations, each with
  its unit.
- An `import` names a library of the dependency input (FR-099), never a
  unit of the package. A unit reaches a sibling unit's declarations through
  the package namespace, never through an alias.
- If the dependency input supplies a library whose source has the
  (authority, identity) of a unit of the package, then the compiler SHALL
  refuse with `invalid_package`/`conflicting-definition`, naming the unit
  and the library entry, before any unit is parsed: one source cannot be
  both a unit of the package and a library it imports. An `import` naming
  a library the dependency input does not supply is refused as FR-099
  refuses it (`missing_import`/`missing-selection`).
- Cycles across units (alias cycles, definition and call-graph cycles,
  FR-142 dimension and unit cycles) are refused as the same cycle within one
  unit is, with the cycle's edges named in source order.

### Linking across units

- The S3 checker SHALL link over the declarations of every unit together.
  FR-083 dispatch, FR-302's one-body rule ("across all source units linked
  into one package"), redefinition chains and FR-104's resolution of a
  clause to its operation all read the whole namespace.
- A member's own `pre` and `post` clauses SHALL be gathered from every unit
  whose clauses resolve to that member's operation identity, after each
  unit's local alias is resolved. `view_clauses` and FR-120's expansion
  conjoin them in ascending clause name under FR-006's short-circuit, so
  neither the unit that declares a clause nor the request order changes an
  outcome (QSpec FR-151-AC-19, TC-196 D10).
- FR-091's assembler SHALL build one `PackageDeclarations` from every unit's
  parsed unit, each declaration keyed under its own unit's `SourceOwner`.
  The unit-graph topology check takes QSpec FR-142's turn list over the
  whole unit list.

### Ownership, identity and emission

- Each declared structural node SHALL carry the `SourceOwner` (authority,
  identity) of the unit that declares it (QSpec FR-322-AC-51). A node's key
  therefore depends on its unit: moving a declaration to another unit
  changes its node id and the `package_id`, as QSpec FR-322-AC-53 states for
  two owners.
- The admitted package SHALL NOT depend on request order. `lock.sources`
  lists every unit's `RawSourceRef` in ascending UTF-8 byte order of
  (`authority`, `identity`), the **canonical unit order**. The emitter
  writes the graph as the one-unit emission writes it, taking the units in
  canonical unit order and writing a node shared by several units (an
  application-keyed node, a builtin, a model-owned node) once, at its first
  position.
- Occurrences and their ordinals SHALL be numbered across units in
  canonical unit order: the occurrences recorded in an earlier unit of that
  order come first, and within one unit FR-093's insertion order and
  FR-092's source-position order apply unchanged. A node shared by several
  units therefore has one sequence of occurrences and `source_map` rows,
  whatever the request order. Refusals keep source order.
- Reordering the unit list therefore leaves the package bytes and the
  `package_id` unchanged.
- Each declared node's `declaration` occurrence regions SHALL name its own
  unit's source in the source map (QSpec FR-322-AC-52).
- The spine SHALL return one checked unit per source unit. Each unit carries
  that source's one `RawSourceRef`; its graph and type report are outputs of
  that source unit's parse, as FR-362 defines. The package graph lists every
  unit reference in `lock.sources`.
- The emitted source provision (FR-278 `package`) SHALL carry every unit's
  source bytes under their source identities.

### Replay and import of a several-unit package

- A package reference's `sources` (QSpec FR-323) names the package's
  `lock.sources`: one or more `quire.source.bytes/v1` sources. QSpec states
  no order for that list, so the FR-098 executor SHALL accept it in any
  order and recompile it as one unit list in canonical unit order, which
  gives the package that `lock.sources` describes. A `dependencies` entry's
  `sources` likewise lists that library's units.
- FR-098 SHALL refuse, before any recompile, a `sources` list that is empty,
  names a definition document or repeats one (authority, identity), with
  `invalid_package`/`invalid-value` at `/package/sources` (or the entry's
  `sources`).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-480-AC-1 | QSpec TC-196 D10 split across two units: unit `U1` holds D01, `PX` and `pre PA`; unit `U2` holds `pre PB1` and `pre PB2`, both on `M::B::size` through `U2`'s own `model M` alias. Evaluating `I` for `d1` gives D10's first result (`undefined`, reason `absent-key`, with `PB2` and `PA` not evaluated). With the two bodies' names swapped it gives D10's second result (`undefined`, reason `precondition-false`, after the false clause and `PA`, the lookup clause neither evaluated nor charged). Each result is equal, charges included, for the unit lists `[U1, U2]` and `[U2, U1]`, and so is each result with `PB1` in `U1` and `PB2` in `U2`, again in both link orders. Each placement gives byte-equal packages with one `package_id` in both orders, and `lock.sources` lists `U1` and `U2` in canonical unit order. The body node of `false`, shared by `pre PA` in `U1` and `pre PB2` in `U2`, has its `expression` occurrences numbered `U1`'s first in both orders, with equal `source_map` rows. | Test |
| FR-480-AC-2 | Unit `A` declares `record Point { x: Integer; }` and unit `B` declares `function px using v(p: Point): Integer pure { p.x }`. The list `[B, A]` checks, and `px`'s parameter type is `A`'s `Point` node. With `record Point { x: Integer; }` also declared in `B`, the compile refuses `ambiguous_declaration`/`ambiguous-name`, naming both declarations with their units. A `B` that writes `import A` refuses as an import of a library that the dependency input does not supply (FR-099). With a dependency input that also supplies a library whose source has `A`'s (authority, identity), the compile refuses `invalid_package`/`conflicting-definition` before any parse, naming the unit `A` and the library entry. | Test |
| FR-480-AC-3 | QSpec FR-151-AC-16: units `A` and `B` select one domain package as `model M` and `model N` respectively, and each declares an `operation-body` for a different operation of it; the package links. When `B`'s body targets `A`'s operation instead, the compile refuses `conflicting_declaration`/`duplicate-declaration` (FR-302) with no partial package. Two units selecting that package's identity under two versions refuse `invalid_package`/`conflicting-definition`, naming both selections. | Test |
| FR-480-AC-4 | Through the spine compile (FR-278's composition over the unit list, not FR-027's command), two `1-draft` units whose headers select the same two profiles in different orders refuse `invalid_package`/`conflicting-definition` at the later unit's header, naming both headers, and so do two units selecting different profile sets; two units spelling one profile's alias `v` and `w` check. An empty unit list refuses `invalid_package`/`invalid-value`, and two sources with one (authority, identity) refuse `invalid_package`/`conflicting-definition` before any parse. | Test |
| FR-480-AC-5 | AC-2's package with `Point` moved from `A` to `B`, and `B` keeping `px`, has a `Point` node whose `owner` is `B`'s `SourceOwner`, a different `Point` node id and a different `package_id`. A source-map `declaration` region of `Point` names `B`'s source. | Test |
| FR-480-AC-6 | QSpec TC-187 U11q through QSL: document 1 declares `dimension T;` and `unit z : T = rational(0, 1) * z2; unit z2 : T = rational(1, 1);`, and document 2 declares `dimension L;` and `unit u_self : L = rational(1, 1) * u_self;`. The list `[1, 2]` refuses row 1 at `z`, and `[2, 1]` refuses row 6 at `u_self` (FR-091, FR-142 source order). | Test |
| FR-480-AC-7 | AC-1's package replays: an FR-098 request whose package reference lists `U1` and `U2` in canonical unit order, with both sources in the byte provision, recompiles to the same `package_id` and replays the D10 call. The same reference with the two sources reversed recompiles to the same `package_id` and replays; with one source repeated it refuses before any recompile, `invalid_package`/`invalid-value` at `/package/sources`. With `U2` dropped, `U1` alone recompiles to another `package_id`, and the executor refuses it on FR-098's `package_id` comparison (`stale_dependency`/`content-mismatch`). A reference naming a definition document refuses before any recompile. | Test |
| FR-480-AC-8 | The spine returns two checked units for AC-2's package, each carrying its own source reference and a type report produced from that source unit's parse. The API does not accept an independently supplied type report or source for rebinding. | Test |

## Dependencies

- QSpec FR-151 (linked source units, native declaration names, the
  one-body rule across units), FR-322 (`lock.sources`, `SourceOwner`, graph
  order and `package_id`), FR-323 (package reference `sources`), FR-142
  (source order of several documents), TC-196 D10 and TC-187 U11q.
- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md),
  [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md),
  [FR-098](FR-098-execute-a-replay-request.md),
  [FR-099](FR-099-compile-against-supplied-libraries.md),
  [FR-110](FR-110-resolve-header-profile-selections-at-e3.md),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-278](FR-278-parse-select-check-and-package-as-library-operations.md),
  [FR-302](FR-302-link-dispatch-during-unit-compile.md),
  [FR-362](FR-362-bind-a-checked-unit-to-its-source-by-digest.md), ADR-033.

## References

- Linear STD-384 (spec gap for QSL-939).
