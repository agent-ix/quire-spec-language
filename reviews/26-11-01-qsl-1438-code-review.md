---
id: SR-202610310159
title: "Code review — QSL-1438 per-state reuse and parallel LT-16 explorer"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-eval/src/simulation/model.rs; qsl-eval/src/simulation/model/application.rs; qsl-eval/src/simulation/model/encode.rs; qsl-eval/src/simulation/protocol.rs; qsl-eval/src/simulation/protocol/memory.rs; qsl-eval/src/simulation/protocol/ra.rs; qsl-eval/src/simulation/protocol/steps.rs; qsl-analyze/tests/it/fill.rs"
review_set: subset
---

## Summary

Ticket: QSL-1438. Code review with the Rust lane (rust-review and rust-style) of PR #2061, `git diff origin/main...HEAD`, four commits. The PR encodes each model observation's memo identity and FR-120 observation once, caches the map-key names of model state keys (per run) and of RA component keys (per encoding), splits the application memo by observed state, and expands each breadth-first level of the test explorer in parallel.

## Verdict

**PASS with low findings.** No correctness defect was found. Each exactness claim was traced by hand:

- **Model state keys are byte-identical.** `KeyNames` caches `reference_enc(object_reference(..))` and the population name. Both depend only on the run's `StateModel`, and every `typing()` caller passes `self.model`. `sorted_map_enc` uses the same stable sort as `map_enc`, and the bytes come from the same `to_vec(.., Limits::new(u64::MAX)).unwrap_or_default()`.
- **RA keys are byte-identical.** `Names` is local to one component's `enc()`, and `carried_key` gets a fresh one. `self.location_enc(l)` is `location_enc(l, shared.get(l).cell)`, which is what `view_enc` used before. The hand-built edge bytes `[` a `,` b `]` match the `quire-canonical` writer at rev 35987f1: `begin_array` writes `[`, `before_value` writes `,` between elements, and there is no depth limit. TC-674 pins the edge order.
- **Memo behaviour and budget charging are unchanged.** The nested `identity -> at` map is equivalent to the flat 5-tuple key. A hit still charges the `charged` value recorded on the miss. A state without a key is still not remembered. A failed `observation_of` is not cached.
- **The `steps.rs` hoisting is pure.** `observe` and `reads` do not depend on the attempt node, and `readable` keeps the `BTreeMap` order of the load candidates.
- **The parallel BFS gives the same result as the FIFO search.** Expansions are collected by frontier position and admitted in frontier order, so indices and steps equal those of the sequential FIFO search.
- **Locking is sound.** No lock is nested, and no lock is held across another lock. Poison recovery matches the existing `into_inner` convention. Two threads racing on the same miss insert identical values.
- **No ceremony.** There are no `#[allow]`, no caps and no pins.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Each `reachable` call spawns `available_parallelism()` workers inside the test harness's own thread pool (cores x test threads), and holds a whole level's successor states in memory at once | qsl-analyze/tests/it/fill.rs:609-651 |
| FND-002 | low | `Sync` supertrait and `Send + Sync` on `Component` were added to the public `MemoryModel` trait only to serve a test helper; the bound belongs at the use site | qsl-eval/src/simulation/protocol/memory.rs:247-249 |
| FND-003 | low | `KeyNames::reference` allocates an owned `(DeclarationKey, String)` lookup key on every call, hits included, and serializes all workers on one Mutex held across `object_reference` | qsl-eval/src/simulation/model/encode.rs:408-424 |
| FND-004 | low | `remembered` clones `AppliedKind` by a hand-written match; derive `Clone` on `AppliedKind` instead | qsl-eval/src/simulation/model/application.rs:473-486 |
| FND-005 | low | The `reachable` doc comment has an unwrapped line of about 140 characters | qsl-analyze/tests/it/fill.rs:656 |
| FND-006 | low | `self.potential.cell(location)` is looked up twice per location; the second `let ... else { continue }` cannot fail after the `readable` filter | qsl-eval/src/simulation/protocol/steps.rs:470-480 |

## Review detail

FND-001: `expand_level` sizes its pool from `available_parallelism()` on each call. The `it` suite runs many `reachable` tests at once under libtest's default thread count, so the number of runnable threads approaches cores squared. Each level also keeps every successor `ProtocolState` before admission, where the old code kept one state's successors. This is not wrong behaviour, but the host has stalled before under oversubscription.

FND-002: `ProtocolSystem<M>: Sync` is needed only by `fill.rs`. Writing `M: MemoryModel + Sync, M::Component: Send + Sync` on `expand_level` and `reachable` would keep the public trait unchanged.

FND-003: Use a nested `BTreeMap<DeclarationKey, BTreeMap<String, KeyName>>` and look up by `&str`, so a hit does not allocate. Under the parallel explorer every object of every state key takes this one lock.

## Coverage

- Code review and Rust review (rust-style baseline) ran together in this artifact.
- Independently run: `cargo fmt --check -p qsl-eval -p qsl-analyze` (clean). `cargo clippy -p qsl-eval -p qsl-analyze --all-targets -- -D warnings` on toolchain 1.98 fails on `clippy::type_complexity` at qsl-eval/src/simulation/protocol/steps.rs:126 (`successors_and_terminal`) and on six lints in qsl-replay. That code is not in this diff and is identical on origin/main, so it is pre-existing. With that lint passed on the command line only, qsl-eval had no other warnings.
- Not run, per the brief: the full gates. The orchestrator reports LT-16 passing 5 times, and qsl-analyze `it` showing 27 failures that also fail on main.
- Existing tests that back exactness: TC-668 (FR-223-AC-8) asserts LT-16's 7,607 states and 19,574 edges. TC-674 (FR-229-AC-1) pins the SC-graph JSON, including the edge order. TC-471 (FR-120-AC-4) asserts the state-key envelope.

## Findings data

```yaml
clean: false
findings: [FND-001, FND-002, FND-003, FND-004, FND-005, FND-006]
```
