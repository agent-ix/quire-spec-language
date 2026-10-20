---
id: SR-13482
title: 'Spec review of PR #1095: FR-149 simulation certificate checker follow-up, integrity'
type: SpecReview
analysis: integrity
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

Ticket: QSL-549. PR: quire-spec-language#1095, head 5fc5e5eb5, scoped to `git diff refs/remotes/origin/main...5fc5e5eb5` (one commit, two files, +69/-21). Spec-only. Integrity review: consistency with ADR-020, FR-144, FR-338 and FR-163, and TC-556's scope and steps. The PR is the spec follow-up to code PR #1089 (head ba1d01ad2; no fix round is pushed yet, the PR has that one commit), and answers SR-13430 FND-003 to FND-007 and SR-13431 FND-002. Each new rule was checked against the merged text and the #1089 code. Matches: `CertifiedEdge.transition: Option<ModelTransition>` with `None` as the SM-4 stutter is what the code writes, and a refinement is always under an infinite-trace profile (ADR-020's `refinement <Name> using <infinite-trace profile>`, FR-142's stutter edge), so an unconditional stutter edge is right. `FailedStep` restricted to Fails/Undefined/Undetermined is the code's type; the six wire kinds are the code's `FailedStep::kind()` strings, and `Fails(RefinementFailure::Undefined)` cannot come from `check_step` (FR-141: the engine writes it from `StepVerdict::Undefined`), so the six cover every reachable verdict. SuccessorsDiffer comparing transitions only and PostDiffers judging post and `Taken` is what the code does and what AC-2 needs; ADR-020 CT-2 (b) and (c) still read true. `Stopped(IncompleteCause)` with `LimitReached{limit, value}` / `Cancelled{source}` matches `TemporalCertificateCheck::Stopped` and the code's `IncompleteCause` on main. BadState as 'a concrete contract undecided' is the code's `ContractUndetermined` guard and FR-338-AC-3's meaning. Does not match: the wire locus. #1089's serializer writes `position` as a JSON number and `verdict` as the bare kind string; FR-149 now asks for decimal strings and a kind-plus-fields object. That is the owner's decision and an implied code change the PR body lists, not a spec defect. FND-003's own reproduction (SR-13430) had the meter run out in `check_initial` at work 1 and 5 (`InitialMismatch`) as well as in a mapping row at work 50 and 100 (`StepFails`); the new Meter stops rule covers only the second (base FND-001). Gates on the head: make check-spec-validation exits 0 with 17 warnings, the same count as on main, none in FR-149 or TC-556. tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). Methods: spec-review/base (SR-13480), spec-review/ears (SR-13481), spec-review/integrity (SR-13482). code-review and gap-analysis were not run, because no code or test file changed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-149-outputs-rules | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-outputs-stopped | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-package-identity | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-AC-6 | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-AC-7 | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-AC-8 | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| TC-556-scope | spec/test-cases/TC-556-the-simulation-certificate-checker-accepts-and-rejects.md | examined |
| TC-556-steps-6-9 | spec/test-cases/TC-556-the-simulation-certificate-checker-accepts-and-rejects.md | examined |
| ADR-020-CT-2 | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md | context_only |
| ADR-020-CT-3 | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md | context_only |
| FR-144-certificate-bullet | spec/functional/FR-144-request-and-settle-a-refinement-item.md | context_only |
| FR-338-rules-and-check | spec/functional/FR-338-check-an-en-1-closure-certificate.md | context_only |
| ADR-018-SM-4 | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The identity split contradicts unchanged text. ADR-020 CT-3 says 'The checker refuses a certificate whose identities differ from the item's', and CT-1 names all four identities as the certificate's. FR-149 now says FR-144 compares the package ids and obligation and 'the checker reads the refinement node only'. FR-144, the actor of the new SHALL, is unchanged: its Certificate bullet says nothing of a pre-check, so a reader of FR-144 cannot find a duty it now owns. Fix (the owner's decision stands): amend CT-3 to say where each comparison is made, and state the comparison in FR-144's Certificate bullet. FR-149 can then cite it rather than state FR-144's duty. | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:124-128 |
| FND-002 | low | 'FR-338's `CertificateRule` gains `Malformed`, ..., `BadState`' says two existing rules are new. `BadState` is in FR-338's own enum (with the `ContractUndetermined` meaning, FR-338-AC-3), and FR-163 already uses `Malformed` 'before any recomputation' for the product-closure checker. Say that this checker uses `BadState` and `Malformed` and adds `InitialMismatch`, `SuccessorsDiffer`, `StepFails` and `PostDiffers`. | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:94-99 |
| FND-003 | low | `Stopped` is now only `LimitReached` or `Cancelled`, but two cases are left out. First, an FR-120 expansion stop that names no limit and is not an internal failure: #1089 returns `Stopped(ResourceExhausted, None)` for it, and FR-149 now has no alternative for it. Second, FR-144's Certificate bullet settles 'a checker stopped by a limit V-7 naming the limit' and says nothing of a cancelled check. Fix: say what an unnamed expansion stop gives (a named limit, `ResourceExhausted`, or a fault), and let FR-144's bullet cover `Cancelled`. | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:81-92 |
| FND-004 | low | Steps 7 and 8 name no fixture, and TC-556's Fixtures line still lists only `CasRefinesCounter` and its lost-update model. Neither has a terminal state (`peek` is always enabled, SR-13430 FND-002) or a position with an undecided concrete contract. SR-13430 gave working fixtures (`CasRefinesCounter` without `ensure` plus `pre PeekNever`/`pre BeginBNever` for a terminal chain, and a refused contract for `ContractUndetermined`). Name them in the Fixtures line or in AC-6 and AC-7, so the tests that close SR-13430 FND-002 are pinned. TC-556 Scope (AC-1 to AC-8) and the new steps 7 to 9 are otherwise consistent with the ACs. | spec/test-cases/TC-556-the-simulation-certificate-checker-accepts-and-rejects.md:34-37, 52-55 |

## Verdict

One medium finding (ADR-020 CT-3 and FR-144 not amended for the identity split) and three low ones. What holds: AC numbering runs AC-1 to AC-8 with no gap, TC-556's Scope runs to AC-8, each new AC has a step and an expected result, AC-4's and TC-556 step 4's `Stopped` shape agree with Outputs, and ADR-018 SM-4 and ADR-020 CT-2 read true under the new stutter edge and transitions-only `SuccessorsDiffer`. FR-149's relationships and Dependencies already name FR-141, FR-144 and FR-338. Index completeness passes.
