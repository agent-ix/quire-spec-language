---
id: SR-16249
title: "spec-review of ToolFailure per FR-462 defect and the Z3 cross-check off the terminal record (quire-spec-language PR 1329)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3f9f9da0e80cb6bccba9f5e35296356b39674638; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md; spec/functional/FR-291-bound-plugin-runs-and-settle-plugin-failures.md; spec/test-cases/TC-774-plugin-results-settle-as-typed-records-and-plugin-proofs-carry.md; spec/test-cases/TC-775-plugin-failures-deadlines-and-cancellation-settle-only-the-plugin.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-291
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: references
---

# Spec review of ToolFailure per FR-462 defect and the Z3 cross-check off the terminal record (quire-spec-language PR 1329)

## Summary

Ticket: STD-372 (QSL-598). This is one spec-review pass over
`git diff origin/main...3f9f9da0e`: FR-291's `ToolFailure` table (one
payload-free variant per QSpec FR-462 `defect`), its serialization and the
restated AC-1, FR-290's "no Z3 cross-check member" paragraph, Behavior
bullet and AC-16, and the TC-774 and TC-775 steps. The oracles were
STD-372's gap list, QSpec FR-462 "Failures", QSpec FR-331 (`failed` causes,
`FailedCause` in `proposals/backend-provider-v1/schema.json`, the
`cross-check-absent{tool}` warning), QSpec FR-349, QSpec FR-354 and QSpec
TC-269.

Gates: `QSPEC_DIR=~/dev/quire-specification make check-spec-validation`
at 3f9f9da0e exits 0 with no warning on a touched file. origin/main
(c21bc57a) is an ancestor of the head. FR-290-AC-16 appears in no other
open PR, and no other open PR edits FR-290, FR-291, TC-774 or TC-775.

Examined, clean:

- **The fourteen variants** match QSpec FR-462's failure table row for row
  (`crash` to `missing-terminal-record`). This replaces the wrong "eight
  events".
- **The serialization** `{"type": "tool-failure", "defect": D}` matches
  QSpec FR-331's `FailedCause`. AC-1's read-back makes the mapping
  testable in both directions.
- **AC-16's absent-cross-check case.** A `cross-check-absent{tool}`
  warning leaves the value and the certification unchanged, and the record
  keeps the warning, as QSpec FR-349-AC-6 requires.
- **The lead defaults are flagged** in the PR body as "lead default, not an
  owner answer".

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Scope-deferral: after this PR the Z3 cross-check result is recorded nowhere. QSpec FR-354-AC-5 requires it to be "recorded alongside the certification as an independent parity signal", and QSpec TC-269 TB-08 expects it "recorded beside the certification". The certification lives on QSL's terminal record (FR-290 settles the SMT row). The PR moves the signal to "CG's run record for the item". No QSpec or QSL requirement defines such a record or a cross-check member on one: neither repository's spec mentions a CG run record with a cross-check. An agreeing cross-check therefore leaves no trace, and the capability FR-354-AC-5 states is lost. Fix: give the QSL terminal record the cross-check member, through a new STD ticket if QSpec FR-331 needs an envelope member. Otherwise cite a defined record that already carries it. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:98-105, :479-481, :534 | missing-requirement |
| FND-002 | high | Scope-deferral. Lead default 3 says "QSL-598's scope excludes QSpec FR-354-AC-2..5 (FR-314 certificate checking), which belong to FR-314's ticket", and names no ticket. QSL-598 is "TrustBasis including uncertified". FR-354-AC-2 to AC-4 are the `certified`, `uncertified` and `certificate-rejected` settlements of that trust basis, and AC-5 is the cross-check in FND-001. Under the standing rule a ticket's own scope is not split, and a needed follow-up gets a named STD ticket. Fix: specify those settlements in this PR or show they are already specified and tagged on main. If they belong elsewhere, name the STD ticket that owns them. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:98-105 | missing-requirement |
| FND-003 | high | Scope-narrowing. On main, FR-291 makes a frame or total-byte limit record name the limit and its value. The PR removes both, and lead default 1 makes their return conditional: "carry the limit and value only if QSpec FR-462 and FR-331 add such members; this PR does not add them". Matching the wire is right, since FR-331's `FailedCause` has `defect` alone. But the diagnostic is dropped with a conditional deferral and no ticket. Fix: open the QSpec STD ticket that adds limit and value members to `tool-failure{frame-length}` and `{total-bytes}`, and reference it. Otherwise keep the limit and value on the in-process `ToolFailure` and state that the wire omits them. | spec/functional/FR-291-bound-plugin-runs-and-settle-plugin-failures.md:42, :66-70, :102 | wrong-requirement |
| FND-004 | medium | The PR asserts how a cvc5/Z3 disagreement reaches QSL ("already settled `inconclusive` (QSpec FR-349)"), but QSpec is inconsistent on this, and AC-16 has no disagreement case. QSpec FR-349-AC-3 settles a disagreement `inconclusive`. QSpec TC-269 TB-08 expects the same certificate with a disagreeing cross-check to "settle `proved`, `certified`", and QSpec FR-354 says the cross-check "never changes the certification". Fix: add a disagreement case to AC-16 that states which result QSL receives, and raise the FR-349 and TB-08 conflict as a QSpec STD ticket. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:101-103, :534 | wrong-requirement |

## Verdict

Not mergeable. FND-001 to FND-003 are scope-deferrals or narrowings under
the standing rule. The Z3 parity signal is moved to a record no spec
defines, the FR-354-AC-2 to AC-5 settlements are excluded without a named
ticket, and the byte-limit diagnostic is dropped under a conditional
deferral. FND-004 is an unresolved QSpec inconsistency that AC-16 does not
cover. The fourteen-variant `ToolFailure` table and its serialization are
correct.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 53cd9e9a: "The terminal record holds the SMT backend's Z3 cross-check beside the certification". FR-290's cross-check table gives `cross_check: Some(CrossCheck { tool, verdict })` on `proved`. QSpec #412 (STD-423) adds FR-331's `cross_check` member, and the body sets the merge order with #412 first. |
| FND-002 | fixed | 53cd9e9a: FR-290-AC-17 covers QSpec FR-354-AC-2 to AC-4 (`certified`, `uncertified` with no certificate and with a `hole` step, and `CertificateRejected { QueryMismatch, Unrolling }`), with TC-774 step 13. Nothing is handed off. |
| FND-003 | fixed | 53cd9e9a: `FrameLength { limit, value }` and `TotalBytes { limit, value }` keep the payload, and FR-291-AC-1 asserts `"limit": 1048576, "value": 1048577` and `"limit": 4194304, "value": 4194305`. QSpec #412 adds the members to FR-462 and FR-331's `FailedCause`. |
| FND-004 | fixed | 53cd9e9a: FR-290-AC-16 has disagreement cases: certified with Z3 `Refuted` stays `proved`, `certified`; uncertified with `Refuted` and a counterexample with `Proved` settle `CrossCheckDisagreement`. QSpec #412 amends FR-349-AC-3 and TB-08 to the same rule. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-005 | medium | The cross-check table has no row for cvc5 `inconclusive`. A cvc5 `inconclusive` with Z3 `Proved` or `Refuted` is a disagreement, but the table does not say whether it settles `inconclusive` with `CrossCheckDisagreement { primary: Inconclusive, .. }`, as QSpec #412's `primary` enum allows, or with cvc5's own cause. The table is also silent on whether a certified proof with Z3 `Inconclusive` counts as a disagreement. Add the cvc5-`inconclusive` row, and state the outcome to match QSpec #412 (SR-16250 FND-003). | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md (cross-check table) | missing-requirement |
