---
id: SR-1770001
title: "code review (with rust-review) of PR #1770 (QSL-580u): urgency emission"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1770 git diff origin/qsl-580v-slot-bits...37cc994a9 (11 files)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1770001: code review of PR #1770 (QSL-580u)

## Summary

Head 37cc994a9, base qsl-580v-slot-bits (#1763). Checked against:
- QSpec FR-322 "Timed-model nodes": an `urgency` is exactly `kind`, plus `predicate` for `when`, or `model` and `operation_name` for `operation`;
- FR-322-AC-260 (the order key);
- FR-415;
- FR-230 and the new AC-53;
- TC-685 step 36;
- STD-526.

**Soundness holds.**
- `has_unwritten_members` still refuses guards, resets, invariants and `Clock` parameters. It now refuses an `urgent when` only when its predicate is not lowered.
- So no guard or invariant is written as true or as empty. A model whose only timed content is clocks and urgency emits exactly its places and urgencies.
- A predicate is written only when it has one alternative with no own-place presence and reads no `PlacePresence` or `SlotPresence`. Every other form refuses `TimedModelNotEmitted`. I probed `present(self.v)`, and a guard beside an urgency.
- Eval only gains a `..` in a pattern, so urgency semantics there are unchanged.
- Each urgency node is located at its own declaration. The `urgencies` list is sorted by span, which matches the AC-260 order key for one source. Its predicate's nodes are located at the predicate.

**Probes** (emit plus I2 read-back):
- `urgent T when { true }` reads back with no `self` parameter node.
- `urgent N when { not (self.versionNumber > 0) or self.versionNumber > 5 }` reads back.

**FND-001 (medium).** Two `urgent` declarations with equal content fail emission with `EmitRefusal::SourceMapInvariant`. S3 admits the model.
- Reproductions: `urgent A when { self.versionNumber > 0 }` with `urgent B when { self.versionNumber > 0 }`; or `urgent A O` with `urgent B O`.
- An `urgency` carries no name, so both declarations key to one node. The emitter then writes two source-map entries for one node id, from `urgency_regions` and the per-declaration push.
- Fix: decide the meaning and implement it. Either write one node located at both regions and listed once (FR-322 identity is by content), or refuse at S3 with a named cause. Then test it.
- An internal invariant must not be the outcome of a checked model.

**FND-002 (low).** An urgency predicate's site is keyed as `Origin::TypeDeclaration { name: "urgency <model> <i>" }`. It relies on a space so that no type matches.
- Anything that reads a `TypeDeclaration` origin as a type name will see a non-type. That includes diagnostics and any future origin resolver.
- A dedicated origin variant, or the existing expression origin with a site index, states the intent.

**Rust.**
- `ClauseDeclarations::at()` is a field-by-field copy. If the struct is `Copy`-able, deriving `Clone` would remove it.
- `typed_in` and `typed_alternative_in` keep the old entry points as thin wrappers. That is acceptable.

**Gates** at 37cc994a9 (`--all-features`, debug info off, QSPEC_DIR = the shared checkout at 161d231d):

| Suite | Result |
| --- | --- |
| qsl-package lib `clock_places timed` | 21 passed |
| qsl-semantics it `clock_places timed urgen` | 104 passed |
| qsl-semantics lib | 812 passed |
| qsl-eval `timed_subject` | 71 passed |
| clippy (3 packages) and fmt | exit 0 |
| `make check-spec-validation` | exit 0 (warnings pre-existing in other FRs) |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two `urgent` declarations with equal content (`when` with the same predicate, or the same operation) fail emission with `SourceMapInvariant`; S3 admits the model | FR-322 Timed-model nodes, FR-230-AC-53 |
| FND-002 | low | The urgency predicate site is a spaced pseudo-name under `Origin::TypeDeclaration`, which reads as a type to any origin consumer | FR-230 Urgency |

## Verdict

Changes requested (FND-001).

## Dispositions

Round 1, head 55ad00e89.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Equal-content urgencies are one node, listed once, with one `expression` occurrence per declaration (ordinals in declared order, each located at its own declaration); FR-230 Urgency and AC-53 state it. Probes: interleaved `P, Q, P` lists `[P, Q]` with P at both regions; `(x > 0)` merges with `x > 0`; `0 < x` and `x > 0` stay two nodes; mixed operation and `when` merge per kind; I2 reads each back |
| FND-002 | deferred | Coordinator disposition: the repo convention (Origin is in quire-semantic-value, six QSL sites use the same pattern, no consumer reads it as a type) |
