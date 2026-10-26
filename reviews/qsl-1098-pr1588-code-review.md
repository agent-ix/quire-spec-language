---
id: SR-60023
title: "Code, gap and spec review of QSL PR #1588: TP-4 state-key bridge and closure replay (QSL-1098)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1c77ea062638668b9371d53197e872ee8da8436c (qsl-eval/src/automaton, qsl-replay/src/certificate/closure.rs, qsl-replay/src/model_check/item.rs, tests; FR-338, FR-339, TC-525, TC-526)"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-525
    type: reviews
---
## Summary

Ticket: QSL-1098 (slice B of QSL-893; slice C is QSL-1099). Verdict: FAIL.
One blocker (the aggregate gate is red on clippy) and three medium findings.

Gate: `make ci` fails in `ci-default-features` at `cargo clippy -D warnings`
(clippy::type_complexity, qsl-eval/src/automaton/formula.rs:723). Not
inherited: the code is new in this PR. `cargo fmt --check` and `xtask
string-edge` are clean. `cargo test -p qsl-eval -p qsl-replay` passes (742 and
130 tests plus replay). The remaining lanes were not reachable because the
lint stops the graph.

Probes (temporary tests, reverted): 11 TP-4 bodies x 3 activations (origin,
each, each-when) x ordinary/protocol subject, exploring every reachable
StateId under all letters. Every reached state resolves and the key
re-admits; key sets are identical across two builds explored in opposite
letter order; no two StateIds with equal key had different successor-key sets.
The bridge is a many-to-one projection, not a bijection: 9 StateIds map to
4 keys (ordinary) and 20 to 8 (protocol) for `always eventually (p and once[1,1] q)`.
Waiting and active phases get distinct keys. Bodies with finite interval
nesting, `once[0,3]` and `since[2,*]` are refused `NotTranslated` by the key
context (pre-existing scope).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | `make ci` is red: clippy::type_complexity on the return type of `occurrence_nodes`. Fix: name the pair as a struct or type alias. Rerun the aggregate gate; other clippy findings may be hidden behind this one. | qsl-eval/src/automaton/formula.rs:723 | implementation-bug-despite-evidence |
| FND-002 | medium | The TP-4 replay test cannot detect a key tamper. Its fixture (`eventually holds(v.l = 0)`, all-false letters) reaches 4 product states that all carry the identical automaton key. Probed mutations (replace one state's key with another's, swap two states' keys, duplicate, reverse) all remain Accepted, so the claimed key mutations are vacuous; only drop-a-state and foreign-context are exercised. The certificate is also built by iterating the checker's own SuccessorMissing rejections, so acceptance is circular and never compares a key to an independent expectation. Fix: add a fixture with at least two distinct automaton keys (an `on each ... when` clause with a true activation, plus a past clause) and assert the exact expected key octets, then a swapped-key mutation that must reject. | qsl-replay/tests/it/vault_world.rs (tp4_closure_replay_uses_context_bound_state_keys) | correct-requirement-no-evidence |
| FND-003 | medium | `build_instances` swallows a TP-4 key-context refusal: `BuchiStates::clause(...).ok()` turns any refusal into `None`, and the clause then falls to `SafetyTranslation::reachable_invariant`. A liveness clause the key algebra cannot encode is silently routed to the safety path instead of refusing with the key refusal. Fix: propagate the refusal as an ItemRefusal. Also `include_tp4: bool` plus the parallel `Instance`/`ClosureInstance` and `Translation`/`ClosureTranslation` types, with the unreachable `tp4-translation-not-produced` fault arm in `instances()`, is a compatibility layer; fold into one type or make the producer path total. | qsl-replay/src/model_check/item.rs | implementation-bug-despite-evidence |
| FND-004 | medium | AC tags claim more than each test asserts. `FR-338-AC-23` has four clauses (StateId resolved through the issuing automaton; equal octets across discovery orders with differing StateIds; accepts the complete set and rejects a missing successor; foreign-context ShapeMismatch). The eval test asserts only the second (and its `different_ids_for_same_key` is an `|=` over a loop with no per-body assertion); the replay test asserts the third and fourth. Each carries the full tag. Also AC-23's evidence column says plain `Test` while TC-525 step 12 is written as the procedure. Fix: split the tests per clause or tag only what is asserted, and give AC-23 a TC reference. | qsl-eval/tests/it/automaton_key.rs; qsl-replay/tests/it/vault_world.rs; spec/functional/FR-338-check-an-en-1-closure-certificate.md | correct-requirement-no-evidence |
| FND-005 | low | `buchi_state` returns a `BuchiState` whose obligation and acceptance vectors are in engine order, so it is not equal to the value `admit` decodes from its own key (observed for `eventually (always p or always p)`). Key octets are canonical, so closure is unaffected; sort and dedup in the bridge or document that only the key is canonical. | qsl-eval/src/automaton/translate.rs | implementation-bug-despite-evidence |
| FND-006 | low | `property_automata.rs` loosens `state_count() < 16` to `<= 17` to fit history retained for keys, so engine state growth is accepted by editing the existing bound. Record the cost (ordinary 9 StateIds for 4 keys) in the ticket or bound the growth. Stale comment: "`activation_guarded` is set by translate_formula below" in translate.rs. Direct `buckets[index]` indexing and `count as usize` in past.rs should use checked access. | qsl-eval/tests/it/property_automata.rs; qsl-eval/src/automaton/past.rs | implementation-bug-despite-evidence |

## Verdict

FR-339 and TC-526 are untouched and no FR-339 AC is claimed (nothing here belongs to slice C). The FR-338 key algebra is preserved: the PR adds only the TP-4 identity paragraph and AC-23, and the reader now accepts current Boolean obligations for BuchiStates, consistent with FR-338 `[o,s]` for an unbounded/current subformula.

FAIL until FND-001 is fixed (gate red); FND-002 to FND-004 should be fixed in the same PR.
