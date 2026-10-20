---
id: SR-13481
title: 'Spec review of PR #1095: FR-149 simulation certificate checker follow-up, EARS'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@5fc5e5eb5; git diff refs/remotes/origin/main...5fc5e5eb5 (PR #1095, base main 469f8b49c): spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md, spec/test-cases/TC-556-the-simulation-certificate-checker-accepts-and-rejects.md; read against FR-141, FR-144, FR-146, FR-338, FR-339, FR-163, ADR-018 PC-2/PC-3/SM-4, ADR-020 CT-1 to CT-3, QSpec FR-331/FR-379, and the code of PR #1089 (head ba1d01ad2, qsl-replay/src/certificate/simulation.rs, qsl-replay/src/certificate.rs) and qsl-replay/src/proof_result.rs on main; ticket QSL-549'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-149
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-556
  type: reviews
---

## Summary

Ticket: QSL-549. PR: quire-spec-language#1095, head 5fc5e5eb5, scoped to `git diff refs/remotes/origin/main...5fc5e5eb5` (one commit, two files, +69/-21). Spec-only. EARS review of the changed FR-149 Behavior bullets and AC-5 to AC-8. The PR is the spec follow-up to code PR #1089 (head ba1d01ad2; no fix round is pushed yet, the PR has that one commit), and answers SR-13430 FND-003 to FND-007 and SR-13431 FND-002. Each new rule was checked against the merged text and the #1089 code. Matches: `CertifiedEdge.transition: Option<ModelTransition>` with `None` as the SM-4 stutter is what the code writes, and a refinement is always under an infinite-trace profile (ADR-020's `refinement <Name> using <infinite-trace profile>`, FR-142's stutter edge), so an unconditional stutter edge is right. `FailedStep` restricted to Fails/Undefined/Undetermined is the code's type; the six wire kinds are the code's `FailedStep::kind()` strings, and `Fails(RefinementFailure::Undefined)` cannot come from `check_step` (FR-141: the engine writes it from `StepVerdict::Undefined`), so the six cover every reachable verdict. SuccessorsDiffer comparing transitions only and PostDiffers judging post and `Taken` is what the code does and what AC-2 needs; ADR-020 CT-2 (b) and (c) still read true. `Stopped(IncompleteCause)` with `LimitReached{limit, value}` / `Cancelled{source}` matches `TemporalCertificateCheck::Stopped` and the code's `IncompleteCause` on main. BadState as 'a concrete contract undecided' is the code's `ContractUndetermined` guard and FR-338-AC-3's meaning. Does not match: the wire locus. #1089's serializer writes `position` as a JSON number and `verdict` as the bare kind string; FR-149 now asks for decimal strings and a kind-plus-fields object. That is the owner's decision and an implied code change the PR body lists, not a spec defect. FND-003's own reproduction (SR-13430) had the meter run out in `check_initial` at work 1 and 5 (`InitialMismatch`) as well as in a mapping row at work 50 and 100 (`StepFails`); the new Meter stops rule covers only the second (base FND-001). Gates on the head: make check-spec-validation exits 0 with 17 warnings, the same count as on main, none in FR-149 or TC-556. tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). Methods: spec-review/base (SR-13480), spec-review/ears (SR-13481), spec-review/integrity (SR-13482). code-review and gap-analysis were not run, because no code or test file changed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-149-identity | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-package-identity | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-shape | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-undecided-state | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-successors | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-steps | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-meter-stops | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-limits | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-AC-5 | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-AC-6 | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-AC-7 | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-AC-8 | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two SHALLs contradict as written. Steps: the checker 'SHALL reject with rule `StepFails` ... when it returns `Fails`, `Undefined` or `Undetermined`', with no exception. Meter stops: for a meter-only `Undetermined` the checker 'SHALL stop ... and SHALL NOT reject'. Read separately, a meter-only `Undetermined` must be rejected and must not be. Fix: make the exception part of the Steps statement ('when it returns `Fails`, `Undefined`, or an `Undetermined` that Meter stops does not cover'), or state Meter stops as taking precedence. | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:147-151 |
| FND-002 | low | The changed Limits bullet now carries four SHALLs (count positions and edges, stop at a reached limit, end on a `StepVerdict::Stopped`, end on a cancellation), each separately testable. The grammar lint did not flag it (no warning in FR-149). Split it into one statement per stop, which also gives the cancellation rule its own line. | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:159-164 |
| FND-003 | low | The new bullet has two SHALLs (compare, refuse) and a third clause about another actor ('the checker reads the refinement node only'), which restates the Identity bullet as a non-requirement. Write one statement ('Before it calls the checker, FR-144 SHALL refuse ... a certificate whose ... differs from the item's recompile'), and drop the restatement or fold it into Identity. | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:124-128 |

## Verdict

One medium finding (Steps and Meter stops contradict as written) and two low compound statements. The Shape, Undecided state, Successors and Identity bullets are well-formed single-subject statements (Shape's three conditions are one rule). Meter stops' 'SHALL stop ... and SHALL NOT reject' states one behaviour and its negation, which is fine. AC-6 to AC-8 are direct, falsifiable assertions; AC-5's meter clause is base FND-002. The changed text describes behaviour and its reason, not a log of edits.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | In the new Step stop and Expansion stop bullets the subject of SHALL is a verdict or a stop ('A `StepVerdict::Stopped` ... SHALL end the check', 'An FR-120 expansion stop ... SHALL end the check'), not the system that must act. Every sibling bullet's subject is the checker. Write them the same way, for example 'When a `check_step` it calls returns `StepVerdict::Stopped`, the checker SHALL end the check with ...'. | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:194-197 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after: - **Steps.** For each edge, the checker SHALL run `check_step` (FR-141) from the position and SHALL reject with rule `StepFails`, carrying the verdict, when it returns `Fails`, `Undefined`, or an `Undetermined` that Meter stops does not cover. |
| FND-002 | fixed | after: - **Count of positions.** The checker SHALL count positions against `max_states`, with checked arithmetic. - **Count of edges.** The checker SHALL count edges against `max_transitions`, with checked arithmetic. - **Limit stop.** The checker SHALL stop at a reached limit with `Stopped(LimitReached{limit, value})`. - **Step stop.** A `StepVerdict::Stopped` from a `check_step` it calls SHALL end the check with `Stopped(LimitReached{limit, value})` naming that limit. - **Expansion stop.** An FR-120 expansion stop that names no limit and is not an internal failure SHALL end the check with `Stopp... |
| FND-003 | fixed | after: - **Package identity.** Before it calls the checker, FR-144 SHALL refuse with `stale_dependency`/`content-mismatch` a certificate whose `concrete_package`, `abstract_package` or `obligation` differs from the item's recompile (FR-098). |

Round 1 verdict: Round 1: FND-001 to FND-003 are fixed. The Limits split gives one SHALL per bullet. Steps carves out Meter stops, and Post is its own bullet. One new low finding: in Step stop and Expansion stop the subject is a verdict, not the checker (FND-004). Stops' 'SHALL end ... and SHALL NOT turn into a rejection' states one behaviour, which is fine.
