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

The `quire-exact` kernel crate ADR-011 §6.1 layer K names does not exist yet
([#213](https://github.com/agent-ix/quire-spec-language/issues/213) S-1). This
requirement therefore checks each rule against its *current* call-site
location, so the rule set is live data #213 updates in place once the kernel
crate exists, rather than a check that is retired and rewritten.

## Inputs

- A rule set: for each rule, the textual call pattern(s) that identify a call
  to the constructor or facade the rule protects, the module path prefix(es)
  a caller is allowed to have, and the file path the rule's target currently
  requires to exist (so a rule whose target has not landed yet is
  distinguishable from a rule whose target exists and is violated).
- The QSL source tree under `src/`.

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
call patterns, allowed caller module prefixes, a required file path, and a
pending-reason string), so #213 can update the call patterns, allowed
prefixes and required path in place when `quire-exact` lands, without
changing the check's evaluation logic.

At the time of this requirement, the rule set SHALL contain exactly these
three rules:

| Rule | Protects | Allowed callers (today) | Requires |
| --- | --- | --- | --- |
| T12-A | The layer-6 `replay` facade module | (CG is a separate crate; this rule reports pending until the facade module exists at the path this rule names) | `src/replay.rs` |
| T12-B | The kernel `NodeKey` constructor (ADR-013 O-04) | `value::expression::check`, `model::checked_dispatch`, `value::library` (ADR-011's own "today" mapping for the S3 `check` stage) | the constructor's current source file |
| T12-C | The kernel `EffectiveId` constructor (ADR-013 O-05) | `model` | the constructor's current source file |

### Pending vs. live vs. failing

If a rule's required file path does not exist in the source tree under
inspection, the check SHALL report that rule **pending**, naming the missing
path, and SHALL NOT scan for call sites under that rule.

If a rule's required file path exists, the check SHALL scan every `.rs` file
under `src/` for the rule's call pattern(s) and SHALL report **failing**,
naming each call site's file, line and enclosing module, for every call site
whose enclosing module path is not exactly one of the rule's allowed prefixes
or a descendant of one (matched on a `::`-segment boundary, so
`model_query` is not treated as inside `model`).

A rule with no call site outside its allowed callers SHALL report **passing**.

### Scanning method and its stated limitation

The check SHALL scan QSL source text for each rule's literal call patterns.
It does not resolve import aliases or macro-expanded call sites; a call
reached only through a renamed import is a known limitation of this check,
not a silent pass -- the check's own report SHALL state this limitation.

### Honest reporting of real findings

Running the check against QSL's real source tree at any commit SHALL report
whatever it finds, including a pre-existing violation ADR-013's own text
already names as known architecture debt (OBS-018,
`value/model_query.rs:108,123,155`). The check SHALL NOT be tuned to exclude
a known finding to make a run pass.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-060-AC-1 | A rule whose required path does not exist reports `pending` with the missing path named, and contributes no call-site scan. | Test (TC-157) |
| FR-060-AC-2 | A rule whose required path exists and has no call site outside its allowed callers reports `passing`. | Test (TC-157) |
| FR-060-AC-3 | A rule whose required path exists and has a call site outside its allowed callers reports `failing`, naming the call site's file, line and module; a caller module that is a textual prefix but not a `::`-segment descendant (for example `model_query` under an `model` allow-list) is not treated as allowed. | Test (TC-157) |
| FR-060-AC-4 | Run against real QSL source at head, rule T12-A reports pending (the `replay` facade module does not exist yet), and rules T12-B and T12-C report the exact real call sites ADR-013 OBS-018 already names. | Test (TC-157) |

## Dependencies

- ADR-011 §7.1 T-12 and §6.1 layer K
  (`ix://agent-ix/quire-spec-language/ADR-011`).
- ADR-013 O-04, O-05 and OBS-018
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
`arch-lint api-surface` subcommand (`tools/arch-lint/api_surface.rs`). T12-A is
pending (the `replay` facade does not exist); T12-B and T12-C are live and
currently fail against real QSL head at the OBS-018 locations, which is
pre-existing, already-tracked debt this requirement does not remediate.
