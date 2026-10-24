---
id: SR-12213
title: "Code review (Rust lane) of QSL-920 part 1 FR-185 S3 for random, reward and workload declarations (PR #1067)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f60a3bdb3; qsl-semantics/src/check/{probabilistic.rs,probabilistic/exact.rs,probabilistic/model.rs,mod.rs,check.rs,check/typing.rs,lowering.rs,state_clause.rs,assemble.rs}, qsl-package/src/emit.rs, qsl-package/src/emit/tests/probabilistic.rs, qsl-replay/src/spine.rs, qsl-semantics/tests/it/probabilistic_declarations.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-620
    type: reviews
---
# SR-12213: Code review (Rust lane) of QSL-920 part 1 FR-185 S3 for random, reward and workload declarations (PR #1067)

## Summary

Ticket: QSL-920. PR quire-spec-language#1067, branch qsl-920-impl, head f60a3bdb3. The review covers `git diff origin/main...HEAD` only: 14 files, 3 commits, one of them a merge of main. The rust-review lane is folded into this file.

No gates were run, per the brief. One narrow run was made: `cargo test -p qsl-semantics --test it probabilistic_declarations`, with a temporary probe test appended. All 16 PR tests pass. The probe was then reverted and `target/` deleted. Probe results:

- A workload whose first weight for `Server::request` is `0` and second is `2` refuses only `invalid-weight`. No `duplicate-weight` is raised.
- `reward duration on Server::attempt = self.attempts;` followed by the valid `reward duration on Server::attempt = d;` refuses only `state-read-in-reward`. The second, valid declaration of the same name is not refused `duplicate-reward`.
- A refused distribution of `d` (weight 0) plus a `pre` that reads `d` refuses only `invalid-weight`. The precondition read is not reported.
- Support-size timing (debug build). `random Server::attempt(d)` was given n support values, each weighted `rational(1, 2^65534 + 2k + 1)`, so every weight is within the 65,536-bit ceiling. Wall time for the whole check: n=4 took 48 s, n=8 took 409 s, n=12 took 1,247 s.

## Verdict

**FAIL (one major).** FND-001 blocks: a within-limit, 12-value distribution stalls S3 for about 20 minutes on a debug build. Time grows roughly with the cube of the support size, and the work is not charged to any budget and does not poll cancellation. Everything else is medium or low.

What is sound:
- **Operation resolution.** It works by alias, artifact id and the effective view (FR-104). Duplicate distributions and rewards are keyed on the resolved operation's `DeclarationKey`, not on spelling, and f60a3bdb3 makes a refused first declaration still count.
- **Support values.** They are typed with `check_as` against the parameter type and compared as reduced `CheckedLiteral` values, so `Ok` and `Service::Outcome::Ok` collide.
- **Exact-value measure.** `power_of_ten_bits` computes `⌊s·log₂10⌋+1` in u128 integer arithmetic. The product stays under 2^127 for `s < 2^32`. From 2^32 up, `3s` is a valid lower bound past the ceiling, and a scale beyond u64 saturates. A unit test checks the result against the expanded power for s up to 20,000, plus 65,536 and 100,000.
- **Ceiling order.** The bound is applied before `Rational::new` or `divided_by_power_of_ten`, so `rational(n, n)` with a 70,000-bit `n` and `decimal(1, 4294967295)` are refused without expanding anything.
- **S4.** Emission refuses a non-empty `CheckedProbabilisticModel` before reading candidates. It does not drop the declarations silently, and every public emit path goes through `emit_package_inner`.
- **Panic surface.** No `unwrap`, `expect` or indexing panic was added in production code, and no `unsafe`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Unbounded exact arithmetic at S3. `random` folds every support weight into `total` with `Rational::add`, then divides each weight by it. Nothing limits this, charges `nodes` or the work budget, or polls `cancel`. With weights `rational(1, 2^65534+2k+1)` (each within the ceiling), the total's denominator grows to about n·65,536 bits. A debug-build probe took 48 s for n=4, 409 s for n=8 and 1,247 s for n=12, roughly cubic. A hostile or careless 50-value support therefore never finishes, and S3 cannot be cancelled. Fix: measure the sum's bits incrementally against the rational-bits ceiling (or `CheckingLimits` work) and refuse with the limit refusal, and poll `cancel` per pair. Add a test with many coprime large denominators. | qsl-semantics/src/check/probabilistic/model.rs:662-677 |
| FND-002 | medium | A value past the rational-bits ceiling refuses `unsupported_construct`/`not-yet-implemented` (`ExactValueAboveCeiling`). FR-185 "Exact values" and FR-185-AC-2 require a limit refusal (`LimitExceeded`), with only its setting name and limit kind pending QSL-883. The value is still refused, so no wrong acceptance results, but the code and the cause differ from the AC. Separately, Linear shows QSL-883 as Done, yet no `rational_bits` or `RationalBits` exists in this tree or the locked quire-semantic-value. Either emit `StageLimitExceeded` with a provisional limit kind, or record this deviation on QSL-883 and reopen it. | qsl-semantics/src/check/probabilistic.rs:286-296, 400-405 |
| FND-003 | medium | `duplicate-weight` is missed when the operation's earlier weight is invalid. The duplicate check scans `weights`, which holds only admitted weights. So `weight Server::request = 0; … weight Server::request = 2;` refuses only `invalid-weight` (probe). FR-185 says a workload that gives one operation two weights is refused `duplicate-weight`. This is the same class f60a3bdb3 fixed for random and reward. Track seen operation keys separately from admitted weights. | qsl-semantics/src/check/probabilistic/model.rs:797-811 |
| FND-004 | medium | Workload names have no identity check. `workload` is never `declare`d in assembly and `Pass::workload` does not compare names, so two `workload Steady` declarations both check. `CheckedProbabilisticModel::workload("Steady")` then silently returns the first. FR-185 makes a workload "a named, checked node with an identity, so a claim can name it", and FR-186 will bind it by name. Refuse a second workload of one name, at least on one model (the cause is a spec gap: SR-12214 FND-005). | qsl-semantics/src/check/probabilistic/model.rs:763-838, 227-231; qsl-semantics/src/check/assemble.rs:817-819 |
| FND-005 | low | `duplicate-reward` is missed when the first reward of the name reads state. `reward` returns on `state-read-in-reward` before resolving the operation, so the name is never pushed to `self.rewards`. A later valid reward of the same name is then admitted and recorded (probe). The package is still refused, so the only loss is the missing cause. Resolve and record the name before the state-read return. | qsl-semantics/src/check/probabilistic/model.rs:694-712 |
| FND-006 | low | `random-read-in-precondition` is checked only against admitted random parameters (`self.model.random_parameters`). A precondition that reads a parameter whose distribution was refused (for example for `invalid-weight`) is not reported (probe). Match against `self.distributions`, which f60a3bdb3 already records, so every cause shows in one run. | qsl-semantics/src/check/probabilistic/model.rs:854-874 |
| FND-007 | low | A support value is silently dropped if `weight.div(&total)` returns `None`. `filter_map` discards it, and the comment says it cannot happen. If it ever did, a recorded distribution would have fewer values than declared and probabilities that do not sum to 1. Return an internal fault (or refuse) instead of filtering. | qsl-semantics/src/check/probabilistic/model.rs:666-677 |
| FND-008 | low | Packages with random parameters, rewards and workloads now pass S3. `qsl-eval`'s simulator takes a `CheckedGraph` directly and does not read `probabilistic_model()`, so it enumerates a random parameter over its whole type domain, not its support, until FR-187 lands. No CLI path reaches it today, because S4 refuses first. Before FR-187, have `StateModel` admission refuse a graph with a non-empty probabilistic model, or document that the support is ignored. | qsl-eval/src/simulation/model.rs:597-610; qsl-semantics/src/check/mod.rs:3426-3430 |

## New findings (disposition pass 1)

Disposition pass 1 at head 1df0c5feb (fix commit 1df0c5feb over 92fdee243 and a merge of main). No gates were run. The claim that no rational-bits setting exists was checked: there is no `rational_bits` or `RationalBits` in QSL's Rust sources or in the locked quire-semantic-value. The only `max_rational_bits` is FR-203's exact-engine budget, which is spec-only.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | The interim limit refusal reports `CheckingLimitKind::WorkBudget`, which maps to `s3.work_units`, with `limit: 65_536`. The setting's real default is 16,777,216, and raising it does not change the fixed rational-bits ceiling. A user who reads the refusal and raises `s3.work_units` sees no effect. The PR body discloses this (spec gap 2). Add a dedicated limit kind with the rational-bits setting once FR-255 has one. Until then, keep the refusal text from naming a setting that does not govern it. | qsl-semantics/src/check/probabilistic.rs:163-174 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 1df0c5feb: `probabilities` checks `total.max_part_bits()` against the 65,536-bit ceiling after every addition and refuses with the limit refusal. It polls `cancel` before each addition and each division. Each addition therefore works on operands of at most 65,536 bits, so the cost is linear in the support size instead of cubic. My 12-value coprime-denominator probe is now the test `a_weight_sum_past_the_ceiling_refuses_promptly` (one limit refusal, under 60 s); the sum passes the ceiling at the second addition. Re-run at 1df0c5feb, debug build: all 23 `probabilistic_declarations` tests, this one included, finish in 0.83 s, where the round-0 probe took 1,247 s for its 12-value case alone. |
| FND-002 | fixed | 1df0c5feb: `ExactValueAboveCeiling` (`not-yet-implemented`) is gone. An over-ceiling value or sum refuses `stage_limit_exceeded` (`ResourceExhausted`, limit 65,536, actual = the measure), and `exact_values_are_bounded_as_written` asserts it for all four over-ceiling values. Verified that no rational-bits setting exists anywhere to use instead. The setting label is FND-009. |
| FND-003 | fixed | 1df0c5feb: `weighted` records every resolved operation key whatever its weight. Test `a_second_weight_after_an_invalid_one_is_still_a_duplicate` expects both `invalid-weight` and `duplicate-weight`. |
| FND-004 | fixed | 1df0c5feb: `Pass::workloads` records each name, and a second workload of a name refuses `conflicting_declaration`/`duplicate-declaration` at that name (test `a_second_workload_of_one_name_refuses`). The cause is QSL-chosen; the spec gap is SR-12214 FND-005. |
| FND-005 | fixed | 1df0c5feb: `reward` resolves the operation and records the name before returning on a state read. Test `a_reward_after_one_that_reads_state_is_still_a_duplicate` expects both causes. |
| FND-006 | fixed | 1df0c5feb: `preconditions` matches against `self.distributions`, which records every resolved distribution. Test `a_precondition_read_of_a_refused_distribution_still_refuses` expects both causes. |
| FND-007 | fixed | 1df0c5feb: `weight.div(&total)` failing is now an `InternalFault` (`probabilistic-weight-sum-is-nonzero`) through `ok_or_else`, not a `filter_map` drop. |
| FND-008 | deferred | Left by the coder and recorded in the PR body (item 7). No CLI path reaches the simulator, because S4 refuses first. FR-187, QSL-920 part 2, is where the simulator starts reading the supports. |
