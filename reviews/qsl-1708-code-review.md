---
id: SR-2347001
title: 'Code review of PR #2347: FreeDelays witness production and canonical action bodies (QSL-1708), code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@8c7a64414; git diff origin/main...8c7a64414 (PR #2347, branch peter/qsl-1708-freedelays-witness): qsl-replay/src/product_action.rs; qsl-replay/src/lib.rs; qsl-analyze/src/exact_probabilistic/witness.rs; qsl-analyze/src/exact_probabilistic.rs; qsl-analyze/src/exact_probabilistic/unbounded.rs; ticket QSL-1708'
review_set: subset
date: '2026-11-05'
---

## Summary

Ticket: QSL-1708. PR: quire-spec-language#2347, head 8c7a64414, merge base 28604a044. The trial merge with origin/main 823b9a9aa was clean and touches exactly the PR's 6 files. There is no Cargo.toml or Cargo.lock change. Method: code-review with the rust-review lane, one pass scoped to the PR diff. Spec: FR-196, FR-198, FR-202, FR-204 (Rulings, AC-6..12) and ADR-028 WS-1, WS-3 and TA-6 on main. QSpec FR-181's decimal-string rule was read by ID from a detached worktree at quire-specification origin/main e85daed5.

Checks run at the PR head:

- Focused tests: qsl-analyze `--test it` (free_delays, exact_probabilistic, digital_clocks) passed 68 of 68. qsl-analyze lib exact_probabilistic passed 26. qsl-replay lib product_action and probability_certificate passed 6.
- clippy `-D warnings` on qsl-replay and qsl-analyze `--all-targets` was clean with default features and with `--all-features`.

Checks run on the trial merge:

- `cargo check --locked --workspace --all-targets` was clean.
- The qsl-semantics scan tests (lib, `--features test-support`) passed 2 of 2.
- arch-lint `api-surface --qsl-only` and `qualified-core` passed.
- `xtask string-edge` FAILED with 4 findings, all in this PR's code (FND-001).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ActionBody closed set, to_bytes, parse | qsl-replay/src/product_action.rs:154-289 | examined |
| Schedule::of reshaping from transition identity bytes | qsl-replay/src/product_action.rs:76-116 | examined |
| Schedule::from_node, decimal, pairs, product_key | qsl-replay/src/product_action.rs:123-350 | examined |
| FreeDelaysRace::new / admit / given_row / free_row | qsl-replay/src/product_action.rs:545-656 | examined |
| FreeDelayRow move and re-export | qsl-replay/src/product_action.rs:33-44; qsl-analyze/src/exact_probabilistic.rs:48 | examined |
| policy_action, action_steps (merged steps) | qsl-analyze/src/exact_probabilistic/witness.rs:101-192 | examined |
| induce, reaching, produce_witness, entries, resolves | qsl-analyze/src/exact_probabilistic/witness.rs:244-415 | examined |
| same_action visibility | qsl-analyze/src/exact_probabilistic/unbounded.rs:1130 | examined |
| ModelTransition identity encoding (locked rev 30c24931) | quire-semantic-value src/model_transition.rs:153-283 | context_only |
| quire-canonical read (duplicate names, any depth) | quire-canonical src/read.rs (locked rev 35987f1) | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new main gate `xtask string-edge` fails on the trial merge with 4 unmarked branch-gating string comparisons, all in this PR: the member-name match in `Schedule::of` (lines 98-100) and `text == "0"` in `decimal` (line 314). The scanner did not flag the parser's tag match (`match (tag, members) { ("identity", ..) .. }`, lines 262-286) or the `Node::String(PRODUCT_KEY_TAG)` pattern (line 343), but both are the same wire-decode string edge. Fix: mark `Schedule::of`, `ActionBody::parse`, `decimal` and `product_key` with `#[ix_trace_rs::string_edge]`, or restructure them as typed decodes, so the scan reports 0 findings. | qsl-replay/src/product_action.rs:98-100; qsl-replay/src/product_action.rs:314; qsl-replay/src/product_action.rs:262-286; qsl-replay/src/product_action.rs:343 |
| FND-002 | medium | Witness-search memory is bounded only by `max_transitions` times out-degree, and each frontier push clones a full `ProducedStep` into its `Link`, including the whole `FreeDelays` action with every row's identities. Measured: with the pruning line disabled, the Loop minimum test was SIGKILLed at the 20 GiB lane cap before reaching the default `max_transitions`. A wide induced chain whose accepting mass is deep reaches the same growth with pruning on. Fix: store `(state, step index)` into `induced` in `Link` and materialize steps only for accepted paths. Count frontier pushes against the limit as well. | qsl-analyze/src/exact_probabilistic/witness.rs:194-206; qsl-analyze/src/exact_probabilistic/witness.rs:359-373 |
| FND-003 | medium | `entries` names only the states the emitted paths leave. ADR-028 WS-1, FR-202 Emission bullet 3 and FR-204 Evidence require entries for "every digital evidence/continuation state", including the checked admissibility continuation support. The `&[Option<usize>]` policy also cannot express the randomized TS-5 entries of FR-204-AC-2 and FR-202-AC-3. The PR's cut line does not list either gap. Fix: add both to the PR body and QSL-1708's remaining work, or compute entries over the continuation support. | qsl-analyze/src/exact_probabilistic/witness.rs:378-397; qsl-analyze/src/exact_probabilistic/witness.rs:309-317 |
| FND-004 | low | `produce_witness` ignores `limits.max_rational_bits`. Path probabilities and the running sum grow without a bit bound, unlike the finite-horizon solver, which returns `None` past the bound. Fix: check `rational_fits` on each pushed probability and on the sum, and return `Ok(None)` past it. | qsl-analyze/src/exact_probabilistic/witness.rs:339; qsl-analyze/src/exact_probabilistic/witness.rs:365 |
| FND-005 | low | `Schedule::from_node` accepts any type for the operation member. Probed: `["identity",[null,{"o":"1"},[]]]` and `["identity",["x",{"o":"1"},[]]]` parse Ok. FR-196 makes the operation FR-322's NodeId object, and the `parse` doc says a mistyped member refuses. Admission still refuses by byte compare, so no wrong acceptance results. Fix: require `Node::Object` for the operation and add the two probes as refusal cases. | qsl-replay/src/product_action.rs:125-141 |
| FND-006 | low | `product_key` checks only the version tag and the 6-member length. Probed: `["post-state",["quire.probability-product-key/v1",1,2,3,4,5]]` parses Ok. FR-196 requires "a parsed full canonical key". Fix: state in the doc that the check is syntactic pending the cut-line product-key encoder, or validate the member shapes. | qsl-replay/src/product_action.rs:338-350 |
| FND-007 | low | `policy_action` re-derives the solvers' policy index with a third grouping rule: it matches any earlier edge and counts zero-probability edges. `unbounded` groups contiguous positive edges, and `finite_horizon::action_groups` groups by find, including zero edges. They agree only under the builder invariant that each action's edges are contiguous and positive. Fix: share one grouping helper with both solvers. | qsl-analyze/src/exact_probabilistic/witness.rs:118-129 |
| FND-008 | low | `reaching` is a repeated full sweep to a fixpoint, which is O(states x chain length). Fix: one reverse BFS from the accepting states over the induced edges. | qsl-analyze/src/exact_probabilistic/witness.rs:277-293 |
| FND-009 | low | FR-196 names the qsl-replay form `ProductAction`, with `Identity(ScheduledIdentity)`, `WorkloadResolve{identity,..}` and `FreeDelays(Vec<FreeDelayRow>)`. The PR introduces `ActionBody` over `Schedule` bytes, with a second row type `BodyRow`. The follow-up slices will build `WitnessEntry` on this name. Fix: rename to FR-196's name, or amend FR-196's qsl-replay form in the same PR. | qsl-replay/src/product_action.rs:144-180 |

## Verdict

CONDITIONAL. FND-001 must be fixed in this PR because it turns main's string-edge gate red. FND-002 and FND-003 should be fixed, or recorded as named remaining work on QSL-1708. The lows are cheap to fix in the same round.

What is sound (verified):

- **ActionBody closed set.** It matches FR-196's nine bodies, with exact tags and member counts. `parse` reads through quire-canonical and compares `to_vec(root)` with the input octets, which rejects key order, whitespace and escapes. The reader rejects duplicate member names (probed: NotCanonical). Extra and missing members and unknown or case-variant tags refuse as Shape. Delays refuse leading zeros, an empty string, `1e2`, a sign and u64 overflow; u64::MAX is accepted. Pairs must be strictly ascending by full schedule bytes. The round trip `parse(to_bytes(x)) == x` holds for all nine bodies (unit test).
- **Schedule reshaping.** Reshaping from ModelTransition's `{arguments,operation,receiver,type}` is lossless: the operation's JSON string is re-read as its document, the receiver and arguments are carried as documents, and only the constant `type` is dropped. Equality and order use the complete JCS bytes.
- **FreeDelaysRace::admit.** It matches FR-196 and FR-202. Racers are sorted by schedule and zipped positionally after a length check, so a row must name exactly the given and the free identities; a subset, a superset, a wrong identity or an unsorted direct body all refuse. Given delays must be in support and free delays in window. Rows must be unique, their count must equal the product of the supports (checked_mul), and their order must be lexicographic numeric in `given` order. Every refusal maps to `invalid_runtime_input`/`invalid-value`.
- **Real mutations.** Relaxing `free_row`'s length check so that a row naming B alone admits was killed by 2 tests. Disabling the merge in `action_steps` was killed by the FR-196-AC-14 merged-step test.
- **produce_witness.** The best-first search on products of factors no greater than 1 emits completed paths in nonincreasing probability. It stops at the first `sum > past` (strict, as WS-3 and FR-202 require). Paths stop at the first accepting state, so they are prefix-free. Pruning drops only partials at states from which no accepting state is reachable in the induced chain, which are zero-contribution paths, so it is correct for minimum and maximum alike. The merged step sums row-choice edges by `(to, rewards)` and checks a total mass of 1. Detour was hand-verified: row [1] with B at 2 gives A winning at 1 ms with mass 1/2; row [3] with B at 1 gives B winning, 1/2, then G wins the 2 ms tie, 1/2, so 1/4; total 3/4 > 7/10. The Loop maximum entry is B delay 2 with mass 1.
- **Layering.** `FreeDelayRow` has one definition, in qsl-replay, re-exported from qsl-analyze. The qsl-analyze -> qsl-replay and qsl-replay -> qsl-eval edges already existed, and the manifests are unchanged.
- **Other.** There is no compat layer and no weakened test; the only test-doc edits remove "not asserted" notes for now-asserted behaviour. There is no panic path or unchecked indexing in the new code.
- **Slice coherence.** The public API (`ActionBody`, `FreeDelaysRace`, `produce_witness`) is test-only-consumed but complete for what it claims, and it is keyed honestly by `ProbStateId`. Nothing is half-wired in a way that could be misused into a false `refuted`.

## Dispositions

- FND-001: fixed. `Schedule::of`, `ActionBody::parse`, `decimal` and `product_key` carry `#[ix_trace_rs::string_edge]`; `cargo run -p xtask -- string-edge` reports 0.
- FND-002: fixed. `Link` stores `(from state, step index)` into `induced`; steps are cloned only for accepted paths; frontier pushes count against `max_transitions`. A test asserts a limit of 2 refuses (mutation: disabling the push check fails it).
- FND-003: deferred: recorded as remaining work in the PR body (entries over every evidence and continuation state; randomized TS-5 entries need a policy type beyond `&[Option<usize>]`).
- FND-004: fixed. Each pushed path probability is checked with `rational_fits` against `max_rational_bits`; a test with 1 bit refuses (mutation-checked).
- FND-005: fixed. `Schedule::from_node` requires an object operation; a refusal case was added.
- FND-006: deferred: the member shapes of a full product key are not fixed until the cut-line product-key encoder exists; the check stays syntactic.
- FND-007: deferred: shared grouping helper across the solvers is a refactor outside this slice.
- FND-008: fixed. `reaching` is one reverse breadth-first search.
- FND-009: deferred: the FR-196 name `ProductAction` over `ScheduledIdentity` versus this PR's `ActionBody`/`BodyRow` over `Schedule` bytes is recorded as remaining work in the PR body.
