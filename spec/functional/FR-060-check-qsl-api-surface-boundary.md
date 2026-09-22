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
  allowed caller modules exists; each is reported with file and line), or
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
| T12-B | Qsl | The kernel `NodeKey` constructor (ADR-013 O-04) | `value::expression::check`, `check::checked_dispatch` (**amended by FR-074, ADR-011 §7.3 M-2, QSL-7, 2026-09-21: was `model::checked_dispatch`, moved to `check` by M-2, realising ADR-011:694-697's "only `check` calls the kernel `NodeKey` constructor"**), `value::library` (ADR-011's own "today" mapping for the S3 `check` stage), and `value::node` (the crate-internal helper `node_key_of`'s own defining module -- #249 review R1; see Status) | the constructor's current source file |
| T12-C | Qsl | The kernel `EffectiveId` constructor (ADR-013 O-05) | `model` | the constructor's current source file |
| T12-D | Qsl | The kernel `PopulationId` constructor (ADR-013 O-13 Population row, QC-21; ADR-011 T-12(d)) | `model` | `src/model/population.rs` |

A rule's call patterns SHALL include every textual spelling that constructs
the protected value, not only its primary constructor name: T12-B's patterns
are `NodeKey::of(`, `NodeKey::from_bytes(` and `node_key_of(` (the
crate-internal helper that wraps the constructor -- #249 review R1), so a
caller that mints a `NodeKey` only through the helper is still scanned.
T12-D's pattern is `PopulationId::from_digest(`, the kernel `PopulationId`'s
one public constructor.

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
silent pass. It also does not exclude a call pattern's text when that text
appears inside a comment or a string literal, rather than as real code; a
match inside either is a known limitation of this check (a possible false
positive), not a resolved parse. The check's own report SHALL state both
limitations (#249 review, MEDIUM-5).

### Honest reporting of real findings

Running the check against QSL's real source tree at any commit SHALL report
whatever it finds, including a pre-existing violation ADR-013's own text
already names as known architecture debt (OBS-018,
`value/model_query.rs:108,123,155`), and any real call site the check finds
that OBS-018's or ADR-011 §1's text does not enumerate. The check SHALL NOT
be tuned to exclude a known finding, or widened to admit an unwanted one, to
make a run pass (#249 review R1). At the time of this requirement, running
T12-B against QSL's real head reports five real call sites, not the three
ADR-011 §1's S3 "today" mapping names for the `check` stage -- see Status for
the exact sites and the discrepancy this surfaces.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-060-AC-1 | A rule whose required path does not exist reports `pending` with the missing path named, and contributes no call-site scan. | Test (TC-157) |
| FR-060-AC-2 | A rule whose required path exists and has no call site outside its allowed callers reports `passing`. | Test (TC-157) |
| FR-060-AC-3 | A rule whose required path exists and has a call site outside its allowed callers reports `failing`, naming the call site's file, line and module; a caller module that is a textual prefix but not a `::`-segment descendant (for example `model_query` under an `model` allow-list) is not treated as allowed. | Test (TC-157) |
| FR-060-AC-4 | Run against real QSL source at head, rule T12-A reports pending (the `replay` facade module does not exist yet); rule T12-C reports failing at exactly the two OBS-018 sites (`value/model_query.rs:123,155`); rule T12-B reports failing at exactly five real sites -- `value/enumeration.rs:117,162`, `value/unit.rs:198,311` and the one OBS-018 site `value/model_query.rs:108` -- none of which the check excludes, and none of which are the three modules ADR-011 §1's S3 "today" mapping names for the `check` stage; rule T12-D reports passing with zero call sites (no module outside `model` calls `PopulationId::from_digest(`). | Test (TC-157) |

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
QSL-role, live, and fails at exactly the two OBS-018 locations
(`src/value/model_query.rs:123,155`). T12-D is QSL-role, live, and passes
with zero call sites: `quire-exact`'s `PopulationId::from_digest` (QSL-131
Slice B) has no caller outside `model` (tests
`tc_arch_lint_api_surface_012_population_id_disallowed_caller_is_a_violation`
and `tc_arch_lint_api_surface_013_population_id_allowed_caller_is_not_a_violation`
back its failing and passing paths).

T12-B is QSL-role, live, and fails at exactly five real sites:
`src/value/enumeration.rs:117,162`, `src/value/unit.rs:198,311` and the one
OBS-018 site `src/value/model_query.rs:108`. Each is a call to the
crate-internal helper `node_key_of`, which itself calls `NodeKey::of`
directly from its own defining module, `src/value/node.rs`; #249 review R1
exempts that defining module from T12-B's allowed-caller list (a helper
calling the constructor it wraps, from the module that defines it, is not a
foreign caller), so the check reports the helper's five real *callers*, not
the helper's own internal call. This is real, new information this check
surfaces, not a false positive tuned away: none of `value::enumeration`,
`value::unit` or `value::model_query` is among the three modules ADR-011
§1's S3 "today" mapping names for the `check` stage
(`value::expression::check`, `check::checked_dispatch` (amended by FR-074:
was `model::checked_dispatch`, moved to `check` by ADR-011 §7.3 M-2), `value::library`) --
a discrepancy between what ADR-011 §1 documents as today's minting callers
and what real QSL head contains. This requirement reports that discrepancy;
it does not resolve it, and does not remediate any of these five sites or
the two OBS-018 `T12-C` sites -- that is #211/#213's remediation, not this
requirement's. Remaining work: #211.
