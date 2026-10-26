---
id: SR-1633002
title: "Gap analysis of quire-spec-language PR #1633: time units of different owners, Quantity positions (580f)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3f80fb9ba73f9e6d5dc81d5201e13426e3cd8e09; PR #1633 against FR-230-AC-15, AC-16, AC-17, AC-18, AC-19, AC-20, AC-21, AC-23, AC-24, AC-27, AC-28, AC-33 and FR-230 Unit-typed members, TC-685; QSpec FR-154 and FR-322 package member rows and shared grammar type-ref (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Gap analysis of quire-spec-language PR #1633

## Summary

Ticket: QSL-580, slice 580f. Plan completion: not assessed. This review used
the trace attributes on the PR head, the AC text and the code.

Backed by tests: FR-230-AC-18 in full, covering the S3 dimensions, the
accepted and refused constants and periods, the `len` and `Duration` units,
and three package identities. FR-230-AC-15 is backed except for the `Map`
vector. The fix also supplies AC-33's "every time-valued quantity uses that
dimension".

Deferrals checked against the spec:
- **AC-27 and AC-28, dimension equivalence:** an implementation gap. They are
  already required to be built, so they are not flagged. Their package-identity
  halves also depend on STD-493 item 9 (FR-322 timed node bodies).
- **No source `Map` type (item 12): real gap, but too wide** (FND-002). QSpec
  `type-ref` has `Option`, `Sequence`, `Set`, `Bag`, `OrderedSet`,
  `Reference`, `Quantity` and `Population`, and no `Map`.
- **Package intake has no `sequence`/`map` type definitions or unit/clock alias
  targets (item 13): not a spec gap** (FND-001).
- **AC-21's time-invariant `forall`/`exists` vectors (item 14): real spec
  gap.** It is the same `clock-bound` grammar limit as AC-9.

## Verdict

**CONDITIONAL.** This PR's own ACs are met. The deferrals the PR reports for
AC-16, AC-17, AC-19, AC-20, AC-21, AC-23 and AC-24 rest on two reasons. One
is not a spec gap, and the other covers more vectors than it explains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-16, AC-17 and AC-23 are deferred as spec gaps (STD-493 item 13) because QSL's package intake reads no `sequence`/`map` type definitions and admits only native alias targets. The spec defines all of these. QSpec FR-154 lists "alias `target`, sequence `items`, map `values`" as typed members, and FR-154-AC-24 and AC-27 admit `ix://quire/unit/ms` and `ix://quire/native/Clock` there. QSpec FR-322-AC-109 reads package `sequence`/`map` types over `Clock`. FR-230 "Unit-typed members" says a package alias `target`, sequence `items` and map `values` of `ix://quire/unit/<name>` read as `Quantity<u>`. This is a QSL intake implementation gap that can be built now. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:514-521, 595-602; quire-specification spec/functional/type-model/FR-154-admit-domain-package-model.md:92, 193, 532, 535 |
| FND-002 | medium | Item 12 (no source `Map`) defers AC-19, AC-20, AC-21 and AC-24 whole, but it explains only their `Map` vectors. AC-24 has no `Map` vector at all (`Reference<Clock>`, `deref`, `ref`, a slot reference). AC-19's fields belong to the package object type `Call`, so they are reachable through package multiplicities (FR-322-AC-109) and, once FND-001's intake is built, through package `map` types. AC-20's sequence and set writes and AC-21's sequence selection, `forall`/`exists` guards and collection refusals can all be written in source. Only `Map`-keyed source vectors (AC-21 `self.m[k]`, AC-19's `Map<Timer, Int>` key refusal) and AC-21's time-invariant bodies (item 14) are real gaps. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:598-603 |
| FND-003 | low | The test tagged FR-230-AC-15 omits the AC's `Map<Text, Quantity<ms>>` position, which source cannot write. The tag claims the whole AC, and only the doc comment says what is missing. | qsl-semantics/tests/it/unit_typed_members.rs:68-80 |
