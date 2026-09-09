---
id: SR-106
title: "Native package risk and complexity review"
type: SpecReview
analysis: risk-complexity
scope: "US-002, FR-019/020/021, NFR-007, IT-007, TC-078–091, TM-005 and native package wire/API/schema"
review_set: all
evaluated_revision: "41da6e5eb86bb727fbad5370fd23b38d330bcfea"
supplement_evaluated_revision: "2c6b9b83dc5c87c68666ebcd24c41b198f4c339b"
review_date: "2026-09-09"
---

## Summary

The highest risks are identity inclusion/encoding and accepting forged input
through an overly permissive decoder. Named vectors, mutation controls,
independent counters and actual compiler reconstruction are planned mitigations.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | Mitigate fixture-precondition risk with a demonstrated successful actual native setup before canonical/vector assertions. | TC-078; TC-087; TC-090 |
| FND-001 | low | High identity/trust-boundary risks have concrete planned mitigations; no unreviewed implementation or unbounded prototype is authorized by this report. | FR-020; FR-021 |
| FND-002 | low | Shared native-domain registration is externally coordinated and remains an acceptance gate, preventing local producer success from closing FS05. | FR-021; IT-007 |

## Risk register

| Requirement | Technical risk | Volatility | Driver | Mitigation |
| --- | --- | --- | --- | --- |
| StR-001 | High | Medium | Full cross-system identity and actual assessment | Preserve compiled backend/Quire acceptance beyond this milestone |
| FR-019 | Medium | Medium | Large exact manifest and future projection changes | Independent field census, fixed closed wire, explicit unlowered inventory |
| FR-020 | High | Medium | Untrusted bytes and authority substitution | Strict typed records, external bindings, real frontend replay, multi-defect controls |
| FR-021 | High | High | Cryptographic derivation contract and independent shared-domain adoption | Domain-separated independent bytes/hash vectors; A/B registration gate; separate typed role |
| NFR-007 | High | Low | Enforced bounds across several traversals | Charge-before-operation controls, per-pass accounting and isolated coupled-limit tests |

## Top hazards

1. A self-consistent encoder/reader bug can hide incorrect canonical bytes:
   TC-090's expectations precede implementation and do not call the producer.
2. Raw digest recomputation can hide a forged checked claim:
   TC-085–087/091 update the selector and require the intended later refusal.
3. Serde recursion/string scratch can disagree with advertised limits:
   TC-088 measures entered containers and retained content at every pass.
4. Projection availability can accidentally enter native identity or grant
   reuse: TC-091 changes only that excluded metadata and still requires refusal.
5. Cargo feature unification can affect existing JSON audit/IR behavior:
   retain full Rust audit and existing regression checks after feature selection.

## Failure-domain cross-check

SR-102's identity/purity/cycle/extension analysis is part of this review set.
No callback, distributed writer, wall-clock promise or runtime population enters
the package. The versioned local payload isolates shared adoption changes;
it does not promise that B already understands the new native canonical domain.

## Admitted source setup correction — 2026-09-09

Re-reviewed specification 2c6b9b83dc5c87c68666ebcd24c41b198f4c339b using this
installed QUOIN lens, superseding the initial review's header-only positive
fixture assumption at 41da6e5. The original PASS and its missed precondition
remain visible at 69588ad. Escape cause: wrong-requirement.

One nonempty checked model/clause package test already passes while three header-based tests fail in the parser. This discriminates the setup defect from an encoding result and reinforces the no-masked-refusal gate. Other technical/volatility scores and mitigations remain unchanged.

Task-016 began under the original completed review gate. Its initial API-red
run is followed by a first implementation run with one passing nonempty case
and three parser setup failures. Further implementation paused for this
specification correction and all-eight re-review; no parser behavior was
relaxed. The initial stdout/stderr is retained in reviews/data/native-packages/.
No claim of executed full package, schema or canonical-vector qualification is
made. PASS to continue against the corrected admitted-source fixture contract.

## Verdict and provenance

PASS for planning and implementing this producer/reader slice. Agent A applied
the installed QUOIN 0.22.5 skills serially under the owner's existing all-review
selection; no required AssuranceProfile applies. This is the author's recorded
review, not independent B/C acceptance. All package cases remain planned.
Changes to the reviewed requirements or interface reopen specify/spec-review.
Full LC02/FS05 interchange, LC04 backend qualification and LC05 Quire integration
remain required. No Cargo build, additional agent or hosted CI dispatch ran.
