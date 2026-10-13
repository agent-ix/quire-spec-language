---
id: SR-8728
title: "QSL-746 code and Rust review of PR #810"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2ce03c67ef33f41288507b9b5fe411f2042d57ac; base impl/business-model-meanings; qsl-semantics/src/model/normalize.rs; qsl-semantics/tests/it/business_meanings.rs; qsl-semantics/tests/it/model_normalization.rs; qsl-semantics/tests/it/model_systems.rs; QSpec TC-195 N01, N02, N06, N10, N14-N18; QSpec FR-150-AC-1, FR-150-AC-6, FR-150-AC-8; QSpec model-complete phase 3 rule; QSpec value-accounting normalize.fact, redefinition-check and conflict-check rows"
review_set: subset
---
## Summary

Ticket: QSL-746. PR #810 at `2ce03c67ef33f41288507b9b5fe411f2042d57ac`, base `impl/business-model-meanings`. Code review with the Rust lane folded in. Reviewer-only: no edits and no builds. `cargo fmt --all -- --check` was run read-only and passed.

The source change is correct. The phase 3 member pass no longer gives an inherited member a qualify fact or a qualify charge. It records one inherit fact per ancestor path, with inputs `[path..., member]`. This matches the QSpec model-complete phase 3 row, which defines only inherit facts for `(T, m)`, and the published vectors n01-member-B-x, n02-member-B-x, n02-member-C-x and n02-member-D-x. In each vector the derivation is inherit facts only, starting at ordinal 0. The charge still comes before `work_step()`, so a denied fact does no work. Phase 4 redefine facts are still appended after the inherit facts: the operation member derivation is `[inherit, redefine]` and N06's `(D, B/x2)` is `[inherit, redefine]`. Neither the conflict-check `f(o)` nor the redefinition-check `m` changes, because `f(o)` counts type facts only and `m` counts effective members, not member facts. No other code reads member derivation facts. A repository grep finds `RULE_QUALIFY` and `.derivation` only in `key.rs` and `normalize.rs`. Nothing in qsl-package, qsl-eval or qsl-replay reads them.

Every changed expectation was recomputed from the specs rather than from observed output:

- N01: records 3, facts 5, cycle-check L=1, declarations 4 and hashes 6 make 19 work units. Hashed bytes are 352+517+477+560+335+2789 = 5030. The 18-unit run stops at the view hash with next 1. The 5029-byte run has consumed 2241 with next 2789. The 4-fact run has consumed 4 with next 5. All of these equal TC-195 N01.
- N02: 5 qualify plus 6 type inherit plus 4 member inherit make 15 facts. Work units are 5+15+8+8+10 = 46. Bytes are 5137+335+6640 = 12112. All equal TC-195 N02. (D, A/x) has two facts, matching n02-member-D-x.
- N10: the duplicate-path mutation now targets `/derivation/1`, which is exactly the published mutation.
- Each N06, operation and interleave fixture drops one unit per inherited member: N06 drops 5 (B/A.x, C/A.x, D/A.x, D/B.x2, D/C.x3), the single-redefiner fixture 2, the two operation fixtures 1 each, the operation diamond 5 and the field/operation interleave 5. The N06-resolved total re-derives to 8+30+8+11+18+13+13+2 = 103.
- In the lib test, the three fact charges are now B's inherit plus two member inherits, so (3,3) steps.

The published N01 prefixes 23664417, ca6ff932, 3729a6fa, 42cf1c2c, ce039e96 and dae94514 match the QSpec vectors file. Asserting only the prefix is acceptable here. TC-195 itself writes identities as their first eight hex digits. A 32-bit prefix cannot be satisfied by accident when a preimage changes. The full universe digests are asserted in `oqe_a_single_connected_component_hashes_to_the_pre_partition_digest`.

The systems end pair is unchanged at its owner: `pipe` keeps two qualify facts at `Sys`, as tested by `systems_members_have_effective_owner_entries`. Inherited, it has one inherit fact per path. The phase 3 rule gives one fact per path for each member, and N10 refuses two identical input paths, so grouping the end pair loses no information the spec requires.

The red log shows exactly the six intended failures (N01 x4, N02 x2), each for the expected reason. The final logs show clippy (workspace, all targets, all features, `-D warnings`) at exit 0, `it` with 607 passed, `lib` with 518 passed, xtask with 182 passed, and spec validation at exit 0. No fmt log exists; the read-only fmt check above covers it.

PR hygiene: the title has no ticket id, the body has `Refs QSL-746`, and there are no local paths or private text.

## Verdict

CONDITIONAL. The fix is correct and spec-derived. One medium test-oracle defect in a test whose expectations this PR changed, and three low documentation or idiom defects, should be fixed in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `n06_conflict_refusal_waits_for_every_phase4_charge_to_admit` sets `derivation_facts = 17` and says this exhausts at the last phase-2/3 fact, one short of the first phase-4 fact. `fixture_n06_conflict` has 19 phase-2/3 facts (7 qualify, 6 type inherit, 6 member inherit). The same test's 50-unit case pins 34 units through phase 3: 7 records + 8 cycle-check + 19 facts. So the denial lands inside phase 3, and the case does not test the phase-4 wait it claims to. The old value, 22, was equally wrong (24 facts then). The PR lowered the value by 5 instead of deriving it. Correct values: limit 19, consumed 19, next_charge 20. | qsl-semantics/tests/it/model_normalization.rs:4093-4113 |
| FND-002 | low | Stale numbers remain in the same test's doc and comments: "30 derivation_facts (22 through phase 2/3 ...) and 54 work_units (37 through phase 3 ...)" should be 27 (19+8) and 51 (34+5+8+4); "`derivation_facts = 22`" and "`16..=53`" are the old values; "`42..=49`" should be `40..=47`; the panic says "work_units=56", but the case runs at 51. | qsl-semantics/tests/it/model_normalization.rs:4071-4087, 4116, 4217 |
| FND-003 | low | The doc comment of `systems_members_inherit_with_ordered_duplicate_end_facts` still says that keeping only one connection end makes the assertion fail. After this change the inherited `pipe` derivation is the same whether the owner has one end or two, so that mutation now passes this test. Only `systems_members_have_effective_owner_entries` (pipe = 2 qualify facts) catches it. | qsl-semantics/tests/it/model_systems.rs:492-497 |
| FND-004 | low | The rewritten N02 charge doc still says "ten ten `(normalize.declaration, normalize.hash)` pairs, the universe and view". TC-195 N02 has eight declarations and ten hashes in total (eight declaration hashes plus the universe and the view). | qsl-semantics/tests/it/model_normalization.rs:2398-2399 |
| FND-005 | low | The explicit `match member_preimages.entry(key)` with `Occupied`/`Vacant` arms existed only to charge qualify facts on the vacant arm. Its vacant arm now has no side effect, so it is `.entry(key).or_insert_with(...)`, the form the direct-member loop a few lines above already uses. | qsl-semantics/src/model/normalize.rs:2840-2851 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | The rewritten revert-probe doc says every `work_units` case below 51 reports `Refused` once `build()` returns the phase-4 refusal directly. The `work_units = 24` case is denied at a phase-2/3 fact; phase 4 never runs, so that revert cannot change it, and it stays `Incomplete`. The claim holds only for the cases that reach phase 4 (`derivation_facts = 19`, `work_units` 40 and 50). | qsl-semantics/tests/it/model_normalization.rs:4110-4113 |
| FND-007 | low | The rewritten systems doc says dropping the ancestor-member grouping makes "the inherit facts and the fact-boundary assertion below" fail. The boundary case stops at the 21st fact, before any inherited member fact. Dropping the grouping only adds duplicate inherit facts after that point, so the 22nd charge is still a `normalize.fact` and `consumed == 21` still holds. Only the inherit-fact assertions catch the grouping mutation. The boundary catches the ancestor members being dropped entirely, as its own comment says. | qsl-semantics/tests/it/model_systems.rs:492-497 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4108116476401c3043e4325b24d7f4c3f46cedb6 |
| FND-002 | fixed | 4108116476401c3043e4325b24d7f4c3f46cedb6 |
| FND-003 | fixed | 4108116476401c3043e4325b24d7f4c3f46cedb6 |
| FND-004 | fixed | 4108116476401c3043e4325b24d7f4c3f46cedb6 |
| FND-005 | fixed | 4108116476401c3043e4325b24d7f4c3f46cedb6 |
