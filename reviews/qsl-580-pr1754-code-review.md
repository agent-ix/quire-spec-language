---
id: SR-1754001
title: "code review (with rust-review) of PR #1754 (QSL-580t): general presence, PR-01/PR-05"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1754 git diff origin/qsl-580s-clock-place...732a1c044 (19 files): qsl-semantics check/check.rs, check/typing.rs, ir.rs, timed_model.rs, state_clause.rs, model/object_environment.rs; qsl-eval simulation/model/timed.rs, state_predicate.rs, value/expression/evaluate.rs; tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1754001: code review of PR #1754 (QSL-580t)

## Summary

Ticket: QSL-580. Head reviewed: 732a1c044, on qsl-580s-clock-place (5dffd7e48).

**The head is red (FND-001).**
- `clock_places::another_objects_presence_is_a_data_read`, the PR's own S3 test, fails at 732a1c044. `time invariant Peer when { present(self.peer.v) }` refuses `Temporal(PresenceNotChecked)` at `present(self.peer.v)`.
- The test passes at 5abec2a8b and at 697b4e40e. The 580s merges (c61b80db3, 732a1c044) broke it. Those merges change no timed file, so the interaction is indirect.
- On the same head, `urgent U when { present(self.peer.v) }` and `time invariant N when { not present(self.peer.v) or self.late }` check, and the qsl-eval test `another_objects_presence_reads_its_bit` passes.

**Guards and postconditions now give the wrong refusal (FND-002).**
- `Located` now skips the presence refusal for another object's place in every timed clause.
- Only the time-invariant and urgency `when` predicates are typed with `presence_places`.
- A precondition or postcondition is typed as an ordinary state clause, with `presence_places: &[]`. There, `present(self.peer.v)` reads `self.peer.v` as a `Clock` field and refuses `ill_typed`/`type-mismatch` (`StateModel`, `receiver: None`) at the clause. That is PR-05's cause, used for a well-typed read.
- On the base 5dffd7e48, all three forms (`pre G { present(self.peer.v) }`, `pre G { present(self.peer.v) and self.x >= 1 ms }`, `post Q { present(self.peer.v) }`) refused `PresenceNotChecked` (`unsupported_construct`/`not-yet-implemented`) at the read. That was the honest "not yet" refusal.
- FR-230-AC-51 records `present(other.v)` "as data predicates of the convex part", which is the guard form.

**What works (eval, two objects that are each other's `peer`).**
- In each of the four initial states, the time invariant bounds `x` exactly where the peer's `v` is live. Bounds per state were 0/1/1/2, matching the live bits.
- `urgent U when { present(self.peer.v) }` is urgent exactly when the peer's place is live.
- An owner outside the system (`c1` alone, its `peer` `c2`) is refused at admission (`InvalidRuntimeInput`/`invalid-value`). So `place-presence-unrecorded` was not reached there.
- PR-05 (`present(self.late.v)`, a non-object owner) refuses `type-mismatch`. The typer's fallback reads a same-named non-clock `Option` field as an ordinary `present`.
- `PlacePresence` refuses at lowering, since S4 emits no timed predicate yet.

**Rust.** `place_operand` matches only by field name and case across all models' places; `place_presence` then filters by `conforms(owner)`, so a type holding no such place falls back to the ordinary field read. `PlacePresences` is a `BTreeMap` keyed by (object, field, case).

Gates at 732a1c044 (locked-build, jobs 4, `--all-features`, debug info off):
- qsl-semantics it `clock_places`/`timed`: 97 passed, 1 failed (FND-001).
- qsl-semantics lib: 811 passed.
- qsl-eval it `timed_subject`: 65/65.
- clippy `-p qsl-semantics -p qsl-eval --all-targets -D warnings`: exit 0.
- fmt: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR's own `another_objects_presence_is_a_data_read` fails at 732a1c044 (`PresenceNotChecked` at `present(self.peer.v)` in a time invariant's `when`). It passes at 5abec2a8b, so a 580s merge broke it. | FR-230-AC-51, QSpec FR-322-AC-155 |
| FND-002 | medium | `present(other.v)` in a precondition or postcondition now refuses `ill_typed`/`type-mismatch` at the clause instead of the base's `PresenceNotChecked` at the read. Either type guards with `presence_places` (FR-230-AC-51's convex-part form) or keep the not-yet-implemented refusal there. | FR-230-AC-51, FR-230 "Presence bit" |

## Verdict

Changes requested: FND-001 and FND-002.
