---
id: SR-2026110216376
title: "QSL-1517 code review"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quire-spec-language; qsl-forms/src/dispatch.rs, qsl-replay/src/call_site.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/lifecycle.rs, qsl-replay/src/spine/lifecycle/tests.rs, qsl-replay/src/spine/refinement.rs"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: reviews
---
# QSL-1517 code review

## Summary

Ticket: QSL-1517. PR: #2178. Manual Rust review completed; model-assisted `/code-review` was attempted but could not inspect the diff because the app-server nested shell is blocked by the socket-directory permissions check.

## Verdict

Not mergeable yet. The admission checks mostly match the requested data flow, but the header conflict refusal lacks the source identities and header regions needed to meet FR-480-AC-4's required diagnostic. Sibling-import refusal also omits its source locus, unlike FR-099.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-60329 | medium | Header conflicts report only unit indices, so the refusal cannot locate the later header or name both headers as AC-4 requires. | qsl-replay/src/spine/lifecycle.rs:469-472; qsl-replay/src/spine.rs:487-494 |
| FND-60330 | medium | Sibling-import refusal retains only the import identity and sibling index, so it cannot carry the offending import's source region as FR-099 requires. | qsl-replay/src/spine/lifecycle.rs:484-488; qsl-replay/src/spine.rs:515-518 |

## Review method limits

`codex exec review` was run from a writable detached worktree with a writable CODEX_HOME, but its nested sandbox rejected shell commands with “app-server socket directory must be a user-owned directory with mode 0700.” Quoin matrix and `quire matrix` were started; both remained silent beyond the timeout window and were stopped. `git diff --check origin/main...HEAD` passed. This is a manual Rust review and manual acceptance-to-test check, not a completed model-driven review or Quoin run.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-60329 | fixed | HeaderConflict now carries both unit identities and spans, and the second header's SourceRegion is exposed through CompileRefusal::region(). The test checks both identities and the second source region. |
| FND-60330 | fixed | SiblingImport now carries the sibling SourceIdentity and the importing unit's SourceRegion at import.identity_span; the focused test checks the source and non-empty region. |
