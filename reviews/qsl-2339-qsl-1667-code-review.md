---
id: SR-2339001
title: "Code review of PR #2339: FreeDelays action for mixed given/free races (QSL-1667), code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e2e8126e5; git diff origin/main...e2e8126e5 (PR #2339, branch peter/qsl-1667-freedelays-code): qsl-analyze/src/exact_probabilistic.rs, qsl-analyze/src/exact_probabilistic/{digital,finite_horizon,rows,unbounded}.rs, qsl-eval/src/simulation/model.rs, qsl-analyze/tests/it/{digital_clocks,free_delays,main}.rs; ticket QSL-1667"
review_set: subset
date: "2026-11-04"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: references
---

## Summary

Ticket: QSL-1667. PR: quire-spec-language#2339, head e2e8126e5. Method: code-review with the rust-review lane. The PR adds `ProductAction::FreeDelays` and `FreeDelayRow`, builds row choices at a mixed given/free workload race (`free_delays_moves`), and evaluates them through evaluator-internal row nodes (`rows.rs`) for bounded events, reachability/until/always (and thresholds) and expected reward. It also adds `free_delays_choice` and `ModelSystem::scheduled_identity_key`.

The design holds. The row nodes never reach the product, its keys or its transition count. `V(s) = Σ_g P(g)·V(n_g)` and `V(n_g) = opt_a Σ_o (P(g)·m_o / P(g))·V(o)` is exactly FR-204's `Σ_g P(g) opt_a Q(g, a)`. The 1/P(g) division is exact rational, and a non-positive or disagreeing row mass refuses (`given_mass`). An empty given vector is the one-row, P = 1 case. Prob0/Prob1/MEC on the evaluation see the parent's single action over all row nodes, which is the union of row supports. This matches FR-196/FR-204 text and does not meet ruling (a)'s reopen condition: every admitted objective is still linear in the successor distribution.

The defects are an evaluator entry point family the PR did not wire in (fair.rs), the zero-mass given identity case, and one unbounded pre-check allocation.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| Evaluation / rows_of / given_mass / free_delays_choice | qsl-analyze/src/exact_probabilistic/rows.rs | examined |
| free_delays_moves, Odometer, workload_moves routing | qsl-analyze/src/exact_probabilistic/digital.rs:544-580,1530-1768 | examined |
| backward_induction wrapper | qsl-analyze/src/exact_probabilistic/finite_horizon.rs:152-200 | examined |
| reachability / threshold / expected-reward wrappers | qsl-analyze/src/exact_probabilistic/unbounded.rs:225-600 | examined |
| scheduled_identity_key | qsl-eval/src/simulation/model.rs:975-992 | examined |
| decide_fair_reachability / decide_fair_expected_reward | qsl-analyze/src/exact_probabilistic/fair.rs:294-380,568-700 | examined |
| orderings / chain_edges | qsl-analyze/src/exact_probabilistic/digital.rs:1803-1850 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The fair entry points do not use the row-factored Evaluation. `decide_fair_reachability` and `decide_fair_expected_reward` build `Graph::new(product)` on the raw product and read the evaluation-extended policy against the raw choice groups. Reproduced with a scratch probe on AC-12's Loop under Max: at a single-row state the policy returns `ActionChoice(0)`, which is the row choice `B` delay 1 whose chain never reaches `b`, where the correct choice is delay 2. With A ~ {1 ms, 2 ms} (two rows), the same call refuses `Product(InvalidProbability)`. FR-204 requires the row-factored operator for every objective EN-5 decides, including the §13a elapsed supremum. Fix: run the fair paths on `Evaluation` (graph, MECs, labels extended, values restricted, policy mapped through `free_delays_choice`), or refuse a FreeDelays product with a typed error until that is wired, and add a test. | qsl-analyze/src/exact_probabilistic/fair.rs:303-372, fair.rs:578-600 |
| FND-002 | medium | A declared given identity whose conditioned support lies outside its window does not race (ADR-026 SD-2), but `workload_moves` sets `given` from declared windows. The state therefore goes to `free_delays_moves` with no racers and emits `FreeDelays` rows with `given: []`. FR-204 makes such a state all-free (`UnitDelay` plus one `WorkloadResolve{first, 0}`), and FR-202 treats a FreeDelays body at a non-mixed state as a disabled choice. With two or more free identities this also lets the scheduler pick the winner instead of the workload's weights. The updated `digital_clocks.rs` test asserts `row.given.is_empty()` and so locks the departure in. Fix: decide mixed versus all-free from `racers` after conditioning, route the no-racer case to `free_moves` and exclude non-racing given identities there, then update the test to expect WorkloadResolve. Alternatively, amend FR-204/FR-202 if the lead prefers the coder's reading. | qsl-analyze/src/exact_probabilistic/digital.rs:1537-1590, qsl-analyze/tests/it/digital_clocks.rs:651-664 |
| FND-003 | low | When several free identities tie at one assignment, `race_edges` calls `orderings`, which builds every k! ordering in memory before `free_delays_moves` checks `max_transitions`. The PR newly makes ties among k ≥ 2 free identities reachable. Fix: count or stream orderings against the remaining transition budget. | qsl-analyze/src/exact_probabilistic/digital.rs:1803-1850, digital.rs:1728-1747 |
| FND-004 | low | `Evaluation::of` clones the whole product, including keys, edges and labels, whenever any FreeDelays edge exists. That doubles peak memory over a product that `max_transitions` bounds only once. Fix: borrow the product and append node edges to a separate overlay. | qsl-analyze/src/exact_probabilistic/rows.rs:146-188 |

## Verdict

**CONDITIONAL.** Fix FND-001 (wire in or refuse) and FND-002 (or get a spec ruling on it) before merge. FND-003 and FND-004 are low.

Verified (all on e2e8126e5 unless noted):

- Focused tests: `--test it` free_delays/digital_clocks/exact_probabilistic/scheduler_fairness gave 71 passed. `--lib exact_probabilistic` gave 26 passed.
- Clippy `-D warnings` on qsl-analyze and qsl-eval `--all-targets`, default and `--all-features`, is clean.
- `cargo check --locked --workspace --all-targets` on a trial merge with origin/main 26bc56773 is clean. Cargo.lock is unchanged.
- Mutations, each caught and then reverted. Dropping the 1/P(g) scaling failed AC-8 and AC-10. Reading every row's policy from the first row's node (wrong row optimum) failed AC-8 and AC-10. Sorting the given support in descending order failed AC-8, AC-10 and FR-196-AC-14.
- Hand-checked values: AC-10 Detour post-draw max (1+1/2)/2 = 3/4 with witness [1]→B2, [3]→B1, and min (3/4+0)/2 = 3/8 with witness [1]→B1, [3]→B2. Pre-draw values are 5/8, 1/2 and 1/2. AC-6 gives 1/0 with the tie at 1/2 each. AC-7 gives max 1/2 and min 0. AC-8 gives 3/4 and 1/4 with the least attaining delay per row. AC-11 gives max (B1, C2) and min (B2, C1). All match the spec and the tests.
- Edge count: the check runs after each assignment against `self.edges.len() + planned.len()` with `saturating_add`, so it overshoots by at most one assignment's edges. Odometer enumeration is lazy. Indexing uses `get`, and the `planned[first..]` slices are in bounds by construction.
- Ordering: rows are in numeric lexicographic order (support sorted by delay, last index fastest), and assignments use ascending window delays. Identities sort by `scheduled_identity_key`, the same `frames::identity_bytes` that orders FR-187 actions.
- The mixed-state `WorkloadResolve` loop is deleted, and the all-free `free_moves` path is unchanged. The digital_clocks test keeps the values 0, 99/100 and 9999/10000, and it keeps the mass-1-per-choice assertion (see FND-002 for its new `given` assertion).
- Witness: `free_delays_choice` reads the node entries. Finite-horizon picks the least attaining choice (first strict improvement in edge order), and unbounded Max takes the MEC exit (AC-12 delay 2). Policy vectors gained trailing entries. The only consumers outside the evaluators are fair.rs (FND-001) and tests, and there are no external crates.

Spec findings for the lead:

- (a) FR-204-AC-12's tie narrative is wrong and the coder is right. Under ADR-026 SD-3, with `B` first, `B` resets `x` and keeps `Idle`, so `A` is still enabled at its turn and sets `b`. With `A` first, `A` sets `b`. Both orders reach `b`, so the row B delay 2 has value 1, not "B returns 1/2". The AC's conclusions still hold: max 1, witness delay 2 (the MEC exit), and the delay-1 chain has value 0. The AC should read "`B` delay 2 (tie: either order reaches `b`: `A` first sets `b`; `B` first resets `x` and `A`, still enabled, sets `b`)".
- (b) See FND-002.

## Dispositions (pass 1)

| FND | Outcome | Evidence |
| --- | --- | --- |
| FND-001 | fixed | `decide_fair_reachability` and `decide_fair_expected_reward` (fair.rs) now run on the row-factored `Evaluation`. Labels are extended, values are restricted, row nodes are dropped from components and their identities credited to the parent state, and the policy is read through `free_delays_choice`. `Evaluation::of` is idempotent on a product that already carries row nodes, so the evaluators these paths call inside do not expand twice. New tests in `free_delays.rs`: `fair_reachability_takes_the_row_factored_exit` (AC-12 Loop under Max: value 1, witness `B` delay 2) and `fair_decisions_read_two_rows_through_the_row_factored_operator` (A ~ {1 ms, 2 ms}: fair Max reach 1; fair expected `B` steps Min 1/4 with rows [1]→B2 and [2]→B2, Max 3/2). Both failed on e2e8126e5: the first gave a non-FreeDelays policy entry, the second `Product(InvalidProbability)`. |
| FND-002 | fixed | `workload_moves` now decides mixed versus all-free from the conditioned `racers`. A given identity with zero conditioned mass in its window does not race. With no racer the state goes to `free_moves`, which excludes given-delay identities, so it offers UnitDelay and one `WorkloadResolve{first, 0}`. The `digital_clocks.rs` test now expects `WorkloadResolve{send, 0}`, resolving to `send` only with mass 1, and no FreeDelays row with an empty given vector. Under `Now` (deadline 0) no resolution is reached. Values 0, 99/100 and 9999/10000 are unchanged. Mutation: dropping the given-identity exclusion fails the test. |
| FND-003 | fixed | `orderings` is replaced by `each_ordering`, which visits one tie ordering at a time and checks cancellation and the remaining `max_transitions` budget after each ordering's chain edges. No k! list is held in memory. |
| FND-004 | partly fixed | `Evaluation::of` no longer clones the product and then replaces its edge list. It builds the evaluation product field by field: keys and labels extended once, edges built once. A borrowed overlay would make every evaluator (Graph, action groups, fair collapse) generic over a product view. That is out of proportion for this low finding, so the keys are still copied. |
