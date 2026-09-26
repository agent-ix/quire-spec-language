---
id: SR-672
title: "QSL-272 code and Rust review of the FR-101 finite simulation implementation"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@05db9adf6e2c557f30c2911ab286d02d946bdcb2; qsl-eval/Cargo.toml; Cargo.lock; qsl-eval/src/simulation/mod.rs; qsl-eval/src/simulation/explore.rs; qsl-eval/src/simulation/frontier.rs; qsl-eval/src/simulation/key.rs; qsl-eval/src/simulation/not_simulated.rs; qsl-eval/src/simulation/order.rs; qsl-eval/src/simulation/sample.rs; qsl-eval/src/simulation/trace.rs; qsl-eval/tests/it/finite_simulation.rs; tests/it/family_outcome_layering.rs; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md; spec/test-cases/TC-390-family-outcome-and-refusal-layering.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: reviews
---

## Summary

Ticket: QSL-272 (PR agent-ix/quire-spec-language#467, head 05db9adf, gate
commit f2c0d37e). `/code-review` with the `/rust-review` lane over
`git diff origin/main...HEAD` (merge base 9f4853d2). QSpec reference:
agent-ix/quire-specification origin/main 0d53cf2 (FR-181,
`proposals/quire-v1/definitions/simulation-sampler.md` revision `1-draft.1`).

What holds:

- The sampler matches `simulation-sampler.md` exactly. The preimage is
  `{"draw","seed","step","trace"}`, each a decimal string, encoded through
  `quire-canonical`. `U256::from_be_bytes` puts the most significant byte in
  limb 0, and the derived `Ord` on `[u64; 4]` compares from limb 0, so the
  number is read big-endian. The threshold is `floor(MAX / n) * n`. When `n`
  divides `2^256` exactly, the threshold would be `2^256`, and the code
  accepts every draw (`MAX % n == n - 1`). The index is `v mod n`. `n = 1`
  returns 0 without a digest but still advances `step`. `n = 0` never reaches
  the sampler. `draw` restarts at 0 at every step. `divmod_u64` and
  `mul_u64` fit in `u128` as their comments say.
- Independent recompute: a hand-written JCS in Python with `hashlib`, sharing
  no code with the PR. It reproduces every literal vector (table below).
- JCS for state keys, transition identities and preimages runs only through
  `quire-canonical` (ADR-013 §2). No `serde_json` or `sha2` is added to
  shipped code; both stay dev-only (ADR-011 X-8).
- Coalescing compares full `StateKey` bytes (`HashSet<StateKey>`), never
  digests. Signed zeros and NaN payloads stay distinct because the key
  carries the bits hex.
- Traces, frontiers and every `ReplayError` digest field are `DigestRecord`s
  under `DigestDomain::SimulationStateKeyV1`. Replay recomputes each
  candidate state's digest.
- Layering: `[dependencies]` is exactly the X-8 set. `qsl-replay` does not
  appear. `qsl-cst` appears only as a dev-dependency, which was already on
  main and which X-8 allows.
- `explore`, `sample` and `Sampler` are `pub(crate)`. `explore_request`,
  `sample_request` and `replay` are the only public entry points.
- Coder deviation 1 (the markdown-only commit 05db9adf after the gate run):
  accepted. This review re-ran `make ci` at 05db9adf.
- Coder deviation 3 (`GeneratorMismatch` is checked before requires-bound):
  conformant. FR-101 says "`sample_request` first compares the supplied
  `DefinitionRef`". Both checks come before any `TransitionSystem` call.
  `EmptyInitial` needs `initial()` and comes last.

Independent recompute (scratchpad `rv467/recompute.py`):

| Vector | Spec / test literal | Recomputed |
| --- | --- | --- |
| step-0 preimage | `{"draw":"0","seed":"424242","step":"0","trace":"0"}` | same bytes |
| step-0 digest | cb7d4b3b…6682622 | cb7d4b3b8b8491d8310ccc1e07c3ee236d3fe86cf713d1851533a85ef6682622 |
| seed 424242, trace 0, n=5 | 0,0,4,4,4 | 0,0,4,4,4 |
| seed 424242, trace 1, n=5 | 1,4,3,4,0 | 1,4,3,4,0 |
| seed 424242, trace 0, n=3 | 2,1,0,2,1 | 2,1,0,2,1 |
| mixed n=1,1,5, step 2 | 4 (0 if the counter skips) | 4 (step-0 draw gives 0) |
| float64 +0 | 943ae638…a1cc95e6 | 943ae638f84583f2a35a7a92f1eac7f58c380c045298892fb1412756a1cc95e6 |
| float64 -0 | 92a3e955…d54e4b01 | 92a3e9557f9aaf672a61aaab71eb2bfad13a0d88662953998de4057ed54e4b01 |
| NaN 7ff8000000000000 | a3d5ecff…96f4bdec | a3d5ecff68c7cfb60a687aa72b743a04e1dc8513e348b9c3f64393dd96f4bdec |
| NaN 7ff8000000000001 | 62c344cc…4356a955 | 62c344cca9a4942b80644ca8527bc7ccced905f01bf67262a9bd5e824356a955 |

Every draw in the vectors is accepted on its first digest, so none of them
exercises rejection.

## Verdict

**CHANGES REQUESTED.** The sampler, canonical order, state key and digest
plumbing are correct, and every vector recomputes. But `Outcome::Cancelled`
has no `cause` field (FND-001). FR-101-AC-6 and ADR-014 TR-7 require one, and
the FR-101 Status paragraph this PR adds says it is implemented. Two
retagged tests also lost the property they exist to discriminate
(FND-003, FND-004).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `Outcome::Cancelled` is `{stats, frontier}` and has no `cause` field. FR-101 Behavior ("`Cancelled` carries `cause: CatalogCode` … always `CatalogCode::new(\"cancelled\", \"caller-cancelled\")`"), FR-101-AC-6 and ADR-014 TR-7 (`Cancelled{stats, frontier, cause}`) all require it. No `caller-cancelled` code appears anywhere in `qsl-eval`. The FR-101 Status text this PR adds says "`Outcome::Cancelled` carries `cause: CatalogCode::new(\"cancelled\", \"caller-cancelled\")`", which is false. Failure: a consumer that maps an incomplete outcome to its O-16 catalog code cannot tell caller cancellation from a limit stop without matching on the variant, and the spec claims a conformance the code does not have. Fix: add `cause: CatalogCode` to `Cancelled`, set it at explore.rs:173, and assert it in `cancellation_stops_the_run_and_returns_the_frontier`. | qsl-eval/src/simulation/explore.rs:90-96,172-181; qsl-eval/tests/it/finite_simulation.rs:611-647; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:192-195,223,292-293 |
| FND-002 | medium | `state_key`, `canonical_bytes` and `plain_digest` panic when `quire-canonical` refuses to encode a value. The value is `TransitionSystem::Key` or `TransitionId`, and `TransitionSystem` is a public trait any downstream crate implements. The code comment treats the value as engine-built, citing `sha256_and_len`, but that precedent encodes preimages its own module builds. Here the implementer builds the value. `quire-canonical` refuses an integer that is not an exact double (`Error::InexactInteger`), a non-finite float, a non-string map key and nesting deeper than `MAX_DEPTH`. Failure: an implementer whose key holds a `u64` counter above 2^53 aborts the process from `explore_request`, `sample_request` or `replay` instead of getting an error. Fix: return a typed refusal (for example a `NotSimulated`/`Outcome`-level encoding error or a `ReplayError` variant), or narrow `Key` to a type the engine controls. | qsl-eval/src/simulation/key.rs:74-106; qsl-eval/src/simulation/explore.rs:24-46 |
| FND-003 | medium | The retag of `sample_then_replay_round_trips_with_duplicate_transition_ids` dropped the assertion that made it discriminate. On main it pinned that the draw took the second `recv` edge (`assert_eq!(sampled.steps[0].key, key(2))`). The new test samples at seed 0. Recomputed, seed 0 with n=2 selects index 0, which is `"1"`, the first-listed edge. The test also asserts no key. A `replay` that takes the first successor with a matching transition identity therefore passes, so FR-101-AC-5's "replay takes the one whose digest equals the recorded digest" goes untested. FR-101's disposition table says a retagged test "keeps its assertion". Fix: use seed 1 (recomputed: index 1, `"2"`) and assert `sampled.steps[0].key == key("2")`. | qsl-eval/tests/it/finite_simulation.rs:1332-1346 |
| FND-004 | medium | TC-453 step 1's fixture does not tell the canonical rule from sorting by post-state key. `z` goes to `"9"` and `a` goes to `"3"`. `key("3") < key("9")` bytewise (recomputed), so an engine that sorts successors by post-state key gives the same frontier `[3, 9]`. TC-453 requires "`a`'s post-state key is greater than `z`'s", which is SR-641 FND-002's fix. Steps 2 and 3 do discriminate. Fix: swap the targets (`z` to `"3"`, `a` to `"9"`) and expect `[key("9"), key("3")]`. | qsl-eval/tests/it/finite_simulation.rs:245-273 |
| FND-005 | low | The sampler vectors do not pin the byte order or the limb order of `v`. 256 ≡ 1 (mod 3) and (mod 5), so `v mod n` for n ∈ {3, 5} is the same for any permutation of the digest bytes. Recomputed: a little-endian reading gives exactly the same five-way and three-way vectors. Rejection is also never exercised. The implementation is correct by inspection, but nothing stops a regression. Fix: add an in-crate `U256` test at a discriminating `n`, for example seed 424242, trace 0, n=2 gives BE `0,0,0,0,1` and LE `1,1,1,1,1`, and n=7 gives BE `0,2,6,2,4` and LE `3,5,0,6,2`. Add a `divmod_u64`/`mul_u64` test on a synthetic `v` at or above the threshold. | qsl-eval/src/simulation/sample.rs:35-82,127-167 |
| FND-006 | low | `state_key` encodes each key twice: `quire_canonical::to_vec`, then `quire_canonical::sha256` re-serializes it. `ordered_successors` calls it for every successor of every expanded state, and replay calls it again. Fix: hash the bytes already produced (`ByteDigest::of(&bytes)`, as `qsl_semantics::model::key::sha256_and_len` does). | qsl-eval/src/simulation/key.rs:80-89 |
| FND-007 | low | `unreachable!` in shipped code guards `successors.into_iter().nth(index)`. The invariant holds (`next_index` returns `< n`), but a panic-free form is simple: `swap_remove(index)` on the `Vec`, or return `Option` from the draw. | qsl-eval/src/simulation/sample.rs:201-206 |
| FND-008 | low | `ReplayError::KeyMismatch.actual` is the digest of the first matching successor in the order the `TransitionSystem` lists them. The engine ignores that order everywhere else, so the same mismatch can report different `actual` digests for two listings of one system. Fix: iterate `ordered_successors` in replay, or document that `actual` is the canonical first match. | qsl-eval/src/simulation/trace.rs:121-136 |

## Rust review

- Panic surface: FND-002 and FND-007. The `const LIMITS` `panic!` runs at
  compile time and is fine. Test code is excluded.
- Integer casts: `n as u64`, `initial.len() as u64` and
  `(trace_index % m) as usize` are widening or bounded by construction, as
  the comments say. The `u128`-to-`u64` casts in `divmod_u64`/`mul_u64` are
  exact truncations of in-range values.
- `draw: u64` cannot overflow in practice (it would take 2^64 rejections).
  `step` uses `wrapping_add`.
- `#[allow(clippy::too_many_arguments, reason = …)]` on `sample_request`
  matches the signature FR-101 pins.
- `#[cfg(test)] digests_computed` is test-only instrumentation and does not
  change release behaviour.
- Error types derive `thiserror::Error`. `NotSimulated` does not implement
  `CatalogCoded`, which the gap analysis (SR-673) covers.

## Dispositions

Disposition pass at 54732520 (rebased; fixes in e8c5f6c7 and 54732520),
checked against the code and re-run with `rv467/recompute.py`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e8c5f6c7: `Outcome::Cancelled` gains `cause: CatalogCode` and is set to `CANCELLED_CAUSE` = `cancelled`/`caller-cancelled` (explore.rs). Both cancellation tests assert it literally. |
| FND-002 | fixed | e8c5f6c7: `state_key` and `canonical_bytes` return `Result<_, EncodingRefusal>`, surfaced as `NotSimulated::KeyEncoding` and `ReplayError::KeyEncoding`. FR-101-AC-11 and TC-453 step 9 were added, with a test using the key `2^53 + 1`. `plain_digest` still panics, but it encodes only the engine-built `DrawPreimage`, which is acceptable. |
| FND-003 | fixed | e8c5f6c7: the test now uses seed 1 and asserts `steps[0].key == key("2")`. Recomputed: seed 1, trace 0, step 0, n=2 selects index 1. The replay has to skip the first-listed `"1"`. |
| FND-004 | fixed | e8c5f6c7: the fixture is now `z` → `"3"` and `a` → `"9"`, and the test expects `[key("9"), key("3")]`. Sorting by post-state key would give `[3, 9]`, so the test now discriminates. |
| FND-005 | fixed | e8c5f6c7: byte order is now pinned by an n=2 vector `0,0,0,0,1` and an n=7 vector `0,2,6,2,4`, which match my recompute. The little-endian reading differs. The rejection half is not fixed: see R1-FND-001. |
| FND-006 | fixed | e8c5f6c7: `state_key` encodes once and hashes the bytes with `ByteDigest::of`. |
| FND-007 | fixed | e8c5f6c7: `unreachable!` is replaced by `swap_remove(index)`, which cannot panic because `index < n`. The initial-state pick uses the same pattern. |
| FND-008 | fixed | e8c5f6c7: replay walks `sorted_initial` and `ordered_successors`, so `KeyMismatch.actual` is the first match in canonical order. |

New findings, round 1:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| R1-FND-001 | low | `u256_divmod_and_mul_on_a_synthetic_value` never exercises rejection. It checks `100 / 7`, `14 * 7` and `98 % 7` on small values. The acceptance threshold at n=7 is `floor(2^256/7)*7`, not 98, and `next_index`'s `v < quotient.mul_u64(n)` branch is never reached with a rejected `v`. The test comment calls 98 "the least value the sampler would reject at n = 7", which is wrong. Fix: split out `accepts(v, n)` and test it at `v = floor(2^256/7)*7` (rejects) and one less (accepts). Not blocking: the rejection logic is correct by inspection. | qsl-eval/src/simulation/sample.rs:340-365 |
