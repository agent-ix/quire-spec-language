---
id: SR-12241
title: "Code review (Rust lane) of QSL-559 E15 PR 3a ProtocolSystem<Sc>: compensation templates (PR #1129)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8a7cdfbff3ffcf0b4f04e17b68ab5532c4119364; qsl-eval/src/simulation/protocol{.rs,/compensation.rs,/footprint.rs,/identity.rs,/program.rs,/steps.rs,/channel.rs}, qsl-semantics/src/check/{ir.rs,protocol_clause/types.rs}, qsl-replay/src/model_check/counterexample.rs, qsl-analyze/tests/it/{protocol_compensation.rs,protocol_footprints.rs,protocol_system.rs,protocol_constructs.rs,fill.rs,main.rs,weak_memory.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-207
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: reviews
---
# SR-12241: Code review (Rust lane) of QSL-559 E15 PR 3a (PR #1129)

## Summary

Ticket: QSL-559. PR quire-spec-language#1129, branch qsl-559d-impl, head 8a7cdfbff. The review covers `git diff origin/main...HEAD` only: 17 files, +2327/-100. The rust-review lane is folded into this file. QSpec is cited by id only.

No gates were run. Two narrow probes ran as temporary tests appended to `qsl-analyze/tests/it/protocol_compensation.rs` through `locked-build.sh`. Both were reverted, and `target/` was deleted.

1. **Key canonicity (FND-001).** The probe explored a protocol breadth-first and checked that every state reached under an already-known key has the same successor set (transition identities and successor keys) as the stored state. It also explored exactly, deduplicating on (key, successor set), and counted distinct keys. The fixture was §7.1's `Pay` with the template on `touch`, `as (fwd: Boolean)`, registration capture `b = fwd`, trigger guard `{ f = b }` and recovery `{ b }`. Its `run` was `C; repeat max 2 { effect Paid (pd: Boolean); event Fail (failed: Boolean) }; event Fail2 (g: Boolean)`. The result: 12 equal-key states with different successors, and 173 distinct reachable keys against the 171 that key-merged exploration finds. The PR's own fixtures (§7.1, `twice`, `settled("")`, `settled(touch)`) show 0 mismatches.
2. **Commutation with compensation interplay.** The probe ran the `every_independent_pair_of_steps_commutes` check over four fixtures and recorded the kinds of independent pairs:
   - `settled(touch)` and a retrying variant (`attempts 2`, retry `true`): independent `(Attempt, CAttempt)`, `(Attempt, CEnd)` and `(Join, CEnd)` pairs all commute.
   - A sibling-trigger variant, where `settle` takes `event Fail2 (g: Boolean)` before `Committed`, while a registration is live and the sibling commits: no failures.
   - The FND-001 fixture: independent `(CAttempt, CAttempt)` and `(CEnd, CEnd)` pairs commute.

   There were 0 failures in all four. Each commit-versus-`cattempt` pair is dependent through `reg(c, ·)`.

## Verdict

**Not mergeable: one major (FND-001).** Medium and low findings are recorded below.

What is sound:

- **Footprints over-approximate every new step kind (FT-1, FT-3).**
  - A forward effect, a commit step and every trigger-typed binding read and write `reg(c, ·)`.
  - A trigger reads the `ctl` of candidate compensation threads up to the last ordinal its waiting registrations can take, and writes each new `ctl`.
  - `cattempt` puts `reg(c, ·)` in its reads and enabling set (`Builder::enabling` inserts into both), together with its own `ctl` and the full-clause model footprint.
  - `cend` reads and writes `reg(c, r)`, which meets `reg(c, ·)`.
  - `finish` writes `reg(c, ·)` of every template, and still reads the `ctl` of every thread, compensation threads included.
  - Every state change of §7.1 and of `twice` lies inside the write footprint, and the commutation probe found no independent pair that fails to commute.
- **Smallest free ordinal.** Compensation threads are removed at `cend`, so "not in `threads`" is "not held by a live sibling" (PS-2). Several activations in one step take successive free ordinals. FR-205-AC-5's two orders are tested.
- **Finish waits for compensation threads.** Compensation threads are `Running` until `cend` removes them. ST-9's existing "every other thread has left `running`" check therefore covers them, and `FinishWaiting` names them.
- **Fold order.** Closure runs before activation, and activation before registration (CP-1, CP-2, CP-4). It is called at every binding step: event, attempt, send including lost, and receive. The `await` match goes through those same paths. An undecided guard or capture drops the step and keeps its finding.
- **Replay** through `cattempt`/`cend` is tested end to end (`a_prefix_through_a_compensation_replays`).
- **Panic surface.** The production code adds no `unwrap`, `expect`, `panic!`, indexing or `as` casts. Counts use `saturating_add`. The ordinal searches are `(0u64..).find` over finite maps.
- **arch-lint FR-100-AC-8.** `qsl-replay` only adds a `WaitCause` arm and calls `PathStep::text_len`. It adds no `pub use` of a `qsl_eval` item.
- **Compatibility.** No compatibility layer was added. `Unsupported::Compensation` was deleted, not kept.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The state key does not record which registration a compensation thread compensates. `Compensating.registration` is state that decides future behaviour, and it is in no key member. The thread's `thread-state` holds only rest and disposition, the `attempt-count` holds only the count, and `registration-state` holds only status and captures. Two registrations activated in opposite orders give threads `(0, Refund, 0)`/`(0, Refund, 1)` mapped crosswise, with byte-identical keys. Their `cattempt`/`cend` identities differ, and so do the registration whose status each `cend` sets and the captures the retry relation reads. Probe 1 measured 12 equal-key states with different successor sets, and 2 reachable keys (173 against 171) that key-merged exploration never visits. This breaks PS-8 ("two states with equal keys are one state ... a function of what the protocol can still do") and QSpec FR-425-AC-1. ADR-027 PS-8 and QSpec FR-425 have the same gap. Fix in this PR: put the registration key in the key, for example a `registration` member on the compensation thread's `thread-state`, or the thread path on `registration-state`. Record it with the commit-occurrence entry as a spec-gap reading (FND-002). | qsl-eval/src/simulation/protocol.rs:385-390, qsl-eval/src/simulation/protocol.rs:1026-1075, qsl-eval/src/simulation/protocol/compensation.rs:272-290 |
| FND-002 | medium | The commit-occurred `control` entry `{"type":"commit","instance","node"}` → `{"type":"commit-state","occurred":true}` is necessary. Without it, a committed instance with no live registration keys equal to an uncommitted one, although a later effect registers in only one of them. The entry is not in ADR-027 PS-8 or QSpec FR-425's member table, and the key is a cross-repo byte contract. The reading is acceptable for this PR. It needs a spec amendment (a QSpec FR-425 row and a PS-8 sentence), filed with FND-001's member. The `instance` field duplicates information that every other control entry carries through its thread path. | qsl-eval/src/simulation/protocol.rs:1054-1066 |
| FND-003 | medium | `Unsupported::DetachedEffect` misreads ST-2. "for an `effect`, its attempt's binder is bound" is an enabling condition that the step can check at run time against the thread's environment. It is not a structural rule. The static `precedes` walk descends only `sequence` children and stops at a `branch`, so it refuses valid placements: an attempt in a `parallel` branch joined `all` with the effect after the join (the binder escapes through `Out`, PS-5), or the attempt in every case of a `choice`. ADR-027 RU-1 says no construct settles `unsupported` for want of a design. No test exercises the refusal, and the PR body does not list it as a remainder. Fix: enable `event(n)` for an effect when the attempt's binder is in the thread's environment, and put that binder in the enabling footprint. | qsl-eval/src/simulation/protocol/program.rs:705-713, qsl-eval/src/simulation/protocol/program.rs:733-771 |
| FND-004 | medium | `Unsupported::UnboundAttemptRecord` is only partly a spec gap. ST-1 binds the record (`applied.result`, as `attempt` already does), and `cattempt` has that application in hand. The real gap is that PS-8 and QSpec FR-425 have no key entry for the previous attempt record, which FT-1 calls "the previous attempt's binder". The refusal is acceptable until that member exists. It is untested, and the PR body's remainders do not list it. | qsl-eval/src/simulation/protocol/program.rs:344-348, qsl-eval/src/simulation/protocol/compensation.rs:369-386 |
| FND-005 | low | `cattempt` writes through `Access::local()`, and its memory footprint is `AccessClass::Local`, under every memory model. A template operation on a shared object under `tso` is then treated as a local write. Today this is moot under `Sc`. The spec does not give a template an access class, so this needs a reading. | qsl-eval/src/simulation/protocol/compensation.rs:404-417, qsl-eval/src/simulation/protocol/footprint.rs:770-779 |
| FND-006 | low | A compensation thread's rest is `{"kind":"node"|"retry-point","node":"<protocol>::<template>"}`. QSpec FR-425's `<rest>` `node` is a node's static declaration path, and a template is not one. It is a reasonable reading (it names the template, as PD-4 does), but it is not recorded in the spec. | qsl-eval/src/simulation/protocol.rs:901-908 |

## Readings judged

- **Recovery view (`Unsupported::RecoveryView`).** This is a genuine spec gap, and the refusal is right. ST-12 and QSpec FR-427 evaluate the predicate over "the recovery view the model state gives". `M::RecoveryView` appears only in §7.1 and QSpec TC-380, and no rule maps a model state to that binder's value. FR-207-AC-2's `bal = 0` half is correctly left untagged.
- **Live registration = `registered` or `activated`.** This is consistent with CP-1 and FR-425's ordinal rule. An ended or closed registration stays in the key with its status, which FR-427's status list requires, until its ordinal is reused.
- **Qualified template name `<protocol>::<template>`.** It matches PS-7's channel naming. It shares PR 2's FND-007 caveat (no package qualifier).
- **`cattempt` selects every clause.** A template has no `contracts` list, so this is the only reading ST-11's "ST-1's conditions ... for an application of the template's operation" admits.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The root can now wait at an `effect` whose attempt's binder is unbound (on the `no` case of a `choice`, as `an_effect_waits_for_its_attempts_binder` shows). Its wait cause falls through to `ControlStep::Event`, so it reports `Constraint`, although no constraint was evaluated. ADR-027 PD-4 and QSpec FR-429 have no cause for this wait. It belongs with STD-233 item 3 (ST-2). | qsl-eval/src/simulation/protocol/steps.rs:1159, qsl-eval/src/simulation/protocol/steps.rs:349-355 |

## Dispositions

Round 1, head 0d4ac0c4b.

The focused tests ran through `locked-build.sh` and passed: the `protocol_` and `weak_memory` filters, 77 tests.

FND-001 was checked by reverting its fix locally, which made `a_compensation_thread_keys_its_registration` fail. The revert was then undone, and `target/` deleted.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 0d4ac0c4b |
| FND-002 | accepted-no-change | The entry is necessary and kept. It is recorded in the doc comment and in the PR body as a spec-gap reading that needs an ADR-027 PS-8 sentence and a QSpec FR-425 row. STD-233 item 1(a) owns that amendment. |
| FND-003 | fixed | 0d4ac0c4b |
| FND-004 | fixed | 0d4ac0c4b |
| FND-005 | fixed | 0d4ac0c4b |
| FND-006 | fixed | 0d4ac0c4b |

Notes:

- **FND-001.** The key is now canonical, and it does not split states needlessly.
  - Every compensation thread's `thread-state` gains `registration`, and only compensation threads carry it.
  - The mapping decides the thread's `cattempt`/`cend` identities and their effects, so two states that differ in it differ in behaviour. Splitting them is required.
  - `assert_keys_determine_successors` explores the crosswise fixture without expanding a known key twice and requires each repeat of a key to step alike. Key-merged exploration is therefore exact there.
  - The test fails when the member is removed.
- **FND-003.** The static `precedes` walk and `Unsupported::DetachedEffect` are deleted.
  - `event_node` now enables an effect only while its attempt's entry is in the thread's environment.
  - An effected attempt with no result binds an `attempt-record` (receiver, arguments). That is ST-1's record, so the extra `observations` entry is spec-conformant.
  - The effect is added as a keeper of that binder.
  - The footprint writes `bind(attempt, t)` for an effected attempt, and the effect's enabling set reads it.
  - Tests cover an all-joined branch attempt with the effect after the join, which registers, and a `choice` with the attempt in one case: it steps on `yes` and waits on `no`.
- **FND-004.** `a_retry_over_attempt_records_is_refused` asserts the refusal. The PR body lists the previous-attempt-record member as a spec-gap remainder, and STD-233 item 1(c) owns it.
- **FND-005.** A doc comment and the PR body now give the reason: no resolved memory model governs a compensation thread (ADR-025 MM-3), so its attempt is local.
- **FND-006.** The PR body records the rest-point reading.
