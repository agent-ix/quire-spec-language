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

ADR-011 §7.3 M-3 requires QSL to add the S2 `forms` stage — the sole edge-2
(E2) producer of parsed semantic forms from a lossless CST, at the layer-2
position ADR-011 §6.1 assigns it (`forms` core, below the family form
builders) — and to retire SEAM-5, the structural lowering
`complete::package::lower_source_graph` / `LoweredSourceGraph` that
ADR-010 §2.6 X4 found has no consumer anywhere in `src` outside the module
that defines it. QSL SHALL add the `forms` core module. QSL SHALL delete
`LoweredSourceGraph`, `LoweredDeclaration` and `lower_source_graph` in the
same change (ADR-011 §7.3 T-3 same-change rule): no build after this
requirement lands carries both the new S2 boundary and the old SEAM-5 dead
end.

This requirement builds the `forms` core: the closed, family-keyed dispatch
entry table (ADR-012 §3) over the parsed-form enum and its leading-token-kind
enum, and the shared parsed-form contract (span-only, no semantic identity;
ADR-011 §2.2 E2 row). It does not add any family's own grammar production
function or that family's variant of the parsed-form enum. Each family's own
migration ticket adds its production function and its enum variant to the
entry table this requirement builds, when that family migrates onto S2
forms — the same incremental pattern ADR-011 §7.3 M-6e uses for family
checker modules at S3, applied here to S2.
[FR-065](FR-065-migrate-function-application-to-checked-family.md) already
treats a family's parsed forms as a given input it consumes, not a mechanism
it builds; this requirement is that mechanism's shared core.

This requirement does not retire SEAM-1's native `syntax`/`parser` or
SEAM-2's composed `syntax`/`parser`. Those retire per lane, under ADR-011
§7.3 M-6a to M-6e, as each family's own migration ticket cuts that family
over onto S2 forms and its S3 checker; this requirement only adds the S2
boundary they will eventually cut over to.

## Inputs

- A lossless CST with no error or recovery node (ADR-011 §2.1 E1 output;
  §2.3 E2 admitted input).
- The closed leading-token-kind enum that a CST root construct's first token
  selects, and the family-keyed dispatch entry table over it (ADR-012 §3).
- For a family already wired into the entry table by its own migration
  ticket: that family's grammar production function.

## Outputs

- A parsed form carrying the span of its originating CST node and no
  semantic identity, declaration key, or other check-time-minted value
  (ADR-011 §2.2 E2 row: identity "none: forms carry position only").
- A refusal with diagnostics, and no parsed form, when the input CST carries
  an error or recovery node (ADR-011 §2.3 E2 row: "No. A form is built from
  a complete CST or not at all.").
- Removal, from the compiled crate and from every module's re-export list,
  of `LoweredSourceGraph`, `LoweredDeclaration` and `lower_source_graph`.

## Behavior

### The forms core is the sole E2 producer

The `forms` core module SHALL be the only producer of a parsed semantic form
from a CST. It SHALL depend on layer 1 (`cst`) and F only (ADR-011 §6.1).
Given a CST with no error or recovery node, whose root construct's leading
token has an entry in the dispatch table, the forms stage SHALL return a
parsed form built from that CST. Given a CST that carries an error or
recovery node, the forms stage SHALL return a refusal with diagnostics and
SHALL NOT return a parsed form built from the incomplete tree, matching
every other S1-to-S4 stage's "no partial output" rule (ADR-011 §2.3).

A parsed form the forms stage returns SHALL expose an accessor for the span
of its originating CST node and SHALL NOT expose any accessor that returns,
or from which a caller could derive, a semantic node identity, a declaration
key, or any other value checking mints. Identity is minted only at check
(S3); a form built at S2 SHALL carry no identity at all, not a placeholder
or a provisional one.

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
and no family's variant of the parsed-form enum. A family's own migration
ticket adds both when that family migrates onto S2 forms.

### SEAM-5 retires in this change

After this requirement's implementation, `LoweredSourceGraph`,
`LoweredDeclaration` and `lower_source_graph` SHALL be absent from the
compiled crate's symbols, from `complete::mod`'s re-export list, and from
every source file under `src/`, including test targets. The pre-migration
test that exercises `lower_source_graph` directly
(`complete::package_tests`) SHALL be deleted in the same change, not left
present behind `#[ignore]` or a feature gate that disables it without
removing it.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-067-CON-1 | This requirement's implementation adds the `forms` core module and deletes `LoweredSourceGraph`, `LoweredDeclaration` and `lower_source_graph` in the same change; no change under this requirement leaves both the S2 forms core and any of these three symbols present in the crate at once. | Process | Test |
| FR-067-CON-2 | This requirement's implementation adds no family's grammar production function and no family-specific parsed-form-enum variant; each family's own migration ticket adds its own production function and enum variant when that family migrates onto S2 forms. | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-067-AC-1 | Given a lossless CST with no error or recovery node, whose leading-token kind has an entry in the forms core's dispatch table, the forms stage returns a parsed form built from that CST. Given the same CST with one node marked as a recovery node, the forms stage returns a refusal with diagnostics and no parsed-form value, even though the same dispatch-table entry exists. A test using a test-only leading-token-kind variant and a stub production function demonstrates both branches, so the seam is exercised without depending on any family having migrated onto S2 forms. | Test (TC-167) |
| FR-067-AC-2 | A parsed form exposes an accessor for the span of its originating CST node and no accessor that returns a semantic node identity, a declaration key, or any other check-time-minted value; a test that attempts to read a semantic identity off a parsed form fails to compile, because no such accessor exists on the type, not merely because none is called at runtime. | Test (TC-167) |
| FR-067-AC-3 | Each entry in the forms core's dispatch table makes exactly one call into its family's production function and holds no other conditional, lookup or loop; a code-shape test (an AST or line-count check against a fixed budget, the same style FR-065-AC-4 applies to `infer_form`) fails if a future change adds branching logic directly inside a dispatch-table entry instead of inside the family production function it calls. | Test (TC-167) |
| FR-067-AC-4 | The forms core's dispatch table's `match` over the leading-token-kind enum, and the parsed-form enum it produces, are among FR-063's checked-in seam locations; adding a variant to either enum without a matching arm at every FR-063-listed seam produces `E0004` at that seam, demonstrated by FR-063's seam probe (FR-063-AC-1, FR-063-AC-6) on every full-gate run, not re-verified here by source inspection. | Test (TC-161) |
| FR-067-AC-5 | After this requirement's implementation lands, `LoweredSourceGraph`, `LoweredDeclaration` and `lower_source_graph` are absent from the compiled crate's public and crate-internal symbols and from `complete::mod`'s re-export list; a grep-equivalent symbol scan over the compiled crate confirms the absence of all three names. A change that adds the forms core while leaving any of the three symbols reachable from any module does not satisfy this criterion. | Test (TC-168) |
| FR-067-AC-6 | The crate, including its test targets, compiles with zero source references to `lower_source_graph`, `LoweredSourceGraph` or `LoweredDeclaration` anywhere under `src/`; the pre-migration test that exercised `lower_source_graph` directly (`complete::package_tests`) is deleted in the same change, not left calling a now-missing symbol (which would only fail to compile) and not left disabled behind `#[ignore]` or a feature gate. | Test (TC-168) |

## Dependencies

- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.1 and §2.2 (the E2 edge's admitted input and preserved/dropped
  columns), §2.3 (E2's "no partial output" refusal rule), §6.1 (the `forms`
  core layer-2 position and its allow-listed dependencies), §6.2 (the
  SEAM-5 retirement condition and the `complete::package` module-map row),
  §7.3 M-3 (this requirement's ADR row) and T-3 (the same-change deletion
  rule).
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §3
  (a family's own grammar productions and the one closed entry table over
  the leading-token-kind enum), §4.3 (the thin-dispatch-seam rule this
  requirement's entry table follows), §5.1 (the closed parsed-form enum
  FR-063 governs).
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

Each family's own migration ticket adds that family's grammar production
function and parsed-form-enum variant to the `forms` core this requirement
builds, the same incremental pattern ADR-011 §7.3 M-6e uses for family
checker modules. This requirement's own implementation is bounded to the S2
`forms` core, its dispatch entry table, and the same-change deletion of
SEAM-5 (`LoweredSourceGraph`, `LoweredDeclaration`, `lower_source_graph`).
SEAM-1's native and SEAM-2's composed `syntax`/`parser` modules are
unaffected by this requirement and retire per lane under ADR-011 §7.3
M-6a to M-6e.
