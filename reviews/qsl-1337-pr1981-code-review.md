---
id: SR-1957108
title: 'QSL-1337 PR #1981 code-review: final IR pin'
type: SpecReview
analysis: code-review
date: '2026-11-01'
scope: 'agent-ix/quire-spec-language; PR #1981 Cargo.lock; frozen f6a49669e9fbaad6c18feebff88be352fd12a0dd'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-264
  type: references
---

## Summary

Ticket: QSL-1337. Frozen head `f6a49669e9fbaad6c18feebff88be352fd12a0dd`; exact base `4bf9292c88e14a26d1f61b116a46d815fdd92355`. Scope: the two-hunk Cargo.lock pin only, excluding unrelated merged main. No applicable AssuranceProfile found in spec/. No new build or test execution, per dispatch.

The entire actual upstream delta 867cb547 to 154227dc is three additive IR-1194 Exact projection commits (#744, #746, #747). Typed-read #741 and linear Debug #740 are already ancestors of the old 867cb547 pin. Existing reader/Debug code is retained. New projection helpers are reached through the new projection method; no QSL package/replay/root caller invokes it. The new direct IR-to-Exact dependency matches the upstream crate manifest and resolves to the one existing Exact source, 82e8f9b4d9583ba4eee505550ff80a211333696f.

Evidence inspected: /home/peter/dev/worktrees/qsl-1337-final-pin-logs/run-context-after-1980.txt and clippy-after-1980.log. Command: cargo clippy --locked -p qsl-package -p qsl-replay -p quire-spec-language --all-targets --all-features -- -D warnings. Exit 0; elapsed 4:14.63; maximum RSS 1,274,736 KiB; one job; 20 GiB cap; no swap. Context records successful diff, fmt and locked no-deps metadata checks. This is compile/lint evidence, not test execution or full CI success.

## Scope

```yaml
scope:
- id: Cargo.lock:quire-contract-model
  path: Cargo.lock:1655
  role: examined
  excerpt: source = "git+https://github.com/agent-ix/quire-contract-ir?branch=main#154227dcbcda09f3abf635375777893132fc7686"
- id: Cargo.lock:IR-exact-edge
  path: Cargo.lock:1665
  role: examined
  excerpt: ' "quire-exact",'
- id: Cargo.lock:quire-exact
  path: Cargo.lock:1692
  role: examined
  excerpt: source = "git+https://github.com/agent-ix/quire-exact?branch=main#82e8f9b4d9583ba4eee505550ff80a211333696f"
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS for the lock-only pin. Rust-review is folded into this artifact. The dependency is shared rather than vendored, with one IR and one Exact locked source. No source, manifest, feature, cap, warning suppression, skip, assertion or CI gate changes occur. Existing-callsite compilation and unchanged reader/Debug paths support compatibility.

## Coverage

Read AGENTS.md, CLAUDE.md, CONTRIBUTING.md, workspace manifests/lints and deny.toml; Agent IX Rust API/testing guidance; the entire diff; upstream manifests and changes to existing source paths; additive projection entrypoints; QSL reader/Debug callers. No fresh gates were run as instructed. Source redesign/concurrency/unsafe/wire changes and spec-review are not applicable to this lock-only PR. No full-green CI or QSL-1337 completion claim.
