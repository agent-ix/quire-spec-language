---
id: SR-16620
title: 'Code review of PR #1425: independent RC11 oracle and LT-12/LT-13 vectors, code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@60a259b33; git diff origin/main...60a259b33 (PR #1425, branch qsl1001-rc11-oracle): qsl-analyze/tests/support/rc11.rs, qsl-analyze/tests/it/rc11_oracle.rs, qsl-analyze/tests/it/litmus.rs, qsl-analyze/tests/it/weak_memory.rs, qsl-analyze/tests/it/main.rs; ticket QSL-1001'
review_set: subset
date: '2026-10-24'
---

## Summary

Ticket: QSL-1001. PR: quire-spec-language#1425, head 60a259b33fd3ea4f2fea2d6b76d55b648f73259b. Method: code-review with the rust-review lane. Tests and test support only; no production or spec file changed.

The PR adds an RC11 execution-graph checker (`tests/support/rc11.rs`) that enumerates `rf` and per-location `mo`, and keeps a candidate when `hb` is irreflexive, `hb ; eco?` is irreflexive, `sb ∪ rf` is acyclic and `psc` is acyclic. It adds `rc11_oracle.rs`, which runs ADR-025 §15's LT-12 and LT-13 under `sc`, `tso`, `ra` and the checker, plus a calibration test of the checker. The litmus fixture gains an opt-in third cell `z`.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| RC11 checker: events, rf/mo enumeration, `allowed` | qsl-analyze/tests/support/rc11.rs:1-448 | examined |
| `check`, `lt12`, `lt13`, the LT tests | qsl-analyze/tests/it/rc11_oracle.rs:51-125 | examined |
| `the_rc11_checker_matches_the_known_litmus_outcomes` | qsl-analyze/tests/it/rc11_oracle.rs:127-177 | examined |
| `Documents::with_cells`, `Documents::universes(&self)`, `IsZ`/`ReadsZ<n>` contracts | qsl-analyze/tests/it/litmus.rs:200-445 | examined |
| `Case::with_three_cells`, `trace_subject` universes | qsl-analyze/tests/it/weak_memory.rs:72-78,215 | examined |
| `Ra::sc_event` (PSC-3 pruning), mutated in a throwaway worktree only | qsl-eval/src/simulation/protocol/ra.rs:583-645 | context_only |
| ADR-025 §15 PSC-3, PSC-7, LT-12, LT-13 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:598-611 | context_only |
| FR-223-AC-5 | spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md:97 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The assertion `ra.difference(&rc11) == [weak]`, documented as "ra adds only the PSC-7 execution", cannot fail when the other assertions hold. RC11 allows every other register combination of both programs (7 of 8 for LT-12, 3 of 4 for LT-13), so `ra \ rc11` is at most the weak outcome. The LT tests therefore cannot detect `ra` admitting too much. Demonstrated: with `Ra::sc_event`'s cycle pruning disabled, both LT tests stay green, while `seq_cst_store_buffering_prunes_the_old_message_choice` and `seq_cst_iriw_is_forbidden_and_release_acquire_iriw_is_not` fail. The test does fail when `ra` prunes too much: an over-pruning mutation makes LT-12's weak outcome unreachable. Either reword the doc comment and the PR body, or add a program where RC11 forbids an outcome that `ra` must also forbid, such as LT-14 through the same oracle comparison. | qsl-analyze/tests/it/rc11_oracle.rs:51-78 |
| FND-002 | low | The calibration test covers SB (with and without a `seq_cst` fence), MP with release/acquire, and IRIW. It does not exercise release sequences, fence synchronisation (release and acquire fences in `sw`), no-thin-air, or the `psc_F` fence clause, although the module doc and the PR body claim them. Each of these mutations leaves all three tests green: `rs` reduced to the write itself, fence heads dropped from `sw`, fence tails dropped from `sw`, the `sb ∪ rf` acyclicity check removed, and `psc_F` removed. The code is correct today: a probe test (MP with release/acquire fences, a release sequence through a later relaxed store, CoRR, LB) passes on the unmutated checker and kills the first four mutants. Add those rows to the calibration test, so the support module stays a trustworthy oracle for later vectors. | qsl-analyze/tests/support/rc11.rs:313-357; qsl-analyze/tests/it/rc11_oracle.rs:127-177 |

## Verdict

Approve with two low findings. No change is needed before merge, and the findings can be fixed in this PR or deferred.

What is sound:

- **Independence.** `rc11.rs` imports only `std::collections::BTreeSet`. `rc11_oracle.rs` imports `Ra`, `Sc` and `Tso` only as the subjects under test. No code is shared with `qsl-eval`'s `ra.rs`.
- **Fidelity to RC11 (Lahav et al., PLDI 2017).** `rf` ranges over every same-location write, including the initial write. `mo` puts the initial write first, then takes every permutation of the thread writes per location. `rb = rf⁻¹ ; mo` and `eco = (rf ∪ mo ∪ rb)⁺`. `rs = [W] ; sb|loc? ; [W]`, and with no RMW the `(rf ; rmw)*` tail is empty. `sw = [E⊒rel] ; ([F] ; sb)? ; rs ; rf ; (sb ; [F])? ; [E⊒acq]`. `hb = (sb ∪ sw)⁺`, with the initial writes ordered before every thread event. Coherence is `hb ; eco?` irreflexive. Atomicity holds vacuously, because there is no RMW. No-thin-air is `sb ∪ rf` acyclic. `scb = sb ∪ sb|≠loc ; hb ; sb|≠loc ∪ hb|loc ∪ mo ∪ rb`, where fences carry no location, so `sb` to or from a fence counts as `sb|≠loc`. `psc_base = ([Esc] ∪ [Fsc] ; hb?) ; scb ; ([Esc] ∪ hb? ; [Fsc])` and `psc_F = [Fsc] ; (hb ∪ hb ; eco ; hb) ; [Fsc]`. Every one of these is RC11 as published.
- **psc acyclic as the SC condition.** This is RC11's own SC axiom, not a substitute for it. It is equivalent to the existence of a total order `S` on SC events that contains `psc`: an acyclic relation on a finite set has a linear extension, and a strict total order containing `psc` makes `psc` acyclic.
- **Vectors by hand.** LT-12's weak execution has the cycle `X:=1 →(sb|≠loc;hb;sb|≠loc) b:=Z →(rb) Z:=1 →(sb) c:=X →(rb) X:=1`. LT-13's has `a:=Y →(rb ; hb? to the fence) F →(hb? ; rb) X:=1 →(sb) a:=Y`. Removing `sb|≠loc;hb;sb|≠loc` fails LT-12, and removing the fence ends of `psc_base` fails LT-13, so the two PSC-7 kinds (a) and (b) are what the checker forbids.
- **Not a tautology.** Breaking the checker in the relations the vectors use fails a test: coherence, `sb|≠loc;hb;sb|≠loc`, the fence ends of `psc_base`, and the whole `psc` check. An over-pruning mutation of `ra.rs` fails LT-12. FND-001 records the direction the vectors cannot see.
- **Fixture safety.** `Documents::new()` is `with_cells(&["x", "y"])` and builds the same objects, so the default snapshot is unchanged. `universes` became `&self` and every caller was updated; the `por_world` and `flag_world` `universes` are their own methods. The `IsZ`/`ReadsZ<n>` contracts change the shared contract text, and the full `it` suite (727 tests) passes.

Gates at 60a259b33 (own target dir, one cargo job at a time):

- `cargo test -p qsl-analyze --test it`: 727 passed.
- `cargo clippy -p qsl-analyze --tests -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.
- `cargo deny` was not run (local cargo-deny 0.20.2 rejects `--config`, known environment noise).
- GitHub checks: `cla / cla-check` reports FAILURE and the merge state is BLOCKED. That check is not part of this review.

Mutation runs, in the throwaway worktree, with the switches set from the environment and the worktree restored afterwards:

| Mutant | Result |
| --- | --- |
| checker: coherence removed | calibration fails (MP release/acquire) |
| checker: `sb\|≠loc;hb;sb\|≠loc` removed | LT-12 fails |
| checker: fence ends of `psc_base` removed | LT-13 fails |
| checker: `psc` check removed | calibration, LT-12 and LT-13 fail |
| checker: release sequence, fence `sw` heads, fence `sw` tails, no-thin-air, `psc_F` | all green (FND-002); the probe kills all but `psc_F` |
| `ra.rs`: cycle pruning disabled | LT tests green (FND-001); `seq_cst` SB and IRIW tests fail |
| `ra.rs`: `F(ℓ)` summary or `owed` dropped | LT tests green |
| `ra.rs`: prune every choice with a later SC write | LT-12 fails |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit f53e17f66: `check` became `compare`, which asserts `ra \ rc11` equals the given PSC-7 extras. The new test `ra_reaches_exactly_the_rc11_outcomes_outside_psc7` runs SB `seq_cst`, SB `seq_cst` with an observer, and IRIW `seq_cst` with no extras. The doc comments and the PR body now say the LT tests alone cannot detect an `ra` that admits too much. Reviewer re-run at f53e17f66: with `Ra::sc_event`'s cycle pruning disabled, the new test fails and LT-12 and LT-13 stay green. After: `/// Programs outside PSC-7 on which ra reaches exactly RC11's outcomes, so an ra that allows an RC11-forbidden outcome (for instance by not pruning a cyclic SC choice) fails here.` |
| FND-002 | fixed | Fix commit f53e17f66: `the_rc11_checker_matches_known_litmus_verdicts` has 15 rows: SB relaxed, release/acquire, seq_cst and fenced; MP relaxed, release/acquire, release fence only, acquire fence only, and both fences; a release sequence, with a control where another thread's store breaks it; CoRR stale and fresh; LB; and a `psc_F` eco program. Reviewer re-run at f53e17f66: each mutant (release sequence, fence `sw` heads, fence `sw` tails, coherence, no-thin-air, `psc_F`) now fails this test. `sb\|≠loc;hb;sb\|≠loc` fails LT-12, the fence ends of `psc_base` fail LT-13, and removing `psc` fails all four tests. After: rows such as `("MP release sequence", ..., vec![2, 0, 3, 3], false)` and `("seq_cst fences ordered through a relaxed write and an acquire read", ..., vec![0, 1, 0, 3], false)`. |

Gates at f53e17f66 (own target dir): `cargo test -p qsl-analyze --test it` 728 passed; `cargo clippy -p qsl-analyze --tests -- -D warnings` clean; `cargo fmt --all -- --check` clean.
