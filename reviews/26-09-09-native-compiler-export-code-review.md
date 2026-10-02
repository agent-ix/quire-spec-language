---
id: SR-183
title: "Code and Rust review of standalone compiler export"
type: SpecReview
analysis: code-review
scope: "src/command; src/command.rs; src/wire_format.rs; src/cli.rs; src/main.rs; tests/compile_command.rs"
review_set: subset
---
## Summary

Author PR-readiness review of f0cfe7b using actual code-review, rust-review and
portable rust-style skills. No applicable AssuranceProfile or deny.toml exists.

## Verdict

**CONDITIONAL** — requested compiler/dispatch corrections are implemented;
broader format schemas and public command naming remain follow-up work.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The format catalog now owns all six selectors, but dedicated schemas still cover only linked packages and native results. Do not claim complete wire-schema coverage. | FR-024; FR-025; FR-027 |
| FND-002 | low | RunError/RunCause still name the shared command error API after its first command; retain compatibility until the cross-cutting error work in issue 27. | FR-026; FR-027 |

## Checks

Reviewed the closed request types and their associated format constants, one
Command parser and shared stream handling, named file-count charges, and compiler
functions separated from file intake. Error details use typed serialization.
The output copy deliberately outlives local model ownership. No new unsafe,
panic, dependency, unchecked cast or hosted workflow change was introduced.

Six compile tests include exact producer-byte identity and verified reread from
a directory containing only selected sources, exhaustive typed adverse cases,
command-specific usage, file-group exhaustion and actual Linux /dev/full output.
The reported ENOSPC-success defect did not reproduce: non-BrokenPipe errors exit
2. The documented quiet BrokenPipe policy is retained.

At f0cfe7b the full local suite passed 300 tests and three compile-fail doctests,
with four existing assurance tests ignored. All 17 focused command tests,
formatting, strict all-targets/all-features Clippy, cached no-default-features
build and warnings-denied rustdoc passed. Cargo phases ran serially with one
build job. No hosted CI ran. This is author review, not independent acceptance.
