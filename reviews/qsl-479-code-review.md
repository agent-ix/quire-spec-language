---
id: SR-1199
title: "Code review of quire-spec-language PR #590: NodeKey::decode_admitted and arch-lint T12-F"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@def95b6896e0ff83303cb9e0329cb443d00823f7; PR #590 diff against origin/main: quire-exact/src/node.rs, tools/arch-lint/api_surface.rs, qsl-package/src/checked_v2/tests.rs, spec/functional/FR-060-check-qsl-api-surface-boundary.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md"
review_set: subset
---
# Code review of quire-spec-language PR #590

## Summary

Ticket: QSL-479. The PR adds `NodeKey::decode_admitted` to `quire-exact`. It
hashes nothing and wraps 32 bytes, like `from_digest`. It adds arch-lint rule
T12-F, which allows `decode_admitted` only in `qsl-package`'s `checked_v2`, and
updates the FR-060 rule table and ADR-011 T-12 to match.

- Rust lane (rust-review): no panics on the production path and no unsafe code.
  The new public fn matches the `from_digest` shape. Doc comments state the
  precondition and the allow-list.
- The T12-F rule entry mirrors T12-E (crate plus module allow-list, `shipped_only`,
  no debt list). `tc_arch_lint_api_surface_027` checks the bare path as a call and as
  a function value, a `checked_v2` in another crate, and that a `from_digest` in
  `checked_v2` still fails T12-B. These oracles are strong.
- Gates run at this head with a worktree-local target dir: `cargo test -p
  quire-exact -p qsl-package -p arch-lint -p xtask` passed, and `cargo clippy
  --all-targets -D warnings` on quire-exact, qsl-package and arch-lint was clean.

The spec contradiction around the T12-F allow-list and the test-oracle point
are recorded in the gap analysis (SR-1200), not here.

## Verdict

Clean for code-review and rust-review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
