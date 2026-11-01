---
id: SR-202610310156
title: "Focused gap analysis of QSL-518 PR #2042 AC10"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1f6bb469dd75a3de1bf8f8e87eee900f6bb11f3a; PR #2042 diff: xtask/tests/refinement_layering.rs; FR-345-AC-10; TC-868"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-345
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-868
    type: reviews
---

## Summary

Ticket: QSL-518. Focused manual gap analysis of the test-only PR #2042 at `1f6bb469dd75a3de1bf8f8e87eee900f6bb11f3a`. Plan completion: not assessed. Every changed test behavior is owned by FR-345-AC-10 and tagged `TC-868` plus that criterion. The test calls the real corpus generator, the FR-278 compile composition, and QSL's production registry; it compares the no-negotiation report byte-for-byte with the CLI and verifies negotiated results and per-candidate request cardinality. The AC's codegen dependency condition was checked with `cargo tree` and is clear. No code behavior was introduced without a requirement.

The full repository matrix could not be assessed in this run: `quire matrix --scope . --format json` emitted module and archetype diagnostics and remained active beyond 90 seconds, so it was interrupted. The focused criterion-to-test check below is not a repository-wide matrix claim.

## Verdict

**CONDITIONAL** — within the PR's one-criterion scope, the test is tagged and covers the reports and candidate requests, but it does not assert the required empty stdout/stderr for the two direct `compare` calls. Plan completion: not assessed.

## Scope

- `FR-345-AC-10`: examined every acceptance obligation changed by this PR.
- `TC-868`: examined the tagged test-case mapping.
- `xtask/tests/refinement_layering.rs`: examined the only changed path for unowned behavior and hollow or misplaced test coverage.

## Coverage

- `FR-345-AC-10` → `production_comparison_matches_cli_and_negotiates_each_witness`, tagged `TC-868` and `FR-345-AC-10`.
- The report comparison and request cardinality are asserted. The process-stream assertion for the two direct core calls is missing; the test only asserts the spawned CLI child's stderr is empty.
- No production implementation changed; no reverse gap was introduced by this test-only diff.
- Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The test does not capture stdout/stderr around either direct `compare` call. It checks only the child CLI's stderr and the returned report, so a stream write from either core call could pass, contrary to AC10 and TC-868 step 9. Add an assertion that both direct calls leave both process streams empty. | xtask/tests/refinement_layering.rs:173-217 |
