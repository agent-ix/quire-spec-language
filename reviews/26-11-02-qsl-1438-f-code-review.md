---
id: SR-202611020539
title: "Code review — QSL-1438 F: derived partial applications, observation and frame caches, dev opt-level 3"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2089 head 959952ac6; Cargo.toml; qsl-eval/src/simulation/model/application.rs; qsl-eval/src/simulation/model/frames.rs; qsl-eval/src/simulation/model/frames/judge.rs; qsl-eval/src/simulation/protocol.rs; qsl-eval/src/simulation/protocol/steps.rs; qsl-analyze/src/model_check/refine.rs; qsl-analyze/src/zone_check/refine.rs; qsl-analyze/tests/it/refinement_certificate.rs; qsl-analyze/tests/it/refinement_protocol.rs"
review_set: subset
---

## Summary

Ticket: QSL-1438. PR: quire-spec-language#2089, branch `qsl-1438-F`, head 959952ac6, `git diff origin/main...HEAD` after fetching origin/main a9025eaac. Method: code-review with the Rust lane (rust-review and rust-style). Three commits:

1. The application engine derives a selected-transition application from the remembered full application. The memo holds `Arc`s. Synthesized observations are remembered by memo identity. `ObservedIdentities`/`observed_from` reuse identity encodings within one expansion. Frame decisions are cached per reference-valued arguments, and model footprints once per (attempt node, transition).
2. qsl-analyze is adapted to qsl-replay's boxed `StepVerdict` fields.
3. The dev profile builds qsl-eval and qsl-analyze at opt-level 3.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| apply_attempt derived branch | qsl-eval/src/simulation/model/application.rs:654-669 | examined |
| restricted | qsl-eval/src/simulation/model/application.rs:471-493 | examined |
| identical / Held / departure | qsl-eval/src/simulation/model/application.rs:340-418 | examined |
| ObservedIdentities / observed_from | qsl-eval/src/simulation/model/application.rs:420-610 | examined |
| remembered_observation / keep_observation / AppliedMemo.observations | qsl-eval/src/simulation/model/application.rs:500-590 | examined |
| frames::apply charged output | qsl-eval/src/simulation/model/frames.rs:186-300 | examined |
| ArgumentFrames / unchanged / writable / decide_touched | qsl-eval/src/simulation/model/frames.rs:500-1100 | examined |
| model_footprint cache | qsl-eval/src/simulation/protocol/steps.rs:640-676 | examined |
| attempt observation reordering | qsl-eval/src/simulation/protocol/steps.rs:1268-1350 | examined |
| StepVerdict unboxing | qsl-analyze/src/model_check/refine.rs:2182; qsl-analyze/src/zone_check/refine.rs:817 | examined |
| dev profile | Cargo.toml:199-217 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `AppliedMemo.observations` keeps every synthesized `Observation` for the whole run. Before this PR an observation died with its `Observed`. The PR measures time but not the extra retained memory. | qsl-eval/src/simulation/model/application.rs:507 |

## Verdict

Mergeable. One low finding. The test-adequacy gap on the derived charge is recorded in the gap-analysis artifact (SR-202611020540).

Exactness, checked by reading the code:

- **Derived charge.** In `frames::apply` the only budget charges are one per argument vector, which is charged before the `selected_transitions` filter, and the candidate charges of an enabled vector, which are recorded in `charged` from `before`. `conjunction`, the footprint and `walk_observed` charge nothing. A fresh application with `Some(selected)` therefore charges the full total minus the unselected vectors' candidate charges. That is exactly what `charged.checked_sub(unselected)` computes. The budget check is monotone, so the single `charge_many` fails exactly when an incremental fresh run would. A non-budget stop inside a selected vector would also have stopped the full application, which would then not have been memoized.
- **Restricted results.** Successors keep the full application's canonical order. Filtering the stable-sorted contract findings gives the same list as sorting the filtered findings. Both construction sites build `ContractUndetermined`, so the `BROKEN` arm is unreachable.
- **observed_from.** Identity bytes depend only on `state.state`, and the anchor is in the key. A field judged unchanged is equal to the base: Booleans, integers, strings and references are compared with `==`, and option, composite, union and collection values with `Arc::ptr_eq`. A composite value that differs from the base makes `Held::of` return `None`, which falls back to a fresh encoding. Equal departures therefore mean equal `ModelState`s.
- **Observation memo.** `walk_of` builds the observation from `state.state` alone, so it is a function of the identity.
- **Frame caches.** They are keyed per (row, application) call and per reference arguments. Only `deletes p` reads arguments, and neither an unchanged population nor a single-field probe deletes anything, so the key is conservative and correct. `build` changes only the deleted, created and dimension populations, which are the `touched` set.
- **Footprint cache.** `footprint_selecting` takes no state, so its result is a function of the node and the transition.

Concurrency: the `Mutex` guards drop before the insert (no re-entrance), poison is handled as in the existing code, and racing inserts store identical values. `RefCell`/`Rc` stay inside one expansion or one `frames::apply` call. The parallel explorer still compiles, so nothing non-`Sync` crosses threads.

Commit 2 is still needed. origin/main a9025eaac still matches `StepVerdict::Passes { post, taken } => Ok((post, taken))` against the `Box` fields that #2082 introduced, so qsl-analyze does not compile on main. The derefs are correct.

The opt-level change has real value: by the PR's own measurement LT-16 drops from 83 s to 39–42 s on a single core. The PR also measures opt-level 2 at the same instruction count (203.3 G against 202.9 G). Debug assertions and overflow checks stay on. It is not ceremony.

There are no `#[allow]`/`#[expect]`, no caps, no pins or digests, no compatibility layer, and no new `unwrap`/`expect`/`panic!`.

Evidence:

- Instrumented baseline (stderr print only): `cargo test -p qsl-analyze --test it -- rc11_oracle weak_memory release_acquire` passed 68 tests, LT-16 included. The derived branch ran 101,680 times, most with nonzero unselected charges.
- Mutation `identical` with integers always equal: 25 tests fail (rc11_oracle LT-12/13/16, release_acquire, weak_memory, race_freedom). The `observed_from` departure is guarded.
- Mutation keying both frame caches without the reference arguments: all 1,181 qsl-analyze `it` tests pass (the 6 failures are only `QSPEC_DIR` unset), and all of qsl-eval's lib, `it` and golden tests pass. This is an equivalent mutant, for the reason above.
