---
id: SR-16275
title: "spec-review of SMT encoding limits as request settings and SMT solver run limits (quire-spec-language PR 1356)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@656069b2; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md; spec/functional/FR-182-settle-a-hyper-verdict.md; spec/functional/FR-314-check-an-smt-proof-certificate.md; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md; spec/test-cases/TC-1202-smt-encoding-and-solver-run-limits-are-request-settings.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: references
---

# Spec review of SMT encoding and solver run limits (quire-spec-language PR 1356)

## Summary

Ticket: STD-408. This is one spec-review pass over
`git diff origin/main...656069b2`. It covers:

- FR-315 "Size limits" and "Solver run limits", `SmtEncodingLimits`,
  `SmtRunLimits`, the `ModelCheckLimit` members and
  `HyperQueryRequest.encoding`;
- FR-314 "Encoding limits";
- FR-182 rule 3 and the new V-7 row;
- the FR-255 rows and AC-14, the ADR-018 V-7 edit, FR-315-AC-40/41,
  FR-182-AC-14, FR-314-AC-7 and TC-1202.

The oracles were merged #1288 (the ADR-018 V-6 stop order), open #1312
(FR-314/FR-127 handoff) and QSpec #411.

Checked and sound:

- The limits gate printing only and never the bytes. So FR-314's rebuild
  under the recorded limits is deterministic.
- The solver run limits do not enter the rebuild at all. A time budget
  outside the request identity therefore leaves FR-314's rebuild
  deterministic: a proof found under any time budget checks the same way.
- The stop order (cancellation, then backend wall clock, then
  `smt.max_solver_*`, then `unknown`) matches ADR-018 V-6 as merged by
  #1288 and FR-182 rule 3.
- `LimitStopped` naming an encoding limit refuses `LimitNotApplicable`,
  which is consistent.
- `HyperProductIdentity` excludes the limits, which is consistent.

No deferral language was found.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | TC-1202 is used twice. Open #1349 (STD-375) adds `spec/test-cases/TC-1202-a-schedulability-claim-emits-a-v2-claim-node.md` with `id: TC-1202`, and FR-454-AC-1..4 cite it. This PR adds another TC-1202. Fix: renumber whichever lands second. | spec/test-cases/TC-1202-smt-encoding-and-solver-run-limits-are-request-settings.md:2 | wrong-requirement |
| FND-002 | high | No type carries the recorded encoding limits. FR-314 "Encoding limits" says the item's `CertificateRequest` SHALL record `SmtEncodingLimits`, and the rebuild of `check_refinement_smt_proof` uses them too. Yet FR-338's `CertificateRequest` and FR-314's `RefinementCertificateRequest` gain no member. FR-126's `ModelCheckRequest`/`ModelCheckLimits`, "the item's request" that CG encodes under, carry neither `SmtEncodingLimits` nor `SmtRunLimits`. Only `HyperQueryRequest.encoding` is typed. The deterministic rebuild depends on a value that no interface carries, so two implementers would plumb it differently. Fix: add the members (for example `ModelCheckRequest.smt_encoding`/`smt_run`, `CertificateRequest.smt_encoding`, `RefinementCertificateRequest.smt_encoding`) with an AC. | spec/functional/FR-314-check-an-smt-proof-certificate.md:62-65, :196-203; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:620-630, :639; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:58-81 | missing-requirement |
| FND-003 | high | The encoding limits change the result but are missing from the request identity and the provider wire. A `too-large` stop settles V-7 `LimitReached`, and a raised limit lets the item be proved. FR-315 says each `smt_encoding.*` limit is raised "at every entry point". QSpec #411 (lead default 4) keeps them out of `TemporalLimits`. Neither PR puts them in the request identity or cache key. So a provider request cannot raise them, and two requests that differ only in an encoding limit share an identity although their results differ. That conflicts with FR-368's "a different budget can produce a different result" rule, and under the standing rule it is a narrowing. Fix: state that the `smt_encoding.*` values are part of the request identity and cache key, and carry them on the QSpec wire (#411). | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:591-630 | missing-requirement |
| FND-004 | medium | FR-314-AC-7's adverse case says only "never `Verified`". Main's FR-314 has no rule for a `Refused` rebuild in `check_smt_proof`. Open #1312 adds one (reject `QueryMismatch`). The AC therefore has no outcome to test against. Fix: name the outcome, `QueryMismatch` at `Query { part: Unrolling }`, consistent with #1312, and its settlement. | spec/functional/FR-314-check-an-smt-proof-certificate.md:301 | missing-requirement |

## Verdict

**Not mergeable.** FND-001 to FND-003 block it. Two things are clean:
FR-314's rebuild does not depend on the run limits, and the stop order
matches #1288.

Ids FR-315-AC-40/41, FR-182-AC-14, FR-314-AC-7 and FR-255-AC-14 are
unique across open PR heads. TC-1202 is not (FND-001).

Merge: `git merge-tree` against #1354 conflicts only at the FR-315 AC
table, and against #1312 only at the FR-314 AC table. Both are
append-after-the-same-row conflicts; resolve by union in id order.
#1356 merges cleanly with #1311.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The TC is renumbered TC-1250, free on main and on every open PR head. #1349 keeps TC-1202. Every citation in FR-315, FR-314, FR-182 and FR-255 now names TC-1250. |
| FND-002 | fixed | The carrying types now exist. FR-126's `ModelCheckRequest` gains `smt_encoding: SmtEncodingLimits` and `smt_run: SmtRunLimits`. FR-338's `CertificateRequest` inputs gain `smt_encoding`, the run's values, and FR-314 reads `CertificateRequest.smt_encoding`. FR-315's refinement queries carry no size measure, so `check_refinement_smt_proof` reads no encoding limit and `RefinementCertificateRequest` needs no member. New FR-315-AC-42 (TC-1250 step 6). |
| FND-003 | fixed | Lead ruling (lead default, not an owner answer; Peter may veto). FR-315 states that the `smt_encoding.*` values are part of the request identity and the cache key, and that a result is reused only for equal values. On a provider request they travel as QSpec FR-368's `temporal.smt_encoding` member (closed schema, `PositiveCount` members), added in QSpec #411. The run limits travel as `temporal.limits.max_solver_steps` and `max_solver_time`. |
| FND-004 | fixed | FR-314 "Encoding limits" now states the refused-rebuild rule: a `too-large` rebuild under the recorded limits is rejected `QueryMismatch` at that part and settles `inconclusive`, `CertificateRejected`. This matches #1312. FR-314-AC-7 names that outcome. |
