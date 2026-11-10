---
id: SR-2026111012423400000004
title: "QSL-1826 predicate main-fix gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1826; PR #2668; stopped reward predicate error precedence and existing TC-1403 regression"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1403
    type: references
---

## Summary

Scoped manual requirement/test/code gap check for the deferred predicate-error repair. No gaps found in the examined slice.

## Verdict

PASS for the manually examined regression slice only. This is not a whole-repository matrix or plan-completeness verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reviewed frozen source head 28818c652dd64ee69d29b400716a42cf01b94a42 on main 15f07c7f1168d1cf98f2607bc6c4e5ed51423810. The final diff is one production file, qsl-replay/src/probability_certificate/document.rs, with 24 additions and ten deletions. No public API, test, fixture, specification, dependency, lock or CI change exists. Ticket: QSL-1826; PR #2668.

Independently traced the public reward preflight through acyclic subject_graph_with_identity, stopped cyclic-DTMC admission, and mdp::admit_reward. The Workload fixture is eligible for DTMC evaluation but MDP admission explicitly requires ProbScheduler::Every and otherwise returns Unsupported. The stopped traversal's relevant predicate error was therefore discarded by the old final fallback. The first acyclic failure is deliberately not authoritative: evaluating after the first target can encounter an irrelevant undefined expression, so retrying the stopped profile remains necessary.

The patch borrows the latest stopped-DTMC error to classify only UndefinedPredicate and Build(ExactProbBuildIssue::Predicate). It still calls MDP admission. Only the combination of that classified stopped error with a later Unsupported restores the stopped error. A later success, cancellation, resource limit, identity failure, or another specific failure is forwarded unchanged. Unsupported/Cycle failures are not converted into predicate errors. Shared meter/storage references, caller limits, identity and cancellation inputs are unchanged; the earlier acyclic error is never restored. No cloned error, display-string classification, new unchecked input, allocation, panic or acceptance token is introduced.

The existing unchanged cyclic_reward_certificate::predicate_undefined_only_after_target_is_not_evaluated is a meaningful regression: its positive half requires successful admission and accepted two-row first-hit evidence when undefined evaluation occurs only after B. Its negative half requires the direct admission to preserve the same exact before-target DivisionByZero error as independent document checking. Reverting this production patch makes that assertion fail on main. The fixture identity remains stable; changing fixture identities or weakening expected errors would not repair the production loss. The existing trace attribute is TC-1403 / FR-201-AC-36. Its after-target subclause binding was inspected; this review does not claim the single test proves every clause of that compound AC.

Read the executed evidence rather than rerunning it: /tmp/qsl1826-predicate-repro.log and .exit show the original focused failure, exit 101; /tmp/qsl1826-predicate-green.log shows the same unchanged test passes. /tmp/qsl1826-predicate-full-it.log reports the FULL unfiltered qsl-analyze integration binary: 1516 passed, zero failed, ignored or filtered, runtime 366.33s. Formatting, qsl-analyze integration-target Clippy, and changed qsl-replay library Clippy exited zero. These are coder-run observations inspected by this reviewer, not fresh reviewer executions. The explicit reviewer instruction excludes Cargo reruns; the lead owns the final full-binary gate. No make ci, Kani, full workspace suite or repository-wide computed matrix was run by this reviewer.

History inspection confirms merged #2664 / 99251159e2 added the third MDP fallback; #2660 supplied the regression and earlier two-profile logic. Codex2's unmerged #2667 adds adjacent None arguments to the subject-graph calls, not this error-precedence repair. No protected branch, PR or worktree was modified. Applicable local AGENTS/CLAUDE instructions and Rust implementation, safety, boundary and testing rules were inspected. No applicable AssuranceProfile is declared in the reviewed spec tree; measurement plans and AD-001 are not silently substituted as profiles.

Plan completion: not assessed.

The changed behavior has an existing owner in FR-201 first-target/deferred-evaluation requirements and an existing executable TC-1403 regression. No new trace tag or coverage promotion is introduced. The original failing behavior, unchanged positive lazy-evaluation guard and exact negative cause check align with the repaired branch. The full integration run supplies regression evidence, not completion of every compound FR-201 criterion. A repository-wide computed matrix was intentionally not run under the dispatch scope; this scoped receipt is not a handwritten TestMatrix.

## Examined Scope

- FR-201-AC-36 — role: examined; spec/functional/FR-201-check-a-probability-certificate.md; excerpt: An admitted A-to-B subject with a positive-cost cycle reachable solely after B accepts first-hit evidence with rows only for A and B; adding after-B value/rank/target keys rejects the specified coverage/objective rule.

- FR-201-AC-38 — role: examined; spec/functional/FR-201-check-a-probability-certificate.md; excerpt: Earlier model refusals and deferred undefined/negative evaluation are not coerced to zero.

- FR-201 — role: examined; spec/functional/FR-201-check-a-probability-certificate.md; excerpt: Earlier FR-187 refusals, including negative reward, retain precedence;
  post-admission undefined/negative query evaluation retains typed
  nonacceptance.

- TC-1403 — role: context_only; spec/test-cases/TC-1403-check-cyclic-dtmc-expected-reward-certificates.md; excerpt: Step 3 counts entry-B cost exactly once, no cost after B, and excludes
  after-B-only nodes from the certificate.
