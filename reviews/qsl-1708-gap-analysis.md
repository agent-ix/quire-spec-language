---
id: SR-2347002
title: 'Gap analysis of PR #2347: FreeDelays witness production and canonical action bodies (QSL-1708), gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@8c7a64414; git diff origin/main...8c7a64414 (PR #2347): qsl-analyze/tests/it/free_delays.rs; qsl-replay/src/product_action.rs tests; against FR-196-AC-14, FR-202-AC-8, FR-204-AC-10, FR-204-AC-11, FR-204-AC-12; ticket QSL-1708'
review_set: subset
date: '2026-11-05'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-202
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: references
---

## Summary

Ticket: QSL-1708. Plan completion: not assessed. `quoin matrix --repo .` (quoin 0.29.1) did not finish within 600 s on this repository, so the AC-to-test check was done manually: each new test's `#[trace]` tags were read against every clause of the AC it claims. Three mutations were run at the PR head:

- Admitting a row naming B alone was killed by 2 tests.
- Skipping the merge in `action_steps` was killed by 1 test.
- Removing the pruning was "killed" only because the test binary was SIGKILLed at the 20 GiB memory cap. No assertion caught it.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-202-AC-8 | spec/functional/FR-202-replay-a-probabilistic-witness.md:197 | examined |
| FR-196-AC-14 | spec/functional/FR-196-build-the-probabilistic-product.md:842 | examined |
| FR-204-AC-10 | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:209 | examined |
| FR-204-AC-11 | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:210 | examined |
| FR-204-AC-12 | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:211 | examined |
| detour_maximum_witness_is_a_two_path_set_of_merged_steps | qsl-analyze/tests/it/free_delays.rs:846-924 | examined |
| detour_witness_body_admits_and_its_variants_refuse | qsl-analyze/tests/it/free_delays.rs:927-982 | examined |
| a_witness_row_naming_b_alone_refuses | qsl-analyze/tests/it/free_delays.rs:985-1026 | examined |
| loop_witness_reaches_b_through_the_exit | qsl-analyze/tests/it/free_delays.rs:1031-1083 | examined |
| free_delays_bodies_are_in_numeric_row_order_and_malformed_ones_refuse | qsl-analyze/tests/it/free_delays.rs:1090-1190 | examined |
| product_action unit tests (round trip, malformed) | qsl-replay/src/product_action.rs:658-747 | examined |
| ADR-028 WS-3 path-set evidence | spec/decisions/ADR-028-exact-probabilistic-engine.md:235 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two tests carry `#[trace("TC-637", "FR-202-AC-8")]`. AC-8's operative clause is that the Detour witness "replays `refuted`" with merged probabilities recomputed by replay, and that the variants refuse at replay. Replay is cut from this slice: the tests check production and stand-alone admission against a hand-stated race, not FR-202 replay. The matrix will count FR-202-AC-8 as covered while QSL-1708's replay is open. Fix: drop the FR-202-AC-8 tags (keep FR-204-AC-10 and FR-204-AC-11) until `replay_probabilistic_witness` lands. | qsl-analyze/tests/it/free_delays.rs:846; qsl-analyze/tests/it/free_delays.rs:927 |
| FND-002 | low | The pruning rule has no assertion that fails when it is removed; only the memory cap catches it. The Loop minimum case also pairs an E-target with a minimum policy and the 9/10 bound, which is not WS-3's shape: a `>= θ` refutation uses not-E paths summing above `1 − θ`. Fix: add a small test that runs with a low `max_transitions` and asserts that `Ok(None)` comes from the frontier emptying, for example by also asserting that it returns before a tiny limit would stop it. | qsl-analyze/tests/it/free_delays.rs:1031-1083 |

## Verdict

CONDITIONAL on FND-001, which is a tag change only. The other bindings are correct:

- FR-196-AC-14: the byte-exact bodies with "9" before "10" were checked, the AC's six refusal variants all refuse, and one merged edge has probability 1.
- FR-204-AC-10 and FR-204-AC-11: the entry rows, the path set [1/2, 1/4] and the refusal of a row naming B alone are checked.
- FR-204-AC-12: the maximum entry is B delay 2 and the path set sums to 1. "Replays refuted" is still uncovered, as the PR body states.

## Dispositions

- FND-001: fixed. `FR-202-AC-8` is dropped from both Detour test tags; `FR-204-AC-10/11` and `TC-637` stay.
- FND-002: deferred: a dedicated pruning assertion is not part of this round; the Loop minimum test covers pruning only through the memory cap.
