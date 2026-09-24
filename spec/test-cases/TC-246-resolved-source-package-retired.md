---
id: TC-246
title: "ResolvedSourcePackage is retired, with no dangling caller"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-246: ResolvedSourcePackage is retired, with no dangling caller

## Description

Verify that `ResolvedSourcePackage` (`qsl-semantics/src/complete/package.rs`, complete-V1
lane C2) and `complete::resolve_source_package` are removed in one change,
made once both of their successors exist, with no compatibility alias,
feature-flagged fallback, or wrapper keeping the old name or its
construction path reachable, and no caller left referencing it. Scope:
FR-087-AC-7, FR-087-CON-4.

The two successors (FR-087 Behavior, "`ResolvedSourcePackage` retires when
both successors exist"; ruling on QSL-229):

- **Dependency half.** `import` selections resolve in layer-3 `library`
  against I2 import views (`VerifiedPackage`, `ImportView`) that the M-4 I2
  reader produces. Ticket: QSL-6 (ADR-011 M-4).
- **Header-selection half.** `profile`, definition and `model` selections
  resolve in E3's identity-preimage builder, through the QSpec value-lock
  accessor and the domain packages admitted at I1 (ADR-011 §2.4). Gated on
  QSL-189 (the QSpec lock accessor the builder reads).

## Test Procedure

1. Search the whole compiled crate (`src/`, `tests/`, `examples/`) for every
   occurrence of `ResolvedSourcePackage` and `resolve_source_package` by
   name.
2. Search for any re-export, type alias, or `pub use` naming the removed
   type or function under any spelling.
3. Build the crate. A build that succeeds with `ResolvedSourcePackage`
   reachable under any conditional compilation flag is a duplication
   finding, not a pass (the TC-170/TC-246 shared standard).
4. For every former caller of `resolve_source_package`, and every scenario
   its tests cover, confirm the successor for each half: its `import`
   selections resolve in `library` against a `VerifiedPackage`/`ImportView`
   pair, and its `profile`, definition and `model` selections resolve in
   E3's identity-preimage builder. A `model` scenario runs on a path that
   admits the selected domain package at I1, since spine `compile` admits
   none and refuses a `model` declaration (ADR-011 §2.4). The set of covered scenarios is the same
   or larger; no test is deleted without a replacement covering the same
   scenario against the successor for its half.

## Expected Results

- Steps 1-2: zero occurrences of `ResolvedSourcePackage` or
  `resolve_source_package` anywhere in the compiled crate, its tests, or
  its examples.
- Step 3: the crate builds with no reachable definition of the old type
  under any feature combination.
- Step 4: every former caller's scenario has an equivalent against the
  successor for its half: import scenarios against `library`'s I2 import
  views (QSL-6), and profile, definition and model scenarios against E3's
  identity-preimage builder (QSL-189). A scenario silently dropped rather
  than moved fails this step.

## Status

Planned; no test backs this case. It is gated on QSL-6 and QSL-189 and
runs once both have landed their successor. Until then
`ResolvedSourcePackage` stays in place. See FR-087 Status, AC-7 and CON-4.
