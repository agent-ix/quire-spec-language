---
id: SR-1629002
title: "Gap analysis of quire-spec-language PR #1629: FR-230 Clock-type facet gate (580d)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@96de9b635e4b49bf0d0fc394f91fcd476f114eb3; PR #1629 against FR-230-AC-1, AC-3, AC-4, AC-7, AC-8, AC-29, AC-30, AC-31, AC-32, AC-34, FR-231-AC-2, FR-236-AC-1, FR-236-AC-4 and TC-685/686/691; QSpec FR-322 Timed-model nodes and shared grammar (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-236
    type: reviews
---
# Gap analysis of quire-spec-language PR #1629

## Summary

Ticket: QSL-580, slice 580d. Plan completion: not assessed. This review used
the trace attributes on the PR head, the AC text and the code. It did not use
a `quoin matrix` run.

Backed by tests: FR-230-AC-3 (S2 and S3), AC-4 (S2, S3 and the 6,400 places
with `10^40 ms`), AC-31 and AC-34 (I2 vectors, including missing, extra and
mismatched `resolved_path` rows), and FR-236-AC-1. FR-230-AC-7 is backed
except for the owner (FND-001).

Deferrals checked against the spec:
- **AC-1, bound `T` (STD-493 item 6): real spec gap.** QSpec `clock-bound`
  takes a `quantity-literal`.
- **AC-7, the "no `profile`" vector (item 8): real spec gap.** The
  shared-grammar `composed-unit = header, profile, { profile }, ...` requires
  at least one profile.
- **AC-29, the S3-to-S4 half of AC-30, AC-32 beyond the time source, and the
  identity half of AC-1 (item 9): real spec gap.** In QSpec FR-322 "Timed-model
  nodes", `clock_reset` is exactly `place`, `value` and
  `clock_guard`/`time_invariant` are exactly `predicate`, and constraints form
  one flat root aggregate. Nothing binds a guard or a reset to its operation
  or groups constraints into convex parts. Writing the current model into
  these bodies would lose meaning and could collide identities.
- **FR-236-AC-4, byte equality (item 7): real, low-impact spec gap.**
  `TemporalCounterexample` has no byte encoding, and FR-126-AC-7's
  "byte-equal" is value equality.
- **AC-8 "definition_selections" (item 9): not a spec gap** (FND-003).

## Verdict

**CONDITIONAL.** Two acceptance criteria are tagged complete but are not met
as written (FND-001, FND-002). AC-8 is deferred for a reason its text does
not contain (FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-230-AC-7 requires each refusal to retain "its owning declaration", which FR-230 defines as the model for a `time invariant` or `urgent` declaration. The test tagged FR-230-AC-7 asserts the declaration names `Deadline` and `Eager`, so the tag claims an AC the code does not meet. | qsl-semantics/tests/it/timed_declarations.rs:700-740; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:559-564 |
| FND-002 | medium | FR-231-AC-2's last clause ("under event-position false-extension the execution closes at position 2") is tagged on a test that chooses the closed reading itself (`idle_tail = false`, `Finite { len: 3 }`). No code maps the unit's temporal profile to that reading, so the profile half of the AC is unbacked. | qsl-eval/tests/it/timed_subject.rs:588-655 |
| FND-003 | medium | FR-230-AC-8 has no FR-230-AC-8 tag, and the deferral routes it to STD-493 item 9 ("definition_selections for a unit with clocks"). AC-8's text never mentions `definition_selections`. It asks for E3 resolution (FR-110-AC-13, already tagged and tested), for S3 admission under the selected layer (already exercised by the AC-1..AC-5 tests, which select `quire.temporal.complete/v1`), and for AC-7's refusals without it (built in this PR). AC-8 can be built and tagged now. | qsl-semantics/tests/it/timed_model.rs:205; qsl-replay/src/spine/clause/tests/layers.rs:170 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The AC-7 test asserts the model `Config` as the owner of the `time invariant` and `urgent` refusals (commit f67887da9). |
| FND-002 | fixed | The FR-231-AC-2 event-position half now derives the closed reading from the clause's profile through `TimedBehaviour::clause_reading`. |
| FND-003 | fixed | `a_unit_selecting_the_timed_facet_passes_the_gate` is tagged FR-230-AC-8. It checks that the selected unit gets no `unsupported_construct`/`declaration-form` refusal, and its doc comment points to FR-110-AC-13's E3 test and the AC-1..AC-5 tests for the other parts. |
