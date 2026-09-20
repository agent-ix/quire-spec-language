---
id: FR-067
title: "Add the S2 forms stage and retire SEAM-5"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-007
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-063
    type: traces_to
---
# FR-067: Add the S2 forms stage and retire SEAM-5

## Description

ADR-011 §7.3 M-3 requires QSL to add the S2 `forms` stage and to retire
SEAM-5, the structural lowering `complete::package::lower_source_graph` /
`LoweredSourceGraph`. ADR-010 §2.6 X4 records `LoweredSourceGraph` as
`absent: LoweredSourceGraph in src excluding src/complete` — the two
occurrences besides its own definition, `complete::package_tests`'s call and
`complete::mod`'s re-export, are both inside `src/complete`, so X4's claim
and this requirement's same-change deletion hold together, but X4 does not
say "no consumer anywhere," and this requirement does not claim that either.
QSL SHALL add the `forms` core module. QSL SHALL delete
`LoweredSourceGraph`, `LoweredDeclaration` and `lower_source_graph` in the
same change: the M-3 row's Compatibility column states this directly
("none: `LoweredSourceGraph` is deleted in the same change"); no build after
this requirement lands carries both the new S2 boundary and the old SEAM-5
dead end.

The S2 forms stage is QSL's E2 producer of parsed semantic forms from a
lossless CST. ADR-011 §2.1 names its owner as "QSL `forms`; family form
builders (ADR-012 §2)" — the core and the family builders together, not the
core alone. This requirement builds the `forms` core, at the layer-2
position ADR-011 §6.1 assigns it: the closed, family-keyed dispatch entry
table (ADR-012 §3) over the parsed-form enum (the one `Expression` enum,
ADR-012 §4.3) and its leading-token-kind enum, and the shared parsed-form
contract (ADR-011 §2.2 E2 row).

**Scope split (owner ruling, 2026-09-20): M-3a and M-3b.** ADR-012 §4.3's
final paragraph states: "The one `Expression` enum (S2) is defined in the
`forms` core. Each variant's owning family is the family whose hook its arm
calls, and that family adds and removes the variant." ADR-012 §5.1's S2 row
names the seam's owner as "QSL, owning family": the core's shape is QSL's,
each variant is its family's. This requirement is M-3a, the QSL-owned part:
the `forms` core, its dispatch entry table, the SEAM-5 retirement, and the
`value::expression::syntax` module move. ADR-011 §6.2's module table stamps
`value::expression::syntax` — the module that today defines the `Expression`
enum, `FunctionDeclaration`, `BinaryOperator`, `ClauseKind`,
`DeclaredClauseKind`, `FieldInitializer`, `BinderQuery` and `Accumulation` —
to layer-2 `forms` under M-3; that move is core content, not family content,
so it lands here, not with a family's migration ticket. Each family's own
variant of the parsed-form enum and its own production function are M-3b,
added incrementally by that family's own migration ticket, the same pattern
ADR-011 §7.3 M-6e uses for family checker modules at S3.
[FR-065](FR-065-migrate-function-application-to-checked-family.md) already
treats a family's parsed forms as a given input it consumes, not a mechanism
it builds; this requirement is that mechanism's shared core.

This requirement does not retire SEAM-1's native `syntax`/`parser` or
SEAM-2's composed `syntax`/`parser` (the top-level `syntax`/`parser`
modules, distinct from `value::expression::syntax`). Those retire per lane,
under ADR-011 §7.3 M-6a to M-6e, as each family's own migration ticket cuts
that family over onto S2 forms and its S3 checker; this requirement only
adds the S2 boundary they will eventually cut over to.

## Inputs

- A lossless CST with no error or recovery node (ADR-011 §2.1 E1 output;
  §2.3 E2 admitted input), carrying the edition read from source (ADR-011
  §2.2 E1 row) and any declared bound or extent as syntax.
- The closed leading-token-kind enum that a CST root construct's first token
  selects, and the family-keyed dispatch entry table over it (ADR-012 §3).
- The existing `value::expression::syntax` module's content — the
  `Expression` enum, `FunctionDeclaration`, `BinaryOperator`, `ClauseKind`,
  `DeclaredClauseKind`, `FieldInitializer`, `BinderQuery` and
  `Accumulation` — to relocate under `forms` (ADR-011 §6.2 module-map row).
- For a family already wired into the entry table by its own migration
  ticket: that family's grammar production function.

## Outputs

- A parsed form carrying the span of its originating CST node and no
  semantic identity, declaration key, or other check-time-minted value
  (ADR-011 §2.2 E2 row: identity "none: forms carry position only").
- A parsed form carrying its CST's edition unchanged (ADR-011 §2.2 E2 row:
  version "Edition carried") and, for a construct with a declared bound or
  extent, that bound or extent carried as syntax (ADR-011 §2.2 E2 row: proof
  metadata "Declared bounds and extents carried as syntax").
- A refusal with diagnostics, and no parsed form, when the input CST carries
  an error or recovery node (ADR-011 §2.3 E2 row: "No. A form is built from
  a complete CST or not at all.").
- The `Expression` enum and its sibling types (listed under Inputs) defined
  exactly once, under `forms`, and no longer under `value::expression::syntax`.
- Removal, from the compiled crate and from every module's re-export list,
  of `LoweredSourceGraph`, `LoweredDeclaration` and `lower_source_graph`.

## Behavior

### The forms core is QSL's E2 producer, together with family form builders

The `forms` core module and each family's own form builder SHALL together
be the only producers of a parsed semantic form from a CST (ADR-011 §2.1 E2
owner: "QSL `forms`; family form builders"); the core alone is not a
complete producer, and this requirement does not claim otherwise. The
`forms` core SHALL depend on layer 1 (`cst`) and F only (ADR-011 §6.1).
Given a CST with no error or recovery node, whose root construct's leading
token has an entry in the dispatch table, the forms stage SHALL return a
parsed form built from that CST. Given a CST that carries an error or
recovery node, the forms stage SHALL return a refusal with diagnostics and
SHALL NOT return a parsed form built from the incomplete tree, matching
every other S1-to-S4 stage's "no partial output" rule (ADR-011 §2.3).

A parsed form the forms stage returns SHALL expose an accessor for the span
of its originating CST node and SHALL NOT expose any accessor whose return
type is `NodeKey`, `DeclarationKey`, or any other type ADR-013 O-04 or O-07
defines as a check-time-minted identity. Identity is minted only at check
(S3); a form built at S2 SHALL carry no identity at all, not a placeholder
or a provisional one.

### The parsed form carries its edition and any declared bound or extent

A parsed form the forms stage returns SHALL carry the edition its
originating CST recorded, unchanged (ADR-011 §2.2 E2 row, version: "Edition
carried"). For a construct that declares a bound or an extent, the parsed
form SHALL carry that declared bound or extent as syntax, unchanged from the
CST (ADR-011 §2.2 E2 row, proof metadata: "Declared bounds and extents
carried as syntax"). Neither value SHALL be recomputed, defaulted or
dropped between the CST and the parsed form; both are read again from the
parsed form, unchanged, when the S3 `check` core derives a checked item's
`Requirements` (ADR-012 §2's `Requirements` part), so a value the forms
stage drops or recomputes here is a value `Requirements` cannot recover
later.

### Family dispatch through the forms core is a closed, thin seam

The forms core SHALL own one closed dispatch table keyed by the closed
leading-token-kind enum (ADR-012 §3). Each entry in that table SHALL make
exactly one call into its family's own production function and SHALL hold no
other conditional, lookup or loop (ADR-012 §4.3 thin-dispatch rule); building
the call's argument and propagating its result are not semantic logic, and a
branch or lookup performed directly inside an entry is, and SHALL NOT appear
there.

The parsed-form enum and the leading-token-kind enum this dispatch table
matches on are two of the four closed enums
[FR-063](FR-063-exhaustive-family-extension-seam-probe.md)'s seam probe
governs. This requirement does not re-specify that exhaustiveness mechanism:
the dispatch table's `match` SHALL carry no `_` or catch-all arm, the same
rule FR-063 states for every S1-to-S4 seam, and FR-063's probe is what
demonstrates that rule holds on every full-gate run.

This requirement's `forms` core SHALL add no family's production function
and no family's variant of the parsed-form enum (M-3b). A family's own
migration ticket adds both when that family migrates onto S2 forms.

### The `value::expression::syntax` module moves to `forms` (M-3a)

ADR-011 §6.2's module table maps `value::expression::syntax` to layer-2
`forms`, stamped M-3. The `Expression` enum, `FunctionDeclaration`,
`BinaryOperator`, `ClauseKind`, `DeclaredClauseKind`, `FieldInitializer`,
`BinderQuery` and `Accumulation` SHALL each be defined exactly once, under
`forms`, after this requirement's implementation, and SHALL NOT remain
defined under `value::expression::syntax`. `value::expression::syntax`
SHALL NOT exist as a module after this requirement's implementation.
`value::expression`'s own `check`, `evaluate`, `facts`, `ir`, `refusal` and
`termination` modules MAY import these types from `forms`; importing them is
not a second definition. This move carries the `Expression` enum unchanged
in shape: this requirement moves the type's defining location, and SHALL
NOT add, remove or rename a variant, a field, or a method on any of the
eight types as part of the move.

### SEAM-5 retires in this change

After this requirement's implementation, `LoweredSourceGraph`,
`LoweredDeclaration` and `lower_source_graph` SHALL be absent from the
compiled crate's symbols, from `complete::mod`'s re-export list, and from
every tracked file in the repository, including test targets, `xtask` and
any benchmark harness, not only `src/`. The pre-migration test that
exercises `lower_source_graph` directly (`complete::package_tests`) SHALL be
deleted in the same change, not left present behind `#[ignore]` or a
feature gate that disables it without removing it, and SHALL NOT be
relocated to `tests/`, `xtask` or anywhere else instead of being deleted.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-067-CON-1 | This requirement's implementation adds the `forms` core module and deletes `LoweredSourceGraph`, `LoweredDeclaration` and `lower_source_graph` in the same change, from every tracked file in the repository; no change under this requirement leaves both the S2 forms core and any of these three symbols present anywhere in the repository at once. | Process | Test |
| FR-067-CON-2 | This requirement's scope (M-3a) is the `forms` core, its dispatch entry table, the SEAM-5 retirement, and the `value::expression::syntax` module move. It adds no family's grammar production function and no family-specific parsed-form-enum variant (M-3b); each family's own migration ticket adds its own production function and enum variant when that family migrates onto S2 forms. | Design | Inspection |
| FR-067-CON-3 | The `value::expression::syntax` module move relocates the `Expression` enum and its seven sibling types without changing any variant, field or method on any of them; a change under this requirement that alters one of these types' shape while moving it does not satisfy this constraint. | Design | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-067-AC-1 | Given a lossless CST with no error or recovery node, whose leading-token kind has an entry in the forms core's dispatch table, the forms stage returns a parsed form built from that CST. Given the same CST with one node marked as a recovery node, the forms stage returns a refusal with diagnostics and no parsed-form value, even though the same dispatch-table entry exists. A test using a test-only leading-token-kind variant and a stub production function demonstrates both branches, so the seam is exercised without depending on any family having migrated onto S2 forms. | Test (TC-167) |
| FR-067-AC-2 | A parsed form exposes an accessor for the span of its originating CST node and no accessor whose return type is `NodeKey`, `DeclarationKey`, or any other type ADR-013 O-04 or O-07 defines as a check-time-minted identity; a test that attempts to call such an accessor on a parsed form fails to compile, because none exists on the type. This criterion checks the absence of accessors returning this named set of identity types; it is not a claim that no accessor of any other shape could leak positional information that behaves like an identity. | Test (TC-167) |
| FR-067-AC-3 | Each entry in the forms core's dispatch table makes exactly one call into its family's production function and holds no other conditional, lookup or loop; a code-shape test (an AST or line-count check against a fixed budget, the same style FR-065-AC-4 applies to `infer_form`) fails if a future change adds branching logic directly inside a dispatch-table entry instead of inside the family production function it calls. | Test (TC-167) |
| FR-067-AC-4 | The forms core's dispatch table's `match` over the leading-token-kind enum, and the parsed-form enum it produces, are among FR-063's checked-in seam locations; adding a variant to either enum without a matching arm at every FR-063-listed seam produces `E0004` at that seam, demonstrated by FR-063's seam probe (FR-063-AC-1, FR-063-AC-6) on every full-gate run, not re-verified here by source inspection. | Test (TC-161) |
| FR-067-AC-5 | After this requirement's implementation lands, `LoweredSourceGraph`, `LoweredDeclaration` and `lower_source_graph` are absent from the compiled crate's public and crate-internal symbols and from `complete::mod`'s re-export list; a grep-equivalent symbol scan over the compiled crate confirms the absence of all three names. This criterion alone is weak — rustc emits no symbol for an unreferenced non-generic struct, so this scan can pass while `LoweredDeclaration` still exists in source — and FR-067-AC-6's source-reference scan is the load-bearing check for that gap, not this one. | Test (TC-168) |
| FR-067-AC-6 | The repository, including every tracked file — `src/`, test targets, `xtask` and any benchmark harness, not `src/` alone — contains zero source references to `lower_source_graph`, `LoweredSourceGraph` or `LoweredDeclaration`. The pre-migration test that exercised `lower_source_graph` directly (`complete::package_tests`) is deleted in the same change: not left calling a now-missing symbol, not left disabled behind `#[ignore]` or a feature gate, and not relocated to `tests/` or `xtask` instead of deleted. A change that moves `lower_source_graph` and its test out of `src/` into `tests/` or `xtask` does not satisfy this criterion, even though it would satisfy a `src/`-only scan. | Test (TC-168) |
| FR-067-AC-7 | A parsed form's edition equals the edition its originating CST recorded; a test that builds two parsed forms from CSTs recorded under two different edition values (for example the existing top-level and `composed` edition constants) and reads each form's edition back observes the two different values, not one value shared by both, showing the edition is carried from the CST rather than hardcoded or defaulted. | Test (TC-167) |
| FR-067-AC-8 | For a construct that declares a bound or an extent, the resulting parsed form carries that declared value unchanged; a test that declares two different bound or extent values on two otherwise-identical constructs and reads each resulting form's carried value back observes the two different values, showing the value is carried from the declaration rather than a shared default. | Test (TC-167) |
| FR-067-AC-9 | After this requirement's implementation, `value::expression::syntax` does not exist as a module, and the `Expression` enum, `FunctionDeclaration`, `BinaryOperator`, `ClauseKind`, `DeclaredClauseKind`, `FieldInitializer`, `BinderQuery` and `Accumulation` are each defined exactly once, under `forms`. A module-tree scan confirms `value::expression::syntax`'s absence; a scan of type definitions confirms none of the eight names has a second, independent definition remaining under `value::expression`. `value::expression`'s own code importing these types from `forms` does not itself fail this criterion; a second, independent definition of any one of them does. | Test (TC-169) |

## Dependencies

- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.1 (the E2 edge's owner: "QSL `forms`; family form builders") and §2.2
  (the E2 row's five preserved/dropped columns: syntax, provenance, identity,
  version and proof metadata — this requirement covers all five, not only
  identity and provenance), §2.3 (E2's "no partial output" refusal rule),
  §6.1 (the `forms` core layer-2 position and its allow-listed dependencies),
  §6.2 (the SEAM-5 retirement condition and the `value::expression::syntax`
  → `forms` module-map row, both stamped M-3), §7.3 M-3 (this requirement's
  ADR row and its Compatibility column, the actual authority for the
  same-change deletion — not T-3, which lists M-6b to M-6e and does not name
  M-3).
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §3
  (a family's own grammar productions and the one closed entry table over
  the leading-token-kind enum), §4.3 (the thin-dispatch-seam rule this
  requirement's entry table follows, and its final paragraph: "The one
  `Expression` enum (S2) is defined in the `forms` core. Each variant's
  owning family is the family whose hook its arm calls, and that family adds
  and removes the variant" — the basis for this requirement's M-3a/M-3b
  split), §5.1 (the closed parsed-form enum FR-063 governs, whose S2 row
  names the owner "QSL, owning family").
- [ADR-010](../decisions/ADR-010-observed-architecture-baseline.md) §2.6 X4
  records `LoweredSourceGraph` as `absent: LoweredSourceGraph in src
  excluding src/complete`; this requirement's Description quotes that claim
  as written rather than restating it more broadly.
- [FR-062](FR-062-implement-checked-family-contract.md) takes "parsed forms
  produced by a family's own grammar productions" as an input it consumes;
  this requirement builds the shared mechanism that produces them.
- [FR-063](FR-063-exhaustive-family-extension-seam-probe.md) is this
  requirement's mechanism for demonstrating the dispatch table's
  exhaustiveness (FR-067-AC-4); this requirement does not re-verify it by
  inspection.
- [US-007](../usecase/US-007-trust-a-single-s2-forms-producer.md).

## Status

Specified under QSL-138 (ADR-011 §7.3 M-3), split out of
[#214](https://github.com/agent-ix/quire-spec-language/issues/214) by owner
ruling, 2026-09-20: #214 was already at the limit of its own bounded-effort
criterion, and M-3 keeps its own same-change deletion obligation intact
under the new ticket. ADR-011's §7.3 Order column still names #214 for M-3;
that is a recorded follow-up to amend the ADR's text, not a contradiction
this requirement resolves. Not yet implemented.

**M-3 splits into M-3a and M-3b (owner ruling, 2026-09-20).** This
requirement is M-3a: the `forms` core, its dispatch entry table, the
SEAM-5 retirement, and the `value::expression::syntax` module move — the
QSL-owned content ADR-012 §4.3's final paragraph and §5.1's S2 row assign to
"QSL"/"the `forms` core," as distinct from each variant, which those same
passages assign to "owning family." M-3b is each family's own parsed-form-
enum variant and production function, filed as its own ticket and added
incrementally by that family's own migration ticket alongside ADR-011 §7.3
M-6a to M-6e, the same pattern already used for family checker modules at
S3. M-5's Order cell ("after M-3") is amended to point at M-3a specifically,
since M-3 no longer names one point in time once M-3b is incremental. The
coordinator amends ADR-011's text for the M-3 split, the Order column, and
M-5's predicate; this requirement does not.

SEAM-1's native and SEAM-2's composed `syntax`/`parser` modules (the
top-level `syntax`/`parser`, distinct from `value::expression::syntax`) are
unaffected by this requirement and retire per lane under ADR-011 §7.3
M-6a to M-6e.
