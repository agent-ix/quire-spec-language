---
id: SR-20261105101030
title: "Gap analysis of QSL-1751 frame-clause test helper refactor"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-semantics/tests/it/state_clauses.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/spec/spec.md
    type: references
---

## Summary

This scoped gap analysis examined the QSL-1751 acceptance criteria against the touched integration-test helpers and available head-specific gate evidence. The change is test-only and does not add or alter product behavior. The repository-wide computed Test Matrix could not be obtained: `quoin matrix --repo <worktree> --json` stalled without output for more than five minutes and was interrupted. Manual checks of this change found no criteria/test/code mismatch; this is not a repository-wide matrix claim.

Ticket: QSL-1751.

## Verdict

CONDITIONAL. The changed test-helper scope has no observed gap, and the recorded gates are green, but repository-wide matrix verification is incomplete because the matrix command stalled. Plan completion: not assessed.

## Findings

| ID      | Severity | Summary                  | Refs |
| ------- | -------- | ------------------------ | ---- |
| FND-001 | low      | No findings (placeholder) | - |

## Coverage

- **Criteria inspected:** QSL-1751 AC1 and AC2. AC1 requires removing the lint allowances and passing both Clippy modes; AC2 requires the same tests to pass with the same count. Ticket evidence on head `230e475cbe73f614447556aee4d610956f0de288` reports both Clippy modes and the 135-test focused integration suite passing.
- **Manual diff/test trace:** only `qsl-semantics/tests/it/state_clauses.rs` changed. All six target-file allowances are gone; no test or `#[trace]` attribute was added or removed (135 tests before/after; 132 trace attributes before/after). The replacements package the prior helper arguments and call sites in borrowed input structs. No production source changed, so this diff introduces no new product behavior requiring an owner criterion.
- **Matrix limitation:** `quoin matrix --repo /home/peter/dev/worktrees/qsl-1751-frame-clause-cleanup --json` produced no output for over five minutes and was interrupted. No computed-matrix conclusion is claimed. The manual trace is scoped to this PR and its ticket ACs.
- **Reverse-gap/stub review:** the only changed file is an existing integration-test helper file; the diff adds no production behavior, stub, or new externally visible contract.
- **Optional semantic review:** skipped; it was not requested. No plan was supplied; plan completion was not assessed.

## Scope examined

- QSL-1751 AC1 and AC2, as written on the ticket.
- `qsl-semantics/tests/it/state_clauses.rs` diff and relevant test helper/call-site structure.
- Existing head-specific Linear gate evidence for focused tests, Clippy, string-edge, and combined workspace check.
