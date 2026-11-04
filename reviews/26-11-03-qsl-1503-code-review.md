---
id: SR-2224001
title: "Code review (rust-review lane) - QSL-1503 temporal evidence instances, ledger and call, PR 2224"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-eval/src/temporal/evidence/{trigger,activation,ledger,request,call,work}.rs; qsl-eval/src/temporal/finite_window/run.rs; qsl-eval/src/temporal/ledger.rs; qsl-replay/src/spine/clause/{progress,temporal}.rs; qsl-eval/tests/it/{temporal_activation,temporal_relation}.rs"
review_set: subset
---

## Summary

Ticket: QSL-1503. PR 2224 head d49930ea0, already based on current origin/main 66687365a (the trial merge is a fast-forward). Diff: 17 files, +4639/-16, no file deleted, nothing under the src/temporal root removed.

Gates run in the trial merge with qsl-slot and qsl-timeout (QSPEC_DIR=/Users/peter/dev/quire-specification):

- cargo fmt --all -- --check: exit 0
- cargo check --locked --workspace --all-targets: exit 0
- cargo clippy --locked --workspace --all-targets -D warnings, default features: exit 0; with --no-default-features: exit 0
- arch-lint qualified-core --qsl .: exit 0; arch-lint api-surface --qsl . --qsl-only: exit 0
- cargo test -p qsl-eval: all green (181 unit, 1019 it, 42 doctests including the compile_fail ones)
- cargo test -p qsl-replay --no-fail-fast: 19 failures (14 lib, 4 dispatch_link_compile, 1 doctest). The same 19 names fail on clean origin/main; the delta is empty.

Production code has no unwrap, expect, panic, allow, thread_local or static. Counts are cast with `as u64` from usize only, and every arithmetic that can grow a counter goes through the checked charge path or `checked_add`.

Mutation probes, run on the trial and reverted (qsl-eval `it` tests filtered to `temporal`, 97 tests):

- lineage charge raised by one: killed, but only through a panic in the chain helper, not through an assertion on the ceiling-3 completion (FND-001).
- authority_digest cleared in the echoed correspondence: survived (FND-002).
- evicted-valuation matcher no longer compares the atom: survived (FND-004).
- withdrawn-completeness non-value branch disabled: survived (FND-003).
- silent-sample valuation changed from False to True: survived (FND-005).
- closed/integral check of ClauseWork::of for timed intervals removed: survived (FND-006).

The evidence call follows FR-516, FR-524 and FR-526 in the places the tests reach: refusal order (binding, control, progress, relation, list mismatch, contradiction, trigger selection) happens before any charge; the relation causes are checked in FR-526's order; the ledger has private fields, `pub(super)` constructors and compile_fail doctests; per-occurrence keys (ObligationKey and LedgerBinding.occurrence) are used on both the evidence and the progress ledger, and the replay test for FR-502-AC-12 fails if the occurrence is dropped from the key.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-526-AC-12 says the call completes with the lineage ceiling at 3. The test only asserts `chain(3).lineage == 3`, which a stopped result also carries (`lineage: facts.lineage.unwrap_or_default()`), so the completes half has no assertion. FR-516-AC-7's loop skips every counter whose usage is 0, and an Original call uses 0 lineage, so no test checks the ceiling boundary of `lineage` for a completing call. A charge of lineage + 1 was killed only because the chain helper then panicked on a stopped predecessor. Fix: assert truth is Some and incomplete is None at ceiling 3. | qsl-eval/tests/it/temporal_relation.rs:231-268 |
| FND-002 | medium | FR-516-AC-9 requires each of contract, schema_digest, digest and authority_digest to be echoed byte for byte. The test sets `contract` and `schema_digest` to the values they already hold (empty), asserts only `correspondence.digest` on the result, and never reads the echoed position observation (`support[].observation`). Clearing the echoed authority_digest passes. Fix: use distinct non-empty values for all four, and assert the echo on the correspondence and on a support observation. | qsl-eval/tests/it/temporal_activation.rs:1125-1147 |
| FND-003 | medium | FR-516-AC-3 withdraws only `decision_closure`. The code has a second branch for a withdrawn completeness record and the AC names a withdrawn support fact; neither is exercised (disabling the completeness branch passes). The test also does not assert the truth, so "the FR-043 values of the remaining evidence" is not checked. Fix: add the completeness and support-fact withdrawals, and assert truth, settlement and support. | qsl-eval/tests/it/temporal_activation.rs:1150-1167 |
| FND-004 | medium | FR-524-AC-24 asserts only `EvictedValuation { position: 0, .. }`. The matcher also compares the atom and the instance, and dropping the atom comparison passes. The AC says the result names the evicted subject. Fix: assert instance and atom, and add a case where an evicted record for another atom or another instance leaves the verdict untouched. | qsl-eval/src/temporal/evidence/call.rs:773-790, qsl-eval/tests/it/temporal_activation.rs:976-997 |
| FND-005 | medium | The new `EvidenceLetters::valuation` reads every atom at a silent sample as False (FR-504 false extension). No test reaches that branch through the call: changing it to True passes all 97 temporal tests. Fix: a call whose window extends past the last observation with `p` false in the silent tail and true at the last observation, asserting the verdict and the `evaluation_steps` charge. | qsl-eval/src/temporal/evidence/call.rs:760-772 |
| FND-006 | medium | `ClauseWork::of` now accepts a timed interval that is closed with an integral upper bound. Nothing tests either side: removing the check passes every test, so an open or fractional interval is no longer shown to return `ReachFault::Timed`. The check also differs from the finite-window planner (qsl-eval/src/temporal/plan.rs:94-97 requires both bounds integral and both ends closed; work.rs requires only the upper bound integral), and an unreadable clause is turned into `CallFailure::Fault("clause-reach-is-finite")`, an internal fault, rather than a typed refusal. Fix: unit tests for accepted and rejected intervals, the same predicate in both places, and a typed refusal for an interval the profile does not admit. | qsl-eval/src/temporal/evidence/work.rs:191-207, qsl-eval/src/temporal/evidence/call.rs:1100 |
| FND-007 | medium | A clause `on origin` that declares captures can never activate. Origin arrivals pass `captures: &[]` to `establish`, whose count check returns a Missing fault naming the first declared capture. FR-337-AC-8 admits captures under `on origin` (slots 1 and 2), and `ExecutionOrigin` has no captures member, so every such clause is `Unavailable` with `non_value.dimension` = the capture name. No test covers it, and the origin tests use clauses without captures. The spec lane must rule how an origin supplies captures; until then the behavior should be tested and named. | qsl-eval/src/temporal/evidence/activation.rs:344-352, qsl-eval/src/temporal/evidence/activation.rs:494-496 |
| FND-008 | low | Refusal causes `activation-list-mismatch`, `temporal-refusal`, `progress-inconsistent` and `progress-ledger-limit` are not in FR-516's refusal list, and `temporal-refusal` is returned from `evaluate` after the call has charged units, so its charge log is not empty although FR-516 says every refusal of the call itself charges nothing. Fix: name the causes in FR-516, and either move the refusal ahead of the first charge or return it as an Incomplete/Unavailable result. | qsl-eval/src/temporal/evidence/call.rs:1219-1227 |
| FND-009 | low | Identity members are built from Debug text: the relation predecessor (`format!("{prev:?}")`), `ValueType` of each capture and `UndefinedCause` of a valuation. A change to a Debug implementation silently changes every `result_identity`. FR-516 and FR-522 call the identity an equality of members, not a string. `ResultIdentity` has bytes to feed to the encoder directly. | qsl-eval/src/temporal/evidence/call.rs:359, :516, :564-567 |
| FND-010 | low | Three "unchanged" assertions cannot fail. FR-524-AC-20 compares `first.ledger` with `first.ledger.clone()`, FR-526-AC-3 and AC-14 compare an owned `r1` with the clone taken before the later calls, and FR-516-AC-17's "progress ledger unchanged" checks `kept.len() == 1`. The behavior is true by construction (the call returns a new ledger and `Err` returns none), but the assertions protect nothing. Fix: assert on the ledger the call returned (for example that a contradicted redelivery leaves `activate`'s returned state equal to the input) or drop the claim from the test comment. | qsl-eval/tests/it/temporal_activation.rs:428-446, qsl-eval/tests/it/temporal_relation.rs:108-126 |

## Verdict

No correctness defect was reproduced in the code that the tests reach, and every gate is green with an empty qsl-replay delta against clean main. The PR is not mergeable as it stands: five test-strength findings (FND-001 to FND-005) show behavior of the new call that a wrong implementation would pass, FND-006 leaves the one change outside the evidence module (ClauseWork::of) unprotected, and FND-007 is a behavior that silently makes origin clauses with captures unusable. All are fixed by tests and one spec ruling; none needs a design change. The author's deviations are sound: `EvidenceSubject` is required because the in-memory CheckedPackage has no clock rows; FR-516-AC-18 and FR-524-AC-23 are uncovered here and are recorded in SR-2224002.
