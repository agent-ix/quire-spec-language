---
id: SR-213
title: "Code and Rust review of the actual Quire consumer"
type: SpecReview
analysis: code-review
scope: "src/quire_source.rs; src/quire_source/preflight.rs; src/source.rs; tests/quire_source.rs; optional dependency and workflow"
review_set: subset
---
## Summary

Author PR-readiness review of 55649b3 using actual code-review, rust-review and
rust-style skills. No applicable AssuranceProfile or deny.toml exists.

## Verdict

**CONDITIONAL** — required review corrections pass; shared identity grouping
continues at the command wire boundary in PR23.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Style follow-up: native/formal body identities remain separate selection fields; group them with the command wire identity records when adopting PR23. | src/quire_source.rs:33 |

## Checks

The workflow retains minimal-feature lint/test lanes and adds separate enabled
lanes; workflow_dispatch remains its only trigger. All four unresolvable IT
scenario tags were removed, retaining FR/TC attributes and a feature note in TM-007.

Named constants own both protocol versions and hard input ceilings. Typed
preflight causes carry actual/expected values for source identity, path, package,
contract version and semantic-core skew; byte/line budget failures retain measured
counts and effective limits. Tests distinguish each variant before extraction.
Clause selection, fence geometry, exact byte verification and map construction
are separate bounded helpers. Language is borrowed from the actual extraction.
Quire-owned result/context types are deliberately re-exported with pin ownership.
No new dependency, parser, unsafe block, request panic or concurrent work was added.

At 55649b3 both full configurations passed: 316 ordinary tests with all features
and 311 with minimal features, each plus three compile-fail doctests and four
existing ignored assurance tests. The five real extraction tests retain exact
LF/CRLF/Unicode provenance, actual runtime truth/refusal and fresh retry. Strict
all-targets Clippy passes in both configurations; formatting, cached minimal build,
warnings-denied rustdoc and scoped Quire validation pass. Cargo phases ran serially
with one build job. No hosted CI ran. This is author review.
