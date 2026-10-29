---
id: SR-487
title: "Risk and complexity review of ADR-013 canonical type, package and conversion ownership"
type: SpecReview
analysis: risk-complexity
scope: "spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-487: Risk and complexity review of ADR-013

## Summary

Reviewed commit: 660aa25 (branch `task/211-type-ownership`). The subject is
ADR-013 and its row in `spec/spec.md` (lines 167 and 398). Spot checks ran
against QSL `origin/main`, QSL PR #200 at head 9e59dde, IR PR #139 at head
417ec86, QSpec `origin/main` (AD-016, FR-323 and
`proposals/checked-package-v2/schema.json`), the RT and CG `Cargo.toml` on
`origin/main`, and the live text of issues #211, #213 and #231.

The ownership decisions hold up. Every object has one owner and a named
equality kind, and the lane-private rule keeps the native-v1 and composed
lanes from turning into compatibility bridges. The problems are in tasking and
in how the record depends on work that has not landed:

- #213 is named as the implementer on 23 of the 27 objects (O-01 to O-23). It
  also carries the `quire-exact` type move, which is cross-repository work.
  The §7 row for #213 lists a different, smaller set than the tables do.
  #213 cannot fit the epic's 1–3 session slices as written. This is FND-001,
  the one blocking finding.
- The kernel extraction has no owner for the RT and CG side. There is also a
  cross-repository revision cycle (FND-002).
- Three decisions depend on AD-016 amendments that QSpec has not approved
  (OQ-3). Nothing in the record says what #213 and #231 do until they land
  (FND-003).
- The replay path has two unowned or conflicting pieces: where the executor
  gets the source it must recompile (FND-004), and a conflict with #231's
  "no replay execution" non-goal (FND-005).

Verdict: REVISE (1 high, 7 medium, 2 low). FND-001 blocks. Its fix is local
to §3 and §7.

## Method

Each §3 object (O-01 to O-27), each §4 conversion (C-01 to C-16) and each
§7 consumer row was scored on technical risk and volatility. Drivers: an
unmerged PR or a pending QSpec amendment it depends on, a crate that does not
exist yet (`quire-exact`: no `crates/` directory and no `quire-exact` entry in
QSL `origin/main:Cargo.toml`), cross-repository pin coupling, and the size of
each implementing ticket compared with the epic's 1–3 session slice. I counted
ticket load by listing each `#### O-NN` section that names #213 or #231 and
compared that list with the §7 rows. I checked each witness and intake claim
at the cited PR head.

## Risk register

| Item | Tech risk | Volatility | Drivers | Mitigation named in the record? |
| --- | --- | --- | --- | --- |
| #213 scope (O-01 to O-23) | High | High | 23 objects plus kernel types in one ticket. Gated on #209 (Q209-2, Q209-4), #222 and #229 | No. FND-001 |
| O-13, O-16, O-21 `quire-exact` kernel | High | Medium | The crate does not exist. RT `exact` and CG oracles must move to it. Revision cycle QSL ⇄ CG/RT | Partly (Q209-4). FND-002 |
| O-05, O-11, O-25, O-26 | Medium | High | Each needs an OQ-3 amendment to accepted AD-016 (rows 238, 269, 295) | Raised as OQ-3, not gated. FND-003 |
| O-26, C-13 replay by recompilation | High | Medium | FR-323 `package` is only an FR-322 reference. The v2 lock names sources by `RawSourceRef`. No contract delivers the source bytes | No. FND-004 |
| O-24, O-26, C-13 tasking | Medium | Medium | #231's non-goal excludes replay execution. O-24's owner is IR, but a QSL ticket implements it | No. FND-005 |
| O-27 replay result | Medium | High | OQ-2 has not decided the record. The parity field is `OPEN — decided in WP9` | Raised, not sliced. FND-006 |
| O-25, C-10 witness | Medium | Medium | IR PR #139 is open. `Witness` has `pub transcript` plus `Deserialize`, so validation happens only when an accessor is called | Partly. FND-007 |
| §6 lane-private types (R-09) | Medium | Medium | They live next to canonical types with no enforcement and no deletion date (Q209-1) | No. FND-008 |
| O-01, O-03 intake | Low | Medium | PR #200 is open. FCD semantic IR 2.0.0 is an external contract | Yes. Stale line: FND-009 |
| O-17, O-22, O-23 codes, versions, pins | Low | Low | Rules to enforce, and the tests are named | Yes |
| O-08, O-19, O-20 frame, capability, proof mode | Low | High | The substance is deferred to #210, #222 and #229 | Yes (Q210-1, Q210-2). Legitimate deferral |
| O-09, O-10, O-14 clause, kind, types | Low | Low | Existing layer-owned types with total maps (C-05, C-06) | Yes |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | #213 is too large for one slice, and §7 does not match §3. Each of O-01 to O-23 names #213 as an implementing ticket, so 23 of the 27 objects land on it. That load includes the `quire-exact` type move (O-13), the single meter fold (O-21), the node-keyed source map (O-12), typestate (O-15), catalog codes (O-17), the digest record (O-18), the Capability type (O-19), the bound types (O-20, O-21), the QSL readers (O-22) and the removal of the revision literals (O-23). The §7 #213 row lists a smaller set. It leaves out O-01 (identity type), O-03, O-07, O-08, O-09 (clause node id), O-10, O-11, O-14, O-22 and O-23, even though the tables name #213 for each. It also gives O-03 to #131. The work waits on three unfinished decisions: Q209-2 and Q209-4 (#209), #222 (bounds) and #229 (capability). The epic wants slices of 1–3 focused sessions. As written, #213 cannot start until all of those land, and it cannot finish in one slice. Fix: (1) Make the §7 #213 row list exactly the objects the tables assign to #213, and remove O-03 from #213 in O-03 or in §7. (2) Split #213 in §7 into ordered slices, each with its upstream gate. S1: digest record, readers and pin literals (O-18, O-22, O-23), with no upstream gate. S2: kernel type move and meter fold (O-13, O-16 evaluation, O-21 meter), gated on Q209-4. S3: identities (O-02, O-04 to O-07, O-12), gated on S2. S4: typestate, clause kind, names and types (O-10, O-11, O-14, O-15), gated on Q209-2. S5: `catalog_code()` (O-17). S6: Capability and bounds (O-19 to O-21), gated on #229 and #222. | ADR-013 §3 O-01–O-23 "Implementing ticket", §7 #213 and #131 rows, §8 Q209-2, Q209-4; issue #213 |
| FND-002 | medium | The kernel extraction has no RT or CG owner, and it creates a revision cycle. O-13 says "QSL `value` and RT `exact` hold no separate kernel types" and that CG oracles consume the kernel. §7 names only #213 (types) and #209 (crate creation). No row owns removing RT `src/exact` kernel types or moving the CG oracles off RT `exact` (CG `Cargo.toml:18,27`). Until that is done, two kernels exist, against R-01, with no named end. The dependency graph also becomes a cycle across repositories. QSL dev-depends on CG (QSL `Cargo.toml:43`). CG depends on QSL (CG `Cargo.toml:28`, and as a normal edge for the executor, Q209-5). RT and CG would depend on `quire-exact` in the QSL repository. Every kernel change would then need QSL → RT/CG → QSL pin bumps in lockstep. Fix: add §7 rows for RT and CG, naming their kernel retarget tickets or routing them to #209. State the order: kernel crate at a QSL revision, then RT retarget, then CG retarget, then the QSL pin bump, with a heads run first (O-23). Add the revision cycle to Q209-4. | ADR-013 O-13, O-16, O-21, §7, §8 Q209-4, Q209-5; AD-016 Shared-type row (line 295) |
| FND-003 | medium | Decisions depend on AD-016 amendments that QSpec has not approved, and no gate stops implementation before they land. AD-016 is accepted. It lists five stored `Witness` fields (line 269), names `function: &str` as the executor selector (line 238), and says the kernel holds "exactly the types listed in this row" (line 295), which does not include `EffectiveId` or reference identities. O-25, O-11/O-26 and O-05 reverse all three and send them to OQ-3. #213 (O-05 kernel placement) and #231 (O-25, O-26) would be built against text that QSpec might still reject. Fix: in Status or Consequences, make O-05's kernel placement, the O-11/O-26 node-id selector and the O-25 stored-field rule conditional on OQ-3 landing in QSpec, and name the QSpec issue that carries it. List the OQ-3 merge as a precondition for #212 or for the #213 S2/S3 and #231 slices. Say which objects reopen if OQ-3 is rejected. | ADR-013 O-05, O-11, O-25, O-26, §8 OQ-3; AD-016 lines 238, 269, 295 |
| FND-004 | medium | Replay by recompilation depends on a source input that no contract carries. O-15 and O-26 require the executor to recompile "the locked source" and recompute `package_id` (C-13). FR-323 `package` is only "exact FR-322 checked-package reference" (FR-323 line 23). The v2 `PackageLock.sources` holds `RawSourceRef`s, which name sources by reference, not by content (checked-package-v2 `schema.json:28`). Recompiling also needs the selected FCD domain-package bytes (O-01). No owner, contract or store is named for getting those bytes to the executor. Either this is a QSpec change the record does not declare (beside OQ-2 and OQ-3), or it is an unowned local resolver, which R-02 forbids. #231's replay request cannot be finished without it. Fix: add an owner question (OQ-4) or a #209 question that says how the executor obtains the source and domain-package bytes named by the lock. Examples: a content-addressed source set carried in, or referenced by, the FR-323 request, or a named resolver owned by a stage. State that a missing or mismatched source refuses with `source_digest_mismatch` or `stale_dependency`. | ADR-013 O-15, O-26, C-13; QSpec FR-323 lines 22–24; checked-package-v2 `schema.json:28` |
| FND-005 | medium | The replay-execution tasking conflicts with #231. Issue #231's non-goals say "No … replay execution is implemented here." ADR-013 O-26 gives #231 the QSL executor side ("Request → execution (QSL): the executor recompiles …, recomputes `package_id` …, selects by node id and calls `CheckedPackage::call`"), and C-13 names "#231 stale-package test". O-24's owner is IR `KaniOutcome`, but its implementing ticket is #231, which is a QSL ticket. Fix: limit #231 in O-24, O-26 and §7 to the typed envelopes and the reader-side refusals: request, result, round trips, and version and stale-identity refusal at decode. Assign executor recompilation and selection (C-13) to #217, or to the ticket #209 names under Q209-5. Change O-24's implementing ticket to IR (#137 or its successor), with #231 carrying the result unchanged. | ADR-013 O-24, O-26, C-13, §7 #231 row; issue #231 Non-goals |
| FND-006 | medium | OQ-2 blocks the replay-result part of #231, and #231 is not split. O-27's serialized authority is open: OQ-2 asks whether FR-323 `results` or FR-352 `native-run-result/2` is the record, and the parity carrier is `OPEN — decided in WP9` (AD-016 line 242). #231 as one ticket cannot finish until QSpec answers. Fix: in §7, split #231 into three parts: (a) the proof-result and counterexample envelopes (O-24, O-25), which can start after #213 S3; (b) the replay request (O-26), after FND-004; (c) the replay result (O-27), gated on OQ-2 and WP9. | ADR-013 O-27, §7 #231 row, §8 OQ-2; AD-016 line 242 |
| FND-007 | medium | The witness decision depends on an open IR PR, and the named type is only checked when it is read. At IR PR #139 head 417ec86, `Witness` is `#[derive(Serialize, Deserialize)]` with `pub transcript: String` (`src/kani/witness.rs:99-108`). That means a `Witness` holding a cover, unwinding or malformed block can be built by deserializing or by struct literal, without calling `parse`. The "cover and unwinding playback refuse" rule in O-25 and C-10 runs only in `parse` and when an accessor is called. The O-25 envelope invariant ("cannot disagree with its own backend evidence") holds, but a packet can still carry a witness that is invalid, and it is refused only when first read. That may be after the packet has crossed a boundary. If PR #139 changes before it merges, the adopted fact (Context, "Witness fact") goes stale. Fix: in O-25, require the #231 envelope reader (and the IR packet reader) to validate the transcript when it decodes, with the same allow-list as `parse`, and to refuse before admission. Say that O-25 is re-checked against IR #139's merged head. | ADR-013 Context "Witness fact", O-25, C-10; IR PR #139@417ec86 `src/kani/witness.rs:99-108,140,167-180` |
| FND-008 | medium | Lane-private types sit next to canonical ones with nothing to enforce R-09 and no end date. R-09 says a lane-private type is "consumed by no new family, stage or boundary", but no check, visibility rule or gate enforces it. Deletion waits on Q209-1, which has no date. Because canonical and lane-private types have near-identical names (`DeclarationKey` ×2, `CheckedPackage` ×2 and more), new code can import the wrong one, and a review would miss it. §6 also lists `state::input::CanonicalDigest` as lane-private "until #213 folds it", while O-18 says #213 folds it into the canonical record. R-09 forbids converting a lane-private type to canonical, so both cannot be true. Fix: name how R-09 is enforced, for example a #226 drift check or a #209 module-visibility rule that refuses imports of §6 paths from complete-V1 modules. Take `CanonicalDigest` out of §6 and record it in O-18 as a type #213 replaces. | ADR-013 R-09, §6, O-18, §8 Q209-1 |
| FND-009 | low | The PR #200 consequence has already been met, and the record cites no PR #200 head. Consequences says "PR #200 must encode native references as `ValueTypeRef::Native` before it merges (O-03)". PR #200 at head 9e59dde already defines `ValueTypeRef{Native(NativeValueType), Package(DeclarationKey)}`, with a doc comment that rules out the "quire/native" pseudo-package (`src/model/domain_package.rs:131-143`). O-03 and O-01 cite PR #200 with no sha, so the claim cannot be re-checked against a fixed head. Fix: cite PR #200@9e59dde in O-01 and O-03. Reword the consequence as a condition to re-check at merge, not as outstanding work. | ADR-013 O-01, O-03, Consequences; QSL PR #200@9e59dde `src/model/domain_package.rs:131-143` |
| FND-010 | low | The `spec/spec.md` index row is correct. Line 398 reads "Proposed; canonical type, package and conversion ownership (#211)", which matches Status, and line 167 adds the relationship. Risk-wise, the Status line ties acceptance to #212, but gives no revision condition if a sibling Layer 1 decision (#209, #210) contradicts a §3 cell. Fix: optional. Add to Status that a #209 or #210 decision that contradicts a §3 cell reopens that cell. | `spec/spec.md:167,398`; ADR-013 Status |

## Top hazards

1. FND-001: #213's size and gating. This is the critical path for all of Layer 2.
2. FND-002: kernel extraction across three repositories, with a revision cycle.
3. FND-004 and FND-005: the replay input that no one owns, and the conflict
   with #231's non-goal.
4. FND-003: OQ-3 amendments that QSpec has not approved, under decisions that
   will be built.

## Failure-domain gaps

No `spec/reviews/type-ownership/failure-domain.md` exists at 660aa25. The
identity and purity gaps this review overlaps with are FND-007 (a witness that
is checked only when read) and FND-004 (where replay sources come from). A
failure-domain review should look at both.

## Round 2 (commit 0042691)

Reviewed ADR-013 at 0042691. I read the full revised record and compared it
with 660aa25. I re-checked IR PR #139 (head still 417ec86, still open) and QSL
`origin/main:src/value/expression/mod.rs:18`, where the evaluator imports
`value::accounting::Meter`.

| Round-1 ID | Status | Reason |
| --- | --- | --- |
| FND-001 | resolved | The §7 #213 row now points to "all of O-01 to O-23 that name #213". Every table cell names a slice: O-23 moves to #215, O-01 and O-03 are split between #131 and S-2. The six slices S-1 to S-6 carry the objects I checked cell by cell, and each slice has its gate. The ticket split is marked as an owner action. The kernel slice's scope and gate have a leftover problem: FND-011. |
| FND-002 | resolved differently | The record names no RT or CG owner. It lists RT and CG kernel adoption (WP5a, WP5b) as work no ticket owns and routes it to OQ-4. Q209-4 now asks how RT and CG pin a kernel in the QSL repository without a pin-bump cycle. Until OQ-4 is answered, the two kernels coexist. The record now states that openly and gives the fix a named owner question. |
| FND-003 | resolved | Context says the five differing cells wait for their QSpec amendment. OQ-3 gains (d) and (e) and a reopen rule. The §7 gates tie S-1 to OQ-3 (c) and #231 to OQ-3 (a). |
| FND-004 | resolved | QC-1 adds a digest-addressed byte provision to FR-323. O-26 says the executor never reads a path or search location and refuses when an input is missing or its digest differs. The rejected alternative is recorded. |
| FND-005 | resolved differently | #231 says "No replay execution". The executor entry (C-13) is work no ticket owns (OQ-4), with a recommendation to put it in #214, rather than going to #217. O-24 gives the IR map to IR with no ticket, and #231 keeps only the QSL-side reader (C-23). |
| FND-006 | resolved differently | #231 is not split into tickets. Its §7 gate says the result half also waits on OQ-2, and QC-7 carries the parity field. That is enough for the owner to split #231 during tasking. |
| FND-007 | resolved | The O-25 Admission row requires admission only through `parse`, including on deserialization. #231 waits for #139 to merge at a recorded sha. Who implements that rule on the IR side is not assigned: FND-012. |
| FND-008 | resolved | R-09 names the #226 drift gate as its enforcement, and §7 gives #215/#226 the R-09 and R-06 static checks. `CanonicalDigest` and `ByteDigest` are out of §6 and fold into O-18 under S-2. |
| FND-009 | resolved | Consequences and §7 cite PR #200 at `9e59dde`. The outstanding work is now #131's adverse test for the pseudo-package refusal. |
| FND-010 | resolved | Status says a #209, #210, #222 or #229 decision that contradicts a §3 cell reopens that cell only. |

New findings from the revision:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | S-1's gate and kernel scope put all of #213 on the slowest dependency, and they split the kernel across slices. All of S-2 to S-6 wait on S-1, and S-1 waits on OQ-3 (c), which is a QSpec amendment. So all of #213 waits on QSpec. Yet OQ-3 (c) only concerns adding `EffectiveId` and the reference identities to the kernel, and that belongs to O-05, which is in S-2. S-1 also moves `NodeKey`, values and outcomes, but not `Meter`, `ChargePoint`, `Incomplete`, `Origin`/`Location`, `BoundedInteger`, `CardinalityBound` or `BoundViolation`. AD-016's kernel row (line 295) lists all of these as kernel types. Kernel `Value` operations use them: `CheckedPackage::call` takes `&mut Meter`, and the evaluator imports `value::accounting::Meter` (QSL `src/value/expression/mod.rs:18`). O-21 puts the "single meter" in S-6, gated on #222 acceptance. Read literally, S-1 builds a kernel whose operations depend on types still in QSL `value`, or it pulls them in without saying so. Moving the whole kernel row is also the largest single move in #213, so S-1 is the slice most likely to exceed 1–3 sessions. Fix: make S-1 exactly the AD-016 kernel row (including `Meter`, `ChargePoint`, `Incomplete`, `Origin`/`Location` and the bound value types), gated on Q209-4 only. Move the `EffectiveId` and reference-identity kernel additions to S-2 with O-05, gated on OQ-3 (c). Limit S-6's O-21 item to the `model::accounting` fold and the #222 bound types. If S-1 still looks larger than 3 sessions, say where it splits (crate and type move; then retargeting QSL consumers). | ADR-013 §7 slice table (S-1, S-2, S-6), O-05, O-13, O-21, OQ-3 (c); AD-016 line 295; QSL `src/value/expression/mod.rs:18` |
| FND-012 | low | Nobody owns the witness admission rule on the IR side. O-25 requires `Witness` to be admitted only through `parse`, including on deserialization. At IR PR #139 head 417ec86 (still open), `Witness` derives `Deserialize` with a `pub transcript` field (`src/kani/witness.rs:99-108`). The §7 #231 gate asks only that #139 merge "at a recorded sha". The OQ-4 list of unowned IR work names the packet members, the WP9 map and reader codes, but not this rule. #231 could start against a merged `Witness` that still admits any transcript. Fix: make the #231 gate "#139 merged with admission through `parse` on deserialization", or add the admission rule to the OQ-4 IR list. | ADR-013 O-25 Admission row, §7 #231 row, OQ-4; IR PR #139@417ec86 `src/kani/witness.rs:99-108` |

Round-2 verdict: ACCEPT WITH FINDINGS (0 high, 1 medium, 1 low). No high finding remains. FND-001 is resolved; FND-011 is the leftover slicing problem and does not block.

## Round 3 (commit 4152eb8)

PR #236 re-review of the delta 5609e3a..4152eb8, against ADR-011 at 22fa948
and ADR-012 at 10664aa. The full finding table is in
[base.md](base.md) Round 3. ADR-013 line numbers are at 4152eb8.

- The S-1 slice is now bounded: exactly the kernel row plus the QC-15
  component types. `CatalogCode` and the category type move to S-5.
- The top remaining risk is PR2-H1. If the kernel enum and sum shape keeps a
  `NodeKey`, CG harness generation and RT evaluation need a public minting
  path. That undoes the single-minter rule that the #213 identity work
  depends on.

New findings:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| PR2-H1 | high | See base.md. The `NodeKey` minting is unenforced, and the RT and CG `NodeKey` source is unspecified. | ADR-013 166, 314, 654, 676 |
| PR2-L4 | low | QC-15 widened to six named types after OQ-3 was accepted. It needs the owner's confirmation before TK-10 is filed. | ADR-013 814, 863, 885 |

Round-3 verdict: CHANGES (PR2-H1).
