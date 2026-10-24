---
id: SR-16638
title: "Gap analysis of QSL-986 FR-154-AC-2/3/5, FR-160-AC-4 and FR-161-AC-4 coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #1432 (branch qsl986-state-space-gaps, head 4c4cfec5b); FR-154-AC-2, FR-154-AC-3, FR-154-AC-5, FR-160-AC-4, FR-161-AC-4; TC-572, TC-573, TC-584, TC-586"
review_set: subset
---

## Summary

Ticket: QSL-986. This was a planless gap analysis. Plan completion: not
assessed. The caller's brief asked for a clause-by-clause comparison, so the
semantic review ran for the five ACs in scope.

`quire matrix --scope . --format json` (quire 0.36.2) at head 4c4cfec5b gives
these binders:

- FR-154-AC-2 is `tagged`, bound to
  `model_population::decide_frame_scopes_a_modifies_entry_to_the_receiver`.
- FR-154-AC-3 is `tagged`, bound to
  `receiver_frames::the_frame_run_and_the_clause_admission_apply_the_receiver_scope`
  (qsl-replay).
- FR-154-AC-5 is `tagged`, bound to
  `receiver_frames::a_deletes_self_frame_grants_the_receiver_only_and_differs_from_deletes_type`
  (qsl-package).
- FR-160-AC-4 is `tagged`, bound to
  `quotient::a_symmetry_refutation_settles_as_the_unreduced_refutation_does`.
- FR-161-AC-4 is `tagged`, bound to
  `quotient::every_counterexample_is_concrete_and_replays_as_an_unreduced_one_does`.

Out of ticket scope, and unchanged: FR-154-AC-1, FR-160-AC-5 and FR-161-AC-5
stay `untagged`. The diff adds no doc comment that cites an untagged AC id.
Two pre-existing mentions are on main: `reduction.rs:1166` ("FR-160-AC-5's
run, without its replay") and a QSpec id in business_meanings.rs.

### FR-154-AC-2, clause by clause

| # | Clause | Asserted? |
| --- | --- | --- |
| 1 | `modifies self.value`, receiver `c1`, no args, pre `w` (c1, c2 at 0), post `c2.value` 1, refused `frame_violation`/`unauthorized-change` naming `c2` and `value` | yes: code, cause string, and `FrameFieldWriteOutsideGrant{object: c2, field: value}` |
| 2 | same post, receiver `c2`, gives a `FrameDelta` with empty `created` and `deleted` | yes |
| 3 | `modifies value`, both receivers give a `FrameDelta` | yes (and both are empty) |

The tag is complete. M1 (receiver guard dropped) kills it.

### FR-154-AC-3, clause by clause

| # | Clause | Asserted? |
| --- | --- | --- |
| 1 | the `Frame` run of `increment` on `self = c1`, post changes `c2.value`, under `modifies self.value`, settles `violation` and the witness names `c2.value` | yes: `FrameViolation`, `FieldWrite{c2, value}`, category Violation, exit 10 |
| 2 | under `modifies value` it settles `success` | yes: `Evaluate(Completed(Boolean(true)))`, category Success, exit 0 |
| 3 | the FR-106 clause-run admission of the same invocation refuses `frame_violation` under `modifies self.value` | yes: `Admit(Refused)` with code `FrameViolation`, cause `unauthorized-change` |

The tag is complete against the AC's text. `increment` is the fixture's
`own`/`all` pair. The AC's single vector cannot show that the admission path
grants the receiver: M5 survives (SR-16637 FND-003).

### FR-154-AC-5, clause by clause

| # | Clause | Asserted? |
| --- | --- | --- |
| 1 | `drop()` framed `deletes self`, receiver `c1`: a post without `c1` (declared deleted `c1`) gives a `FrameDelta` | yes: `deleted == {c1}`, `created` empty |
| 2 | a post without `c2` is refused `frame_violation`/`unauthorized-change` naming `c2` | yes: `FrameDeleteOutsideGrant{c2, Counter}` |
| 3 | framed `deletes Counter`, both posts and a post without either give a `FrameDelta` | yes: all three, with `deleted` checked |
| 4 | the S4 frame nodes of the two frames differ | yes: `frame_id` and `body` differ |

The tag is complete. Both effects are read back from the emitted package,
through admit and then `read_domain_package`. M2 kills it.

### FR-160-AC-4, clause by clause

| # | Clause | Asserted? |
| --- | --- | --- |
| 1 | §7.1 `fair weak whole` refutation from the symmetry run (and the run is reduced) | yes: product states are fewer than the unreduced run's |
| 2 | settles `refuted`, `decisive-counterexample`, after replay | yes, through `settle_model_check` with an FR-128 replay closure |
| 3 | its record names no reduction | yes: the whole `ModelCheckRecord` equals the unreduced run's, and `method` is `EXPLICIT_STATE` |
| 4 | the counterexample, three `upd(b)` from `(0,0,0)`, equals the unreduced run's byte for byte | by `Eq` over every member, because `TemporalCounterexample` has no encoded form (FND-002) |

The tag is complete. M3 kills it.

### FR-161-AC-4, clause by clause

| # | Clause | Asserted? |
| --- | --- | --- |
| 1 | FR-156-AC-3's partial-order counterexample is emitted unchanged and replays | replay yes; "unchanged" only as `prefix.len() == 6` and 7 positions, with no comparison to the search's path (FND-001) |
| 2 | FR-158-AC-1's constrained counterexample is emitted unchanged and replays | replay yes; equal to the unconstrained run's, but nothing shows the constraint was active, and M4 (constraint ignored) survives (FND-001) |
| 3 | no concretised counterexample in AC-1..AC-4 holds a canonical state, a permutation or a reduction name | yes, structurally: exhaustive destructuring of `TemporalCounterexample` and `ModelStep`, applied to all five counterexamples |
| 4 | each decodes as the same `TemporalCounterexample` type as an unreduced run's | by static type only. There is no serialize/decode form, so the ticket's round trip was not possible (FND-002) |
| 5 | concretising AC-1 twice gives byte-equal counterexamples | by `Eq` (FND-002) |

The tag is partial. Clauses 3–5 are met as far as the code allows. Clauses 1
and 2 assert replay, but the "unchanged" half is weak. The AC-2 fixture lists
two initial states: `b` holding, then `a` holding. That is the spec's two
states in the spec's order. The flag_world.rs and quotient.rs doc comments
still say "the AC names one initial state", which is stale but pre-existing
and outside this diff.

## Verdict

CONDITIONAL. All five ACs are tagged and none is untagged. Four tags are
complete against their AC text. FR-161-AC-4's tag is partial (FND-001), and
the "byte-equal"/"decodes" wording of FR-161-AC-4 and FR-160-AC-4 is not
testable as written in this codebase (FND-002). No reverse gap: the diff adds
test code only. No stubs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-161-AC-4 tag is partial: "emitted unchanged" is not shown for either counterexample — the constrained one equals the unconstrained run's but the run is not shown to be constrained (M4 survives), and the partial-order one is checked only by length 6 and replay | qsl-analyze/tests/it/quotient.rs:650-715 |
| FND-002 | low | FR-161-AC-4 ("decodes as the same TemporalCounterexample type", "byte-equal") and FR-160-AC-4 ("byte for byte") are untestable as written: TemporalCounterexample derives only Clone/Debug/Eq/PartialEq and has no encoded form, so the tests assert Eq and static type; the AC text should say Eq over every member, or a wire form should exist | spec/functional/FR-161-concretise-a-reduced-counterexample.md:106; spec/functional/FR-160-settle-reduced-verdicts-and-reduction-causes.md:143; qsl-replay/src/model_check/counterexample.rs:157 |

## Coverage

- ACs in scope: 5. Tagged: 5. Complete: 4. Partial: 1 (FR-161-AC-4).
- Semantic review: run for the five ACs in scope (caller-requested clause
  comparison).
- Plan completion: not assessed
