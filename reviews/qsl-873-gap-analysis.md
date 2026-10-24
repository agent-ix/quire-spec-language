---
id: SR-12932
title: "Gap analysis of quire-spec-language PR #1015: QSL-873 NoInstance outcome tests against FR-125, FR-127, FR-169 and QSpec FR-361-AC-8"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f6c5a7467c31321d1c91065a5151c783e3e25d28; FR-125-AC-4, FR-125-AC-7, FR-127-AC-1, FR-169-AC-1, QSpec FR-361-AC-8; TC-520, TC-522, TC-594, QSpec TC-294; qsl-analyze/tests/it/closure_certificate.rs, qsl-analyze/tests/it/model_check.rs, qsl-analyze/tests/it/model_settle.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: reviews
---
# SR-12932: Gap analysis of quire-spec-language PR #1015: QSL-873 NoInstance outcome tests against FR-125, FR-127, FR-169 and QSpec FR-361-AC-8

## Summary

Ticket: QSL-873 (narrowed: the instancing derivation moved to QSL-886, blocked by STD-195). PR quire-spec-language#1015 at head f6c5a7467. Planless, diff-scoped gap analysis of the three changed test files against the acceptance: tests back FR-125, FR-127, FR-169 and QSpec FR-361-AC-8, nothing beyond. I read the computed matrix with `quire matrix --format tsv` (quire 0.36.2). Every tag the PR adds parses and binds: FR-125-AC-7 to closure_certificate.rs:670 and model_check.rs:707, FR-127-AC-1 to model_settle.rs:523. FR-125-AC-1, AC-2, AC-4 and FR-169-AC-1 remain `untagged`. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `zero_instances_report_no_instance_after_the_requires_bound_pre_check` is tagged `FR-125-AC-7`, which `quire matrix` now reports as `tagged` (model_check.rs:707), but it covers one of AC-7's seven clauses: zero instances give `NoInstance` before exploration, and a population with no universe gives `RequiresBound`. The zero instances come from clearing `Letters.bindings` by hand, not from an empty universe, because instancing is not derived from universes yet (qsl-replay/src/model_check/item.rs:87 reads the caller's bindings; QSL-886). ReachesTwo/ReachesThree giving `a` then `b`, a claim with no `over` giving one instance, `{a}` with no re-check, subtype populations and the empty-universe derivation are all untested and unimplemented, yet the matrix reads AC-7 as covered. What this test does match is QSpec FR-361-AC-8: zero instances give `no-instance` before any state is explored, and a population with no universe gives `requires-bound`, not `no-instance`. The acceptance names that AC, and the repo's `QSpec-` tag convention (AGENTS.md 'Trace tag ids'; schedulability.rs:227) supports it, but the PR only cites it in prose. Retag `#[trace("TC-520", "QSpec-FR-361-AC-8")]` (plus `QSpec-TC-294` if wanted), and leave FR-125-AC-7 to QSL-886. | qsl-analyze/tests/it/model_check.rs:705 |
| FND-002 | medium | `a_clause_with_no_instance_settles_inconclusive_never_proved_or_refuted` is tagged `FR-125-AC-7` (closure_certificate.rs:668). It checks only the clause 'zero instances ... `NoInstance`, settled `inconclusive`, never proved', and only for bindings cleared by hand (its own doc comment says 'deriving them from the subject's universes is not covered here'). It is the settlement half of QSpec FR-361-AC-8 ('settles `inconclusive` with cause `no-instance` ... never `proved` or `refuted`'), which is the AC it should carry. Same fix as FND-001: tag `QSpec-FR-361-AC-8`, not `FR-125-AC-7`. | qsl-analyze/tests/it/closure_certificate.rs:668 |
| FND-003 | low | FR-127-AC-1 lists six rows. `each_outcome_settles_its_row_of_the_map`, which now carries the only AC-1 tag, settles four of them: BoundReached{2}, UndecidedSuccessor, NoInitialState and Stopped{MaxStates, 2}. It also settles NoInstance (V-6) and cancellation. The `Holds{Exhaustive}` -> `Proved{Exhaustive, Certified}` row is settled in `a_proof_settles_certified_only_when_its_certificate_is_accepted` (closure_certificate.rs:474, tagged TC-525 only). The reproduced `Violated` row is settled in `a_deadlock_violation_settles_refuted_through_its_replay` (model_settle.rs:633, tagged TC-522 only). The AC is tested in full across the suite, but the trace points at only part of it. Add `FR-127-AC-1` to those two tests. | spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:212 |
| FND-004 | low | The deferral of TC-594 / FR-169-AC-1 is correct, but the stated reason is wrong. The PR says 'no state-graph checker exists in this repo yet'. #968 (QSL-535) did land the S3 state-graph claims checker (qsl-semantics/src/check/state_graph_claim.rs). What is missing is everything FR-169-AC-1 needs: no state-graph engine outcome (`Witnessed`, `Trapped`, `NoDecision`, `PathPair`), no `ProofBasis::Witness` (qsl-replay/src/proof_result.rs:31-56 has Checks, Exhaustive, BoundedComplete, Inductive, ClosedForm), and no state-graph settle function in qsl-analyze or qsl-replay. A FR-169-AC-1 test cannot be written now. The owner is QSL-536 (E11, Specced, which implements FR-166 to FR-170). The QSL-873 description still lists TC-594 and QSpec TC-294 as tags this ticket delivers. Update the ticket to point TC-594 at QSL-536, and fix the PR body's reason. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:210 |

## Verdict

No production code changed, and none was needed: NoInstance is already wired end to end (check.rs:260-264, settle.rs:202/216, proof_result.rs:279/315). No test goes beyond the acceptance. The problem is accuracy of the trace tags, not missing behaviour. Both new tests claim FR-125-AC-7 in full on evidence for one of its clauses, while the QSpec AC they do match, FR-361-AC-8, is left untagged (FND-001, FND-002). Both are tag-only fixes within this PR. FR-169-AC-1 cannot be tested until QSL-536 lands (FND-004).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The new `a_clause_over_a_subject_with_no_initial_state_explores_nothing` is tagged `FR-125-AC-4`, so `quire matrix` now reports AC-4 as `tagged` (model_check.rs:776). The test covers only AC-4's second sentence (an empty `initial` list is `NoInitialState` before exploration). The first sentence, 'a clause with `over (c: Config::ConfigVersion)` yields one instance per object of `{a, b}`', is the instancing derivation, which is unimplemented and untested (QSL-886). This is the same partial-AC overclaim that FND-001 removed for AC-7. The repo's own precedent refuses it: model_check.rs:875-881 leaves FR-336-AC-2 untagged because its test covers only the refusal half. Drop `FR-125-AC-4` and keep `#[trace("TC-520")]`, leaving AC-4 to QSL-886. The test body itself is sound: it is the only clause-item NoInitialState check (the others use DeadlockFreedom) and asserts zero product states. | qsl-analyze/tests/it/model_check.rs:774-777 |

## Dispositions

Round 1, reviewed at 55d60775348fff9b91c8c231c415069c3bc7acbc: fix commit 9576eb311, plus merge 55d607753 of origin/main. The merge brings in unrelated QSL-542/QSL-568/STD-193 work and touches none of the three test files' QSL-873 hunks. At this head, run with the lane's warm cache (`CARGO_HOME=~/.cargo-draco-lane2`, `--offline`): `cargo test -p qsl-analyze --test it` passes 67/67, including all five touched tests; `cargo clippy -p qsl-analyze --tests -D warnings` is clean; rustfmt --check is clean on the three files. `quire trace --id QSpec-FR-361-AC-8` resolves both retagged tests (closure_certificate.rs:670, model_check.rs:707). `quire matrix` shows FR-125-AC-7 untagged, FR-125-AC-4 tagged (model_check.rs:776), FR-127-AC-1 tagged by three tests (closure_certificate.rs:474, model_settle.rs:523, model_settle.rs:633), and FR-169-AC-1 untagged.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 9576eb311 |
| FND-002 | fixed | 9576eb311 |
| FND-003 | fixed | 9576eb311 |
| FND-004 | deferred | The PR body's reason is corrected: it now says #968 landed the S3 checker, while FR-169's engine outcomes, `ProofBasis::Witness` and the state-graph settle do not exist yet (QSL-536 area). The QSL-873 description still lists TC-594 and QSpec TC-294 as tags. That edit is the lead's Linear bookkeeping, not a PR change, and does not block. |
