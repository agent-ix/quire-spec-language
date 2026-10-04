---
id: SR-1279
title: "Gap analysis of quire-spec-language LC1: FR-275 to FR-278 against their tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ddd162c7bf269ee9dfe4b8e3aec50341993c256b; git diff origin/main...HEAD (PR #624): FR-275-AC-1..5, FR-276-AC-1..5, FR-277-AC-1..2, FR-278-AC-1..3, TC-755..759 against qsl-replay/src/spine/lifecycle/tests.rs, qsl-foundation/src/diagnostic/stage.rs, tests/it/compile_command.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: reviews
---
# Gap analysis of quire-spec-language LC1

## Summary

Ticket: QSL-489 (LC1), PR #624. This is a manual check of each acceptance
criterion against its tests. Linear is locked, so scope comes from the PR's
contract (FR-275 to FR-278) and the FRs themselves.

Backed, with an oracle that would fail on the old behaviour:

- FR-275-AC-1, for the four operations and `execute`: the chain over
  `spine-compile.native` executes `seven` and gets 7.
- FR-275-AC-3, for those operations: two calls give equal outputs. The test
  compares parts, not whole values, because the stage types have no
  `PartialEq`.
- FR-275-AC-4, for `parse` through `package`: 600 deterministic byte
  mutations, with a check that some mutated units still compile.
- FR-276-AC-1, AC-3 and AC-5.
- FR-276-AC-2: done, at reduced size (see FND-002).
- FR-278-AC-1 to AC-3. The comparison with the `compile` command's bytes is
  in `tests/it/compile_command.rs`.

Mutation reasoning for the cross-thread tests:

- If `stage`'s post-run `tripped()` check is removed, the check returns
  `Refused(Check ...)` and the test fails on the match.
- If the check runs to completion, `charges < 20,000` fails.
- If the cancel handle is never polled, the test hangs (SR-1278 FND-006).

So none of these pass vacuously, except for FND-001 below.

Judged out of LC1's scope. Each has its own FR, which is "not yet
implemented":

- `format`, owned by FR-003.
- `inspect`, owned by FR-297.
- `render`, owned by FR-298.
- `analyze`, owned by FR-281, together with FR-276-AC-4.
- `monitor`, owned by FR-283.
- `replay` as an operation, owned by FR-098.
- `check_fences`, owned by FR-355.
- The public `execute` with its backend seam, owned by FR-279, FR-294 and
  FR-296.
- FR-277-AC-2. It needs ADR-030 D-1's recursion-free S3, which is the depth
  work FR-277 cites as QSL-381 (the B-series).

Linear is locked, so I could not name ticket ids for these slices. The rows
must name them (SR-1280 FND-001).

## Verdict

Changes requested. Four in-scope criteria are not met. The cancel test's
oracle cannot see the charge sites that SR-1278 FND-001 says are unpolled.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-276-AC-2's oracle, "the observer records at most one charge after the cancellation", counts only polled charges. `Cancel::observing`'s observer is called from inside `Cancel::poll`. So a charge site that never polls, such as the `Typer` node charge or the I1 and E4 meters (SR-1278 FND-001), does work after the cancel that the observer never sees. The fixture of 4,000 one-line declarations also has no large body, so the unpolled per-node work is never exercised. Fix: once those sites poll, add a case where the cancel lands inside one large function body, for example FR-277-AC-2's long sum at a size S1 accepts, and a case inside `select` over a large model package. | qsl-replay/src/spine/lifecycle/tests.rs:462-504; quire-exact/src/cancel.rs:113-131 |
| FND-002 | medium | FR-276-AC-2 asks for a generated source of 200,000 declarations. The test uses 4,000, because "S1's parse time grows with the square of the unit". The AC was not amended, and no ticket owns the claimed quadratic S1. The claim also conflicts with the repo's own release bench: parser/volume/admitted goes from 3.83 ms at 100 to 88.7 ms at 2,000, which is linear. Fix: measure S1 at 8,000 and 200,000 declarations in release. If it really is quadratic, raise a ticket and amend AC-2 with the reason and the ticket. If not, run the AC size, in release or `#[ignore]`d with a gate target. | qsl-replay/src/spine/lifecycle/tests.rs:451-460; spec/tests.md TC-757 row |
| FND-003 | medium | FR-275-AC-5 is not implemented: no per-stage work counters in an outcome's accounting, and no test. No other FR or slice owns it, and FR-275 is in LC1's contract, so it is in scope. `Staged` needs to carry each stage's counter. The test then asserts that `check` reports zero S1 and S2 work, and `package` and `execute` zero S1 to S4 work. | qsl-foundation/src/diagnostic/stage.rs; spec/tests.md TC-755 row |
| FND-004 | medium | FR-277-AC-1 is not implemented for the operations LC1 builds (`parse`, `select`, `check`, `package`, `execute`). The front end reports a reached limit inside `CompileRefusal`, never as `StageFailure::Limit(LimitExceeded)` naming the limits field. `refusal_or_fault` treats `StageFailure::Limit` as a broken invariant, which contradicts FR-277 Outputs. In scope for those operations. Fix: return `StageFailure::Limit` with the field name (for example `s3.nodes`) and test each field at the counter minus one and at the counter. | qsl-replay/src/spine/lifecycle.rs:110-130 |
| FND-005 | medium | FR-275-AC-2 requires a `compile_fail` doctest that passes package bytes to `execute`. There is none. `execute` is `pub(crate)`, so a doctest cannot reach it. The TC-755 row still says "steps 1 to 3 and 5's compile_fail doctests pass for `parse`, `select`, `check`, `package` and `execute`", which is false for `execute`. Fix: correct the row now. The doctest lands with the public `execute` (FR-279). | qsl-replay/src/spine/lifecycle.rs:26-72; spec/tests.md TC-755 row |
| FND-006 | medium | FR-278 `check` takes "the lock evidence (ADR-011 §2.4)". The signature has no lock-evidence input. The FR-278 row lists it as remaining with no owner. FR-278 is in LC1's contract, so either add the input or name the ticket that owns lock evidence. | qsl-replay/src/spine/lifecycle.rs:243-277; spec/spec.md FR-278 row |

## Dispositions

Round 1, reviewed at a47829387eae0445ada116f6594f599654187cf6 (fix commits
7116333d6, bfd81c9d9, a47829387 on ddd162c7). Owner rulings applied:

- The quadratic S1 parse is a QSL-482 B1 follow-up, and FR-276-AC-2's test
  stays at 4,000 declarations, with TC-757 carrying the remaining-work note.
- `dependencies.depth` goes to B4.
- `execute` stays crate-internal.
- The AC-2 oracle counts polls, which is acceptable once every charge site
  polls.

The coder's claim about the dispatch limits is true. `model.family_steps`
is read only by `build_family`, which only `link_dispatch` calls, and
`model.dispatch_candidates` is charged only in `link_dispatch`. The one
caller of `link_dispatch` is `checked_dispatch_operation`, which no spine
operation calls. The FR-277 and TC-758 rows state this honestly.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 7116333d6 |
| FND-002 | accepted-no-change | Owner ruling: the S1 parse is quadratic (release: 2,000 declarations 195 ms, 50,000 410.6 s), a defect owned by a QSL-482 B1 follow-up after A3c. The test stays at 4,000 declarations, and TC-757 records the measurements and keeps 200,000 as the target. |
| FND-003 | fixed | 7116333d6 |
| FND-004 | fixed | a47829387 |
| FND-005 | fixed | 7116333d6 |
| FND-006 | fixed | 7116333d6 |
