---
id: SR-13426
title: 'Spec review of PR #1087: what S3 decides and what EN-1 decides about access classification, integrity'
type: SpecReview
analysis: integrity
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

Ticket: STD-218. PR: quire-spec-language#1087, head 7f7f7545b, scoped to `git diff refs/remotes/origin/main...7f7f7545b` (one commit, three files, +34/-2). Spec-only. It adds a 'What S3 decides and what EN-1 decides' section to FR-220, one sentence to ADR-025 MA-2, FR-220-AC-7 to AC-10, and TC-665 steps 6 to 9. Merged code PR #1053 (QSL-558) depends on this text. Integrity review: stale S3-classification text across FR-220, ADR-025 MA-2 to MA-4 and TC-665, and consistency with FR-221 to FR-226. Measurement. The S3 engine on main (the code of PR #1053, merged as 1bcc1584d, an ancestor of 812c9543) was run in a scratch clone at 7f7f7545b. A probe test was appended locally to qsl-semantics/src/check/protocol_memory/tests.rs and then reverted. It reuses that file's own fixtures (`litmus`, `sb`, `store_buffering_model`: `Cell{v,w}`, `Core{r}`, `Cell::set`, `Core::read(src)`, `Cell::both`, `Cell::flip`). The results are the same under `sc`, `memory tso` and `memory ra`. AC-7, two `relaxed` `Cell::set` attempts by `X` in the two branches: admitted, each `class: None`, `ordering: Relaxed`, atomic. AC-8, `Ly` then `Core::echo` (which reads `r`) in one branch: admitted, every class `None`. AC-9 (and AC-1), `SB` with `release`/`acquire`: admitted, `Sx`, `Ly`, `Sy` and `Lx` all `class: None`. AC-10, `attempt Lone` outside every `parallel` with `ordering seq_cst` (and also with `relaxed` or `acquire`): refused `ill_typed`/`operator-ineligible`, `AccessOrdering{attempt: "Lone", class: Local}`, located at the ordering word. Without an ordering it checks. Beyond the new ACs: `Cell::both`, which writes `self.v` and `self.w`, is admitted under `memory tso`, although AC-3, MA-4 and FR-220's Access shape say S3 refuses it. `Cell::flip`, an RMW, with no ordering is admitted, although MA-3 and AC-2 say S3 refuses it. `SB` with its loads at `ordering release` (`Core::read` reads `src.v` and writes `self.r`) is admitted, although MA-3 and AC-2 say S3 refuses a load with `release`. A store with `acquire`, a read-only attempt with `release`, and a no-data attempt with `relaxed` each refuse `ill_typed`/`operator-ineligible`. The engine's own 10 protocol_memory tests pass. The full results are in sr-1087-tools/probe-results.txt. Code search: no code on main implements EN-1 sharing, branch-locality or `WeakAccessShape`, so the EN-1 halves of AC-8 and AC-9 cannot be measured today. Text checks. The grammar is `attempt N by R on M::T::op`, with no receiver expression, and ADR-027 ST-1 draws the receiver from the finite domains at each application. So the claim that S3 has no object identity holds, and so does MA-1's sharing over instantiated footprints. FR-226's load-buffering note (merged with #1083) already puts the shape at EN-1's pre-check, which agrees with this PR. `ill_typed`/`operator-ineligible` is the refusal MA-3 already uses for an ordering, and it is what the engine returns for AC-10. Methods: spec-review/base (SR-13424), spec-review/ears (SR-13425), spec-review/integrity (SR-13426). code-review and gap-analysis were not run, because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none of them in the changed files. tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-220-description | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md | examined |
| FR-220-outputs-class | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md | examined |
| FR-220-classify-bullet | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md | examined |
| FR-220-AC-1 | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md | examined |
| ADR-025-MA-2 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | examined |
| TC-665-scope | spec/test-cases/TC-665-access-orderings-and-classification.md | examined |
| TC-665-steps-6-9 | spec/test-cases/TC-665-access-orderings-and-classification.md | examined |
| ADR-025-MA-1 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | context_only |
| ADR-025-MA-3 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | context_only |
| ADR-025-MA-4 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | context_only |
| FR-226-load-buffering-note | spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md | context_only |
| code-1053-classify | qsl-semantics/src/check/protocol_memory.rs (main 812c9543, from PR #1053) | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The PR adds the split but leaves unchanged text that contradicts it. AC-1 says 'S3 classifies `Sx` and `Sy` as atomic stores ... and (`c1`, `r`) and (`c2`, `r`) as branch-local', against the new 'S3 SHALL NOT ... label a location shared or branch-local'. The engine gives `None` for every one of these (measured), and #1053's body records AC-1, AC-2 and AC-3 as 'partial at S3 pending STD-218'. The same contradiction appears in four more places. The Description says 'S3 SHALL classify every attempt in every `parallel` as a load, store, ... or local access over its shared footprint'. The Outputs say '`AccessClass::{Load, Store, Rmw, Local}` symbolically over its shared footprint'. The second Sharing and classification bullet says 'S3 SHALL classify each attempt ... symbolically over receivers and parameters: a load reads its one shared location only ...'. And MA-2's own first sentences say 'S3 classifies every attempt ... symbolically', in the row that now ends with the opposite. MA-3's 'or an RMW with none, refuses at S3' and MA-4's S3 sentence are covered by base FND-002 and FND-003. TC-665's title and step 1 ('read each attempt's class'; expected 'the classes of FR-220-AC-1') inherit the problem. Fix: restate each one to the single split the base findings settle on. | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:133 |
| FND-002 | low | Steps 6 to 8 expect EN-1 results ('two atomic stores', 'the register is branch-local after instantiation', 'the classes of FR-220-AC-9') from a TC whose title and steps 1 to 4 and 6 to 9 are S3 checks. Step 5's pre-check is the only EN-1 step, and no EN-1 sharing code exists on main. The engine's S3 tests for these fixtures already exist with TC-665 tags only, so a test tagged `FR-220-AC-7` to `AC-9` from S3 alone would claim a criterion it covers only in part. Split the expected results into the S3 part and the instantiated part, and name where the instantiated part runs (step 5's pre-check or EN-1's TC). | spec/test-cases/TC-665-access-orderings-and-classification.md:28-31, 45-48 |

## Verdict

One medium finding (unchanged FR-220, MA-2 and TC-665 text still has S3 classify and label branch-local) and one low one (TC-665 steps 6 to 8 expect EN-1 results). What holds: TC-665's scope runs to AC-10, each new AC has a step and an expected result, the tag convention is unchanged, and index completeness passes. FR-221, FR-222 and FR-224 consume 'the classified accesses (FR-220)' with no stage named, so they stay true under the split. FR-226 and MX-4 after #1083 already put the load-buffering shape at EN-1. MA-1 is unchanged and agrees with the new text. This needs a decision. The STD-218 ruling text keeps 'the two shared fields of the receiver shape' at S3. The merged code (PR #1053, whose body records the planner's split) defers that shape and the access kind to EN-1. The PR's own preamble argues that neither can be decided at S3. This review recommends aligning the spec with the code.
