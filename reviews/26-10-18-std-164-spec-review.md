---
id: SR-12811
title: "base review of STD-164 FR-155-AC-7, FR-162-AC-5 and TC-1075"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8b413f340493dc7b760c011401a08ed358196e58; spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md (FR-155-AC-7), spec/functional/FR-162-offer-reductions-through-transition-system-hooks.md (FR-162-AC-5), spec/test-cases/TC-1075-footprints-are-state-free-and-cover-disabled-transitions.md"
review_set: subset
---

## Summary

Ticket: STD-164 (PR quire-spec-language#969). Base checklist over the new
FR-155-AC-7, FR-162-AC-5 and TC-1075: ID format, uniqueness, AC-to-TC links,
and oracle strength.

## Verdict

PASS on ids, FAIL on oracle strength (one medium finding). TC-1075,
FR-155-AC-7 and FR-162-AC-5 are well formed. None of them collides with
main or with any open PR (#967, #968, #966, #964, #963, #962, #957 and older
were all scanned). TC-1075 `verifies` FR-155 and FR-162, and both ACs name
it. No hash, digest or pin use. The `bump(d)` half of AC-7 is a strong
oracle: it gives exact read and write sets, and it fails a footprint that
looks the object up in a state or an application table built from objects
(the pre-rework `UnknownTransition`). The `Base`/`Sub` half is weak.

## Examined Scope

```yaml
scope:
  - {id: FR-155-AC-7, path: spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md, role: examined, excerpt: "`ModelSystem::footprint` of `bump(d)` returns reads `{Field{d, v}, Membership{counters, d}}` and writes `{Field{d, v}}` without error, equal to the footprint it returns after `spawn` creates `d`; ... the footprint of `cancel(s)` writes `Membership{bases, s}` and `Membership{subs, s}` and reads `AnyField{Holder/r}`, for `s` of either type and whether or not `s` exists."}
  - {id: FR-162-AC-5, path: spec/functional/FR-162-offer-reductions-through-transition-system-hooks.md, role: examined, excerpt: "`ModelSystem`'s `footprint` hook, called with `bump(d)` while no state holds `d` and with `bump(a)` while it is disabled, returns `Some` in each case, equal to the footprint it returns when the transition is enabled; the product system's `footprint` for the same transition equals it. A test `TransitionSystem` whose hook is not implemented still settles a request for partial-order reduction V-8."}
  - {id: TC-1075, path: spec/test-cases/TC-1075-footprints-are-state-free-and-cover-disabled-transitions.md, role: examined, excerpt: "3. Call `footprint` for `cancel(s)` with `s` typed `Base`, `s` typed `Sub`, and `s` absent ... Step 3: the same footprint in all three cases, with `Membership{bases, s}`, `Membership{subs, s}` and `AnyField{Holder/r}`"}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `Base`/`Sub` oracle is not exact and is partly unexecutable. AC-7 and TC-1075 step 3 list locations the footprint must hold ("with"), not the whole set. "The same footprint in all three cases" has no meaning, because `s` typed `Base` and `s` typed `Sub` are different reference triples (FR-120 `object_type`), and an "absent" `s` still needs a type in its identity. Failure scenario: an implementation that writes `AnyMembership` for every population, or that relates every population to every type, passes. So does one whose footprint differs by `s`'s type only in the object component. The test cannot fail for over-inclusion or for most-specific-type keying it meant to forbid. Fix: give the exact reads and writes of `cancel(s)`, compare footprints with the object abstracted, and add an unrelated population that must be absent (plus SR-12812's multi-supertype case). | spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:146; spec/test-cases/TC-1075-footprints-are-state-free-and-cover-disabled-transitions.md:27,35 |
| FND-002 | low | "Equal to the footprint it returns after `spawn` creates `d`" and "while it is disabled" are vacuous for a hook that takes no state: the call is identical either way. Name what changes, such as a `ModelSystem` built from a different initial document, or drop the clauses. | spec/functional/FR-162-offer-reductions-through-transition-system-hooks.md:122; spec/test-cases/TC-1075-footprints-are-state-free-and-cover-disabled-transitions.md:25-26,33-34 |
| FND-003 | low | FR-162-AC-5's last sentence repeats FR-162-AC-1 and FR-156's Systems bullet and is unrelated to state-freedom. It is also unclear against the Offering bullet: a system that offers POR with an unimplemented hook settles `Failed`, not V-8. Say "does not offer" or drop it. | spec/functional/FR-162-offer-reductions-through-transition-system-hooks.md:71-76,122 |
| FND-004 | low | "`spawn` is in the candidate of the disabled `bump(d)`" leaves the candidate's seed unnamed. FR-383 candidates are seeded by enabled transitions, and `bump(d)` joins one only through a dependent enabled member. Name the seed. | spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:146; spec/test-cases/TC-1075-footprints-are-state-free-and-cover-disabled-transitions.md:33 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | AC-7 and TC-1075 check object_type only for the receiver. The State-free bullet also requires the reference arguments' object_type for `deletes p` writes and for the item-4 argument memberships, and no vector covers that path. Failure: PR #967 (9250483a0, qsl-semantics/src/model/footprint.rs:312,346-353) derives argument memberships with `populations_relating(declared parameter type)`, and TC-1075 would pass it. With `p: Reference<A>` and argument `c` of type `C extends A, B`, that code omits `Membership{bs, c}`. Fix: add an operation with a reference parameter (for example `drop(p: A)` framed `deletes p`) called with `c` of type `C`, with exact sets that include `Membership{bs, c}` and `AnyField{Holder/h}`. | spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:147 |
| FND-006 | low | The FR-162 State-free footprint hook bullet says 'takes a transition identity and no state' and then 'SHALL NOT take a state' in the same sentence, and the edited line is not rewrapped. Drop the second clause. | spec/functional/FR-162-offer-reductions-through-transition-system-hooks.md:77-81 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | AC-7 and TC-1075 step 3 now give exact read and write sets per object_type, and the sets differ by type (25695410f, FR-155:147, TC-1075:27,35). |
| FND-002 | fixed | The 'after spawn creates d' and 'while it is disabled' clauses are gone; AC-5 now says the hook is called with no state (25695410f, FR-162:122). |
| FND-003 | fixed | The V-8 sentence is removed from FR-162-AC-5 (25695410f). |
| FND-004 | fixed | The candidate is now seeded by the enabled `spawn` (25695410f, FR-155:147, TC-1075:25,33). |
