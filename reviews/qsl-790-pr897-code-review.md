---
id: SR-11701
title: "Code review of quire-spec-language PR #897: clause-set results and per-read runtime value paths"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@204cdaeee9999870e30d24ce7d672f56d0ba3032; qsl-eval/src/value/expression/clause_set.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-eval/src/value/expression/mod.rs, qsl-eval/src/value/expression/s6a/separation.rs, qsl-foundation/src/witness.rs, qsl-replay/src/result.rs, qsl-replay/src/result/wire.rs, qsl-semantics/src/model/observation.rs"
review_set: subset
---
# Code review of quire-spec-language PR #897

## Summary

Ticket: QSL-790. One pass of code-review with the rust-review lane over
`git diff origin/main...HEAD` (8 commits, merge base `23f9f3423`, #891). The
local commit `f1ee1d7d7` on `clause-set-duplicate` is not part of the PR and
was not reviewed. No builds were run, by brief.

Oracles: FR-107 Behavior sections "The receiver path", "Read paths" and
"Clause sets", FR-107-AC-7 to AC-18, TC-1039, TC-1040, QSpec FR-207 (key
order and canonical serialization) and QSpec FR-204 (key), cited by id only.

Examined: `clause_set::evaluate`, `run_group`, `combine` and `verdict`;
`resolve_clause`; the `ClauseSetup` binding and receiver-path construction;
`note_read`, `read_path`, `located` and `note_element` in `evaluate.rs`;
`Trail::read`; `SelfKind` and the `invocation` field in `observation.rs`;
`ValuePathStep::Reference` and the derived `Ord`; the `reference` step wire
codec and its size estimate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A state-machine precondition selected by `PreCall` now gets context-field paths rooted at a declared object identity the context does not have. `ClauseSetup` sets `trail.receiver_invocation` only for `SelfKind::Context` with a selected invocation, so a `PreCall` leaves it `None`. `note_member` then falls to `Provenance::member(observation, <context reference>, member)`, a `member` step under `ValuePathSubject::Object(<context evaluator reference>)`. Two results are wrong. (1) FR-265 witness domains over a context collection field are located at a fake object identity. (2) `reads()` for a quantifier element of such a field (`forall x in self.items: x.f`) gets `Some(Object(ctx), [member items, index i, field f])` through `note_element`. Meanwhile `self.items` itself correctly reports none. FR-107 says a receiver binding with no selected invocation has no path, and that a context's field step is `field`, not `member` (QSpec FR-208). Before this PR the same case named the pre snapshot as the invocation. Fix: when `self_kind` is `Context` and there is no invocation, give context-field provenance no location, and add a test. | qsl-eval/src/value/expression/mod.rs:662-675, qsl-eval/src/value/expression/evaluate.rs:2395-2425, qsl-eval/src/value/expression/evaluate.rs:2527-2550 |
| FND-002 | high | `read_path`/`located` have no case for a relationship navigation, so a selection on `e.end` reports no path. FR-107 "Read paths" requires the referenced object's declared identity as the subject, with the `member` step alone (FR-107-AC-17, TC-1040 step 9). `located` returns `None` for every node kind other than `Local`, `Attribute`, `Field`, `Value`, `Pre` and `Coerce`. FR-107-AC-17 defers itself "until FR-364 lands". FR-364's clause-level navigation evaluation merged on main as #892 (`c697a12f8`), after this branch's base (#891). So the deferral condition is met, and this rule is part of the ticket's per-read-path outcome. Once the branch is rebased, `self.home.level` reads with `path: None` instead of `(N, object(t1), [member level])`. Fix: rebase on main, root a selection whose operand is a `Navigate` node at `Object(<reference read>)` with no Reference step, and build TC-1040 step 9. | qsl-eval/src/value/expression/evaluate.rs:2449-2521 |

## Verdict

FAIL: 2 blocking findings (FND-001 and FND-002). Both are paths that
contradict FR-107 "Read paths".

Recorded, not blocking:
- medium: the derived `Ord` on `RuntimeValuePath`/`ValuePathSubject` departs
  from QSpec FR-207's key order for invocation identities in two ways. It
  compares fields one by one where the spec compares JCS bytes, and it
  compares `revision_namespace` before `revision` where JCS key order puts
  `revision` first. Nothing in production sorts paths today; only the AC-11
  test uses it.
- low: `admit_invariant` hardcodes `SelfKind::Object`. This holds only while a
  state-machine context cannot be admitted from a current snapshot (#895
  specifies that selection).

Clean:
- Clause-set ordering: invariants, then preconditions, then postconditions,
  each sorted by `QualifiedName`. The set admits only unqualified names, so
  this is UTF-8 byte order. Each clause gets a fresh `Meter::new(limits)`.
- Validation runs before any evaluation. `resolve_clause` runs over the whole
  slice first, and `UnknownClause` and `ObservationsMismatch` come back with
  no entry.
- Skip rule: postconditions run only when every precondition was evaluated and
  is `Completed(true)`. An `Incomplete` invariant or precondition stops the set
  as its last entry. A false invariant does not skip anything.
- Outcome table: `Incomplete` beats `False`, which beats `Undecided`, which
  beats `True`, and an empty set is `True`. `FamilyEvaluated(Refused |
  Undefined)` and kernel `Undefined`/`Refused` map to `Undecided`. This
  matches FR-107 "Clause sets".
- Reads are recorded after the `model.navigate` charge and after the value is
  read. A denied charge records nothing. A skipped branch records nothing.
  Operand reads are recorded before the outer selection. Under `pre(..)`,
  the root observation becomes the pre observation.
- Parameter, result and receiver bindings follow the slot order
  (self, result, parameters). With no invocation they get no subject.
  `option_value` comes in through `value(..)`. A `reference` step is added
  for every dereference except `self` of an object receiver.
- The `reference` step wire form is
  `{tag: reference, object: {universe, type, object_identity}}`, which
  matches QSpec FR-207's canonical serialization and FR-204's key. A
  malformed universe refuses at shape validation.
- `receiver_invocation` was replaced by `invocation` plus `SelfKind`. The
  `PreCall` change (none instead of the pre snapshot identity) matches
  FR-107 "The receiver path" and AC-7. All callers in the workspace were
  updated. See FND-001 for the one consumer that was missed.
- There is no `unwrap`, `expect` or panic in the new non-test code, and no
  vendoring, compatibility shim or tracking ceremony.
- The `Cargo.lock` bump of `quire-semantic-value` to `ad6e975` follows main
  of a `branch=main` dependency.

## Dispositions

Round 1, reviewed at `b44ad734dc3416d4270a432dfadf7d551e3d89c5`. Fix commit
`b44ad734d`; merge of main `5c108b057` (one import conflict, resolved cleanly).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | b44ad734d |
| FND-002 | fixed | b44ad734d |

FND-001: `Trail::receiver_unlocated` is set only for a `Context` with no
invocation. `note_member` records no provenance for that receiver, and
`note_element` gives no path to an element of a selected collection that has
no provenance. An invocation-selected receiver is unaffected, because the flag
is false there. `a_context_collection_read_under_pre_call_has_no_path` asserts
`[None, None, None]` under `PreCall` and `(p1, receiver i1, [field items])`
by invocation.

FND-002: `read_path` roots a selection whose operand is a navigation (seen
through `value`, `pre` and coercions) at `Object(<reference read>)`, with the
`member` step alone. `a_selection_on_a_navigation_is_located_at_the_referenced_object`
builds TC-1040 step 9 and asserts exactly one read,
`(N, object(t1), [member level])`.

Recorded in round 1, not blocking:
- medium: the "selected collection with no provenance gives none" branch also
  applies to a located owner's stored collection when it is selected inside a
  quantifier body. Provenance is recorded only at claim level
  (`iterations == 0`). Such elements now get no path where FR-107 names the
  stored location. Before the fix they got a wrong `built` subject, so this
  is absent rather than wrong. A path from `trail.selection_path(source)` plus
  `Index(position)` would give the right location in both cases.
- medium: `selected()` does not look through `value`/`pre`/coercion, unlike
  `navigated()`. So an unlocated context's collection under one of those
  wrappers still falls back to a `built` subject.
- medium: the element-of-navigation branch in `note_element` (a multi-valued
  navigation as a quantifier domain) has no test. No FR-107 AC names that
  case.
- low: the new PreCall test is tagged `TC-1040`/`FR-107-AC-7`, but TC-1040 has
  no `items` step. AC-7 is fully covered elsewhere, so no coverage is
  invented. The AC-17 test builds its observations by hand rather than
  admitting them by FR-106.
