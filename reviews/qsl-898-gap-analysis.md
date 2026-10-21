---
id: SR-12292
title: "Gap analysis of quire-spec-language PR #1172 (QSL-898) against FR-126, FR-128 and FR-338"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@02898aab2cbbda7b837407f805deb76b6f9d9063; PR #1172 diff origin/main...HEAD; FR-126-AC-6, FR-126-AC-13, FR-126-AC-16, FR-126-AC-17, FR-128-AC-6, FR-338-AC-3; TC-521, TC-525, TC-539"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
---
# Gap analysis of quire-spec-language PR #1172 (QSL-898)

## Summary

Ticket: QSL-898. Ticket text was read as data and its four items were
re-measured against the code. Plan completion: not assessed.

Per criterion:

- **FR-126-AC-6, empty-prefix row**: tested by
  `an_undefined_precondition_at_the_initial_state_is_an_empty_prefix_violation`
  (TC-521, FR-126-AC-6): empty prefix, no lasso, `DivisionByZero` at position
  0, initial value 2. Covered.
- **FR-126-AC-6, refused-contract row**: no test. Still open on QSL-898
  (FND-002).
- **FR-126-AC-13**: the guarded refusal test now uses a TP-4 claim. Covered.
- **FR-126-AC-16**: three tests cover all five sentences (TP-2 guarded `= 1`
  and `>= 2`; TP-1 guarded `>= 3` and `= 7` with an empty certificate;
  undefined guard at position 2 with locus `Activation`). Covered. The FR-126
  POR sentence has no test (FND-001).
- **FR-126-AC-17**: `a_violation_on_the_stutter_step_carries_the_stutter_marker`
  checks prefix 3, the marker, length 4 and a reproduction. Covered.
- **FR-338-AC-3**: the existing Counter stand-in test now matches the
  rewritten AC, but it is tagged `#[trace("TC-525")]` only (FND-003).
- **FR-128-AC-6, rewritten last sentence (TC-539 step 6)**: no test, and the
  code refuses it (FND-004).

Ticket items: item 1 is half done (empty-prefix row yes, refused-contract row
no). Item 2 is done for the engine (TP-1/2/3), but S4 still refuses every
`on each` clause, so there is no end-to-end test. Item 3 is done. Item 4 needs
no change here. The PR body lists the remainder under QSL-898, so the ticket
must stay open after merge.

## Verdict

No major finding. The engine work matches FR-126-AC-16/17, verified by probes
(see SR-12291). The open items are untested or unimplemented criteria that the
ticket still owns: one medium POR test gap, two medium carried-over rows, and
one low trace tag.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-126's new bullet says "A partial-order run SHALL include the condition's footprint in the atoms'", but no test covers it, and the test `Letters::activation_reads` returns the empty set. A reviewer probe confirmed the join works (`bump(a)` becomes visible only with `guarded: true` when the guard reads `x.v` and the atoms read nothing), so this is a missing test, not a bug. Add that test to `partial_order.rs`, and add the clause to an AC (AC-16 or a new one) so the matrix shows it. | qsl-analyze/src/model_check/ample.rs:93-100; qsl-analyze/tests/it/model_world.rs:492-494 |
| FND-002 | medium | The FR-126-AC-6 refused-contract row (`Undecided(UndecidedSuccessor)`) still has no fixture. The PR says none is reachable because S3 proves kernel refusals away and a family refusal "needs a population reference (not emitted) or a dispatch family". The second part says a dispatch-family fixture would reach it, not that nothing can (FR-120-AC-9's `test/orders` dispatch fixture already checks and admits). If the row really cannot be reached, the AC is untestable and should be restated. Otherwise build the dispatch-family fixture. Either way it stays on QSL-898. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:439; qsl-analyze/tests/it/model_check_safety.rs:12-16 |
| FND-003 | low | The FR-338-AC-3 test `an_undetermined_contract_or_a_failing_atom_is_a_bad_state` now carries the rewritten AC in its doc comment, and the header no longer says AC-3 is unverified. But it is tagged `#[trace("TC-525")]` only, while TC-525 says to tag `#[trace("TC-525", "FR-338-AC-n")]`. The matrix therefore does not credit FR-338-AC-3. Add `"FR-338-AC-3"`. | qsl-analyze/tests/it/closure_certificate.rs:427 |
| FND-004 | medium | The rewritten FR-128-AC-6 sentence (TC-539 step 6) says FR-126-AC-6's empty-prefix counterexample "reproduces through the undefined `pre CanInc`". No test covers it, and `resolve_item` refuses every authored-clause `UndefinedEvaluation` payload with `AuthoredUndefined` (not-yet-implemented), so it cannot pass today. This was already true of the `test/tallies` text, but the rewrite makes the fixture buildable without saying the replay side is not built. Note it in the ticket's remaining work, or in FR-128, until authored undefined replay lands. | qsl-replay/src/model_check/replay.rs:852-854; spec/functional/FR-128-replay-a-model-counterexample.md:193 |
