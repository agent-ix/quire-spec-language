---
id: SR-2026111117463800000001
title: "code review of PR 2681 typed FR-290 capability Cause"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-route/src/cause.rs, qsl-route/src/lib.rs, qsl-route/src/model_request.rs, qsl-route/tests/it/cause.rs, qsl-route/tests/it/main.rs, qsl-analyze/src/analyze.rs, spec/functional/FR-075-compute-candidates-from-registered-backends.md"
review_set: subset
---

## Summary

Ticket: QSL-1877. Code review with the rust-review lane folded in, over `git diff origin/main...HEAD` of PR 2681 at 86b647a77. Main moved: #2679 (91ada09a7) merged FR-057 "One negotiation cause type" and FR-057-AC-15 for this same ticket. The PR was written against e35d1ac6c and does not implement that spec. It adds a different type, `qsl_route::cause::Cause`, and a new FR-075-AC-23 that contradicts FR-057-AC-15. The PR merges without textual conflicts, but the two specs conflict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR implements an API that conflicts with main's FR-057 "One negotiation cause type" (#2679). Main specifies `NegotiationOutcome` (fieldless, `code()`) and `NegotiationCause` (payloads, `outcome()`, `code()`) in `qsl_semantics::check::capability`, re-exported at the qsl-route crate root, plus `NegotiationSettlement { Cause, Inconsistent, Single(BackendId) }` and `settle_candidates(outcome, kind, named)`. The PR adds `qsl_route::cause::Cause` with different payloads: `UnknownBackend` carries no `BackendId`, and `AmbiguousBackend` carries `Vec<Candidate>` where the spec has `BackendId`s. It has no outcome type, no settlement enum and no `settle_candidates`. FR-075-AC-23 restates the cause type in a way that contradicts FR-057-AC-15, so main would have two specs for one type. | qsl-route/src/cause.rs:26-84; spec/functional/FR-075-compute-candidates-from-registered-backends.md:446 |
| FND-002 | high | FR-075-AC-23 says "QSL settles these codes only through `Cause`". The shipped source does not. Literals remain at qsl-analyze/src/analyze/request.rs:579 (in a crate that already imports `Cause`), qsl-conformance/src/run.rs:646 (which depends on qsl-route), qsl-replay/src/statistical/witness.rs:213, and qsl-replay/src/proof_result.rs:741-766 and 1874. The qsl-replay sites cannot reach a qsl-route type because ADR-011 X-10 keeps qsl-replay off qsl-route. This is why FR-057 puts the types in qsl-semantics. These leftovers are not acceptable. FR-057 requires every such site to build its code from `NegotiationCause::code()` or `NegotiationOutcome::code()`, and all of them can once the types sit in qsl-semantics. | qsl-route/src/cause.rs:9-10; spec/functional/FR-075-compute-candidates-from-registered-backends.md:446 |
| FND-003 | medium | `Cause::from_set` maps every set of two or more candidates to `AmbiguousBackend`, even when a backend was named. FR-057's settlement table gives `Inconsistent` for that case, with no cause; the caller settles `invalid_capability`/`inconsistent-candidates`. The doc comment "None when exactly one candidate serves" also hides that the single-candidate case drops the candidate, which the spec's `Single(BackendId)` keeps. | qsl-route/src/cause.rs:61-74 |
| FND-004 | medium | Code smell: callers build payload-carrying variants with made-up payloads just to read a code. Examples are `Cause::AmbiguousBackend { candidates: Vec::new() }.code()` and `Cause::UnsupportedRequestedCapability { kind, backend: None }.code()`. This shows the missing payload-free outcome type (`NegotiationOutcome`). `from_outcome` and `from_set` are public but have no production caller: qsl-analyze still settles through `routing::Disposition`, so the conversion is only exercised by its own test. | qsl-analyze/src/analyze.rs:985-989; qsl-analyze/src/analyze.rs:1752-1757; qsl-analyze/src/analyze.rs:2272-2277; qsl-route/src/cause.rs:61-83 |

## Verdict

FAIL. `cargo fmt --check` passes. The focused `qsl-route` `it cause` test was not run by this reviewer: it was queued behind the shared luna build lock and cancelled, because the verdict does not depend on it. The change is small and consistent with what it tests, but it implements a design that main's merged FR-057 (#2679) has superseded, and its new AC states a single-source property the code does not have. Fix: drop FR-075-AC-23 and `qsl_route::cause`. Implement FR-057-AC-15 as specified (`NegotiationOutcome` and `NegotiationCause` in `qsl_semantics::check::capability`, re-exported by qsl-route, with `NegotiationSettlement` and `settle_candidates`). Move every remaining literal site in qsl-analyze, qsl-conformance and qsl-replay onto them, and test them under TC-155 step 8. Code with no finding: lib.rs `pub mod cause`, model_request.rs const swap, tests/it/main.rs registration.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified at 80f74e3ec: `NegotiationOutcome` and `NegotiationCause` are in `qsl_semantics::check::capability` and re-exported at the qsl-route root. `NegotiationSettlement` and `settle_candidates` are at the qsl-route root. `qsl_route::cause` and FR-075-AC-23 are removed. |
| FND-002 | fixed | Verified at 80f74e3ec: qsl-analyze request.rs, qsl-conformance run.rs, qsl-replay witness.rs and proof_result.rs `as_str`/`unsupported` all build from `NegotiationOutcome::...code()`. The remaining literals are test oracles only. |
| FND-003 | fixed | Verified at 80f74e3ec: `settle_candidates` gives `Inconsistent` for 2+ candidates with a named backend, and `Single(BackendId)` for one candidate. |
| FND-004 | fixed | Verified at 80f74e3ec: payload-free sites use `NegotiationOutcome`, and the made-up payloads are gone. `settle_candidates` has no production caller, but FR-057 requires it at the crate root and TC-155 tests it. |
