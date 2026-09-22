---
id: FR-060
title: "Check the QSL API-surface boundary reachable from backend and internal callers"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: traces_to
---
# FR-060: Check the QSL API-surface boundary reachable from backend and internal callers

## Description

ADR-011 §7.1 T-12 and ADR-013 O-04/O-05 each name a caller boundary that a
crate-level dependency check (FR-059) cannot see, because the boundary is
about which *module*, not which *crate*, is allowed to call a given
construction site:

- CG's normal dependency on QSL (the FR-059 FB-05 exception) is only sound if
  CG's own source calls into QSL exclusively through the layer-6 `replay`
  facade module, never another QSL module.
- The kernel `NodeKey` constructor (ADR-013 O-04, today at its pre-kernel
  location) is only called by the `check` stage.
- The kernel `EffectiveId` constructor (ADR-013 O-05, today at its pre-kernel
  location) is only called by the `model` stage.
- The kernel `PopulationId` constructor (ADR-013 O-13 Population row, QC-21;
  `quire-exact`'s `PopulationId::from_digest`) is only called by the `model`
  stage (ADR-011 §7.1 T-12(d)).

The `quire-exact` kernel crate is ADR-011 §6.1 layer K
([#213](https://github.com/agent-ix/quire-spec-language/issues/213) S-1). This
requirement checks each rule against its *current* call-site location, so the
rule set is live data #213 updates in place as each constructor moves into the
kernel, rather than a check that is retired and rewritten.

## Inputs

- A rule set: for each rule, the textual call pattern(s) that identify a call
  to the constructor or facade the rule protects, the module path prefix(es)
  a caller is allowed to have, the file path the rule's target currently
  requires to exist (so a rule whose target has not landed yet is
  distinguishable from a rule whose target exists and is violated), and the
  *role* whose source tree the rule scans (see below).
- The source tree the rule's role names: QSL's own tree for a QSL-role rule,
  or a caller-supplied CG checkout for a CG-role rule (#249 review, HIGH-2/
  MEDIUM-4).

## Outputs

- Per rule, one of: **live and passing** (the rule's target exists and every
  call site outside the allowed caller modules is absent), **live and
  failing** (the rule's target exists and at least one call site outside the
  allowed caller modules exists; each is reported with file and line; for
  T12-B and T12-C, a site in a debt-list function is reported as debt rather
  than as a failure, and a stale debt-list entry is a failure), or
  **pending** (the rule's target -- a file the rule requires -- does not
  exist yet, so the rule reports this explicitly rather than passing
  vacuously).

## Behavior

### Rule set (data, not hard-coded per-call logic)

The check SHALL hold its rules as data (one entry per rule: id, description,
call patterns, allowed caller module prefixes, a required file path, a role
and a pending-reason string), so #213 can update the call patterns, allowed
prefixes and required path in place when `quire-exact` lands, without
changing the check's evaluation logic.

Each rule has a **role**: `Qsl` (scan QSL's own tree, the tree already
supplied for the pending-path check) or `Cg` (scan a separately supplied CG
checkout). T12-A is CG-role: its target, the layer-6 `replay` facade module,
is *consumed* by CG, and a violation of "CG calls into QSL only through
`replay`" is only visible by scanning CG's own source, never QSL's (#249
review, HIGH-2/MEDIUM-4 -- evaluating a CG-side rule against the QSL tree
made it impossible for T12-A to ever fail, since QSL's own tree contains no
CG call sites to find). T12-B, T12-C and T12-D are QSL-role: each protects a
constructor QSL itself calls.

At the time of this requirement, the rule set SHALL contain exactly these
four rules:

| Rule | Role | Protects | Allowed callers (today) | Requires |
| --- | --- | --- | --- | --- |
| T12-A | Cg | The layer-6 `replay` facade module | (this rule reports pending until the facade module exists at the path this rule names; once it exists, a CG checkout must be supplied to scan it) | `src/replay.rs` |
| T12-B | Qsl | The kernel `NodeKey` constructor (ADR-013 O-04) | `check` and every descendant module; plus the named debt list below, which only shrinks | the constructor's current source file |
| T12-C | Qsl | The kernel `EffectiveId` constructor (ADR-013 O-05) | `model` and every descendant module; plus the named debt list below, which only shrinks | the constructor's current source file |
| T12-D | Qsl | The kernel `PopulationId` constructor (ADR-013 O-13 Population row, QC-21; ADR-011 T-12(d)) | `model` | `src/model/population.rs` |

A rule's call patterns SHALL include every textual spelling that constructs
the protected value, not only its primary constructor name.
T12-C's pattern matches `EffectiveId::from_digest`, called or passed as a
function value. T12-D's pattern is `PopulationId::from_digest(`, the kernel `PopulationId`'s
one public constructor.

### T12-B: only `check` mints `NodeKey`

**Amended by the layer-rule ruling (2026-09-22).** T12-B's allowed callers
are `check` and every module under it, because ADR-013 O-04 says only
`check` calls the `NodeKey` constructor. `check::family`'s
`mint_declaration_identity` and `mint_call_identity`, and
`check::checked_dispatch`, are allowed under that prefix. `value::node` is
not exempt: it defines the `node_key_of` helper, not the kernel constructor,
and its mints are debt.

T12-B's patterns SHALL match every reference to a `NodeKey` constructor in
shipped code, whether it is called or passed as a function value:
`NodeKey::from_digest` (the kernel constructor), and, while QSL's own
`value::node::NodeKey` type exists, its `of`, `from_bytes` and `from_hex`.
T12-B also matches calls of `node_key_of`, the crate-internal helper that
wraps the constructor (#249 review R1); the helper's own `fn node_key_of(`
definition line is not a mint. A reference passed as a function value, such
as `value::node`'s `NodeIdDocument::key` writing
`.map(NodeKey::from_digest)`, is a mint.

### T12-B and T12-C: shipped code and debt lists

T12-B and T12-C scan shipped code only: items under `#[cfg(test)]` are
excluded, and so is a pattern match inside a comment or a doc comment. A test
fixture that builds a `NodeKey` or `EffectiveId` from literal bytes, or a doc
comment that names the constructor, is not an identity the crate mints.

Each of the two rules has a debt list. Every shipped mint outside the rule's
allowed callers SHALL be in a function on that rule's list. A list is keyed
by enclosing module and function, not by line number, and it only shrinks:
an entry leaves in the change that removes its last mint, and no entry is
added. A mint outside the allowed callers in a function not on the list
fails the rule. An entry with no remaining mint also fails the rule until it
is removed, so a fixed site cannot later hide a new mint. Each debt-list mint
is reported as debt, with file, line, module and function. The lists name
functions rather than counting sites, because a count is met by a broken
state (one new mint and one fixed mint cancel out) and changes whenever the
patterns become more accurate.

T12-B's debt list:

| Module | Function | Why it is debt |
| --- | --- | --- |
| `value::enumeration` | `EnumDeclarationPreimage::node_key` | mints through `node_key_of` outside `check` (ADR-011 FB-13) |
| `value::enumeration` | `EnumMemberPreimage::node_key` | same |
| `value::unit` | `DimensionPreimage::node_key` | same |
| `value::unit` | `UnitPreimage::node_key` | same |
| `value::node` | `node_key_of` | the helper the four entries above call; mints directly |
| `value::node` | `NodeIdDocument::key` | wraps a digest string read from caller-supplied JSON into a `NodeKey`; O-04 says a wire-read id becomes a `NodeKey` only by lookup |
| `value::model_query` | `to_object_reference` | OBS-018: builds a `NodeKey` from a model `ReferenceKey`'s bytes |
| `value::expression::family` | `decode_v2` | wraps the wire-read identity hex of the QSL v2 function-package codec into a `NodeKey`; the entry leaves when that codec is deleted |

T12-C's debt list:

| Module | Function | Why it is debt |
| --- | --- | --- |
| `value::model_query` | `bridge_lookup_key` | OBS-018: builds an `EffectiveId` from a `NodeKey`'s bytes outside `model` |
| `value::model_query` | `resolve_target` | same |

### Pending vs. live vs. failing

If a rule's required file path does not exist in the source tree under
inspection, the check SHALL report that rule **pending**, naming the missing
path, and SHALL NOT scan for call sites under that rule.

If a rule's role names a tree that was not supplied (a CG-role rule given no
CG checkout, once its target exists), the check SHALL refuse with a usage
error naming the flag the caller must supply, rather than reporting a vacuous
pass.

If a rule's required file path exists and its tree was supplied, the check
SHALL scan every `.rs` file under that tree's `src/` for the rule's call
pattern(s) and SHALL report **failing**, naming each call site's file, line
and enclosing module, for every call site whose enclosing module path is not
exactly one of the rule's allowed prefixes or a descendant of one (matched on
a `::`-segment boundary, so `model_query` is not treated as inside `model`).

A rule with no call site outside its allowed callers SHALL report **passing**.

### Scanning method and its stated limitations

The check SHALL scan source text for each rule's literal call patterns. It
does not resolve import aliases or macro-expanded call sites; a call reached
only through a renamed import is a known limitation of this check, not a
silent pass. T12-A and T12-D do not exclude a call pattern's text when that
text appears inside a comment or a string literal, rather than as real code;
a match inside either is a known limitation of those rules (a possible false
positive), not a resolved parse. T12-B and T12-C exclude comments, because a
false positive outside a debt-list function fails them (Behavior, "T12-B and
T12-C: shipped code and debt lists"); a match inside a string literal
remains a known limitation of all four rules. The check's own report SHALL
state these limitations (#249 review, MEDIUM-5).

### Honest reporting of real findings

Running the check against QSL's real source tree at any commit SHALL report
whatever it finds, including a pre-existing violation ADR-013's own text
already names as known architecture debt (OBS-018, `value/model_query.rs`'s
`to_object_reference`, `bridge_lookup_key` and `resolve_target` functions),
and any real call site the check finds
that OBS-018's or ADR-011 §1's text does not enumerate. The check SHALL NOT
be tuned to exclude a known finding, or widened to admit an unwanted one, to
make a run pass (#249 review R1). T12-B's and T12-C's debt lists are not
such an exclusion: each entry is reported as debt in the check's output, and
each list can only shrink.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-060-AC-1 | A rule whose required path does not exist reports `pending` with the missing path named, and contributes no call-site scan. | Test (TC-157) |
| FR-060-AC-2 | A rule whose required path exists and has no call site outside its allowed callers reports `passing`. | Test (TC-157) |
| FR-060-AC-3 | A rule whose required path exists and has a call site outside its allowed callers reports `failing`, naming the call site's file, line and module; a caller module that is a textual prefix but not a `::`-segment descendant (for example `model_query` under an `model` allow-list) is not treated as allowed. | Test (TC-157) |
| FR-060-AC-4 | Run against real QSL source at head, rule T12-A reports pending (the `replay` facade module does not exist yet); rules T12-B and T12-C each report every shipped mint outside their allowed callers (`check` and its descendants for T12-B, `model` and its descendants for T12-C), and each fails if such a mint lies in a function not on that rule's debt list (Behavior, "T12-B and T12-C: shipped code and debt lists") or if a debt-list entry has no remaining mint; each debt-list mint is reported as debt, with file, line, module and function. Mints under the allowed callers (including `check::family`'s `mint_declaration_identity` and `mint_call_identity`), mints in `#[cfg(test)]` items, and matches inside comments are not reported. A reference to the constructor passed as a function value (`.map(NodeKey::from_digest)`) is a mint. **Amended by the layer-rule ruling (2026-09-22)**: the fixed site counts for T12-B and T12-C are replaced by the named debt lists. Rule T12-D reports passing with zero call sites (no module outside `model` calls `PopulationId::from_digest(`). | Test (TC-157) |

## Dependencies

- ADR-011 §7.1 T-12 and §6.1 layer K
  (`ix://agent-ix/quire-spec-language/ADR-011`).
- ADR-013 O-04, O-05, the O-13 Population row (QC-21) and OBS-018
  (`ix://agent-ix/quire-spec-language/ADR-013`).
- [#213](https://github.com/agent-ix/quire-spec-language/issues/213) owns the
  `quire-exact` kernel crate and updates this rule set's targets once it
  lands.
- [FR-059](FR-059-check-backend-dependency-direction.md) establishes the
  crate-level FB-05 exception this requirement's T12-A rule verifies at the
  module level.

## Status

Specified and implemented under
[#215](https://github.com/agent-ix/quire-spec-language/issues/215) as the
`arch-lint api-surface` subcommand (`tools/arch-lint/api_surface.rs`), with a
per-rule role (`--qsl`/`--cg`) and a `node_key_of(` call pattern added at
#249 review (R1, HIGH-2/MEDIUM-4). T12-A is CG-role and pending (the `replay`
facade does not exist; once it lands, `arch-lint api-surface` needs `--cg
<checkout>` to scan it -- the Makefile's `CG_CLONE` variable). T12-C is
QSL-role and live; its only shipped mints outside `model`, on `main` at
`dccf9175` and on #335's branch alike, are its two debt-list functions
(`src/value/model_query.rs` lines 124 and 156). T12-D is QSL-role, live, and passes
with zero call sites: `quire-exact`'s `PopulationId::from_digest` (QSL-131
Slice B) has no caller outside `model` (tests
`tc_arch_lint_api_surface_012_population_id_disallowed_caller_is_a_violation`
and `tc_arch_lint_api_surface_013_population_id_allowed_caller_is_not_a_violation`
back its failing and passing paths).

T12-B is QSL-role and live. Its shipped mints outside `check`, on `main` at
`dccf9175` and on #335's branch alike, are in exactly the eight debt-list
functions (Behavior, "T12-B and T12-C: shipped code and debt lists"). On
#335's branch they are `value/enumeration.rs` 126 and 171, `value/unit.rs`
207 and 320, `value/expression/family.rs` 232, `value/model_query.rs` 109,
and `value/node.rs` 198 (`.map(NodeKey::from_digest)`) and 282.

**Amended by the layer-rule ruling (2026-09-22); not yet implemented.**
`tools/arch-lint/api_surface.rs` still carries T12-B's former allow-list
(`value::expression::check`, `check::checked_dispatch`, `value::node`) and
patterns (`NodeKey::of(`, `NodeKey::from_bytes(`, `node_key_of(`) that omit
`from_digest` and `from_hex` and require an opening parenthesis, scans
`#[cfg(test)]` code and comments, and has no debt lists. Remediating the
debt-list sites is #211/#213's work, not this requirement's. Remaining work:
#211, gate rewrite.
