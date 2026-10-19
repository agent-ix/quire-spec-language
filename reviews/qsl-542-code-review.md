---
id: SR-12809
title: "Code review of quire-spec-language PR #967: read, write and enabling footprints (FR-155)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@52899349d5b0460cfed6acde3ad0a2b47a0ffb31; PR #967: qsl-semantics/src/model/footprint.rs, qsl-semantics/src/model/object_environment.rs, qsl-eval/src/simulation/footprint.rs, qsl-eval/src/simulation/model/footprints.rs, qsl-eval/src/simulation/model/frames.rs, qsl-eval/src/simulation/model/walk.rs, qsl-eval/src/simulation/model.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-eval/tests/it/footprints.rs"
review_set: subset
---
# Code review of quire-spec-language PR #967

## Summary

Ticket: QSL-542. Code review with the rust-review lane folded in. Diff
scope: `git diff origin/main...HEAD` (two commits). Focused check run on the
head commit: `cargo test -p qsl-eval --test it footprints` (7 passed). A
scratch copy of the head commit (never the worktree) was used for one
mutation and one probe test, reported here and in the gap analysis
(SR-12810).

What is right. The derivation follows QSpec FR-383's read-footprint items 1
to 4 for a receiver of the declaring type. It handles a precondition, a
postcondition's `pre(...)` reads, post-state reads outside `W(t)`, and the
receiver's and reference arguments' membership. A navigation `self.cell.v`
reads `Field{Receiver, cell}` and `AnyField{v}`. A `result` read ranges over
the result population. Function calls, dispatch, relationship navigation and
protocol references refuse with `UnsupportedForm`; they are never
under-approximated. `independent` is exactly POR-4. Instantiation drops a
location whose optional argument is absent. Read enforcement works by
logging reads and checking them afterwards. Pre-clause reads carry
`Observation::Pre` and are held to `R(t)` only. Post-clause reads are held
to `R(t) ∪ W(t)`. This matches FR-155 "Read enforcement".

The author's decisions, judged:

- `footprint(state, transition)` against FR-155's `footprint(transition)`:
  this is not only a signature difference. It makes the function partial
  (FND-002). It needs an owner ruling against FR-101/FR-162's hook.
- `Location::meets` without member types: this is sound. It meets in every
  case FR-383 meets and in more. The cost is precision (FND-005).
- Composite fields not scanned for delete-blocking reads: this is latent,
  not live. The engine's own dangling-reference scan skips `Composite` too
  (`frames.rs:1028`), and `ModelSystem` cannot encode a composite field
  value (`encode.rs:192`). So footprint and engine agree today (FND-006).
- The `successors_within` `restrict` closure: the restriction lives on the
  production API and can widen a footprint as easily as narrow it (FND-003).

A real soundness defect the PR does not mention: derivation works per
declaring type, not per receiver type (FND-001). A probe confirmed it.

## Verdict

Not mergeable. FND-001 is a soundness defect in the derivation. FND-002
needs an owner ruling before the hook is wired. The rest are lows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Footprints are derived once per (declaring type, operation), and the deleted and member types are taken from the declaring type. An inherited operation run on a subtype receiver therefore misses locations that FR-383 requires. `deletes self` puts `self.declaring` in `deleted_types` and writes `Membership` only for populations the declaring type conforms to. Item 5 adds `AnyField{g}` only where `conforms(deleted, target)`, so a field typed `Reference<Sub>` is skipped. Probe on the head commit: `Base` declares `cancel()` framed `deletes self`, `Sub` extends `Base`, `Holder` has `r: Option<Reference<Sub>>` and `q: Option<Reference<Base>>`, and the populations are `bases: Base` and `subs: Sub`. Then `derive_footprint` for `Sub`'s `cancel` reads `{Field{Receiver, flag}, AnyField{Holder/q}, Membership{bases, Receiver}}` and writes `{Membership{bases, Receiver}}`. It has no `AnyField{Holder/r}` and no `subs` membership. Failure scenario (FR-383-AC-12 with a subtype): `h.r = s` blocks `cancel(s)`. `unlink(h)` clears `r` and enables it. But `unlink(h)`'s write `Field{h, Holder/r}` meets nothing in `cancel(s)`'s enabling footprint, so the candidate of the disabled `cancel(s)` leaves the enabler out, and the reduced search can miss the violation. Fix: derive per application identity (the receiver's effective type, which `ModelSystem` already keys applications by), or treat deleted and member types up to conformance in both directions, as `populations_relating` already does. Spec question for the owner: does FR-383's "the deleted object's type" mean the receiver's type, as this finding assumes? | qsl-semantics/src/model/footprint.rs:284-311; qsl-semantics/src/model/footprint.rs:332-359; qsl-eval/src/simulation/model.rs:463 |
| FND-002 | medium | `ModelSystem::footprint(state, transition)` resolves the receiver and each reference argument against the rows of `state`. It returns `UnknownTransition` when either is absent there. FR-155 "Instantiation" and "Determinism" make the footprint a function of the package and the transition identity alone. FR-383's candidate rule needs footprints of disabled identities, for example a dependent of a `spawn` whose receiver is a universe key not yet created. Failure scenario: at the Counter start state with universe `{a,b,c,d}`, the candidate seeded by `spawn(a)` must hold `bump(d)`, which depends on it, and `footprint(s0, bump(d))` errs, so FR-162 would settle an internal fault. Owner question: what signature should the FR-101/FR-162 `footprint` hook have, and must it be total over the universe's transition identities? | qsl-eval/src/simulation/model/footprints.rs:329-375 |
| FND-003 | low | `successors_within` takes a public `restrict` closure, and its result replaces the derived footprint unchecked. A caller can widen it, for example by returning `AnyField`/`AnyMembership` of everything, and so switch read enforcement off. FR-155 says `ModelSystem` enforces the derived footprint. Expose a removal-only test seam (intersect with the derived footprint), or move it behind `#[cfg(test)]`/`#[doc(hidden)]`. | qsl-eval/src/simulation/model/footprints.rs:389-406 |
| FND-004 | low | `log_field_read` returns without logging when the field's attribute does not resolve in the object's effective attribute set. That read then escapes enforcement silently, where it should fail as an internal fault. | qsl-eval/src/value/expression/evaluate.rs:2183-2203 |
| FND-005 | low | `Location::meets` treats `Membership`/`AnyMembership` and any `AnyField` (and any `Field`, for `AnyMembership`) as meeting, whatever the population's member type. This is sound but loses independence that FR-383 grants. For example, `spawn()` (writes `AnyMembership{counters}`) depends on `set(a)` on an unrelated `Cell`, so any model with a `creates` loses most of its reduction. Carry the member type, or a population-to-fields table, when the reduction engine lands. | qsl-eval/src/simulation/footprint.rs:96-99 |
| FND-006 | low | `referenced_types` ignores `ValueType::Composite`, so a record field holding a reference never makes a delete read it (FR-383 item 5). This is consistent with the engine today (`collect_references` skips `Composite`; composite field values are unencodable in `ModelSystem`). It is latent: give it a refusal or a tracked follow-up, so that both are fixed together when composites become state. | qsl-semantics/src/model/footprint.rs:164-184; qsl-eval/src/simulation/model/frames.rs:1028 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | `parse_reference`/`unhex` accept non-canonical text. `u8::from_str_radix(_, 16)` accepts uppercase digits and a leading `+` (so `"+a"` parses as `0x0a`). A transition text that `reference_enc` could never write therefore gets the footprint of the canonical reference, where it should get `UnknownTransition`. Crafted object text cannot inject a member, because every field is hex and the splits are on fixed delimiters. The round trip for canonical texts holds, since RFC 8785 sorts the keys as `identity`, `object_type`, `type`, `universe`. Fix: accept only `[0-9a-f]`, or re-encode the parsed reference with `reference_enc` and compare it to the input. | qsl-eval/src/simulation/model/encode.rs:137-179 |
| FND-008 | low | `footprint` zips the declared parameters with `transition.arguments()`, so an argument list that is too short or too long is not rejected. A reference parameter with no text silently loses its `Field`/`Membership` locations and yields a narrower footprint, when it should be `UnknownTransition`. This was missed in the review pass; the zip predates the fix round. | qsl-eval/src/simulation/model/footprints.rs:351-362 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Fixed for `deletes self`: derivation now takes the receiver's type (938aec115), with a test. It is not fixed for `deletes p` or a type-wide `deletes T`, whose item-5 referrer reads still use `conforms(deleted, target)` in one direction only (`footprint.rs:357-365`). A probe on the head commit (`1ad86fa88`) with `Holder.r: Option<Reference<Sub>>`: `kill(victim: Base)` framed `deletes_parameters: [victim]`, and `purge()` framed `deletes Base`, both write `subs` membership but neither reads `AnyField{Holder/r}`, although both can delete a `Sub` that `h.r` names. Fix: in the item-5 loop, match deleted and field-target types by conformance in either direction, as the writes already do. Owner question still open: does FR-383's "whose referenced type the deleted type conforms to" also cover a field typed as a subtype of the deleted type? |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | accepted-no-change | The over-approximation is sound, and its precision cost is now documented on `Location::meets`. Member types belong with the partial-order engine. |
| FND-006 | accepted-no-change | This is latent and consistent with the engine, which neither encodes composite field values nor scans them for references. `referenced_types` now documents it. |
