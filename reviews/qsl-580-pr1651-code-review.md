---
id: SR-1651001
title: "Code review of quire-spec-language PR #1651: a clock of a referenced object (580i)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@33831d742bad3cfdaeb45005f880ecd68d149470; PR #1651 diff against qsl-580h-clock-places (42cb3fa23): qsl-eval/src/simulation/model/timed.rs; qsl-semantics/src/check/lowering/model.rs; qsl-semantics/src/check/timed_model.rs, timed_model/reading.rs; qsl-semantics/tests/it/{clock_places,timed_model}.rs; qsl-eval/tests/it/timed_subject.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Code review of quire-spec-language PR #1651

## Summary

Ticket: QSL-580, slice 580i, stacked on #1646. The Rust review lane is folded
into this review. I reviewed only this PR's diff against
`qsl-580h-clock-places`.

**`ClockConstraint::via`.** Only the reading expansion sets it. Its places now
carry `(via, ClockId)`, and `constraints` copies `via` into both the positive
and the negated DNF. Time invariants read clocks only through `clocks.read`,
so their `via` is always empty, which is correct because an invariant cannot
call a clock-reading declaration. Resets carry no `via`. A direct
`self.peer.x` still refuses `clock-read-outside-constraint`, as restored in
#1646.

**Reference-path arguments.** `through_references` walks only required,
single-valued fields whose type is a package object type (`referenced_object`).
An optional or multi-valued reference therefore stops the walk, and the
argument or body path is not a place. `site_object` takes `self` or a
reference path that is walked in full and conforms to the parameter's type.
`own_clock(self.peer)` and `peer_clock(self)` both end at `x` via `[peer]`.
An object argument that a body only reads as data is grafted as the same
`self.peer…` path it was before, so the data meaning is unchanged.

**Per-state resolution.** `guarded_steps` resolves each constraint's holder
from the state's reference values (`holder`). Zone search and concretisation
consume the resolved `GuardedStep` parts, and `scaled_bounds` collects
constants per clock field, so the zone engine is correct for clocks of other
objects.

**State key.** The holder is a function of the discrete state, and the
reference values are in the model state key. Clock values stay keyed by their
owning place (object, field, path). So this PR needs no new state identity:
two states that differ in `self.peer` already have different keys.

**Absent objects.** A `peer` field is a required single reference, and FR-120
requires that "Every reference of a candidate names an object of the
candidate". So in an admitted or explored state the referenced object is
always present, and the "not enabled" branch cannot be reached. The spec
gives no presence condition or refusal here: an optional reference never
becomes a place (see above). Treating a holder that cannot be resolved as
"not enabled" is still the wrong default (FND-001).

Gates: `cargo test -p qsl-semantics -p qsl-eval -p qsl-analyze --all-features`
ran at 33831d742 through `locked-build.sh` (jobs 4, `QSPEC_DIR` set) and
passed (exit 0).

## Verdict

**APPROVED with low findings.** There are no blocking defects.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `placed` drops a guard part, silently and as "not enabled", whenever the holder cannot be resolved: the referenced object is missing from the state, the field is missing, or its value is not a reference. The same drop applies to the negated part, so a guard and its negation can both be false. By FR-120's closure rule this cannot happen for a required single reference, so reaching it means the state is broken. Return a `TimedStop` broken-invariant fault instead of `None`. That way a defect elsewhere shows up instead of quietly removing transitions. | qsl-eval/src/simulation/model/timed.rs:621-631, 696-752 |
| FND-002 | low | Rust efficiency and clarity. `holder` scans every object of every population and rebuilds each object's `ObjectReference` for every constraint of every state. A lookup by the reference's population and key would avoid that. `referenced_object` scans all package records on every call, and `path_kind` and `site_object` call it for each body node, which is the same pattern #1646 FND-004(c) cached for places. `Place = (Vec<PlaceStep>, ClockId)` is a tuple alias; a small struct with named fields `via` and `clock` would read better at the destructuring sites. | qsl-eval/src/simulation/model/timed.rs:724-752; qsl-semantics/src/check/lowering/model.rs:1158-1200; qsl-semantics/src/check/timed_model/reading.rs:44-49 |

## Dispositions

Round 1. The head is d10f8147c, restacked on #1646 f439558a7; the fix commit is d10f8147c on top of e2c58a57c.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `placed` now returns `Result<_, ExpansionStop>` and stops with `BROKEN` when a holder cannot be resolved, or when a referenced place is not present, instead of dropping the part. The fix has no test, and that is acceptable. The branch can only run on a state where a required single reference names no present object, which FR-120's closure ("Every reference of a candidate names an object of the candidate") rules out for every admitted or explored state. Reaching it through the public API would mean building an inadmissible state. A test that has to forge broken internals would test the forgery, not a behaviour. |
| FND-002 | fixed | `holder` now looks up `objects.get(key)` in each population instead of scanning every object. `referenced_object` reads `references`, which is computed once in `with_places`. `Place` is a struct with `via` and `clock`. |
