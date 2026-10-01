---
id: ADR-032
title: "Checked-input and duplicate-canonical-public-type gates"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-059
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-061
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: depends_on
---
# ADR-032: Checked-input and duplicate-canonical-public-type gates

## Status

Proposed, 2026-10-01. Owner rulings on the four design questions are
recorded under Rulings and folded into the rules below. FR-270 to FR-273
implement this record. The owning ticket and related work are listed under
References.

Item ids `CK-`, `DT-` and `R-` are local to this record. Other artifacts cite
them as `ADR-032 CK-3`.

## Context

ADR-011 §4 states two rules that no running gate checks as a whole.

- **Checking precedes lowering.** S5, S6a and S6b admit only S4 outputs, and
  there is no path from S2 to S5, S6a or S6b. Stage-output constructors are
  private to their stage modules.
- **One type per concept.** Exactly one QSL type carries each stage output,
  and ADR-013 §3 names one canonical owner and one public type for each
  shared concept.

Parts of both are already enforced:

- The S3 and S4 output types have private fields, one owner file each, and
  only the named stage constructors return them (FR-087, TC-243 and TC-244 in
  `xtask/src/typestate_scan.rs`). `compile_fail` tests show that code outside
  the stage module cannot build them.
- `xtask/src/definition_scan.rs` resolves "exactly one defining location"
  claims for named items across the layer crates and `quire-semantic-value`.
  It holds its names and a namesake list as test data.
- FR-059 to FR-061 (`arch-lint`) check crate direction, the API surface of
  four constructors and duplicate lock revisions. FR-064 (`xtask
  string-edge`) checks string dispatch.

The parts that are missing: no check that an execution, lowering or proof
entry point names no pre-check representation in its signature and calls no
pre-check stage; no check that a canonical public type has one definition
across the workspace and the ecosystem crates it builds against; and no
canonical set that a scan can read from code.

Measured on main at the time of writing:

- `qsl-eval` and `qsl-route` name no `qsl-cst`, `qsl-forms` or
  `qsl-source` item and no `PackageDeclarations` in shipped code; only their
  `#[cfg(test)]` modules do. `qsl-replay` outside `spine` names one
  `qsl-forms` item, the fieldless `StateClauseKind`, and constructs
  `qsl_cst::Limits` for its recompile.
- 24 of the 161 public type names declared in `quire-exact` and
  `quire-semantic-value` are also declared, at some visibility, in another
  workspace crate, most of them generic names (`Value`, `Outcome`, `Refusal`, `Meter`) in
  seam modules that retire in M-6c.
- `quire-contract-model`, the IR crate QSL builds against, declares five public
  types whose names match public types of `quire-exact`,
  `quire-semantic-value` or `qsl-replay`: `CollectionType`,
  `ComparisonOperator`, `EnumDeclaration`, `IntegerDomain` and `ValueType`.
  They are v2 wire types that ADR-013 §4 converts at the boundary.

## Decision

Both gates are `xtask` subcommands built on the scanners already in
`xtask` (`typestate_scan` and `definition_scan`, both `syn`-based, both
reading shipped code only). Each reports every finding with its file and
line and exits non-zero when it reports any. `checked-input` is a
prerequisite of `make ci`, as `seam-probe`, `string-edge` and `route-lint`
are. `canonical-types` runs on demand and joins `make ci` once the
namesakes DT-7 resolves are gone (R-1).

### 1. Checked-input gate (`cargo xtask checked-input`)

| ID | Rule |
| --- | --- |
| CK-1 | **Stage entries.** A stage entry is a `pub` function or method in the shipped code of a crate at ADR-011 §6.1 layer 5 (`qsl-eval`: S6a evaluation, simulation, and `model_check` when it lands) or layer R (`qsl-route`: S6b candidate selection), or a `pub` function of the layer-6 `replay` facade outside its `spine` module (the E9 executor entries). The set comes from crate and module membership, so a new entry joins it by being written. QSL's lowering output is the S4 v2 emitter, whose input is `CheckedPackage`; IR lowers from the v2 wire through ADR-011 §4's verified binding. |
| CK-2 | **Pre-check representations.** A pre-check representation is (a) a type with fields, defined in `qsl-source`, `qsl-cst` or `qsl-forms` (ADR-011 layers I3, 1 and 2), that the return type of one of those crates' `pub` functions reaches; (b) the S3 input, the receiver type of the S3 stage constructor in TC-244's constructor table (`PackageDeclarations`); and (c) a type defined in `quire-contract-model`, which only the I2 reader in `qsl-package::checked_v2` admits. A fieldless enum from layers I3 to 2, such as `StateClauseKind`, is closed vocabulary that S3 carries forward, not a representation. A configuration type that no pre-check stage returns, such as `qsl_cst::Limits`, is not one either. |
| CK-3 | **Signature rule.** No stage entry's receiver or parameter type mentions a pre-check representation at any depth, with `use … as` renames and `type` aliases seen through as `typestate_scan` sees them. |
| CK-4 | **Reconstruction rule.** No shipped function in a stage-entry crate, or in `qsl-replay` outside `spine`, names a pre-check stage function: a `pub` function of `qsl-source`, `qsl-cst` or `qsl-forms`, or the S3 stage constructor. A stage entry that builds its own checked value from source or forms is rejected here, whatever its signature. Orchestrators that run S1 to S4 and then a stage (the `spine` module, root `command`, the ADR-011 T-13 driver) are outside the stage-entry crates and reach a stage only with the stage output they produced. |
| CK-5 | **Construction rule.** TC-243 and TC-244 stay as they are: stage-output types have private fields and one owner file, and only the stage constructors return them. With CK-3 and CK-4, this closes the path: a stage entry receives a checked value, and only a stage constructor can produce one. |

What the gate rejects:

- an evaluator entry in `qsl-eval` that takes a `qsl_forms::Expression`, a
  CST node or a `PackageDeclarations`;
- a function in `qsl-eval` or `qsl-route` that calls a parser, a form
  builder or `PackageDeclarations::check`, even with a checked signature;
- a replay entry that takes a `quire-contract-model` package and executes it
  without the I2 reader;
- through TC-244, a `From` impl, decoder or helper anywhere that returns an
  owned stage-output type.

The crate graph already keeps `qsl-eval` and `qsl-route` from depending on
layers I3 to 2 directly. The gate checks names, so a pre-check type that
`qsl-semantics` re-exports, or a dependency that is added later, is still
caught. On main the gate reports nothing (Context), so it joins `make ci` in
the change that builds it.

### 2. Duplicate-canonical-public-type gate (`cargo xtask canonical-types`)

| ID | Rule |
| --- | --- |
| DT-1 | **The canonical set is declared at the definition.** A canonical public type carries the doc-comment tag `/// quire:canonical`, a doc line holding exactly `quire:canonical`, on its definition (R-3). The tag is documentation: it adds no dependency and changes nothing in the compiled item, so `quire-exact` and `quire-semantic-value` carry it with their dependency lists unchanged. A type is tagged when an ADR-013 §3 owner row names it as its owner's public type; the tag is the code-side form of that row. The gate reads the canonical set from the tags in every crate it scans, so no list of type names is kept anywhere. A tag on anything other than a `pub` `struct`, `enum`, `union`, `trait` or `type` alias, or two tagged definitions of one identifier, is a finding. |
| DT-2 | **Identifier rule, inside the owning repository.** In the shipped code of the QSL workspace, any `struct`, `enum`, `union`, `trait` or `type` alias whose identifier equals a canonical type's identifier, defined as a module-level item in any module other than the canonical definition, is a finding, whatever its visibility or shape. An associated type inside an `impl` or `trait` block (serde's `type Value = …`) is not a definition. A `pub use` of a canonical type from a crate other than its owner is also a finding: one public path per canonical type, as ADR-011 §7.2 states for moved items. A namesake is resolved by DT-7. |
| DT-3 | **Copy rule, across repositories.** In an ecosystem dependency, a definition is a finding when its identifier equals a canonical type's identifier and its shape matches: the same item kind and the same member names in the same order (field names for a struct, variant names for an enum, method names for a trait), ignoring documentation, attributes, visibility and the paths of member types. A same-named type with a different shape across a repository boundary is a boundary type, which ADR-013 §4 converts and AD-016 records (the IR and RT `CheckedPackage`s, IR's wire `ValueType`). |
| DT-4 | **Scanned set.** The workspace crates and the ecosystem dependencies both come from `cargo metadata`: every workspace member's `src/`, and every resolved package that FR-061's shared `graph::classify` assigns to an ecosystem repository, read from its manifest directory. No directory list is written into the tool. Test code is excluded as `definition_scan` excludes it today. |
| DT-5 | **Backends run it too.** RT and CG run the same subcommand in their own lint gates, as they run the T-12 API-surface check. In a backend's workspace, QSL's `quire-exact`, `quire-semantic-value` and `qsl-replay` are ecosystem dependencies. Their tags give the canonical set, and DT-3 applies to the backend's own code: a backend's copy of a QSL canonical type fails in the backend's gate. |
| DT-6 | **Reuse of the definition scan.** `definition_scan`'s `DefScanner` and `scan_dirs` already record each item's identifier and location and skip test code. The gate extends them to record traits, unions, `pub use` items, the tag and each item's member names, and takes its directories from DT-4. The per-test name lists and the `SEMANTIC_VALUE_NAMESAKES` list in `definition_scan.rs` are replaced by DT-1 and DT-2. |
| DT-7 | **Delete or rename, per type (R-2).** A namesake with the same meaning as the canonical type is a copy: it is deleted and its uses take the canonical type. A namesake with a different meaning is renamed to say what it is. FR-273 lists the verdict for each namesake measured on main. |

Each finding names the canonical type, its definition's file and line, the
second definition's file and line, and the rule (identifier, re-export or
copy).

A type joins the canonical set when its owner tags it, and from then on the
scan finds any other definition of it, so the gate needs no upkeep as types
move between crates.

## Rulings

Owner rulings, 2026-10-01, recorded on the owning ticket.

| ID | Question | Ruling |
| --- | --- | --- |
| R-1 | Tagging the ADR-013 owner-row types makes DT-2 report every seam-module namesake of a tagged type. When does `canonical-types` join `make ci`? | It runs on demand now and joins `make ci` once the namesakes are gone. Its findings until then are the DT-7 work list. |
| R-2 | Layer-crate namesakes such as `qsl-semantics`'s `LimitKind`, `ChargePoint`, `Incomplete` and `Meter` sit beside `quire-exact`'s types of those names. | Decide per type (DT-7). A namesake with the same meaning is a copy: delete it and use the canonical type. A namesake with a different meaning is renamed to say what it is. The spec lists the verdict for each type (FR-273). |
| R-3 | A marker attribute from `qsl-attrs` would need a new dependency in `quire-exact` and `quire-semantic-value` and an FB-05 change. | No new dependency and no reserved-name list, since a list is a ledger. Canonical types carry the doc-comment tag `/// quire:canonical`, which the xtask parses (DT-1). FB-05 is unchanged. |
| R-4 | The owning ticket asks for a QSpec half. | There is no QSpec half: both gates check QSL's own code structure. |

## Amendments to make on acceptance

- ADR-011 §4: the two bullets on checking precedence and one type per stage
  output cite CK-1 to CK-5 and DT-1 to DT-7 as their running checks.
- `spec/spec.md`: the index rows for the requirements that implement this
  record.

## Consequences

- An execution, simulation, routing or replay entry that takes or rebuilds
  an unchecked program fails `make ci`, and so does a function that returns
  an owned stage-output type outside the stage constructors.
- A canonical type has one definition, and the place that says so is the
  type's own declaration.
- Two copies of a type in two repositories are caught in whichever
  repository's gate builds against both.
- Same-named boundary types across repositories stay, as AD-016 records
  them; same-named types inside QSL do not.

## Alternatives Considered

- **A table of canonical type names and owner crates in the tool**, like
  FR-060's rule table. Rejected. It is a second statement of ADR-013's rows
  that goes stale without failing, and a new canonical type is missed until
  someone edits the table. The tag cannot drift from the type it sits on.
- **Every `pub` type of K and SV is canonical.** Rejected: it reserves all
  161 names workspace-wide, most of them unrelated to an ADR-013 owner row.
- **A `#[canonical]` attribute from `qsl-attrs`.** Rejected (R-3): it adds a
  dependency to K and SV and changes FB-05, where a doc-comment tag does the
  same work with neither.
- **Name-only matching across repositories.** Rejected. It reports the
  boundary types ADR-013 §4 converts and AD-016 keeps, such as IR's wire
  `ValueType`.
- **Structural similarity beyond member names.** Rejected, as
  `definition_scan` already records: member-name equality catches the
  byte-for-byte copies ADR-011 X-1a and X-1b removed, and a general
  similarity engine is speculative cost.
- **Checked input by the API-surface lint (FR-060).** Rejected as the home.
  `arch-lint api-surface` matches tokens with no alias resolution and is not a
  `make ci` prerequisite, whereas `typestate_scan` already sees through
  aliases and runs in CI. CK-4 is the same kind of caller rule, applied to
  the S1 to S3 functions.
- **Crate boundaries alone.** Rejected. They stop a direct dependency but not
  a pre-check type re-exported by `qsl-semantics`, and they say nothing about
  the S3 input that `qsl-semantics` itself defines.

## References

- Owning ticket: Linear QSL-391. Implementation: Linear QSL-13.
- The S-3 typestate scans this builds on: TC-243, TC-244 (FR-087).
- The definition scan this extends: QSL-358 slice 5 (`xtask/src/definition_scan.rs`).
- Requirements: FR-270 (checked-input gate), FR-271 (the canonical tag),
  FR-272 (the canonical-types gate), FR-273 (namesake verdicts); US-009 and
  US-028.
- The T-12 gates beside it: FR-059 to FR-061, run by RT and CG under
  agent-ix/quire-contract-runtime#56 and agent-ix/quire-contract-codegen#89.
