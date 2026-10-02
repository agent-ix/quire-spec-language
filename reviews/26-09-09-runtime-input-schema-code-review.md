---
id: SR-284
title: "Code and Rust review of the runtime input schema"
type: SpecReview
analysis: code-review
scope: "f4679ef against 6d7de6a; schema, runtime_reading schema tests and FR-024"
review_set: subset
---
## Summary

PR-readiness author review applied the actual agent-skills/code-review,
rust-review and rust-style skills. The companion schema covers existing wire
shapes; production decoding and its limits remain unchanged.

## Verdict

**PASS** — no findings in this change. Independent PR review remains required.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Checks

Reviewed the schema against public records, private wire types, IR identifier
constructors and digest codec. Closed records require every field, including
nullable result; oneOf pairs envelope kinds with bodies and all ten variants.
Vector order/multiplicity remain intact. Integer/index/revision bounds retain
exact JSON integer literals. Identifier expressions refuse final line separators;
digest length and alphabet are exact. Every schema reference is local.

Four new traced Rust tests exercise actual producers, schema validation and
readers. Populated records and exact variant/observation sets prevent vacuity;
every populated object is tested for unknown/missing/mistyped fields and arrays.
Original-byte duplicate/numeric-token controls assert their owning stage and
typed cause. Stale selection and cyclic construction stay separate refusals.
The positive u32 maximum is only a scalar decode, not a valid arena claim.

Clones/indexing/unwraps are confined to bounded test fixtures. No production
panic, unsafe code, unchecked conversion, recursion, async, shared mutable state,
mock seam, skip or warning suppression was added. Schema drift is checked at
the real producer boundary; no generator/dependency was introduced for this
small companion format. Reverse discovery found no unowned changed behavior
or source/test stub. Cargo/lock and CI diffs are empty; both feature lanes and
manual-dispatch triggers remain.

## Local verification

Locked/offline Cargo reused target, nice 10, one job and one test thread:

| Gate | Result |
| --- | --- |
| All-target strict Clippy, all/minimal features | Both pass |
| Tests, all features | 357 ordinary + 3 compile-fail doctests pass; 4 existing ignored |
| Tests, minimal features | 341 ordinary + 3 compile-fail doctests pass; 4 existing ignored |
| Runtime reader target | 12 pass in both full suites |
| Cached minimal binaries/config_version_fixtures build | Pass |
| Formatting | Pass |

No deny.toml is installed. Logs: /tmp/agent-a-runtime-schema-*. Optional ignored
assurance lanes were not claimed as executed; no hosted CI ran. SR-276–283 carry
the all-set specification review. The advisor failure and pending installed
catalog adoption do not negate the actual Rust run results.
