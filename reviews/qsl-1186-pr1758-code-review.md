---
id: SR-1758001
title: "code review (with rust-review) of PR #1758 (QSL-1206 / STD-539): nested repeat progress"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1758 git diff origin/main...8dcb8e559 (5 files): qsl-semantics protocol_clause/graph.rs, protocol_clause/mod.rs, protocol_slots.rs, tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1758001: code review of PR #1758 (QSL-1206 / STD-539)

## Summary

Ticket: QSL-1206 (parent QSL-1186). Head reviewed: 8dcb8e559, on main.

The rule implemented is QSpec main a58e0852, FR-464 "Nested `repeat` rule". A `repeat` in another `repeat`'s body steps for it exactly when three things hold:
- its exit is infeasible;
- its `maximum` is absent or at least 1;
- its body steps.

Its `exhausted` control is never counted.

- **(a)** `K::Repeat` now reads only `repeat_body`, the child whose name lies in `form.body`. It no longer reads `exhausted`.
- **(b)** `entered = max.is_none_or(|max| max.value >= 1)` gates the contribution. This closes the latent unsoundness: before, a nested `max 0` repeat with a stepping body counted as progress, although its body never runs.
- **Nesting.** `nested_repeats` now marks a `repeat` nested only when it is in a `repeat`'s body. A repeat reached through the `exhausted` of a repeat that is itself in a body inherits `in_body`. A repeat reached only through the outer repeat's own `exhausted` is not nested, so the exit-feasibility walk does not apply to it.
- **(c)** `a_nested_repeat_counter_restarts_on_each_entry` checks `X E X E F`.

**Soundness probes.** These were run through S3 with an outer `repeat while true` and reverted afterwards. Every case where the outer cannot progress is refused `NoProgress` at `Outer`:
- a nested `max 0` with a body event;
- a nested `while { false }`;
- a nested guard over one visible atom (`a.ready`), so the exit is feasible;
- a `choice` with an event arm and a `max 0` repeat arm;
- deeper nesting, `Mid { Inner max 0 }`, which refuses `Outer` and `Mid`;
- progress only through the `exhausted` of an in-body `repeat`.

These are admitted, as they should be:
- the `choice` arm with `max 1`;
- deeper nesting `Mid { Inner max 2 }`;
- an event before a nested `max 0`;
- a top-level `exhausted` repeat with a feasible guard.

The only previously admitted models that are now refused are those whose progress came through `exhausted` or a `max 0` body. FR-464 now refuses those by rule. Two of them are genuinely unsound, and the exhausted-only case is a spec choice.

**Rust.** `repeat_body` is shared between the progress pass and `nested_repeats`. `nested_repeats` now builds a node map once instead of scanning per node.

**Pre-existing.** The progress pass pushes a `NoProgress` refusal for every non-stepping `repeat`; deeper nesting gives `Outer` and `Mid`. FR-464 says "the first such `repeat` in tree order", singular (FND-001).

Gates at 8dcb8e559 (locked-build, jobs 4, `--all-features`, debug info off):
- qsl-semantics lib `protocol`: 166/166.
- qsl-semantics it `protocol`: 31/31.
- qsl-analyze it `protocol_constructs`/`repeat`: 19/19.
- Root `repeat`/`native_choice`/`native_protocol`: 60/60.
- clippy `-p qsl-semantics -p qsl-analyze --all-targets -D warnings`: exit 0.
- fmt: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Pre-existing: every non-stepping `repeat` gets its own `NoProgress` refusal (nested `max 0` under `Mid` refuses `Outer` and `Mid`), where FR-464 refuses "the first such `repeat` in tree order" | FR-464 Progress, FR-464-AC-23 |

## Verdict

Mergeable. FND-001 is low and pre-existing.
