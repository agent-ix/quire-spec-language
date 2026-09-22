---
id: TC-157
title: "Report pending, passing and failing T-12 API-surface rules"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: verifies
---
# TC-157: Report pending, passing and failing T-12 API-surface rules

## Description

Verify that `arch-lint api-surface` reports a rule whose required path is
absent as `pending` without scanning for call sites, reports a rule with no
disallowed call site as `passing`, reports a rule with a disallowed call site
as `failing` with file, line and module named, respects `::`-segment module
matching (a textual prefix that is not a segment boundary, such as
`model_query` under a `model` allow-list, is not treated as allowed), and
reproduces the real, current pending/failing/passing state of
T12-A/T12-B/T12-C/T12-D against QSL's own head. Scope: FR-060-AC-1 through FR-060-AC-4.

## Test Procedure

1. Run the check with a rule whose required file path does not exist in the
   scanned tree.
2. Construct a small source tree where a rule's required path exists and every
   call to its protected constructor is inside the rule's allowed caller
   module(s). Run the check.
3. Construct a source tree where a rule's required path exists and one call to
   its protected constructor is outside the rule's allowed caller module(s),
   at a known file and line. Run the check.
4. Construct a source tree with a module literally named `model_query`
   containing a call to the T12-C-shaped constructor, under a rule whose
   allowed caller is `model`. Run the check.
5. Run `arch-lint api-surface` against QSL's real source tree at head.
6. T12-B fixtures, each a small source tree:
   - a shipped `NodeKey::from_digest(` call in a `check` submodule;
   - a shipped `.map(NodeKey::from_digest)` in a module outside `check`, in a
     function not on the debt list;
   - a shipped mint in a function that is on the debt list;
   - a debt-list entry whose function no longer mints;
   - a `NodeKey::from_digest(` call inside a `#[cfg(test)]` item outside
     `check`;
   - a doc comment naming `NodeKey::from_digest` in a module outside `check`.
7. T12-C fixture: a shipped `EffectiveId::from_digest(` call in
   `value::model_query`, in a function not on T12-C's debt list.

## Expected Results

- Step 1: the rule reports `pending`, naming the missing required path; no
  call-site scan is attempted or reported for that rule.
- Step 2: the rule reports `passing`.
- Step 3: the rule reports `failing`, naming the call site's exact file, line
  and enclosing module.
- Step 4: the call inside `model_query` is reported as a violation (not
  treated as inside `model`), because segment matching requires a `::`
  boundary, not a textual prefix.
- Step 5: T12-A reports `pending` (the `replay` facade module does not exist
  yet; no `--cg` checkout is given). T12-B and T12-C each report every
  shipped mint outside their allowed callers as debt, each in a function on
  that rule's FR-060 debt list, with file, line, module and function named,
  and each fails if such a mint is in a function not on its list or if a
  list entry has no mint left (FR-060-AC-4). **Amended by the layer-rule
  ruling (2026-09-22)**: fixed site counts are replaced by the debt lists.
  T12-D reports `PASS` with zero call sites: no
  module outside `model` calls `PopulationId::from_digest(`. This output is
  captured for the PR body as real, not synthetic, evidence -- the check is
  not tuned to exclude any finding.
- Step 6: the `check` submodule mint is not reported; the function-value
  mint outside `check` fails T12-B and is named; the debt-list mint is
  reported as debt and does not fail T12-B; the stale debt-list entry fails
  T12-B and names the entry; the `#[cfg(test)]` mint and the doc-comment
  match are not reported.
- Step 7: the call fails T12-C and is named with file, line, module and
  function.

## Metadata

- Priority: P1
- Target Integration: `tools/arch-lint/api_surface.rs`
- Automation: Automated Rust unit tests (steps 1-4, 6, 7) plus one manual real-data run (step 5)

## Dependencies

**Upstream:** [FR-060](../functional/FR-060-check-qsl-api-surface-boundary.md).
**Downstream:** none.
