---
id: SR-1667001
title: "code-review and rust-review of PR #1667 (QSL-583e, FR-242 MightyL bounded copies)"
type: SpecReview
analysis: code-review
date: 2026-10-27
scope: "agent-ix/quire-spec-language; PR #1667 git diff origin/qsl-583d-mightyl...7196018e8: qsl-eval/src/automaton/{timed_claim.rs,error.rs}, qsl-analyze/src/zone_check/search.rs, qsl-analyze/src/model_check/check.rs, qsl-replay/src/certificate/zone_certificate.rs, qsl-eval/tests/it/timed_trace.rs, qsl-analyze/tests/it/zone_search.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-242
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: reviews
---
# SR-1667001: code-review and rust-review of PR #1667 (QSL-583e, FR-242 MightyL bounded copies)

## Summary

Ticket: QSL-583. Head reviewed: 7196018e8. The review is scoped to the diff against `origin/qsl-583d-mightyl` and was led by soundness. No soundness bug was found.

**Multiplicity is exact.** `Lowering::lower` passes `many` down as follows:
- an `until`'s left operand is many, and its right operand inherits the `until`'s own multiplicity;
- a `release`'s right operand is many, and its left operand inherits;
- `Not`, `And`, `Or` and `Implies` keep it;
- under `on each`, the body is the right operand of a root `true until`, so it is started once.

In each case where the flag is not set, a run starts the operand at most once. An `until` is discharged at one position, and a `release` escapes at one. So a once-started node is never restarted, and its single clock is exact. Temporal nodes are no longer shared between contexts (`add_temporal`), so a once-started and a many-started occurrence of one subformula cannot alias.

**Block bounds: I derived them independently, and they match.** Let `d = b - a`.
- **`release` and past.** A start joins the newest block exactly when the windows meet: `t - e <= d` if either end is closed, `< d` if both are open. So the union `[s+a, e+b]` is exact. A pending oldest block has `e1 >= now - b`. Every later block starts after `now - a`, more than `d` after the previous one, or at least `d` with both ends open. So at most `ceil(a/d)` later blocks are pending, `floor(a/d)+1` with both ends open, which gives `m + 1` in all.
- **`until` (greedy latest-discharge run).** Take two consecutive pending blocks `k` and `k+1`. Their discharges satisfy `p_{k+1} - s_k` above `b`, else `s_k` would have used `p_{k+1}`, since `left` holds across both. So `s_{k+2}` lies more than `d` after `s_k`, or at least `d` with both ends open. Every block after the oldest starts at or after `now - a`, using `p_1 >= now`, with equal stamps allowed. So each parity class holds at most `m` blocks, and the total is `2m + 1`.
- **The open/closed split in `spread` is required.** `floor+1` when both ends are open covers the closed `[now - a, now]` range that equal stamps allow.

**The bounds are tight, by mutation.** Each mutant drops capacity by one block. The evaluator is the oracle, on random quarter-ms lassos.

| Mutant | Example word (quarters) | PR tests |
| --- | --- | --- |
| `until` at `2m` | `eventually always[1,2] q` on `[0,2,2,4,4]`, loop to 3 after 3 | fail (forms test) |
| `release` at `m` | `eventually (p and eventually[1,2] q)` on `[0,3,6,7]`, levels `[0,0,1,2]`, loop to 1 after 8 | **pass** |
| past at `m` | `always (q implies once[1,2] p)` on `[0,2,3,7,8]`, loop to 2 after 7 | fail (three tests) |
| both-open as `floor.max(1)` | `eventually always(1,2) q` on `[0,4,4,6,8,8]`, loop to 2 after 6 | **pass** |

The two surviving mutants are a test gap. It is recorded in SR-1667002 FND-001.

**Probe: no mismatches at head.** 176 targeted forms were checked:
- shapes: `until`, `release`, `always`/`eventually`, `once`, `since` and `historically`, plus forced-start `p and eventually_I q` and `p and always_I q`;
- intervals: `[1,2]`, `(1,2)`, `[1,2)`, `(1,2]`, `[1,3)`, `(0,1)`, `(0,1]` and `[2,3]`;
- each under `on origin` and under `on each`;
- 200–300 lassos each (/6 when the form has more than 8 clocks), length up to 9.

**Refusals.**
- `RepeatedPunctualInterval` is raised for a punctual future operator that is many-started and for a punctual past operator read below a future operator (`deep`). A root-level past operator read once stays `Single`, which follows the coordinator's ruling. EN-6 maps the refusal to `UnsupportedForm`, and the checker maps it to `UnsupportedItem`.
- `ClockLimit` applies above 64 clocks.
- The TT-2 scope gap is already on STD-493 (comment 086c74fa).

**The checker rebuilds the same automaton.** FR-245 calls `position_automaton` on the request's clause. Every block clock's `constants()` entry is the window's upper end, and every guard compares with `lower`, `width` or `upper`, all at most `b`, so the LU bounds cover every guard.

**Gates.** All were run with locked-build at jobs 4, `--all-features`, and QSPEC_DIR at bd73a062.
- fmt check: exit 0.
- qsl-eval `timed_trace`: 19/19.
- qsl-analyze `zone_`: 89/89.
- qsl-replay lib `certificate`: 27/27.
- clippy on qsl-eval, qsl-analyze and qsl-replay with `--all-targets -D warnings`: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `ClockLimit` is handled two ways. EN-6 surfaces it as `ZoneCheckRefusal::Automaton(ClockLimit)`, but FR-245's checker folds it into `UnsupportedItem` with `RepeatedPunctualInterval`. The search.rs comment says it is "refused by name", which only EN-6 does. Pick one: either map both to a named refusal, or treat it as unsupported on both sides. | qsl-analyze/src/zone_check/search.rs:420-430; qsl-replay/src/certificate/zone_certificate.rs:719-724 |
| FND-002 | low | When `spread` refuses early (`floor > MAX_CLOCKS`, or a ratio that does not fit `u64`), it reports `ClockLimit { clocks: usize::MAX }`. The message then reads "needs 18446744073709551615 clocks", which is false. Report a saturated lower bound, such as `2 * floor`, or a separate field. | qsl-eval/src/automaton/timed_claim.rs:597-625 |
| FND-003 | low | A new block takes the lowest free slot (`free_slot`). States that differ only by a permutation of slots are therefore distinct `Config`s with renamed clocks, which inflates the automaton and zone graph without changing what they accept. A possible fix is to normalise slots to the creation order, with the oldest block in slot 0, shifting clocks when the oldest retires. This is performance only. | qsl-eval/src/automaton/timed_claim.rs:707-709,1460-1480 |

## Verdict

No soundness bug. The construction is exact, and its copy bounds are correct and tight. The code-review findings are all low.
