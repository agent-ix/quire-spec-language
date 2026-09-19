---
id: SR-486
title: "Evidence analysis of ADR-013 canonical type, package and conversion ownership"
type: SpecReview
analysis: evidence
scope: "spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-486: Evidence analysis of ADR-013

## Summary

Reviewed commit 660aa25 on `task/211-type-ownership`: ADR-013 and its
`spec/spec.md` index row (line 398). ADR-013 is a design record with no AC table.
So `quoin advise` gets no obligations from it: 0 of its rows are ADR-013. This
review therefore takes as obligations the rules the ADR decides, and checks the
evidence each one names:

- the identity rules (R-01 to R-10, §2);
- the conversions C-01 to C-16 (§4, "Test" column);
- outcome category preservation (O-16);
- version refusal (O-22, §5);
- the witness transcript invariant (O-25);
- replay staleness (O-26, C-13).

For each one, this review asks three questions. Does the named evidence exist? Does it test this conversion, or a neighbouring one? Is the method class right: test, analysis, compile-time check or inspection?

Some named evidence exists and fits:

- IR PR #139 `tc_042_*` for C-10: cover and unwinding playback refuse, and a transcript that is not trustworthy refuses.
- IR TC-048 for C-04 and for the v2 `package_id` check in O-02.
- QSpec TC-195 vectors for O-05 `EffectiveId`.
- QSpec node-identity vectors for the node kinds they cover.

Three obligations have no evidence that can discharge them. These are the
`high` findings:

- **C-01, C-03 and C-15** name AD-016 heads checks 1, 2 and 7 as their test. No
  heads workspace exists yet. §4 says each Test is supplied by the implementing
  ticket, but these checks are built by #215 and #226. ADR-013's own O-23 also
  says the current-head lane "never produces release evidence".
- **C-02** (`DeclarationKey` → `NodeKey` through `ModelOwner`) names TC-195 and
  the node-identity vectors. TC-195 tests effective-declaration identity, which
  is O-05, not O-04. No published node-identity vector uses a `ModelOwner`
  owner.
- **C-13, replay staleness**, names a "#231 stale-package test". #231's
  non-goals exclude replay execution, and the check is executor-side: recompile,
  then compare `package_id`.

Verdict: REVISE. There are three `high`, eight `medium` and five `low` findings. Every
`high` has a concrete fix that changes only the Test column or an owner
assignment. None needs the ownership decisions reworked.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | high | C-01, C-03 and C-15 name AD-016 heads checks 2, 1 and 7 as their Test, but no heads workspace exists: ADR-010 §6 records "QI heads workspace (absent)", and #209 decides who owns it (Q209-7). The §4 preamble says "Test names the evidence the implementing ticket supplies". #213 and #131 cannot supply a heads check, because §7 assigns heads checks to #215 and #226. Also, O-23 says the current-head lane "never produces release evidence", so a heads check cannot count as evidence for a conversion. **Fix:** name pinned, per-repository tests as the evidence. For C-01, QSpec TC-195 N04 and N05 through QSL intake, plus an intake test against a pinned FCD conformance fixture. For C-03, a QSL emitter golden test against the vendored `checked-package-v2/fixtures/positive-*.json`, with QSpec TC-233 as the producer-side oracle. For C-15, a unit test per layer that checks its `catalog_code()` against the vendored `native-diagnostics.md` revision. Keep heads checks 1, 2 and 7 as drift detection only, and say so in the Test cell. | ADR-013 §4 C-01, C-03, C-15; O-17; O-23 |
| FND-002 | high | C-02 names "QSpec node-identity vectors; TC-195". TC-195 (QSpec) checks model-normalization provenance: its vectors are `EffectiveId` digests from `model-effective-declaration-vectors.json`, which is the O-05 domain, and none mints a checked node id. Every one of the 13 owners in `node-identity-vectors.json` at QSpec 818f555 is `"kind":"definition"`. None is a `ModelOwner{identity, node}`, and no QSpec proposal file has a model-owned preimage vector. IR `tc_048_model_owners_join_sha256_jcs_domain_package_selections` tests the reader-side owner join with an IR-local fixture. It does not test the digest. So the one normalized identity that crosses from a domain package into the checked graph has no conformance vector, and two conformers could mint different `NodeKey`s. AD-016's Shared-type row made the same citation, and ADR-013 repeats it. **Fix:** add a QSpec change to §8 (or OQ-3) for model-owned node-identity vectors: a `ModelOwner` preimage, its JCS SHA-256, and a stale-owner mutation. Name those vectors as C-02's Test, and move the TC-195 citation to O-05. | ADR-013 §4 C-02; O-03; O-04; O-05 |
| FND-003 | high | Replay staleness (O-26, C-13) names "#231 stale-package test; #217 exemplar". #231's non-goals say "No backend invocation, Kani harness, … or replay execution is implemented here". The staleness check is executor-side: recompile the locked source, recompute `package_id`, require equality. So #231 can test that the typed request refuses an unknown version, but it cannot test staleness. #217's six required scenarios do not include a stale package. As written, the staleness rule has no owning test. **Fix:** assign the stale-package test to the executor owner. Either extend #217 with a seventh scenario, or name the QSL ticket that builds the executor entry (Q209-5). Name its vector: the same packet replayed after one meaning-affecting edit to the locked source refuses as stale, and one presentation-only edit (excluded from `identity_preimage`, FR-322-AC-8) does not. Keep #231 for request-type refusals. | ADR-013 O-26; §4 C-13; §7 |
| FND-004 | medium | C-06 names an "RT mapping test", and none exists. RT `src/observation.rs:26` defines `ClauseKind{Precondition, Postcondition, Invariant, Guard, Consequent}`, marked `#[non_exhaustive]`. IR `quire-contract-model/src/identity.rs:833` (at 417ec86) has `{Precondition, Postcondition, Invariant, Assertion, Case, Information}`. RT does not depend on `quire-contract-model`, and it has no mapping from IR. O-10 and C-06 call the map "total", but three IR kinds have no RT counterpart. `Information` is not executable, and `Case` would map to two RT kinds, `Guard` and `Consequent`. Totality cannot be tested until the target of each kind is decided. **Fix:** state the six-row map in O-10, or route it to #210. Name the test as an exhaustive table test in RT, with a compile-time check that the match has no `_` arm. | ADR-013 O-10; §4 C-06 |
| FND-005 | medium | Outcome category preservation (O-16, C-08, C-09) has no oracle for 5 of the 10 `KaniOutcomeKind`s. The refusal and internal-failure proof cells say "per the IR map", and that map is `OPEN — decided in WP9` in AD-016. C-09's Test, "IR `STD-003`/`TC-051` closure pattern", names a pattern to copy, not a test. IR TC-051 checks the output-mapping refusal registry, not `KaniOutcomeKind`. The table also puts `Unavailable` ("A required tool or dependency is unavailable", IR `src/kani/outcome.rs:19`) in the internal-failure category. #213's acceptance instead asks for a canonical structured outcome for solver absence, separate from internal failure. A category-preservation test written from this table would encode that conflict. **Fix:** give every one of the 10 kinds exactly one category, and put `Unavailable` with unsupported, not internal failure. Then name the C-09 evidence as an exhaustive table test in IR `src/kani/outcome.rs`, plus mutation testing on the map. Name the C-08 evidence as #213 adverse tests, one per O-16 row, including a timeout and a cancellation that keep their causes (FR-323-AC-3 is authored `Analysis` in QSpec, so no QSpec test covers it). | ADR-013 O-16; §4 C-08, C-09 |
| FND-006 | medium | O-22 version refusal names no evidence, and its wording conflicts with the evidence that exists. O-22 says a reader refuses another version "before reading any other member". IR `tests/checked_package_v2_reader.rs:97` (TC-048) tests the opposite order: the strict parse runs first, so a document with an unknown version and a duplicate member refuses as `DuplicateMember`. FR-331 also groups unknown-version, duplicate-key and non-canonical refusals together as "before consumption". Existing evidence per contract: IR TC-048 (v2) and QSpec TC-255 (FR-352-AC-2, AC-5). Nothing is named for FR-323 or FR-331 readers. **Fix:** reword O-22 to "before admitting any other member", after the strict canonical parse. Add a Test cell that names TC-048, TC-255, and the #231 reader tests for FR-323 and FR-331. | ADR-013 O-22; §5 |
| FND-007 | medium | C-11 names the "Seed counterexample vector", and AD-016 marks it "seed, not yet vendored". C-11 spans two owners: IR `decode` and CG reconstruction into kernel `Value`s. IR PR #139 `tc_042_witness_decode_round_trips_declared_schema` and the arity, width and comment refusal tests cover `decode`. Nothing covers CG's lossless `i64` → `Integer` widening, which depends on `quire-exact`, a crate that does not exist yet. **Fix:** split C-11 into IR decode (IR PR #139 `tc_042_witness_decode_*`) and CG widening. For CG widening, name a contract test that runs the seed `concrete_vals` `[[8,0,…],[224,3,…]]` to kernel `Integer` 8 and 992, plus the `i64::MIN` and `i64::MAX` boundaries. Name the ticket that vendors the vector (WP9). | ADR-013 §4 C-11; O-25 |
| FND-008 | medium | C-12 (packet → FR-323 request) is owned by CG. Its Test is "#231 round trip", and #231 is a QSL ticket whose scope covers only "Total QSL conversions". Nothing tests that CG's request carries every member of the O-25 packet, or that a packet missing a member is refused at reconstruction. The method should be `contract-testing`: the CG producer and the QSL reader, each checked against the FR-323 schema. **Fix:** name a CG test. The positive case must carry every O-25 member. The adverse cases drop one member at a time, and each must be refused at reconstruction. Keep the #231 round trip as the QSL-side half. | ADR-013 §4 C-12; O-25; O-26 |
| FND-009 | medium | R-05 and R-06 name no evidence. R-05 says no identity comes from display text or collection position, and R-06 says no name resolution happens after the check stage. #211's acceptance includes this rule. These are negative, whole-codebase properties, so one example test cannot discharge them. The right methods are `architecture-conformance`/`sast`, plus targeted adverse tests. **Fix:** name (a) a static check that no post-check module resolves by `&str` name (OQ-3(b) removes `CheckedPackage::call`'s `function: &str` as the selector), and (b) a #231 adverse test with a Kani `concrete_vals` row reordered against a fixed `WitnessBinding` schema. IR `tc_042_witness_decode_refuses_comment_disagreement` already partly covers (b). | ADR-013 §1 R-05, R-06; O-11 |
| FND-010 | medium | The typestate rules name no compile-time evidence. R-10 and O-15 say checked typestate has no public constructor and is never built from wire bytes. Their only named evidence is "Check refusals (O-17)", which tests diagnostics, not constructibility. #213's acceptance says invalid stage transitions are "unrepresentable through the public API". That is a `compile-time-check` obligation. **Fix:** name `compile_fail` doctests: build a `value::expression::CheckedPackage` outside the checker, and convert v2 bytes or a `protocol_artifact` read into it. Each must fail to compile. Name the same kind of test for `NodeKey` from foreign-domain bytes (O-04). | ADR-013 R-10; O-15; O-04 |
| FND-011 | medium | O-03's refusal of the `quire/native` pseudo-package names no test. Consequences makes it a merge condition for PR #200. The obligation cannot be discharged at merge without a named adverse test. **Fix:** name the PR #200 test. A `DeclarationKey{package: "quire/native", …}` input must refuse at intake with a catalog code. A native field type must round-trip as `ValueTypeRef::Native`. | ADR-013 O-03; Consequences |
| FND-012 | low | C-04 names "IR TC-217", but TC-217 is a QSpec test case (`TC-217-complete-contract-ir-lowering.md`). IR's own reader test is TC-048 (`spec/contract/TC-048-checked-package-v2-strict-reader.md`, `tests/checked_package_v2_reader.rs`), which mirrors QSpec TC-217 FR-322-AC-4, AC-8 and AC-10. **Fix:** "QSpec TC-217 via IR TC-048". | ADR-013 §4 C-04 |
| FND-013 | low | C-16 names "FR-201 vectors", and none exist. FR-201-AC-2 to AC-4 are authored `Analysis`, and AC-6 is `Test (TC-233)`, which covers the definition lock, not digest strings. The digest wire string is an untrusted-input parser, so `property-based-testing` or `fuzzing` fits. **Fix:** name a QSL `digest` test. It admits exact 64-char lowercase hex. It refuses uppercase hex, lengths 63 and 65, a prefixed form, and an unknown or absent domain (FR-201-AC-3). Equal bytes under two domains compare unequal (FR-201-AC-2). | ADR-013 §4 C-16; O-18 |
| FND-014 | low | Several evidence names are vague, or are predictions rather than tests. C-07's "QSpec complete-value vectors" is not one artifact. The v2 `literal` round trip is best tested with the `checked-package-v2` positive fixtures (TC-233) plus a property-based round trip over each `value_kind`. C-14's "AD-016 scenario 6" is a predicted change scenario, re-walked in WP11. Name the nested-span replay test and its owner (#213 builds the source map). Also name the O-12 test for "a tag naming no node refuses at replay". | ADR-013 §4 C-07, C-14; O-12 |
| FND-015 | low | The C-10 cell says "Keeps the transcript verbatim". O-25 says "a round trip preserves the transcript byte for byte". But IR `Witness::parse` keeps only the selected assertion block, trimmed: `src/kani/witness.rs:337` is `transcript: sub_block.trim().to_owned()`. The invariant holds for the stored transcript, not for Kani's raw output. No test is named for the byte-for-byte envelope round trip. **Fix:** in C-10, say "keeps the selected assertion block verbatim after trimming". Name a #231 `golden-approval-testing` round trip of the envelope that compares `transcript` bytes. | ADR-013 §4 C-10; O-25 |
| FND-016 | low | Two more rules name no evidence. O-13 says `Value` → finite harness domain happens only inside the declared domain, and R-07 says a conversion never narrows. Nothing tests either rule. O-23's revision literals are "checked equal to it by a test", but the test has no name, and heads check 4 is not built (FND-001). **Fix:** name a boundary test: a value just outside the declared domain returns `requires-bound` or a refusal. Name the QSL test that reads `Cargo.lock` and compares `ir_revision` and `STANDARD` (OBS-022). | ADR-013 R-07; O-13; O-23 |

## Method

- `quoin advise --json` in the worktree: 0 obligations come from ADR-013, which
  has no AC table. So every recommendation here is the reviewer's judgement,
  matched against the method catalog (`quoin catalog methods`). The catalog
  classes used are `compile-time-check`, `contract-testing`,
  `property-based-testing`, `fuzzing`, `golden-approval-testing`,
  `mutation-testing`, `architecture-conformance` and `sast`.
- ADR-013 was read at 660aa25, and AD-016 at QSpec `origin/main` 818f555,
  including the heads checks, change scenarios, seed vector and Shared-type
  table.
- Named QSpec evidence was checked with `git ls-tree` and `git show` at 818f555:
  - `node-identity-vectors.json`: owner kinds counted, 13 `definition` and 0
    `model`.
  - TC-195, TC-217, TC-233, TC-254 and TC-255.
  - The AC tables of FR-201, FR-321, FR-322, FR-323, FR-331, FR-351 and FR-352.
- IR was checked at PR #139 head 417ec86 (`gh pr view 139`: open,
  `task/137-witness-type`):
  - `src/kani/witness.rs:101-107,140-145,278-340`
  - `src/kani/outcome.rs:8-22`
  - `crates/quire-contract-model/src/identity.rs:833`
  - `tests/kani_replay.rs`: 20 `tc_042_*` tests
  - `tests/checked_package_v2_reader.rs:97,1024`
  - `spec/contract/TC-051`, `STD-003` and the TC-048 matrix rows
  - `git grep` finds no `cargo mutants` configuration.
- RT was checked at `origin/main`: `src/observation.rs:26`, and there is no IR
  `ClauseKind` import.
- FCD `conformance/` exists on `main` (read with `gh api`).
- The QI heads workspace is absent, per ADR-010 §6.
- Issue bodies for #211, #213, #217 and #231 were read with `gh issue view`.
- Nothing was built, run or committed. The only file written is this review.

## Round 2 (commit 0042691)

This round re-checks FND-001 to FND-016 against ADR-013 at 0042691: §4 (C-01 to
C-25), §1 evidence paragraph, O-22 Tests row, O-25 admission, §7 and §8. It
uses the same evidence clones as round 1. Also checked: IR PR #139 is still
open at head 417ec86, and CG #50 is open ("Canonicalize and replay every
backend counterexample through native execution").

| Round-1 ID | Severity | Status at 0042691 | Reason |
|---|---|---|---|
| FND-001 | high | resolved | C-01 names a #131 intake test over a pinned FCD fixture. C-03 names an emitter golden test against the v2 positive fixtures, with TC-233 as the reference. C-15 names a totality test in each layer against the vendored catalog. Heads check 4 appears only as drift detection (O-23), and heads checks 1, 2 and 7 are no longer named. The emitter has no ticket, and §7 and OQ-4 say so. |
| FND-002 | high | resolved | C-02 names model-owned node-identity vectors (QC-3, a QSpec change that blocks #213 S-2) and a #213 S-2 test against them. TC-195 moved to O-05. |
| FND-003 | high | resolved differently | C-13's evidence is now executor tests with three vectors: a meaning-affecting edit refuses by `package_id`, a presentation-only edit refuses by source digest, and a missing input refuses. The owner is not #217, as round 1 proposed. It is an OQ-4 ticket that does not exist yet, and §7 states that #231 excludes replay execution. The obligation is now assigned and blocked in the open, which is acceptable for a Layer 1 record. Presentation-only edits now refuse rather than pass, because O-26 compares `RawSourceRef` digests. That is consistent with the spans argument in O-26. |
| FND-004 | medium | unresolved in part (now low) | C-06 now names an RT test over all six IR kinds, and says each kind "maps to an RT kind or refuses with a typed cause". The record still does not say which RT kind `Assertion`, `Case` and `Information` map to, or which of them refuse. So the test has no fixed expected result. Refusing an admitted IR kind also sits awkwardly with R-07: a conversion is total over its admitted input. Carried forward as FND-017. |
| FND-005 | medium | resolved | Every one of the 10 kinds now has exactly one O-16 row. `Unavailable` maps to unsupported, and `Inconclusive` has its own category. C-09 names an IR test over all ten kinds, C-08 names one adverse test per evaluation row, and C-23 names a #231 test per proof row. OQ-3 (d) carries the WP9 amendment. |
| FND-006 | medium | resolved | O-22 now defers the order of the version refusal to each contract's own refusal order, and adds a Tests row: IR TC-048, QSpec TC-255, and a #231 unknown-version test for each envelope it reads. |
| FND-007 | medium | resolved | C-11 names the vendored seed vector and a CG widening test at `i64::MIN` and `i64::MAX` (CG #50). It also adds the rule that the join is by parameter node id. Who vendors the seed vector (WP9) is still not named. That is minor. |
| FND-008 | medium | resolved | C-12 names a CG contract test (CG #50) plus the #231 round trip of the request type. |
| FND-009 | medium | resolved differently | The §1 evidence paragraph names #213 adverse tests that change display text, diagnostic text and collection order, plus a static check in the #226 drift gate. The reordered-`concrete_vals` case is now covered by C-11's rule that the join is by parameter node id. |
| FND-010 | medium | resolved | §1 and O-15 name `compile_fail` tests on every public constructor path (#213). O-04 removes `NodeKey::from_bytes`. |
| FND-011 | medium | resolved | O-03, C-01 and §7 name a #131 adverse test for the `quire/native` refusal, which must land before PR #200 merges. |
| FND-012 | low | resolved | C-04 now reads "IR TC-048; QSpec TC-217". |
| FND-013 | low | unresolved in part | C-16 now names "Digest members of the v2 positive and negative fixtures (#213 S-2)". At QSpec 818f555, `fixtures/adverse.json` has only two digest mutations, both cross-domain (`cross-domain-package-id`, `cross-domain-source-digest`). No fixture has uppercase hex, a 63- or 65-character digest, a prefixed form or an absent domain. So the format refusals in O-18 have no case. Carried forward as FND-018. |
| FND-014 | low | resolved | C-07 names a round trip of every `value_kind` in the v2 positive fixtures and the complete-value vectors (#213 S-1). C-14 names a source-map lookup test (#213 S-4). The O-12 case "a tag naming no node refuses at replay" still has no named test. That is minor, and it is not carried forward. |
| FND-015 | low | resolved | C-10 now says "Stores the selected, trimmed assertion block", and names a #231 byte-for-byte envelope round trip. |
| FND-016 | low | resolved | C-22 names a CG adverse test with an out-of-domain value. O-23 and §7 assign the pin-equality tests to #215. |

Counts: 12 resolved, 2 resolved differently, 2 unresolved in part (both now
low).

New or carried-forward findings:

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-017 | low | Carried forward from FND-004. C-06 names an RT test, but no expected result. O-10 or C-06 should give the six-row map (IR kind → RT kind, or the typed refusal) so the RT test can check it. If a kind refuses, say why that input is not admitted under R-07. | ADR-013 O-10; §4 C-06; R-07 |
| FND-018 | low | Carried forward from FND-013. C-16's fixtures have no case for the format refusals: uppercase hex, wrong length, prefixed form, absent domain. Either name a #213 S-2 property test over those forms, or add a QSpec change (QC table) for the mutations. | ADR-013 §4 C-16; O-18 |
| FND-019 | medium | The new O-25 Admission row says a `Witness` "is admitted only through `parse`, including on deserialization", and that "a transcript that differs from its own selected block refuses". At IR PR #139 head 417ec86 (still the head), `Witness` has `#[derive(Deserialize)]` with a `pub transcript` field. Its own doc comment says `Deserialize` "bypasses [`Witness::parse`] and its structural validation entirely" (`src/kani/witness.rs:95-107`). The existing tests refuse an untrustworthy transcript at replay (`tc_042_replay_counterexample_refuses_witness_with_untrustworthy_transcript`), not when the witness is deserialized. The record adds a rule that the witness it adopts does not meet, and it names no test and no owner for the change. **Fix:** in O-25 or §7, name the IR PR #139 change: `Deserialize` goes through `parse` (for example `#[serde(try_from = "String")]`), and the field is no longer `pub`. Name its test: deserializing a cover transcript, an untrimmed transcript, and a two-block transcript each refuses. | ADR-013 O-25 Admission; §7 IR PR #139 |
| FND-020 | low | The new O-25 rule "the packet's `input` must equal that decode or reconstruction refuses" has no named test. IR `tc_042_replay_counterexample_marks_agreement_not_witness_backed_for_none` covers only the `witness: None` half. **Fix:** name a CG #50 adverse test in which a witness-backed packet has an `input` that differs from its decode. | ADR-013 O-25; §4 C-12 |

Round-2 verdict: ACCEPT WITH FINDINGS. All three round-1 `high` findings are
closed, and no `high` finding is open. One new `medium` (FND-019) and three
`low` findings remain. FND-019 should be fixed in O-25 or §7 before #231
starts, because #231 waits on IR PR #139 being merged at a recorded sha.

## Round 3 (commit 4152eb8)

PR #236 re-review of the delta 5609e3a..4152eb8, against ADR-011 at 22fa948
and ADR-012 at 10664aa. The full finding table is in
[base.md](base.md) Round 3. ADR-013 line numbers are at 4152eb8.

- The cited sibling text was checked at the current heads. Q209-5 matches
  ADR-011 FB-05. Q209-8 cites `102c8bb`, which is an ancestor of 22fa948. The
  pending-QC-14 note in ADR-011 E7 exists, and so does the E9 `ReplaySource`
  rule. ADR-012 §13.2 Q1 to Q4 resolve.
- The added ADR-010 §7.2 to §7.4 and §8 ticket dispositions cover every
  ticket that round 1 listed.

New findings:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| PR2-M2 | medium | `ReplaySource` changes the accepted AD-016 Packet row and arrow 6 output (`witness: Option<Witness>`), and no QC amends AD-016. ADR-011 E8 still uses the old field. | ADR-013 568, 806, 807, 863 · ADR-011@22fa948 236 |
| PR2-L1 | low | Context pins the sibling ADRs at f781e32 and 43677c9. The delta cites text that exists only at 22fa948 and 10664aa. | ADR-013 60-61 |
| PR2-L3 | low | Q209-2 cites ADR-011 §1 for `ResolvedSourcePackage`. ADR-011 §8 is the section that places it. | ADR-013 825 |

Round-3 verdict: CHANGES (see base.md).
