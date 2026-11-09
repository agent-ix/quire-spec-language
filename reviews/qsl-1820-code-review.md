---
id: SR-20261109172720000005
title: "Code and Rust review of exact probabilistic request routing"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1820; PR 2608; qsl-analyze/src/analyze.rs, qsl-analyze/src/provider.rs, qsl-analyze/tests/it/exact_route.rs, qsl-analyze/tests/it/main.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quire-spec-language/FR-280", type: references}
  - {target: "ix://agent-ix/quire-spec-language/FR-281", type: references}
---

## Summary

Independent code and Rust review of PR 2608 at frozen head 55bae7abeb32afcd29642bb475d95a02a74a61b7. The new exact member, operation-owned subject construction, backend control, shared settlement and provider fallback paths, and all seven new test oracles were inspected.

## Verdict

PASS for this routing slice. The member delegates to the existing EN-5 product, solver and certificate adapter instead of introducing a second semantic implementation. It constructs the subject from the operation's checked package, admitted binding and shared limits. Only the EN-5 backend runs it; a sampling backend receives a typed disposition. Exhaustive enum matches preserve request indices, input order and provider admission fallback. Required aggregation and cancellation use the existing machinery. No new unsafe, panic, dependency, serialization or CI path is introduced.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

No applicable AssuranceProfile was found under spec/. The installed rust-style API, implementation, safety/concurrency, wire/persistence and testing references were applied; no Tokio behavior is changed. No source or branch files were edited.

Examined requirement units:

- FR-281 Inputs, examined: "ClaimItem: one requested item, the typed value of a QSpec FR-331 request item in its in-process form" with its probabilistic settings and limits; the exact typed member supplies the existing engine request over the operation's own subject.
- FR-281-AC-1, examined as partial evidence under current origin/main 26b3c7c4f: "Over ADR-028 §15.4's `Link` model with the claim `probability >= 0.95 [ eventually holds(m.delivered) ]` under every scheduler, an EN-5 item settled `Supported(en-5)` by `negotiate_*` (EN-5 advertises `unbounded`, FR-280) and built from the source bytes and the provision settles `proved` with category success after the EN-5 certificate checker, run over the checked subject (FR-201 "Checking"), accepts its certificate (values `24/25`, `4/5`, 1 and 0, FR-201-AC-1), and the record carries the certificate and names `en-5`."
- FR-281-AC-6, examined as partial evidence under current origin/main 26b3c7c4f: "(a) AC-2's EN-1 item with `limits.max_states` 1 settles `incomplete` with `LimitReached{max_states}`, exit 22, carries no counterexample and is never `refuted` or `proved`. (b) AC-1's EN-5 item with `exact_prob.max_states` 1 settles `incomplete` with `LimitReached{ExactProbMaxStates}` naming `exact_prob.max_states`."
- FR-280-AC-2, examined as partial evidence under current origin/main 26b3c7c4f: "For each TC-762 item, that item settled `Supported` on its QSL engine as a `NegotiatedItem`, passed to `qsl_analyze::provider::run` with the `CheckedPackage`, `SubjectBinding`, `SourceProvision` and `AnalyzeLimits` of the same run, returns a terminal record, and the counterexample where there is one, equal to the ones `analyze` returns, over `Engines::qsl()`, for that item settled `Supported` on the same backend."
- FR-280-AC-6, examined for exact-member ordering and incomplete admission; existing undeclared-population and shared fallback tests provide the remaining admission control. Its full text remains in the owning requirement.

The reversed-order test compares complete records before checking indices. The mixed test asserts proved/declined/incomplete in input order. The off-backend test checks value, absent producing backend, exact diagnostic, required aggregate and optional parity; a mistakenly executed exact or sampling engine would fail its oracle. Pre-cancelled exact work preserves Requested and EN-5 identity. Missing-snapshot admission returns the typed incomplete-input diagnostic without a producing backend.

Verified log evidence: fmt exit 0; clippy qsl-analyze all-targets with warnings denied exit 0; exact_route 7 passed and existing exact_probabilistic_item 7 passed, both exit 0. Logs: /tmp/qsl1820-route-{fmt,clippy,tests,engine-tests}.log. The actual codex exec review completed exit 0 and found no actionable defect (/tmp/qsl1820-route-codex-review.log); it performed inspection only and ran no Cargo, test or formatting commands. The lead owns the final aggregate suite after review.

Known engine limitations remain owned by QSL-1826; external matrix integration remains with QSL-12. Neither is claimed complete by this PR.

Current-contract update: specification PR 2607 landed during review at main 26b3c7c4f. The current FR-280/281 contract additionally requires negotiated items, SourceProvision and operation-built replay/check requests, and ProviderResults with counterexample projection. This slice implements exact member routing on the pre-existing public API only; it does not satisfy those new API/provision obligations or complete current FR-281-AC-1 or FR-280-AC-2. They remain QSL-1820 implementation work, not a waiver. EN-5 certificate details remain engine evidence; public record projection is not newly implemented.

## Final head verification

Disposition pass 1 examined head 7c7546cbdc6e90b01617059529ead84a5e490bdf. `git range-diff 4ba8b813..55bae7ab 7e0fbcdf9..ee874df9b` reports the routing patch unchanged after rebase onto main 7e0fbcdf9 (current specification and checker cancellation). The fix commit changes only test documentation and copies the two original reviewer artifacts verbatim; no implementation or test assertion changed. The PR title and body explicitly call the slice partial, enumerate NegotiatedItem, SourceProvision, generalized request/results, public evidence projection and unbounded manifest work as remaining, and keep QSL-1820 open. No new finding was identified. The lead owns fresh final-head execution evidence; original-head logs are accurately labeled.

Latest verdict: PASS remains applicable to this final head as a partial functional routing slice. No new executable code was introduced by the fix.
