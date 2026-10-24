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

## New findings (disposition pass 1)

Reviewed at `1adffb12578105879d525d7df60e49edb58a66af`. The new `lock_profile_selections` helper
reads each lock row as `{"profile", "value"}`. The emitted v2 lock writes its `profile_selections`
rows as `{"role", "definition": {"authority", "identity"}}`, one per temporal-profile or
protocol-profile law (qsl-package/src/emit.rs:776-795, asserted at
qsl-package/src/emit/tests/temporal_clause.rs:197). `ReplayRequest::decode` admits a request
profile selection only when its `profile` is `quire.profile.v1` (`KNOWN_SEMANTIC_PROFILES`,
qsl-replay/src/request.rs:423, checked at :848). The lock's role-keyed rows cannot be carried
into the request under any `ProfileSelection { profile, value }` mapping without that check
refusing `UnknownSemanticProfile`. The replay itself never compares the request's profile
selections with the recompiled lock. TC-1252's unit has no temporal or protocol clause, so its
lock list is empty and the helper never reads a row.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | FR-484 "Retained claim inputs" says the request's profile selections are the v2 lock's `profile_selections`; those rows are role-keyed clause profiles (`{role, definition}`), while the request admits only `quire.profile.v1` semantic-profile selections, so a value-parity claim over any package with a temporal or protocol clause cannot be built by FR-484's rule without refusing `UnknownSemanticProfile` (spec contradiction; needs a ruling on what the request's selections are) | spec/functional/FR-484-identify-a-function-value-parity-claim-and-retain-its-replay-inputs.md:104 |
| FND-005 | medium | `lock_profile_selections` reads lock rows by guessed keys `profile`/`value` that the emitted lock does not have; it passes only because TC-1252's lock is empty, and would panic on the first real row, so AC-9's "profile_selections = the lock's" is still not exercised | qsl-replay/src/execute/tests/value_parity_identity.rs:501 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 1adffb125 |
| FND-002 | fixed | 1adffb125 |
| FND-003 | fixed | 1adffb125 |
| FND-004 | fixed | Spec ruling STD-443 (#1387, 40077eaa0, on main and merged into this branch): FR-484's "Retained claim inputs" now fixes the request's and the packet's `profile_selections` to the empty list "for every package", states that the lock's role-keyed clause-profile rows are no semantic profile selection and bind into the claim through `package_id` alone, and adds "SHALL NOT read the v2 lock's `profile_selections`". The contradiction is gone. At 8e9ee1f30 the code agrees: no producer path reads a lock row, and the AC-9 packet carries `Vec::new()`. The SHALL this ruling adds (refuse a non-empty list `UnusedProfileSelections`, after the target check and before the recompile) and FR-484-AC-11..13 (untagged in `quire matrix` at 8e9ee1f30) are owned by stacked PR #1390 (QSL-1016b, SR-16310/SR-16311 PASS at 3aa38f6ce). They are not this PR's scope |
| FND-005 | fixed | 94597e8d0: `lock_profile_selections` and its guessed `profile`/`value` keys are deleted. `assert_lock_selects_no_profile` asserts the lock list is `[]`, and the packet carries `Vec::<ProfileSelection>::new()`, which is AC-9's post-STD-443 text (`profile_selections` = `[]`). Residual: the helper's doc comment still says "pending FR-484 profile-selection ruling". #1390 (adf231b70) already rewrites it to cite STD-443 |

## Disposition pass 2 notes

Reviewed at `8e9ee1f3034be2366fb220900511e37c0e24e823`. Changes since 1adffb125: 94597e8d0 (FND-005), four `origin/main` merges (latest 5a4d93ca3), and 8e9ee1f30, which repoints `SuppliedLimits` from `qsl_semantics::value::supplied` to `quire_semantic_value::supplied_stop` in scalar.rs, outcome.rs, proof_result.rs and value_parity_identity.rs. That is the same type that `request.rs:35` and `supplied_limits.rs:5` already use on main. The PR's patch to call_site.rs, execute.rs, parity_identity.rs, value_parity.rs, identity.rs, lib.rs and execute/tests.rs is unchanged from 1adffb125. The one merge-resolution hunk (value_parity.rs imports) keeps main's removal of `SuppliedRun` and the PR's `CheckedPackage` import. `quire matrix` at 8e9ee1f30: FR-484-AC-1..10 are tagged (TC-1252), and AC-11..13 are untagged (they belong to #1390). Gates in an isolated target dir: `cargo test -p qsl-replay --lib` 826 passed / 0 failed; `cargo clippy -p qsl-replay --all-targets -D warnings` is clean; `cargo fmt --check` is clean. No new findings.

Stack note for #1390 (not a finding on #1386): #1390 is based on qsl-1016-impl before the main merges (adf231b70 does not contain 8e9ee1f30). It still names `qsl_semantics::value::supplied::SuppliedLimits` in the same four files. Before #1390 merges, it must take its new base and re-run its gates.
