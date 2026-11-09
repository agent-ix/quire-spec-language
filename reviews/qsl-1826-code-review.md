---
id: SR-20261109172912000018
title: code-review of typed probabilistic replay
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language; QSL-1826; qsl-analyze/src/analyze.rs, qsl-analyze/src/analyze/exact.rs,
  qsl-analyze/src/exact_probabilistic/unbounded.rs, qsl-analyze/tests/it/exact_probabilistic_item.rs,
  qsl-analyze/tests/it/main.rs, qsl-analyze/tests/it/probabilistic_replay.rs, qsl-eval/src/simulation/model.rs,
  qsl-eval/src/simulation/model/probabilistic.rs, qsl-replay/src/lib.rs, qsl-replay/src/probability_certificate.rs,
  qsl-replay/src/probability_certificate/witness.rs, qsl-replay/src/proof_result.rs
review_set: subset
---

## Summary

Ticket: QSL-1826. PR: quire-spec-language#2612. Frozen head: 10f32396946499dd47a65552d5395b95a404200a. Scoped review of all twelve source/test files against merged FR-202 checked-subject replay and ADR-029 boundaries. No applicable AssuranceProfile exists. No spec/plan diff, so spec-review methods do not apply.

## Verdict

**FAIL** — Not mergeable until the identity and resource findings are resolved in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Replay accepts an invented model/ordinal when both supplied identities agree: bind_claim compares only item.node(), so replacing witness.item with ObligationIdentity::new(the_same_node, Some(wrong_model), 123) still mints a token and the terminal constructor accepts that invented identity. Validate the complete canonical obligation identity before acceptance; comparing two caller-owned copies is insufficient. | qsl-replay/src/probability_certificate/witness.rs:127-135 |
| FND-002 | high | Producer resource exhaustion is reported as unsupported: produce_witness returns Ok(None) at max_witness_paths, max_transitions expansions/frontier pushes, or max_rational_bits, and replay_refutation maps all None results to not_yet(). For the existing TooMuch fixture with max_witness_paths=0, this bypasses the incomplete LimitReached settlement. Preserve the exhausted cause and settle incomplete without evidence for this supported checked-subject slice. | qsl-analyze/src/analyze/exact.rs:498-507 |
| FND-003 | medium | The replay graph state storage limit is checked only when dequeuing: with max_states=1, the first expansion can insert arbitrarily many distinct successor keys/states into seen and pending (up to the much larger edge limit) before returning LimitReached on the next dequeue. Enforce max_states before admitting each new state, counting queued plus processed states, so a small state budget actually bounds materialized replay storage. | qsl-replay/src/probability_certificate.rs:409-412,483-484 |

## Coverage

Full computed matrix unavailable: the earlier whole-repository computation consumed about seven minutes without output; it was not repeated. Manual scoped tag/assertion/source mapping only. All eleven new tests carry FR-202 AC-9/10/11 tags and real assertions through ModelSystem; no stubs or producer-supplied mass establish acceptance. AC-9 has explicit 2/3 controls and retained evidence; workload regression independently asserts 391/400 and path masses 17/20 and 3/200, preserving MDP grouping. AC-10 tests mutations, duplicates, first-hit extension, live nonreach prefix and one threshold equality; model/ordinal mutation is absent. AC-11 covers pre/final cancellation, states/edges/rational/path limits and cycles including target continuations; no complete independent acceptance claim is made for fairness, timed, multiple initial/binding and randomized/subsystem exclusions. Full AC-11 remains unproven. No optional gap-analysis semantic campaign was invoked; the direct source/test oracle review was explicitly requested by the dispatch. Plan completion: not assessed

## Examined scope

```yaml
scope:
- id: FR-202-AC-9
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: A checked-subject acyclic reachability fixture replays a target-first-hit
    path set of mass `2/3` against `AtMost 1/2` and a terminal-nonreach path set of
    mass `2/3` against `AtLeast 1/2`; each produces a token permitting only its matching
    EN-5 refuted record, with the accepted evidence retained. Repeating replay gives
    equal semantic outcomes. Neither replay needs an emitted package or analyzer graph
    types.
- id: FR-202-AC-10
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: In AC-9, altering the claim occurrence, package or binding refuses; altering
    an action, draw, post-state or full key cannot produce a token. Duplicate/prefix-overlapping
    paths, inconsistent scheduler choices, an earlier target on a first-hit path,
    and a live non-target leaf cannot reproduce a refutation. Mass equal to the relevant
    threshold yields replay parity.
- id: FR-202-AC-11
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: A pre-cancelled call, cancellation at the final acceptance poll, or exhaustion
    of each configured expansion/work/storage limit returns incomplete without a token
    or a refuted record. Cyclic, subsystem, randomized-policy, fairness, multiple-initial/instance
    and timed inputs report unsupported without a token. A caller cannot construct
    a reproduced token or settle EN-5 refuted from a bare engine result.
- id: ADR-029
  path: spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md
  role: context_only
  excerpt: '**CB-3 What "separable" means.** Separability is a property of crates
    and their dependency closure:'
```

## Validation

Clippy independently passed: cargo clippy --locked -p qsl-replay -p qsl-eval --all-targets --all-features -- -D warnings (exit 0, 33.13s after lock). Mandatory flock/nice/ionice/toolchain/jobs/threads/debug/incremental/wrapper/target settings were used. Coder evidence /tmp/qsl1826-check-summary.md records passing fmt, qsl-analyze Clippy, 11 new replay integration tests, 10 existing EN-5 tests, 18 solver tests, 7 checker tests and one token compile-fail doctest. No broad rerun, make ci, Kani, metadata child tests, or gate changes. Findings are established by direct control/data-flow inspection, not runtime reproduction; a scratch rustc probe could not link a consistent cached feature set and supplied no behavioral evidence.

Wire/recompile replay, codecs, broader forms and remaining malformed-certificate scope remain open. The unrelated merged FR-201/282 objective profile does not change this typed path review. The reviewer changed no source and did not push or merge.
