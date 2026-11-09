---
id: SR-20261109172912000044
title: "code-review of QSL-1826 values rejection locus"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1826; qsl-replay/src/probability_certificate.rs; qsl-replay/tests/it/probability_certificate.rs; qsl-analyze/tests/it/exact_probabilistic_item.rs"
review_set: subset
---

## Summary

Ticket: QSL-1826. Frozen PR quire-spec-language#2621 at bcbc2ec2d078eca5cda9f26d3985447f281fcca4, compared with 927ec42e352b5c3700c88fde46c9de50972dc414. Scope: qsl-replay/src/probability_certificate.rs; qsl-replay/tests/it/probability_certificate.rs; qsl-analyze/tests/it/exact_probabilistic_item.rs. Existing adapter and document admission were inspected as context. No spec, manifest or CI change; spec review methods do not apply.

The three malformed value-set branches now name Member("values"). Strict order checks reject duplicates and unsorted typed rows; membership rejects foreign/extra entries; cardinality then detects missing reachable rows because every admitted key is unique and belongs to the rebuilt graph. Removing the missing-key search preserves cancellation polling and reduces work. Existing typed member translation produces CertificateMember::Values. No API, dependency, unsafe, task, lock, numeric or wire-layout change; no duplicate production authority or stub introduced.

The malformed assessment agrees with actual contracts: FR-201 requires canonical byte admission and distinct identity/objective refusal; FR-203 and FR-281 require actual checker rejections to settle inconclusive/CertificateRejected; FR-282-AC-11 distinguishes rejected changed values from refused identity/objective. QSpec FR-331 permits malformed at certificate member values and excludes malformed at probabilistic product state. Do not convert every document error: serialization/arithmetic/internal faults and input admission retain their distinct routes. The adapter's pre-existing broken() route is a provider invariant failure for its generated document, not grounds for broad rejection conversion.

## Verdict

PASS — no introduced defects found in the frozen slice.

Mergeable for this narrow engineering slice; other QSL-1826 work remains open.

## Assurance Context

No applicable AssuranceProfile artifact found under spec/. Ordinary repository review rules apply. Selected unavailable evidence: computed matrix; broad aggregate gates excluded by dispatch. Active exceptions: none identified. Engineering and incomplete assurance are separated per AGENTS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reviewed supplied logs rather than rebuilding the deleted target: replay 6/6 and analyzer 13/13 pass; scoped clippy finishes successfully; fmt log is empty as expected for success (coder reports success). Red replay/coverage/analyze logs show actual assertion failures with old ProductState versus expected Member values. No aggregate, Kani, full matrix or cargo rerun was performed under the explicit dispatch scope. Broad crate tests and cargo-deny were not rerun; no dependency change.

Missing noninitial 4/5 row leaves ranking intact; the real solver/test-support tamper seam continues through writer, public checked-subject document checker, real adapter and analyze_items. The test asserts exact typed cause, exact OutcomeItem serialized cause, inconclusive result and absence of proof document, exact evidence and probabilistic counterexample. Valid certificates are accepted before tampering. Typed duplicate/extra and reachable unsorted rows exercise the changed checker; duplicate/unsorted byte rows deliberately remain admission errors. Identity and optimization mismatches remain distinct errors. Tests use repository #[trace] attributes and integration placement. The FR-201/203/281 and QSpec tags demonstrate only these scoped branches, not whole broad AC coverage.

## Examined Scope

```yaml
[
  {
    "id": "FR-201-AC-9",
    "path": "spec/functional/FR-201-check-a-probability-certificate.md",
    "role": "examined",
    "excerpt": "Test |"
  },
  {
    "id": "FR-201-AC-10",
    "path": "spec/functional/FR-201-check-a-probability-certificate.md",
    "role": "examined",
    "excerpt": "Test |"
  },
  {
    "id": "FR-203-AC-3",
    "path": "spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md",
    "role": "examined",
    "excerpt": "Test (TC-638) |"
  },
  {
    "id": "FR-281-AC-4",
    "path": "spec/functional/FR-281-analyze-claims-with-in-process-engines.md",
    "role": "examined",
    "excerpt": "Test (TC-763) |"
  },
  {
    "id": "FR-282-AC-11",
    "path": "spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md",
    "role": "examined",
    "excerpt": "Test |"
  },
  {
    "id": "QSpec-FR-331-AC-25",
    "path": "/home/peter/dev/quire-specification/spec/objects/interfaces/FR-331-backend-provider-envelope.md",
    "role": "examined",
    "excerpt": "A `certificate-rejected` cause is admitted with each rule at a locus listed with it, `not-refutation` and `malformed` at a query part included."
  }
]
```

## Dispositions

No real findings; no dispositions required.
