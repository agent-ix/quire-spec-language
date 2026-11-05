---
id: SR-1683001
title: "Code and Rust review of temporal evidence successor tests (quire-spec-language PR 2337)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language PR 2337 @75ab30437; QSL-1683, QSL-1684; qsl-eval/tests/it/temporal_trace.rs; qsl-eval/tests/it/temporal_identity.rs; qsl-replay/src/spine/clause/tests/temporal.rs; qsl-package/src/emit/tests/temporal_clause.rs; tests/it/temporal_package_route.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-502
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-523
    type: reviews
---

## Summary

Tickets: QSL-1683, QSL-1684. One review pass over `git diff origin/main...HEAD`
(+758/-2, six files, tests and tags only) at head
75ab304370ad7f2f9182decf7c098e291e348771, gates run on the head trial-merged
with origin/main 19f33d8fb.

Gates (all green): cargo fmt check; `check --locked --workspace --all-targets`;
clippy `-D warnings` default and `--no-default-features`; qsl-eval `temporal`
tests 142 passed; qsl-replay `clause::tests::temporal` 106 passed; root
`--features checked-handoff` `temporal_package_route` 13 passed; qsl-package
`s3_` 4 passed. arch-lint was not run (no shipped source changed). Trial merge
with open PR 2309 and PR 2336: no textual conflict, `check --workspace
--all-targets` green and qsl-eval temporal tests 142 passed on each.

Mutations that the new tests kill (each reverted): package of the
authenticated binding replaced (FR-522-AC-9); `SamplePeriod` refusal labelled
`Epoch` (FR-522-AC-13); retained-instance member dropped from the identity
(FR-522-AC-22); progress-ledger member dropped (FR-522-AC-23); decision
scope closure dropped from the identity (FR-522-AC-17 and AC-19); profile and
clock bytes dropped from the authenticated member (FR-522-AC-16, 17, 19); a
nondeterministic meter-ceiling member (FR-522-AC-15, 17, 18 and FR-523-AC-11);
covered-positions capacity counted per binding (FR-502-AC-9); the identity
test of the covered position removed (FR-502-AC-10); a refused request
returning no ledger (FR-502-AC-11). Mutations the tests do not kill: dropping
the relation, the deliveries or the origins member of the identity
individually, and dropping trigger, relation, deliveries, origins, evicted,
anchor and correspondence together, leave all eight `temporal_identity` tests
green (FND-001).

The FR-327-AC-8..13 tests assert the AC values (Completed(true), Violated at
the named position) and the AC text matches them. Mutations in `plan.rs` (the
engine `temporal_trace` runs): `always` dual flipped fails AC-8; `release` dual
flipped fails AC-9 and AC-12; `historically` dual flipped (AC-11) and
`triggered` dual flipped (AC-12) leave every PR test green (FND-007). Flipping
the same duals in `timed.rs` changes nothing because `temporal_trace` does not
reach that engine.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-522-AC-17 is tagged but the test varies none of the evidence-request members the AC lists: trigger identity, relation, one delivery (identity, receipt, anchor, payload, guard, capture fact), delivery order, one origin, the anchor, one `evicted` record, each `correspondence` member; nor the profile and clock-name members (the PR body states only the last two). Removing the Relation or the Deliveries or the Origins member from `identity_of` individually, and removing Trigger, Relation, Deliveries, Origins, Evicted, Anchor and Correspondence together, leaves every `temporal_identity` test green. Each missing variation is a one-line edit of `EvidenceRequest` in the existing harness; add them, or remove FR-522-AC-17 from the tag and file the follow-on. | qsl-eval/tests/it/temporal_identity.rs:225-325; qsl-eval/src/temporal/evidence/call.rs:687-719 | correct-requirement-no-evidence |
| FND-002 | medium | FR-522-AC-13 is tagged for the refusal of AC-1 to AC-6, but the test never varies `timestamp_unit` over the timestamped-event clause (AC-6 second half; `env()` in the same harness builds that clause and `kinds()` already uses it) and never asserts that an `epoch` plus `unit` difference reports `Epoch` (AC-5), nor the asserted and expected identities of AC-1. The partial coverage is not stated in the PR body or the docstring. Add the two cases or narrow the tag. | qsl-eval/tests/it/temporal_identity.rs:127-190 | correct-requirement-no-evidence |
| FND-003 | low | FR-502-AC-11 is tagged for both halves; the test asserts only the supplied-ledger half. The no-ledger half cannot be reached: `run_clause` admits progress only when a ledger is supplied (`qsl-replay/src/spine/clause/temporal.rs:747`), and every FR-502 refusal needs one. Keep the tag only with the test docstring saying the second half is vacuous, and ask the spec lane to reword the AC. | qsl-replay/src/spine/clause/tests/temporal.rs:3295-3326; spec/functional/FR-502-reject-a-regressing-watermark-and-a-conflicting-completeness-revision-on-the-spine.md | wrong-requirement |
| FND-004 | low | FR-522-AC-14 names `Finite`, `Lasso` and `Model` traces; the test covers `Finite` and `Lasso` (the replay `TraceInput` has no `Model`; the model lasso lives in qsl-analyze) and adds a positive control. The `Model` part is not mentioned in the docstring. | qsl-replay/src/spine/clause/tests/temporal.rs:3329-3350 | correct-requirement-no-evidence |
| FND-005 | low | FR-523-AC-11 stops only the `positions` counter, over a clause (`env`, `eventually[0,5]`, `p` false) other than AC-3's, and the assertion can fail only if the call is stateful or nondeterministic (it fails under the nondeterministic-member mutation, passes under every other). Acceptable for a pure call, but the AC's AC-5 stop is not exercised per counter. | qsl-eval/tests/it/temporal_identity.rs:437-470 | correct-requirement-no-evidence |
| FND-006 | low | FR-522-AC-20 and FR-524-AC-23 tags are honest (identity equal over both routes; forward capture reference refused with no package, FR-337-AC-8's own test), but FR-522-AC-20 is exercised only with `--features checked-handoff`, and the docstring of the route test still names only FR-516. | tests/it/temporal_package_route.rs:430-436; qsl-package/src/emit/tests/temporal_clause.rs:1844-1849 | correct-requirement-no-evidence |
| FND-007 | medium | FR-327-AC-11 and the since/triggered half of FR-327-AC-12 cannot fail on their dual: with `Historically` mapped to the non-dual `Once` scan, or `Triggered` to the non-dual `Since` scan, `once_and_historically_read_back_from_each_activation` and `since_triggered_until_and_release_agree_with_their_duals` still pass, because at the AC's one position (position 0, where the window holds only a false atom) both readings are false, and the `since` side of AC-12 is `Violated(0)` either way. The release half of AC-12 does have a distinguishing second pair. Add a second assertion per operator at an activation where the dual differs (for example `historically[0,1] holds(c.level <= 0)` at position 1, and a `triggered` pair that holds at the first activation). | qsl-eval/tests/it/temporal_trace.rs:1411-1448; qsl-eval/src/temporal/plan.rs:202,222 | correct-requirement-no-evidence |
