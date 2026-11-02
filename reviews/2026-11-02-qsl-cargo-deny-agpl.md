---
id: SR-202611020540
title: "Code review — QSL cargo-deny license policy"
type: SpecReview
analysis: code-review
scope: "quire-spec-language; deny.toml, Cargo.toml, LICENSE"
review_set: subset
---

## Summary

Reviewed the explicit cargo-deny license policy against QSL's declared license and the resolved workspace dependency graph. The policy allows QSL's AGPL identifier while retaining the graph's existing permissive licenses; the focused license and ban checks pass.

## Verdict

**PASS** — the allowlist addresses the reproduced AGPL rejection without weakening the existing bans policy or removing licenses required by the resolved graph.

## Findings

| ID      | Severity | Summary                       | Refs |
| ------- | -------- | ----------------------------- | ---- |
| FND-001 | low      | No findings (placeholder)     | -    |

## Verification

- Before the change, `cargo deny check licenses` rejected `AGPL-3.0-or-later` for QSL's AGPL-licensed Agent IX dependencies.
- After the change, `cargo deny check licenses` passed through `locked-build.sh`.
- `cargo deny check bans` passed through `locked-build.sh`.
- `git diff --check` passed.

## Gap Analysis

No behavioral or spec requirement changed; this review covered license policy only. The license gate itself provided the applicable before/after evidence. No separate spec test-matrix analysis applies.
