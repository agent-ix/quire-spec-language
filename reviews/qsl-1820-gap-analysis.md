---
id: SR-20261109172720000006
title: "Scoped gap review of EN-5 public request routing"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1820; PR 2608; exact probabilistic request routing and exact_route tests"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quire-spec-language/FR-280", type: references}
  - {target: "ix://agent-ix/quire-spec-language/FR-281", type: references}
---

## Summary

Scoped reverse-gap, traceability and stub/oracle audit of exact probabilistic request routing at 55bae7abeb32afcd29642bb475d95a02a74a61b7. The dispatch explicitly excludes a new broad computed matrix run. The completed pre-change computed matrix at /tmp/qsl1819b-matrix-review.json was read for the relevant criteria, then the added trace attributes, tests and source paths were inspected directly. No Test Matrix was authored.

## Verdict

CONDITIONAL for the scoped routing change until its partial-coverage disclosure is corrected. This is not a repository-wide gap-analysis PASS or a claim that every criterion is covered. The existing FR-281 probabilistic member input and FR-280 equal-record provider contract own the new behavior; no new unspecced solver or settlement path was found. Tests use real compiled source and the production engine, with no fabricated terminal results or test-only production bypass. Existing QSL-1826 and QSL-12 work remains open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new route tests describe their FR-280-AC-2 coverage as limited only by other engine members, while the current criterion also requires NegotiatedItem and SourceProvision. The PR validation wording likewise does not identify these current-contract omissions. Make the test/module and PR evidence explicitly partial: current negotiation/provision, generalized request/result API and certificate/counterexample projection are not delivered by this slice. Retain honest partial trace bindings and keep those obligations open on QSL-1820. | qsl-analyze/tests/it/exact_route.rs:4 |

## Coverage

Plan completion: not assessed

No plan was supplied. No applicable AssuranceProfile was found. Repository-wide matrix and optional repository-wide semantic audit were not run; the requested PR review includes the changed tests' concrete oracle checks.

Examined units and exact owning excerpts:

- FR-281-AC-1, examined as partial evidence under current origin/main 26b3c7c4f: "Over ADR-028 §15.4's `Link` model with the claim `probability >= 0.95 [ eventually holds(m.delivered) ]` under every scheduler, an EN-5 item settled `Supported(en-5)` by `negotiate_*` (EN-5 advertises `unbounded`, FR-280) and built from the source bytes and the provision settles `proved` with category success after the EN-5 certificate checker, run over the checked subject (FR-201 "Checking"), accepts its certificate (values `24/25`, `4/5`, 1 and 0, FR-201-AC-1), and the record carries the certificate and names `en-5`."
- FR-281-AC-6, examined as partial evidence under current origin/main 26b3c7c4f: "(a) AC-2's EN-1 item with `limits.max_states` 1 settles `incomplete` with `LimitReached{max_states}`, exit 22, carries no counterexample and is never `refuted` or `proved`. (b) AC-1's EN-5 item with `exact_prob.max_states` 1 settles `incomplete` with `LimitReached{ExactProbMaxStates}` naming `exact_prob.max_states`."
- FR-280-AC-2, examined as partial evidence under current origin/main 26b3c7c4f: "For each TC-762 item, that item settled `Supported` on its QSL engine as a `NegotiatedItem`, passed to `qsl_analyze::provider::run` with the `CheckedPackage`, `SubjectBinding`, `SourceProvision` and `AnalyzeLimits` of the same run, returns a terminal record, and the counterexample where there is one, equal to the ones `analyze` returns, over `Engines::qsl()`, for that item settled `Supported` on the same backend."
- FR-280-AC-6, examined for reversed items and missing-snapshot fallback; the same requirement also owns invalid population admission and boundary dependencies, unchanged here.
- FR-281 Inputs, examined: kind-specific probabilistic settings and limits on requested ClaimItem members.

The baseline computed matrix reports FR-281-AC-1/6 and FR-280-AC-6 tagged, and FR-280-AC-2 untagged before this PR. The new exact_route proof and budget tests each carry FR-280-AC-2 alongside TC-761 and their FR-281 criterion; their module header explicitly scopes those tags to the EN-5 members. This is inspected source evidence, not a fresh computed post-change matrix result. FR-280-AC-6 gains reversed-record and missing-snapshot assertions. Existing engine tests retain the certificate-content assertions. No trace tags are removed.

The exact route is a thin constructor over the existing engine adapter; its new public types have consumer-facing documentation. No stubs, no-op production branch, duplicate solver, fixture vendoring or gate weakening was found. Tests compile source through prob_world and check full records, typed causes and backend identity. Seven new route tests and seven retained engine tests passed with exit 0; fmt/clippy logs also exit 0. The lead's post-review aggregate gate remains pending.

No completeness claim is made for all TC-762 engine members, the external lifecycle matrix, EN-5 replay, rejected-certificate vocabulary, wider exact claim forms or certificate byte round trips.

Current-contract update: specification PR 2607 landed during review at main 26b3c7c4f. The current FR-280/281 contract additionally requires negotiated items, SourceProvision and operation-built replay/check requests, and ProviderResults with counterexample projection. This slice implements exact member routing on the pre-existing public API only; it does not satisfy those new API/provision obligations or complete current FR-281-AC-1 or FR-280-AC-2. They remain QSL-1820 implementation work, not a waiver. EN-5 certificate details remain engine evidence; public record projection is not newly implemented.
