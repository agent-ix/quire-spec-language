---
id: SR-105
title: "Native package verification and evidence review"
type: SpecReview
analysis: evidence
scope: "US-002, FR-019/020/021, NFR-007, IT-007, TC-078–091, TM-005 and native package wire/API/schema"
review_set: all
evaluated_revision: "41da6e5eb86bb727fbad5370fd23b38d330bcfea"
review_date: "2026-09-09"
---

## Summary

Ran the installed adviser and method catalog. All 27 FR criteria match at
Test-class level; the five quantified limit metrics recommend benchmarks.
The reviewer retains exact boundary testing for those metrics and selects
independent vectors, generated mutations and actual API integration.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Judgment: offered-byte admission is an exact bound, not elapsed performance; retain negative-abuse-testing despite benchmark advice. | NFR-007-M-1 |
| FND-002 | low | Judgment: emitted-byte charge-before-append needs exact boundaries, not a throughput benchmark. | NFR-007-M-2 |
| FND-003 | low | Judgment: decoded string retention and bounded lexical scratch need adverse byte controls, not elapsed performance. | NFR-007-M-3 |
| FND-004 | low | Judgment: member/element accounting is verified by independently counted boundaries and retries. | NFR-007-M-4 |
| FND-005 | low | Judgment: nesting requires exact depth/recursion classification tests; retain negative-abuse-testing. | NFR-007-M-5 |
| FND-006 | low | Fuzzing is applicable to this new untrusted reader. There is no installed repo fuzz harness; select generated structural/byte adverse tests for this gate and record fuzzing as an unexecuted recommendation. | FR-020-AC-2; TC-083; TC-088 |
| FND-007 | low | Identity roles and package lineage need exact runtime observations; lexical temporal/invariance advice does not introduce a temporal model or monitor. | FR-019-AC-1; FR-019-AC-2; FR-020-AC-10; FR-021-AC-2 |

## Deterministic evidence and judgment

Complete quoin advise --json and quoin catalog methods --json outputs are in
data/advice.json and data/methods.json. The initial sandboxed adviser failed
to identify its Quire subprocess version; its stderr is preserved. The same
installed command then completed under automatic approval with empty stderr.
Quire independently reports 0.31.0. No substitute adviser or new helper ran.

The scoped output has 32 records: 27 FR criteria with mismatch, uncatalogued
and inconclusive false; five NFR metrics with mismatch true solely for the
quantified-threshold benchmark rule. Their authored catalog method remains
negative-abuse-testing by the explicit judgments above. No unmatched obligation
is silently defaulted to Test.

Universal/order recommendations are served by generated declaration, feature,
identity and ordered-array transformations. TC-078/079/081 also enumerate
every exported occurrence/obligation through real APIs. TC-082/090 use
independent canonical bytes/hash and multiple-run invariance even where the
adviser recommends only examples. Security-class advice for authored binding
substitution is served by actual adverse reader requests, not a web DAST tool.

## Planned suites and outputs

| Rust suite | Cases | Observable evidence |
| --- | --- | --- |
| package construction/identity | TC-078–082/090/091 | Complete manifest assertions, independent canonical vectors, identity mutations and role refusals |
| strict package reader | TC-083–087/091 | Closed-schema qualification, byte/structural adverse families, selection precedence, external authority and actual frontend failures |
| package limits | TC-088 | Independently counted per-pass boundaries, absent versus measured counters and fresh retries |
| package workflow | TC-089 / IT-007 | Real reconstructed source-model-clause-runtime truth, events, costs and incomplete/refused controls |

These planned suites are the evidence producers; a matrix tag or a serializer
round trip alone is insufficient. No fuzz run, mutation-adequacy score, formal
proof of SHA-256, runtime monitor or concolic campaign is claimed. Generated
input mutation is distinct from mutating program code to measure fault
detection. Loom has no shared scheduler/atomic state to explore in this pure
immutable request API. New concurrency would reopen this assessment.

TC-083 qualifies Draft 2020-12 using the existing locked Rust jsonschema
dependency with local references and explicit draft feature. No schema execution
has yet occurred; syntax inspection is not schema qualification. First-party
assertions and fixtures remain Rust plus static data. Existing cache/serial
build constraints apply when these suites are implemented.

## Verdict and provenance

PASS for planning and implementing this producer/reader slice. Agent A applied
the installed QUOIN 0.22.5 skills serially under the owner's existing all-review
selection; no required AssuranceProfile applies. This is the author's recorded
review, not independent B/C acceptance. All package cases remain planned.
Changes to the reviewed requirements or interface reopen specify/spec-review.
Full LC02/FS05 interchange, LC04 backend qualification and LC05 Quire integration
remain required. No Cargo build, additional agent or hosted CI dispatch ran.
