---
id: SR-4960
title: "QSL-669 code and Rust review of PR #720"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@8eb14aec90fabb189ccfdf485d87099db326e68a; base origin/main; qsl-semantics/src/model/domain_package.rs; qsl-semantics/src/model/key.rs; qsl-semantics/src/model/normalize.rs; qsl-semantics/src/model/population.rs; qsl-semantics/tests/it/identity_golden_vectors.rs; qsl-semantics/tests/it/model_dispatch.rs; qsl-semantics/tests/it/model_normalization.rs; QSpec FR-150-AC-1, FR-150-AC-4, FR-150-AC-6, FR-150-AC-7, FR-150-AC-8"
review_set: subset
---
## Summary

Ticket: QSL-669. PR: quire-spec-language#720 at `8eb14aec90fabb189ccfdf485d87099db326e68a`, compared with freshly fetched `origin/main` (see `git diff origin/main...HEAD`). Round 1 fix-round code/Rust review covered the added qualify-before-inherit fact, ordinal and charge accounting changes, version-free closed identity wires, the separate PopulationId source-version wire shape, updated identity vectors and changed exact-limit/order tests.

The producer now inserts a `qualify` fact before the first `inherit` fact for an inherited member and retains one inheritance fact per contributing path. This matches the ordered normalization phases and the requirement that later phases cannot erase earlier derivation. The vector test now carries both `QSpec-TC-195` and `QSpec-FR-150-AC-6`. The distinct PopulationId preimage retains `version`, while the closed model identity wire omits it. No additional code/Rust defect was found in the changed diff. `git diff --check` passed.

The supplied focused and module logs do not establish evidence for this reviewed head: both identify `465fc1ceeb7d7dbfb634ee452aef031a0f0032e6`, not `8eb14aec90fabb189ccfdf485d87099db326e68`. No full gate was run by the reviewer. Therefore this is a clean code/Rust disposition, not a gate or mergeability approval.

Scope examined:

- `qsl-semantics/src/model/domain_package.rs`, `key.rs`, `normalize.rs`, and `population.rs`: closed RuleRef/ModelSelection identity shape, derivation phase ordering, and PopulationId’s separate source-version input.
- `qsl-semantics/tests/it/identity_golden_vectors.rs`, `model_dispatch.rs`, and `model_normalization.rs`: exact identity vectors, provenance paths, order-dependent outputs, limit charges and exact/one-less bounds.
- QSpec FR-150-AC-1, AC-4, AC-6, AC-7 and AC-8, including the ordered normalization rule phases and published-vector requirement.

## Verdict

No additional code or Rust finding in disposition round 1. The exact-head focused/full gate evidence remains outstanding because the supplied logs are for an earlier commit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
