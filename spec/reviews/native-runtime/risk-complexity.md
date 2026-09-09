---
id: SR-093
title: "Native runtime risk and complexity review"
type: SpecReview
analysis: risk-complexity
scope: "FR-007/008/018, NFR-006, native-runtime input/evaluation contracts, IT-006, TC-055–077 and TM-004"
review_set: all
evaluated_revision: "045025f843346001a91919b8d0816a519e2df337"
review_date: "2026-09-09"
---

## Summary

The highest implementation risk is faithful independent execution with observation capture and exact partial events. Mitigations are bounded source-derived integration cases and independent graph/cost oracles, before optimization.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Evaluation is the first native interpreter in this repository; generated backend or IR proof parity cannot serve as its sole oracle. Independent exact counts and positive-length closure are mandatory. | FR-008; TC-068; TC-074 |
| FND-002 | medium | Shared flat values can hide amplified validation/comparison work. Per-use counters, explicit text work and isolated lower-limit controls are specified before cache optimization. | FR-007-AC-12; FR-008-AC-18; NFR-006 |

## Risk register

| Requirement | Technical risk | Volatility | Drivers | Mitigation |
| --- | --- | --- | --- | --- |
| StR-001 (existing context) | Medium | Low | A true result could be overstated | Retain complete workflow and stage claim boundaries |
| FR-018 | Medium | Low | Exact byte/role identity and bounded flat input | Golden byte vectors, mutations, all root/child bounds |
| FR-007 | Medium | Medium | Closure, frame authority, mixed unavailable/invalid input | Independent shape/frame mutations, canonical complete diagnostics |
| FR-008 | High | Medium | First native execution with captured observations and partial events | Source-derived integration, all small functional graphs, exact budget vectors |
| NFR-006 | Medium | Low | Shared structures amplify visits; several coupled ceilings | Charge before work, explicit usage, isolated exact/one-below tests |

## Top hazards and sequencing

1. Captured observation errors could produce plausible wrong truth: test direct
   pre reads against captured aliases and deleted objects before broader execution.
2. Closure/frame omissions could validate data whose predicate happens to skip
   the defect: include all supplied selected fields and exact population deltas.
3. Counter/event misordering could claim entry that never happened: use exact
   entry and completion capacity boundaries.
4. Source/model identity confusion could attribute truth to another clause:
   retain immutable context and independently mutated reference controls.

No latency/throughput SLA, distributed protocol or authentication behavior is
introduced. The system remains private, Rust and within existing license grants.
The full scope should be implemented in bounded tasks, not a speculative
prototype replacing the objective. See failure-domain.md for resolved callback,
identity, topology and partial-output concerns.

## Verdict and provenance

PASS for implementation of this specified LC03 API scope. Agent A applied the
actual installed QUOIN base and all seven analysis skills serially, under the
owner's existing all-review selection. No additional agents or Cargo builds
ran. No applicable required AssuranceProfile was found. The declined optional
semantic gap comparison remains excluded; this specification review still
checks the actual adopted meaning and existing interface boundaries.

All runtime test rows remain planned. Review approval does not qualify native
execution, finish LC02/FS03 acceptance or complete the original backend/Quire
workflow. Implementation changes to this contract reopen specify/review.

## Constructor setup correction — 2026-09-09

Reviewed dc63f337fcdaee1320d221383eca38599239f62c. Removing the copied
CheckedPackage setup prerequisite reduces unnecessary fixture coupling while
retaining the existing risks: exact bytes, distinct identity roles and bounded
shared arenas. FR-018's risk/volatility scores and named mitigations remain as
registered above; FR-007/008 risks are unaffected. Public integration assertions,
independent byte vectors and hard/lowered boundary controls still address the
constructor risks. PASS; no new spike, dependency or parallel task is needed.
