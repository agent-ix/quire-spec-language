---
id: SR-4941
title: "QSL #704 code and Rust review disposition"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0e542acffe7604050c4d44a047de82f90cc29773; baseline origin/main@2a90a2df0b29617e7c63aa440d291371efe906b4; qsl-semantics/Cargo.toml"
review_set: subset
---

## Summary

Reassessed the medium dependency-revision finding against the exact PR head and refreshed `origin/main`. Both FCD dependencies already use the same `c620d6be99654a7a77f0ecc2f97d3c7136402651` revision on the baseline, so the concern is not introduced by this PR.

Methods: code-review and rust-review. Quoin gap-analysis was attempted, but `quoin matrix --repo . --json` produced no output and timed out; no Quoin verdict is claimed. No build or tests were run.

## Verdict

**PASS** — the sole previously reported finding is rejected as not a PR delta. This disposition addresses that finding only; it does not represent a fresh full review of every changed file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FCD dependencies use explicit `rev=` pins | qsl-semantics/Cargo.toml:43 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | rejected | Not a PR delta. At refreshed baseline `origin/main` (`2a90a2df0b29617e7c63aa440d291371efe906b4`), lines 43–44 already pin both `agent-ix-extraction-frontend` and `agent-ix-semantic-ir` to `c620d6be99654a7a77f0ecc2f97d3c7136402651`. The PR diff changes the stale merge-base value `68ace480f5c9793290386010563f85896789e61d` to that same current-main value. The PR therefore brings the dependencies into alignment with main; retaining the pin is not a defect attributable to this diff. |
