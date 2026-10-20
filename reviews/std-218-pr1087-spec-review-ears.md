---
id: SR-13425
title: 'Spec review of PR #1087: what S3 decides and what EN-1 decides about access classification, EARS'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@7f7f7545b; git diff refs/remotes/origin/main...7f7f7545b (PR #1087, base main 812c9543): spec/decisions/ADR-025-weak-memory-models-for-parallel.md, spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md, spec/test-cases/TC-665-access-orderings-and-classification.md; measured against the S3 engine on main (qsl-semantics/src/check/protocol_memory.rs, merged with PR #1053) and read against ADR-025 MA-1 to MA-4, ADR-027 ST-1, FR-216, FR-221 to FR-226; ticket STD-218'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-220
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-665
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-025
  type: reviews
---

## Summary

Ticket: STD-218. PR: quire-spec-language#1087, head 7f7f7545b, scoped to `git diff refs/remotes/origin/main...7f7f7545b` (one commit, three files, +34/-2). Spec-only. It adds a 'What S3 decides and what EN-1 decides' section to FR-220, one sentence to ADR-025 MA-2, FR-220-AC-7 to AC-10, and TC-665 steps 6 to 9. Merged code PR #1053 (QSL-558) depends on this text. EARS review of the new FR-220 section and FR-220-AC-7 to AC-10. Measurement. The S3 engine on main (the code of PR #1053, merged as 1bcc1584d, an ancestor of 812c9543) was run in a scratch clone at 7f7f7545b. A probe test was appended locally to qsl-semantics/src/check/protocol_memory/tests.rs and then reverted. It reuses that file's own fixtures (`litmus`, `sb`, `store_buffering_model`: `Cell{v,w}`, `Core{r}`, `Cell::set`, `Core::read(src)`, `Cell::both`, `Cell::flip`). The results are the same under `sc`, `memory tso` and `memory ra`. AC-7, two `relaxed` `Cell::set` attempts by `X` in the two branches: admitted, each `class: None`, `ordering: Relaxed`, atomic. AC-8, `Ly` then `Core::echo` (which reads `r`) in one branch: admitted, every class `None`. AC-9 (and AC-1), `SB` with `release`/`acquire`: admitted, `Sx`, `Ly`, `Sy` and `Lx` all `class: None`. AC-10, `attempt Lone` outside every `parallel` with `ordering seq_cst` (and also with `relaxed` or `acquire`): refused `ill_typed`/`operator-ineligible`, `AccessOrdering{attempt: "Lone", class: Local}`, located at the ordering word. Without an ordering it checks. Beyond the new ACs: `Cell::both`, which writes `self.v` and `self.w`, is admitted under `memory tso`, although AC-3, MA-4 and FR-220's Access shape say S3 refuses it. `Cell::flip`, an RMW, with no ordering is admitted, although MA-3 and AC-2 say S3 refuses it. `SB` with its loads at `ordering release` (`Core::read` reads `src.v` and writes `self.r`) is admitted, although MA-3 and AC-2 say S3 refuses a load with `release`. A store with `acquire`, a read-only attempt with `release`, and a no-data attempt with `relaxed` each refuse `ill_typed`/`operator-ineligible`. The engine's own 10 protocol_memory tests pass. The full results are in sr-1087-tools/probe-results.txt. Code search: no code on main implements EN-1 sharing, branch-locality or `WeakAccessShape`, so the EN-1 halves of AC-8 and AC-9 cannot be measured today. Text checks. The grammar is `attempt N by R on M::T::op`, with no receiver expression, and ADR-027 ST-1 draws the receiver from the finite domains at each application. So the claim that S3 has no object identity holds, and so does MA-1's sharing over instantiated footprints. FR-226's load-buffering note (merged with #1083) already puts the shape at EN-1's pre-check, which agrees with this PR. `ill_typed`/`operator-ineligible` is the refusal MA-3 already uses for an ordering, and it is what the engine returns for AC-10. Methods: spec-review/base (SR-13424), spec-review/ears (SR-13425), spec-review/integrity (SR-13426). code-review and gap-analysis were not run, because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none of them in the changed files. tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-220-s3-en1-preamble | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md | examined |
| FR-220-s3-decides-bullet | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md | examined |
| FR-220-en1-decides-bullet | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md | examined |
| FR-220-outside-parallel-bullet | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md | examined |
| FR-220-AC-7 | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md | examined |
| FR-220-AC-8 | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md | examined |
| FR-220-AC-9 | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md | examined |
| FR-220-AC-10 | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md | examined |
| ADR-027-ST-1 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The first bullet has two SHALLs ('SHALL decide only ...' and 'S3 SHALL NOT refuse an attempt, or label a location ...'), and its first SHALL lists four duties. The second bullet joins two duties for EN-1 ('decide which locations are shared and settle an access shape ...'). The grammar lint did not flag them because each SHALL sits in its own sentence. Write one statement per duty, for example 'S3 SHALL NOT refuse an attempt on grounds of sharing.' as its own bullet, and make the list of what S3 decides a set of separate statements, which FND-001 to FND-003 of the base review will reshape anyway. | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:88-95 |
| FND-002 | low | AC-8 asserts an S3 outcome ('checks at S3 with no refusal') and an EN-1 outcome ('EN-1 treats the register as branch-local') in one criterion, and carries its rationale inside it ('because attempts of one branch only touch it'). It is verified by one TC-665 step. No EN-1 sharing code exists on main, so the criterion cannot pass whole today, although its S3 half does (measured). Split it into the S3 assertion and an EN-1 assertion naming the instantiated branch-local location `(c1, r)`, and move the 'because' to the Behavior text or MA-1. | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:140 |
| FND-003 | low | 'S3 sees an attempt's operation and its receiver and parameters symbolically' and, in the same sentence, 'the grammar's ... carries no receiver' read as contradictory. What is meant is that S3 knows the receiver's and parameters' types, while the objects are drawn from the finite domains at each application (ADR-027 ST-1). Say that. | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:82-86 |

## Verdict

Three low findings. The AC-10 behaviour bullet ('When an attempt carries an `ordering` and lies outside every `parallel`, S3 SHALL refuse it ...') is a well-formed event-driven statement, and AC-10 is a direct, falsifiable assertion that the engine meets. AC-7 and AC-9 are direct assertions; their S3-or-EN-1 ambiguity is base FND-004. The changed text describes the system's behaviour and its reason, not a log of edits. The grammar lint raises no warning in the changed files.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after: - S3 SHALL record `class: Local` on an attempt whose footprint touches no data place. - S3 SHALL record no access class on any other attempt. - S3 SHALL NOT refuse an attempt on grounds of sharing. - S3 SHALL NOT label a location shared or branch-local. - S3 SHALL refuse an ordering that no access kind the attempt's footprint leaves possible takes (Admitted orderings). - S3 SHALL refuse an ordering on an attempt outside every `parallel` with `ill_typed`/`operator-ineligible`, naming the attempt: an ordering applies only to an access of a `parallel` branch. - EN-1 SHALL decide which location... |
| FND-002 | fixed | after: \| FR-220-AC-8 \| A branch whose attempts write a register field of its core and then read that register again (the register-read-again shape of message passing and load buffering) checks at S3 with no refusal and no class recorded. \| Test (TC-665) \| \| FR-220-AC-13 \| EN-1 treats the register of AC-8 as branch-local, because attempts of one branch only touch it. \| Test (EN-1 sharing; TC-665 step 12) \| |
| FND-003 | fixed | after: S3 knows the types of an attempt's receiver and parameters and its operation's declared footprint, but not which objects they are: the grammar's `attempt N by R on M::T::op` carries no receiver, and the objects are drawn from the finite domains at each application (ADR-027 ST-1). Whether a field the operation touches is shared or branch-local is therefore not decidable at S3, because sharing is a property of instantiated footprints (ADR-025 MA-1), and the access class depends on sharing. |

Round 1 verdict: Round 1: FND-001 to FND-003 are fixed. The new What-S3-decides list has one SHALL per bullet, AC-8 is S3-only and AC-13 carries its EN-1 half, and the preamble says S3 knows types, not objects (ADR-027 ST-1). The grammar lint raises no warning in the changed files. Mergeable as far as this method goes.
