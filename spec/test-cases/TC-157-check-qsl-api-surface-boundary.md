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
reproduces the real, currently pending/failing state of T12-A/T12-B/T12-C
against QSL's own head. Scope: FR-060-AC-1 through FR-060-AC-4.

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
  yet). T12-B and T12-C report `failing`, naming the exact call sites ADR-013
  OBS-018 already documents (`value/model_query.rs:108,123,155`). This output
  is captured for the PR body as real, not synthetic, evidence -- the check
  is not tuned to exclude this known finding.

## Metadata

- Priority: P1
- Target Integration: `tools/arch-lint/api_surface.rs`
- Automation: Automated Rust unit tests plus one manual real-data run (step 5)

## Dependencies

**Upstream:** [FR-060](../functional/FR-060-check-qsl-api-surface-boundary.md).
**Downstream:** none.
