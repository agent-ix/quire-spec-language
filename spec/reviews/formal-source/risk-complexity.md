---
id: SR-061
title: "risk-complexity review of native formal source correspondence"
type: SpecReview
analysis: risk-complexity
scope: "FR-014, docs/formal-source-binding.md, TC-035–039 and TM-003 addition"
review_set: all
evaluated_revision: "4eb4ef6"
review_date: "2026-09-08"
---

## Summary

The main risk is accepting a plausible but false location. Independent adverse inputs and a separate coordinate oracle address it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No high-risk or high-volatility item requires an additional spike for this bounded bridge. | FR-014; TC-036; TC-038; TC-039 |

## Analysis

| Requirement | Technical risk | Volatility | Driver | Mitigation |
| --- | --- | --- | --- | --- |
| FR-001 | Low | Low | Existing indexed source lookup | Reuse implementation and existing source ceiling |
| FR-010 | Low | Low | Existing diagnostic compatibility | Reuse code/phase and assert identity/locus |
| FR-014 | Medium | Medium | Exact native/IR coordinate correspondence | Pin IR, test false constructor-valid spans and independent oracle |
| NFR-005 | Low | Low | Established owner Rust directive | No new dependencies or executable languages |

The hazards are byte-versus-scalar confusion, revision coercion and correlated forward/reverse bugs. TC-036 supplies manually authored endpoints; TC-038 injects false coordinates independently of the forward mapping; TC-039 scans Unicode scalars in a separate oracle so a shared bug cannot pass on round-trip alone. Checked incoming offsets include u64::MAX. The source ceiling keeps produced coordinates representable. See failure-domain.md for identity and purity analysis. No performance SLA, cryptographic primitive change or distributed algorithm is added.

## Verdict and provenance

PASS for this specified bridge. Agent A applied the actual QUOIN base and all
seven selected analysis skills serially, with no subagents. Catalog contracts,
source/IR implementations and the scoped acceptance cases were inspected.
There is no applicable required AssuranceProfile. No implementation or full
state-workflow completion is claimed. The declined optional gap-analysis
semantic comparison remains excluded.

