---
id: SR-16305
title: "Gap analysis of QSL-1016 function-value parity obligation identity (QSL PR #1386)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4ece0487943c6c9b60cd3132ee166b347f61780d; FR-484-AC-1..10 against qsl-replay/src/execute/tests/value_parity_identity.rs (TC-1252), FR-357-AC-1..5 TC-904 function tests in qsl-replay/src/execute/tests.rs, and the qsl-replay sources in the diff"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-484
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1252
    type: reviews
---
# SR-16305: Gap analysis of QSL-1016 function-value parity obligation identity (QSL PR #1386)

## Summary

Ticket: QSL-1016. Planless gap analysis over `git diff origin/main...4ece04879`. Plan
completion: not assessed. Each of FR-484-AC-1..10 has one `#[trace("TC-1252", "FR-484-AC-N")]`
test. All ten tests ran and passed. For each one, I checked what source change would make it
fail.

- AC-1, AC-2: exact bytes and digests. The vectors were recomputed independently and match. They
  fail on any encoder member or order change.
- AC-3: member sensitivity, `PositionOrder`, and difference from the operator-parity form.
  `position` sensitivity without a refusal is covered by AC-2's swap.
- AC-4: the identity is checked against a canonical text written independently from the
  package's node ids. `small` gets `None`.
- AC-5: `Agrees`/`Diverged`. `claim()` is checked member by member and against
  `ValueIdentity::sent`.
- AC-6: three wrong identities, plus a stray `y` binding that still refuses `Obligation`. That
  last case fails if the recompute moves after the argument join.
- AC-7: the identity is all-zero in every case, so the test fails if the occurrence check moves
  after the recompute or is removed. One of the three AC vectors is substituted (FND-001).
- AC-8: content identity on four outcomes, and occurrence-key inequality.
- AC-9: the packet reconstructs and replays. Round-trip coverage is partial (FND-002).
- AC-10: B 40 gives `PreCallIncomplete`, `NumericMagnitudeBits`, limit 40. Omitted B equals 4096.
  `grow` is admitted. A and H inequality is checked. The "differs only in B" check is weak
  (FND-003).

Reverse gap: no new code lacks an owning requirement. `ValueParitySite`,
`value_parity_preimage`, `check_identity`, `ValueClaim` and the `ValueIdentity` members are all
FR-484. The `measured_bytes` growth is FR-357's claim-size accounting. No stubs, no ignored tests
and no tautological assertions were found.

The PR's recorded AC-7 gap is not accurate. It says `inc`'s `x + 1` expression occurrence cannot
be read "because the package holds no requirement record". But `graph.occurrences()` lists every
occurrence whether or not it has a record. TC-444 already finds a record-less `x + 1` occurrence
that way, by region text (application_claim.rs:395-410).

## Verdict

**CONDITIONAL**: no untagged criterion and no high finding. Two medium test gaps (AC-7, AC-9) and
one low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-484-AC-7's `expression`-occurrence vector is replaced by (`inc` node, `expression`, 0) instead of the real `x + 1` application occurrence; the recorded reason (no requirement record) is wrong, since `graph.occurrences()` with a region filter finds it as TC-444 does | qsl-replay/src/execute/tests/value_parity_identity.rs:391 |
| FND-002 | medium | FR-484-AC-9 "every member round-trips exactly" is asserted for 7 of 13 packet members (no `package_id`, `source_digests`, `profile_selections`, `run_limits`, `replay_target`, `family_payload`); `profile_selections` is hard-coded `Vec::new()` rather than read from the lock; the replayed request reuses the retained wire's package members instead of the envelope's | qsl-replay/src/execute/tests/value_parity_identity.rs:509 |
| FND-003 | low | FR-484-AC-10 "its `claim()` differs from the first only in B" is asserted as `assert_ne!` only; a claim differing in any other member also passes | qsl-replay/src/execute/tests/value_parity_identity.rs:570 |

## Coverage

- Criteria examined: FR-484-AC-1..10. All are tagged, and all ran and passed at 4ece04879.
- Matrix method: `quire matrix --scope . --format tsv` (quire 0.36.2) did not finish within
  10 minutes while the machine was running gates, so the run was abandoned. The trace index is a
  grep fallback, declared as such. It found exactly one `#[trace("TC-1252", "FR-484-AC-N")]`
  per AC N = 1..10 in `qsl-replay/src/execute/tests/value_parity_identity.rs`, and
  `spec/test-cases/TC-1252-function-value-parity-obligation-identity.md` exists.
- FR-357-AC-1..5 TC-904 function tests were rebuilt on `call_site`-minted identities through
  `parity_request` / `minted_identity`. They pass.
- Semantic review (intent to test to code): performed for FR-484 as part of the reviewer brief.
- Plan completion: not assessed
