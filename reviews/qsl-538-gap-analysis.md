---
id: SR-12202
title: "Gap analysis of QSL-538 E8d FR-169 state-graph certificate and checker against FR-169-AC-9, AC-10 (PR #1046)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@cbbdfd75b07b536b0d940b006ebd5276390ea80f; spec/functional/FR-169-*.md, spec/test-cases/TC-643-*.md, TC-644-*.md; qsl-replay/src/certificate/state_graph.rs, qsl-analyze/src/model_check/state_graph/decide.rs, qsl-analyze/tests/it/state_graph_certificate.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: reviews
---
# SR-12202: Gap analysis of QSL-538 E8d FR-169 state-graph certificate and checker against FR-169-AC-9, AC-10 (PR #1046)

## Summary

Ticket: QSL-538 (E8d), the state-graph half. PR quire-spec-language#1046 at head cbbdfd75b. The scope is the diff `origin/main...HEAD` only. Plan completion was not assessed. quire 0.36.2 (engine 0.50.2), quoin 0.28.3.

The PR tags seven tests:

- **FR-169-AC-9, check half (TC-643):** `the_checker_accepts_the_engines_rank_certificates` and `the_checker_accepts_the_engines_count_certificate`.
- **FR-169-AC-10, check half (TC-644):** `the_checker_rejects_altered_rank_certificates`, `the_checker_rejects_a_certificate_over_an_undefined_target` and `the_checker_rejects_altered_count_certificates`.
- **TC-644 only, with no AC:** two unit tests in `qsl-replay`, `malformed_entries_reject_before_recomputation` and `state_graph_rules_and_loci_write_their_wire_spelling`.

**Independence of expected values.** The expected closures come from `crate::state_graph::reachable`, a breadth-first search the test file runs itself over `ModelSystem`. Ranks come from the world's arithmetic (`va`), and orders and loci come from `ModelSimState` field values. None is read from the engine, so the tests are independent of the engine. They share `qsl-eval`'s successor and state-key code with the checker. That is the trusted base FR-169 names, so it is acceptable.

**Every test can fail on its AC's behaviour.** I traced each one:

- The acceptance tests compare closure equality and the exact rank map, require count 1 with the order descending on every non-`to` edge, and assert `Accepted`.
- `(2, 2)` removed rejects `SuccessorMissing` at `min((1,2),(2,1))`.
- `(1, 0)` re-ranked to 2 rejects `RankBroken` at `(0, 0)`. Its successors `(1,0)` and `(0,1)` both now rank 2. `(1, 0)` still has `(1, 1)` at rank 1, and no other state breaks.
- `(0, 0)` unranked rejects `RankMissing`.
- `Divides` rejects `PredicateUndefined` at the least `va = 2` digest. Predicates are read before ranks, so the hand ranks play no part.
- All counts set to 1 on the unsequenced job passes `OrderBroken` with the test's order. It then rejects `CountBroken` at `000` only, because `000`'s successors `100` and `010` both count 1, giving an expected count of 2.
- The reversed order rejects `OrderBroken` at the least non-`done` digest. All three non-`to` nodes break, and `reachable` is a `BTreeMap`, so `find` returns the least.
- The `max_states` 2 check stops with `MaxStates` and 2.

**The three loci the PR says disagree with the spec (departure 3).** I re-derived each one from FR-169's own rule text, and each disagreement is real. The tests assert what the rules give, which is the honest choice. Recorded as FND-003 for the spec fix the lead routes, not blocking.

**The two TC-644-only unit tests.** Both assert exact loci and spellings, and both fail on a regression. They carry no AC because no AC names Malformed or the wire spellings. That is correct, since tagging them to AC-10 would overclaim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Departure 2 has no test. It exempts count-2 nodes from `OrderBroken`, the one place the checker accepts more than FR-169's text, and nothing covers it either way. No test accepts a `unique path` certificate whose `H` has a count-2 cycle that no `from` node reaches, which is the departure's whole reason. No test shows that a `from` node cannot sit on such a cycle: a `from` node at count 2 must reject `CountBroken`. The reversed-order test pins only the count-1 side. A hand-built certificate over a small world with a cycle that reaches `to`, or an engine run over one, would pin both sides. The soundness argument holds (SR-12201 Summary), so this is a coverage gap, not a defect. | qsl-replay/src/certificate/state_graph.rs:29-35,532-546; qsl-analyze/tests/it/state_graph_certificate.rs:381-460 |
| FND-002 | low | Several paths of `check_state_graph` have no test. (a) `BadState` (departure 4): an undefined or refused contract at a reached state. (b) The `Malformed` closure-domain case: a digest that is not `SimulationStateKeyV1`. (c) The `max_transitions` stop. (d) The reached-state `admit` stop under `max_states`: AC-10's test stops at the `closure.len()` pre-check. (e) Cancellation. FR-169's limit sentence covers (c) and (d). Case (a) is a behaviour the PR adds. | qsl-replay/src/certificate/state_graph.rs:121-126,200-202,401-411,448-456,303-312 |
| FND-003 | medium | Spec defect, routed by the lead and not blocking: FR-169-AC-10 and TC-644's expected results contradict FR-169's own check rules at three loci. (1) "rank of `(1, 0)` changed to 2 (`RankBroken` at `(1, 0)`)": `(1, 0)` keeps `(1, 1)` at rank 1 and passes, and the rule breaks at `(0, 0)`. (2) "`PredicateUndefined` at `(2, 0)`": the target is undefined at every `va = 2` state, and the first in certificate order is the least digest. (3) "`SuccessorMissing` at the state that reaches it": both `(1, 2)` and `(2, 1)` reach `(2, 2)`. Separately, TC-644 step 2 says to rank "every state with `va <= 1`", but the test also ranks `va = 2` at 2. This is harmless, because `PredicateUndefined` fires before ranks are read, but it should be aligned when the TC is fixed. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:219; spec/test-cases/TC-644-state-graph-certificates-that-do-not-hold-are-rejected.md |
| FND-004 | low | Declared remainder, recorded so QSL-538 is not closed on this PR. Still open: the settlement halves of FR-169-AC-9 and AC-10 (FR-169's map, V-1/V-9 `Certified`, V-6 `CertificateRejected`, V-7); AC-9's last sentence (`Proved{Reduced, Uncertified}`); the hyper product-closure certificate (FR-163, TC-645, TC-646, FR-282-AC-8); FR-282-AC-4; and the FR-282 `package_id` binding. Until the settlement lands, AC-9 and AC-10 are only partly met. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:218-219 |

## Verdict

**PASS for this PR's declared scope. Mergeable, with no major finding.** All seven tagged tests bind correctly. Each can fail on the behaviour it claims, and each draws its expected values from an independent BFS and the world's arithmetic, not from the engine. The three spec loci the coder flagged really contradict FR-169's rules, and the tests honestly assert what the rules give (FND-003, for the lead to route). The coverage gaps (FND-001, FND-002) are worth closing in this PR or a follow-up, but no defect stands behind them. At ticket scope, FR-169-AC-9 and AC-10 stay partly met until the settlement map lands (FND-004).
