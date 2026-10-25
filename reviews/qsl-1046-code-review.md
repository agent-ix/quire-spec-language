---
id: SR-60016
title: "Code review of QSL-1046 HP-4 route settlement"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1046 branch from dbfb73488: qsl-analyze/src/analyze.rs, qsl-analyze/src/analyze/items.rs, qsl-analyze/tests/it/hyper_settle.rs, qsl-replay/src/model_check/hyper_settle.rs, qsl-replay/src/proof_result.rs, qsl-route/src/request.rs, qsl-semantics/src/check/hyper_clause.rs, qsl-semantics/src/family/requirements.rs; QSpec FR-396-AC-2 and TC-342"
review_set: subset
---

## Summary

Reviewed the HP-4 classification, route propagation, settlement seam and no-engine result against QSpec FR-396-AC-2 and TC-342. The implementation uses one classifier, calls `settle_hyper_form` from the analyze negotiation adapter, and the integration test routes an HP-4 item through qsl-route before asserting the unsupported-requested-capability disposition and absence of a model-check record.

## Verdict

**CONDITIONAL** — scoped tests, formatting, clippy, and cargo-deny pass; the affected-crate all-features test run has one unrelated sandbox-only timing test failure because `/usr/bin/time -l` cannot read `sysctl kern.clockrate` in this environment.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The affected-crate all-features test run cannot fully pass in this sandbox: `qsl-semantics/tests/it/model_identity_full_size.rs::ten_thousand_edge_chain` fails because its `/usr/bin/time -l` child cannot read `sysctl kern.clockrate` (`Operation not permitted`); the measured child test itself passes. This is an environment limitation, not a changed-code failure. | qsl-semantics/tests/it/model_identity_full_size.rs |

## Coverage

The direct scope comparison used QSpec FR-396-AC-2 and TC-342, because this QSL checkout has a different local FR-396 and does not contain the QSpec artifacts; a meaningful computed matrix for the QSpec-prefixed trace tags is unavailable from this repository's local `quire matrix` scope. The installed `quoin` has no `gap-analysis` command (`quoin gap-analysis --help` returned “command gap-analysis not found”), so the gap check was a direct comparison of the changed code and test with the cited criterion and test case. No workflow files changed and no applicable AssuranceProfile was found.

Passed: `cargo fmt --all -- --check`; focused routed HP-4 integration test; `cargo clippy -p qsl-semantics -p qsl-route -p qsl-replay -p qsl-analyze --all-targets --all-features -- -D warnings`; `make cargo-deny-bans`. The all-features tests for those four crates completed with the single sandbox-only failure above; the changed route and analyze tests passed, and the other crate tests passed.

## Dispositions

- FND-001: deferred: sandbox permissions prevent `/usr/bin/time -l` from reading `sysctl kern.clockrate`; the measured test body passes and no implementation change addresses this host restriction.
