---
id: SR-1932002
title: "gap analysis of PR #1932 (QSL-1337): rank requirement records once instead of building a BTreeMap over deep keys"
type: SpecReview
analysis: gap-analysis
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1932 git diff origin/main...95843faf8 (qsl-semantics check/claims/map.rs, check/claims.rs, check/mod.rs; callers in qsl-route, qsl-replay, qsl-analyze); FR-062-AC-13"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-062
  type: reviews
---
# SR-1932002: gap analysis of PR #1932 (QSL-1337)

## Summary

Ticket: QSL-1337. A manual criterion-to-test check over the diff. Plan completion: not assessed. The focused claims tests were run (21 of 21 passed). The deep_bodies tests and the 100_000-term tests were skipped as the brief directed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-062-AC-13 | spec/functional/FR-062-implement-checked-family-contract.md:379 | examined |
| map tests | qsl-semantics/src/check/claims/map.rs | examined |
| tc_160_two_sites_of_one_node_pair_by_location_not_order | qsl-semantics/src/check/claims/tests.rs:908 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**Clean for major gaps.** The PR changes no behavior that a spec states. It replaces a container and keeps its order, lookup, equality and Debug. The new unit tests cover each part of that contract against a `BTreeMap` oracle: iteration, keys, values, `IntoIterator`, get, contains_key and Debug; extension that ranks the added records and refuses a key already present without changing the map; and last-wins collection. FR-062-AC-13's "the map holds exactly the records" and "checking the same unit twice gives equal maps" stay covered by the existing TC-160 tests, which pass. The performance claim (16k terms in 0.44 s) is backed by the existing QSL-1337 deep-input tests, which this run did not execute. No production path is left untested.
