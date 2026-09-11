---
id: SR-352
title: "Gap analysis — native multi-unit producer handoff increment"
type: SpecReview
analysis: gap-analysis
scope: "examples/protocol-handoff/, examples/native_protocol_handoff.rs, spec/functional/FR-042-publish-compiled-protocol-artifacts.md, spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md, spec/"
review_set: subset
---

## Summary

QUOIN gap analysis of the multi-unit producer handoff increment on
`agent-a/native-ecosystem-handoff` against `origin/main` (`9c145bd`). The increment
introduces **no new unbacked matrix row and no new untraced code**; the verdict is
driven entirely by inherited corpus debt plus the deliberately unmet
FR-042-AC-10 B-side acceptance. Semantic review (step 4) was declined by the owner
and was not run.

**Updated 2026-09-11 (recheck):** correction `b34ab8c` rechecked against `549dd81`,
integrated at `485573b`. Coverage was re-run after the added example trace tags; the
rollup is unchanged. FND-004 is resolved as to execution. The resulting FND-006
trace-binding residual was subsequently resolved by local author verification,
distinct from Claude's recheck below.

## Verdict

**FAIL** — two Test Cases (TC-010, TC-115) carry no backing tagged test and
FR-042-AC-10 remains unbacked. All of it is pre-existing or deliberately deferred; the
increment, including its correction, adds nothing to the gap set. **This FAIL is not a
merge blocker** — it is deferred assurance disclosed in FR-042, TC-121 and both
READMEs, which the owner directive of 2026-09-09 tracks separately from implementation
delivery. See SR-351 for the merge disposition.

## Plan completion (step 1)

No plan bundle in `plan/` references FR-042 or TC-121, so this increment has **no
targeted plan bundle** and step 1 has no subject. Repository-wide state, recorded for
context only: 31 of 32 tasks `done`, `Task-020-qualify-boolean-backend` `in_progress`;
Plan-003/Plan-004 `active`, Plan-008/Plan-009 `in_progress`. None of these owns the
reviewed change.

## Findings

| ID      | Severity | Summary                                                                             | Refs                                                       | Escape Cause                    |
| ------- | -------- | ----------------------------------------------------------------------------------- | ---------------------------------------------------------- | ------------------------------- |
| FND-001 | high     | FR-042-AC-10 unbacked: B's public Rust consumer never accepts the emitted bytes       | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:208 | correct-requirement-no-evidence |
| FND-002 | high     | Inherited unbacked rows: TC-115/FR-036-AC-5/6/8, TC-010/NFR-005-M-1, FR-017-AC-2      | spec/model-linking/tests.md:107, spec/tests.md:45           | correct-requirement-no-evidence |
| FND-003 | medium   | StR-001 has 0 of 2 stakeholder validation criteria backed                             | spec/stakeholder/StR-001-native-assessment-trust.md         | correct-requirement-no-evidence |
| FND-004 | medium   | RESOLVED in b34ab8c — a named ignored release test now executes the recipe and passed  | examples/native_protocol_handoff.rs:29                      | correct-requirement-no-evidence |
| FND-005 | low      | OPEN — the reviewed increment has no owning plan bundle, so plan completion is unverifiable | plan/                                                  | missing-requirement             |
| FND-006 | low      | RESOLVED — local author verification confirms all five ids bind the actual example test | examples/native_protocol_handoff.rs:33                 | correct-requirement-no-evidence |

### FND-001 — the increment's own acceptance, still open

FR-042-AC-10 requires the emitted fixture to be "consumed unchanged by
quire-protocol's public Rust admission/linking interface". A's side is now
demonstrably complete for a four-unit subject; B's side does not exist in this
repository. This is correctly and repeatedly disclosed rather than papered over —
FR-042 now states "A's emission and local reader check do not satisfy FR-042-AC-10 by
themselves", TC-121 step 10 states "A's local emission/reader round trip alone leaves
this B acceptance incomplete", and the example README keeps AC-10 and IT-001 open. The
finding stays `high` because the criterion is unmet, not because the change misstates
it.

### FND-002 — inherited corpus debt, unchanged by this increment

`quire coverage --scope /home/peter/dev/worktrees/quire-language-native-ecosystem-handoff --json`
reports six unbacked rows, none touching FR-042, TC-121 or the example:
`TC-115` with `FR-036-AC-5/6/8` (composed native package linking), `TC-010` with
`NFR-005-M-1` (Manual verification), and `FR-017-AC-2` (Inspection verification). The
last two are `no_symbol_rows` — Manual/Inspection rows that cannot mint a symbol — and
are matrix-shape debt rather than missing tests. This is the same inherited set that
has driven every gap-analysis FAIL in this repository; it is not attributable to the
reviewed change.

### FND-003 — StR-001

Both stakeholder validation criteria under StR-001 are unbacked. Not enumerated in
`unbacked_rows` (the engine reports it only in the group rollup, 0/2), which is why it
is easy to miss; recorded here so the FAIL is not read as consisting solely of FND-001
and FND-002.

### FND-004 — code with no owning test (step 3, reverse gap) — resolved in b34ab8c

As recorded at `549dd81`, `examples/protocol-handoff/producer.rs` and
`examples/native_protocol_handoff.rs` were referenced by no test and no trace tag.
`b34ab8c` adds `stripped_release_producer_keeps_original_owners_and_compensations`,
which runs the real `producer::write` into a temp dir and asserts the four source
owners, six declaration owners and both compensation records; the root executed it at
the integrated head (1 passed, 0 failed, 0 ignored). The reverse gap is closed as to
execution. Its trace-binding residual, FND-006, is now also resolved as described below.

### FND-006 — execution evidence did not bind (low; resolved locally)

Claude's recheck found the test's five ids (`TC-121`, `FR-042-AC-1/4/6/7`) in
`unmatched_tags` because its fully qualified attribute did not match the configured
marker. Local author verification after that recheck confirms the corrected bare
`#[trace(...)]` binds the actual example test: coverage identifies its symbol under
`TC-121`, all five ids are backed and none is unmatched. The three remaining
unmatched tags are inherited `IT-004` tags. The criteria were already backed by
other tests, so the rollup remains unchanged. This is local author verification,
not an additional Claude recheck; see also SR-351 FND-006.

### FND-005 — no owning plan

The increment is not tracked by a plan bundle, so the skill's step-1 assertion has
nothing to assert against. Low severity under the owner's minimal-bookkeeping
directive, but it is why "is the plan done?" cannot be answered here.

## Coverage

`quire coverage --scope /home/peter/dev/worktrees/quire-language-native-ecosystem-handoff --json`
(quire 0.31.0, engine `ca7362d4`):

- Rollup **367/376 backed** (97.6%); 258 criteria, 83 property-shaped, 13 specific-shaped.
- Partially backed groups: FR-017 3/4, FR-036 5/8, **FR-042 9/10**, StR-001 0/2,
  `spec/model-linking/tests.md` 41/42, `spec/tests.md` 9/10.
- 0 status lies. 20 untracked symbols; 5 `oracle-resembles-implementation` suspicions in
  `tests/native_compensation_emission.rs` and `tests/native_protocol_emission.rs` — all
  pre-existing, all owned by the parent compensation/protocol campaigns and not
  re-reviewed here.
- Unmatched tags returned from 8 to **3** after FND-006's correction: only the
  inherited `IT-004` tags in `tests/fixture_audit.rs` remain.
- FR-042's nine backed criteria are backed by `#[trace("TC-121", ...)]` tests in
  `tests/native_compensation_emission.rs` and `tests/native_protocol_emission.rs`.
  FR-042-AC-1/4/6/7 additionally bind the actual example test after FND-006's correction.
  The one unbacked criterion is AC-10 (FND-001), whose verification row also names the
  external `quire-protocol IT-001`.
- Increment delta: **zero**, re-confirmed after the correction. Coverage was re-run at
  the integrated head following the added example trace tags and returned the identical
  367/376 rollup, the identical six unbacked rows and FR-042 still 9/10.
- **Default versus named release execution.** The added evidence runs only under the
  documented `cargo test --release --example native_protocol_handoff … -- --ignored`
  lane: examples are not test-built by `cargo test` (no `[[example]] test = true`) and
  the test also carries `#[ignore]`. `quire coverage` measures authored trace tags, not
  which lane executes them, so it would not have distinguished the two in any case.
- Step 4 (semantic intent↔test↔code review) was **not run** — the optional semantic
  extension was explicitly declined.
- Installed-module `DuplicateArchetype`/`DuplicateInverseEdge` warnings are advisory
  and did not affect the run.
