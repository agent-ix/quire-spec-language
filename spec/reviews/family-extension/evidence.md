---
id: SR-478
title: "Evidence analysis of ADR-012 semantic-family extension contracts"
type: SpecReview
analysis: evidence
scope: "spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/spec.md ADR-012 index row and contains edge"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
---
# SR-478: Evidence analysis of ADR-012

## Summary

Round 1. Reviewed commit 048deb3 on `task/210-family-extension`. ADR-012 is a
design record with no AC rows. Its obligations are the rules it states in
§1–§12, and the tickets it hands them to. This review asks, for each rule,
which verification method would show it holds, and whether the record names
that evidence and gives it an owner. It looks hardest at three places: §5.3
(exhaustive-seam tests), §7 (absent capability and solver absence) and §12
(change-set tests). It also checks that the record gives #212 evidence for
each gate scenario, and gives #213, #214 and #185 evidence they can each run
alone.

The design is sound, and nothing in it contradicts AD-016, QSpec PR #133 or
the fixed ownership. The `spec/spec.md` index row and the `contains` edge are
correct. The gaps are in the evidence.

- **§5.3** names the compile-failure test, but not how it works: how the
  test-only variant gets into the build, or what the test asserts.
- The "no `_` arm" rule has no check.
- S7 is a QSL seam, but no ticket owns its test.
- The S5–S8 tests wait on "when those seams are built", with no ticket and no
  carrier.
- **§7.3:** the absent-capability outcome is settled in CG, so neither #185's
  exit case nor #188/#189 closure can pass inside QSL alone.
- **§7.4:** solver absence names no test. The baseline shows the current
  behaviour is a panic.
- The #210 acceptance rules on registration order and ambient state have no
  evidence.
- **§12:** the "bounded change set" claim has no way to measure it.
- **#212:** the record gives evidence for only three of the seven gate
  scenarios.

Verdict: **ACCEPT WITH FINDINGS** (round 2, commit 8fb238b). Round 1 was
also ACCEPT WITH FINDINGS. The revision resolves most findings. FND-005
(solver-absence test) and FND-008 (§12 change-set measurement) stay open at
medium; see "Round 2".

## Method

1. `quoin advise` on the worktree returned no rows for ADR-012, because an
   ADR carries no AC obligations. So none of the method recommendations below
   comes from a catalog rule. Each one is **reviewer judgement**, matched to
   `quoin catalog methods` entries by the characteristic named in each
   finding:
   - `invariant` or `layering` → `compile-time-check` or
     `architecture-conformance`;
   - `ordering` → `property-based-testing` or `metamorphic-testing`;
   - `fault-tolerance` → `fault-injection`;
   - `state-machine` → `model-based-test-generation`;
   - `cross-repo-boundary` → `integration-testing` or `contract-testing`;
   - `stable-output` → `golden-approval-testing`.
2. Read ADR-012 in full, and its `spec/spec.md` index row and `contains`
   edge.
3. Read issues #210, #212, #185, #213, #214 and #229, and the exit criteria
   of #186–#198. ADR-012 §11 quotes #188 and #189. Both quotes match the
   issue bodies.
4. Read AD-016 at `quire-specification` `origin/main`: the per-arrow evidence
   rows, the Risks table, the heads drift checks 1–7 and the change
   scenarios.
5. Read the QSpec PR #133 diff: FR-290 and AD-010 name CG `negotiate_*` as
   the single negotiation point.
6. Read ADR-010 for the observed baseline: IT-010, the §4.3 dispatch sites
   and the §2.1 panic on a missing cargo-kani.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | §5.3 says "#214 adds one test-only variant and shows a compile failure at each of S1–S4". It does not say how. Today the test cannot be run. A permanent extra variant breaks every normal build. A `trybuild` fixture cannot add a variant to an enum in the real crate. And "the build fails" proves only that one seam fails, not all four. Once #209 splits crates, a failure in the upstream crate stops the downstream seams from compiling at all. **Fix:** state the mechanism and the oracle. The variant sits behind a cargo feature, for example `seam-probe`, on each S1–S4 enum. A test target builds each affected crate with the feature on. It asserts that the set of E0004 (non-exhaustive patterns) locations equals a checked-in list of the S1–S4 seam functions. That is `compile-time-check` evidenced by `golden-approval-testing`, and the normal build never enables the feature. | ADR-012 §5.1, §5.3; #214 acceptance bullet 2 |
| FND-002 | medium | Nothing checks §5.1's rule that "None of these `match` expressions may contain a `_` or catch-all arm". A compile-failure test only exercises the seams it already lists. A seam written with `_`, or a new seam missing from the list, still compiles, so it passes unnoticed. There is a second hole. Each S5, S6 and S8 enum is matched in a different crate from the one that defines it. If it is marked `#[non_exhaustive]`, Rust forces a `_` arm at that match. **Fix:** add two rules to §5.1. First, the seam modules deny `clippy::wildcard_enum_match_arm` and `clippy::match_wildcard_for_single_variants`, and the lint gate runs them (`static-quality`). Second, no S1–S8 enum is `#[non_exhaustive]`. §5.3 should name both checks as the evidence for the rule. This carries out AD-016's Risks control "Exhaustive matches with no `_` arm". | ADR-012 §5.1, §5.3; AD-016 Risks |
| FND-003 | medium | §5.3 hands the S5–S8 tests to "their owning repositories when those seams are built". That names no ticket and no carrier. Two gaps follow. First, S7 is a QSL seam: §5.1 lists its owner as "QSL (#213, #185)". But it falls in the "other repositories" sentence, and §14 gives its compile-failure test to no ticket. Second, a cross-repo seam fails only when the downstream repo bumps its pin, so a per-repo test cannot show the failure across the boundary. **Fix:** give S7's test to #185, which owns the registry advertisement check and the S7 arms. Name the IR, CG and RT tickets in §14 as the holders of the S5, S6 and S8 tests. Name the AD-016 heads drift checks as the cross-repo evidence: check 1 (clause/tag wire strings total both ways) and check 7 (every `catalog_code()` total). Remove "when those seams are built". | ADR-012 §5.1 S5–S8, §5.3, §14; AD-016 Current-head integration |
| FND-004 | medium | The absent-capability evidence path runs from QSL into CG, and the record assigns it as if it stayed inside QSL. Under §7.2 and §7.3, #185 computes only an empty candidate set. The settled `unsupported` with its warning comes from CG `negotiate_*` once it takes the candidate set, and §14 gives that change to a CG ticket "to be opened". Three consequences follow. #185's exit case, "a claim with no registrant settles `unsupported` with the warning", cannot pass inside QSL. §11's test ("waits on #185 … must settle the absent-capability outcome") leaves out the CG dependency. So #188 and #189 closure also waits on the CG ticket. And §3's "one absent-capability corpus case per capability the family requires" has no stated place to run. **Fix:** split the evidence. First, #185 carries a registry unit test: a (kind, mode) pair with no registrant gives an empty candidate set. Second, an integration test (`integration-testing`/`contract-testing`) runs in the QI heads workspace with CG at the new `negotiate_*` head. It asserts `unsupported`, the warning naming the kind and mode, no emitted artifact, and exactly one accounting record per `request_index`. §11 adds "and the CG `negotiate_*` ticket" to the #188 and #189 rows. §14 either names that CG ticket or records that #185's exit waits for it to be opened. None of this changes ownership. | ADR-012 §3, §7.2, §7.3, §11, §14; #185 Exit; #188, #189 Exit; PR #133 FR-290 |
| FND-005 | medium | §7.4 (solver absence) names no evidence, and the baseline shows the opposite of what it requires. ADR-010 §2.1 records that IT-010 panics at `tests/configversion_backends.rs:513` (`expect("cargo-kani 0.67.0 must be installed")`) when the tool is missing. That is neither a typed absence cause nor the #229 outcome. §7.4 also makes three negative claims that need a test: availability is probed "nowhere earlier", there is "no fallback to another backend", and there is "no downgrade to a weaker mode". **Fix:** name a `fault-injection` test for the executing adapter. Run it with cargo-kani absent from `PATH`, and again with the AD-016 pin mismatched. Assert that the #229 outcome names the backend, the expected tool pin and the claim. Assert that no other candidate is invoked and that no probe runs before the execute stage. State that the IT-010 `expect` path is replaced by this outcome, and name the ticket that replaces it (#217 or the CG replay ticket). | ADR-012 §7.4; ADR-010 §2.1, OBS-002; #213 acceptance (solver-absence constructor) |
| FND-006 | medium | #210 acceptance bullet 3 ("Dispatch does not depend on display text, string tags, registration order, or ambient global state") has design rules in §7.1 and §9, but no evidence. §7.1 claims that "Two registries built from the same descriptors in any order are equal and select identically". That is an ordering property, and no test is named for it. The ban on `static`, `OnceLock`, thread-locals and `inventory`/`linkme`/`ctor` is structural, and nothing checks it. The four §5.2 registry failure rows have no tests either. **Fix:** add three kinds of evidence under #185. First, a `property-based-testing` test: for any permutation of the same descriptors, the registries are equal and every candidate set is identical. Second, an `architecture-conformance` check: `cargo-deny` bans `inventory`, `linkme` and `ctor`, and a lint or grep gate finds no `static`/`OnceLock`/`thread_local!` in the registry module. Third, one `unit-testing` case for each §5.2 row: duplicate identity, unknown kind, empty candidates and ambiguous candidates. | ADR-012 §5.2, §7.1, §7.2; #210 acceptance bullet 3; #185 |
| FND-007 | medium | #212 asks, for each of its seven scenarios, for "tests/evidence". The Consequences section maps only scenarios 2, 3 and 7 to this record. Scenario 5 (an unsupported unbounded proof request) is decided here, in §1.1, §7.3 and the #189 row of §11, yet no test is named for it. The same goes for §1.1's rule that an unbounded requirement is never narrowed. Scenarios 1 and 6 use the `Value` seam (§4.3) and the replay hook (§8). Scenario 7's evidence claim, "A checked package is the same bytes whatever backends are registered" (§6), names no test either. **Fix:** add a table in Consequences mapping each scenario to the ADR section that serves it and to its evidence. Scenario 5: a registry whose only backend advertises (kind, `Bounded`) gets an unbounded request. That settles `unsupported` naming (kind, `Unbounded`). With a declared finite bound it settles `requires-bound`. Neither emits an oracle or a harness (AD-016 ledger test). Scenario 7: a `metamorphic-testing` test checks one source against an empty registry and against a registry with two backends, and expects byte-identical checked packages. An `architecture-conformance` check shows the checker module does not depend on the registry type. Scenarios 1 and 6: cite AD-016 change scenarios 1 and 6 and their tests. | ADR-012 Consequences, §1.1, §6, §7.3, §8; #212 Gate scenarios |
| FND-008 | medium | §12 states "A row outside the table is a defect in this design and reopens #210 at #212". It also says "No other family's module changes" (§12.1) and "No `Value`, `SumCase` or `TemporalTrace` module changes" (§12.2). No way of measuring either claim is named. As written, the tables are predictions that nothing checks. **Fix:** state evidence in two parts. At #212, record an `inspection` walk of each table row against the #209 module map. When #221 and #223 land, run an `architecture-conformance` check: the PR's changed-paths list must lie within the modules the table names. This is the same re-walk on landed code that AD-016 schedules for WP11. Name the ticket that runs the check. | ADR-012 §12; #210 acceptance bullet 1; AD-016 Change scenarios |
| FND-009 | low | The §12 test lists do not cover every stage their own tables change. §12.1 has an Evaluate row ("`case` arm selection by variant identity") and cites QSpec #115 as normative input. Its tests include no reference-evaluation test and no #115 vector. §12.2 has an Evaluate row (`frame_violation`/`unauthorized-change`) with no runtime frame-violation test. §12.1 also marks "S7 (explicit arm)" on the Requirements row, yet adds no capability kind, so no S7 arm is forced. **Fix:** add an evaluation test with the QSpec #115 vector to §12.1. Add a runtime `frame_violation` test for each of the three frame sets to §12.2. Change §12.1's S7 cell to "none". | ADR-012 §12.1, §12.2 |
| FND-010 | low | §4.2 calls the builder a "typestate", and also says "An out-of-order clause is a typed refusal raised by the transition it tried to take." Clauses arrive from source as data. So the refusal must come from a runtime transition function, not from the type system, and the evidence is a runtime test, not a compile-time one. §12.2 relies on "out-of-order clause refusal raised by the builder transition". **Fix:** state that the builder exposes a transition function `(state, ClauseForm) -> Result<state, OrderRefusal>`, derived from the §4.2 diagram. Verify it with `model-based-test-generation` over the diagram: every edge is accepted, and every (state, clause) pair without an edge gives the typed refusal. | ADR-012 §4.2, §12.2 |
| FND-011 | low | §9 says "After the edge, no code compares a string to choose behaviour" for twelve sites. It names no evidence that each site is closed. **Fix:** for each §9 row, record closure evidence at the landing revision in ADR-010's negative-evidence form (`absent: <pattern> in <path>`). Add a wire-string ↔ enum totality test with `cargo mutants` for each new edge enum, as AD-016 arrow 2 requires. | ADR-012 §9; ADR-010 Evidence convention; AD-016 Arrow 2 |
| FND-012 | low | §5.3 applies AD-016's `cargo mutants` requirement only to the S5–S8 conversions. S4 `catalog_code()` and the S1 family-prefix map are mapping functions too, and AD-016 requires a totality check of every `catalog_code()` against the copied catalog (heads check 7). **Fix:** extend the §5.3 sentence to S1 and S4. Name a totality test of each family `Cause` against the copied diagnostic catalog. | ADR-012 §2, §5.1 S1 and S4, §5.3; AD-016 Shared-type strategy, heads check 7 |
| FND-013 | low | §1.1 says a bounded result "reports the bound it held over … and is never dropped at a later stage". No test is named for that claim. **Fix:** name a round-trip test. The declared per-argument bound on a `proved` result survives from IR `KaniOutcome` to the FR-331 accounting record (AD-016 arrows 5 and 6). Pair it with the scenario 5 test from FND-007. | ADR-012 §1.1; AD-016 Arrows 5–6 |

## Evidence

- **Sources read:**
  - ADR-012 at `048deb3`;
  - `spec/spec.md` lines 39–40 (`contains` edge) and 388 (index row);
  - ADR-010, sections §2.1, §4.3, §4.4 and the Evidence convention;
  - AD-016 at `quire-specification` `origin/main`;
  - the QSpec PR #133 diff (FR-290 and AD-010);
  - issues #210, #212, #185, #213, #214 and #229;
  - the exit criteria of #186, #187, #188, #189, #191, #192 and #198.
- **Advisor:** `quoin advise` returned no row for ADR-012. The findings are
  reviewer judgement, and each is mapped to a named catalog method.
- **Quotes checked:** ADR-012 §11 quotes #188 ("with no backend registered it
  settles `unsupported` with the warning") and #189 ("a claim over an
  unbounded collection settles `unsupported` with the warning"). Both match
  the issue bodies.
- **Ownership:** no finding moves an owner. Each fix either names evidence
  within the owners §14 already lists (#214, #213, #185, and the IR, CG and
  RT tickets) or names the AD-016 heads workspace as the cross-repo carrier.

## Round 2

Reviewed ADR-012 at 8fb238b (diff from 048deb3). Round-1 verdict: ACCEPT WITH
FINDINGS.

| ID | Round-1 severity | Status | Note |
|----|------------------|--------|------|
| FND-001 | medium | partial | §5.3 states the mechanism and oracle: a probe variant behind the `seam-probe` feature, an `xtask seam-probe` that compares E0004 locations with a checked-in list, run in the full gate. Residue (low): once #209 splits crates, E0004 in the defining crate stops a downstream crate from compiling, so its seams are never reported. The record does not say how the probe reaches seams in a downstream crate. |
| FND-002 | medium | resolved | §5.1 denies `clippy::wildcard_enum_match_arm` and `clippy::match_wildcard_for_single_variants` in seam modules, and bans `#[non_exhaustive]` on S1–S9. |
| FND-003 | medium | partial | §14 gives the S7 seam probe to #185. Residue (low): §5.3 still says "The owners of S5–S9 build the same probe … when those seams exist", which contradicts §14 for S7 and names no IR, CG or RT carrier. The AD-016 heads drift checks 1 and 7 are not named as the cross-repository evidence. |
| FND-004 | medium | partial | §11 and §14 add the CG `negotiate_*` ticket to #185's exit, #188 and #189. §5.3 gives #185 one unit test per §5.2 row, including the empty candidate set. Residue (low): no integration test in the heads workspace is named for §7.3 (`unsupported` with the warning, no artifact, one record per `request_index`). §3's absent-capability corpus case still has no stated place to run. |
| FND-005 | medium | open | §7.4 still names no test. The ADR-010 §2.1 baseline (IT-010 panics through `expect("cargo-kani 0.67.0 must be installed")`) is not mentioned, and no ticket is named to replace it. The negative claims (no probe before routing, no fallback, no downgrade) have no evidence. Fix: in §7.4, name a `fault-injection` test with cargo-kani absent from `PATH` and with a mismatched pin. It asserts the #229 outcome naming backend, tool identity and claim, that no other candidate runs, and that no probe runs before routing. Name the ticket that replaces the IT-010 `expect` (#217 or the CG ticket) in §14. |
| FND-006 | medium | resolved | §5.3 Registry: permutation property test, `cargo-deny` ban on `inventory`, `linkme` and `ctor`, a lint gate for `static`, `OnceLock` and `thread_local!`, one unit test per §5.2 row. |
| FND-007 | medium | partial | Consequences maps scenarios 2, 3, 5 and 7 to evidence, including the scenario 5 `negotiate_*` tests and the scenario 7 metamorphic byte-identity test. Residue (low): scenarios 1 and 6 use §4.3 and §8 of this record but are not mapped, and the architecture check that the checker does not depend on the registry type is not named. |
| FND-008 | medium | open | §12 still says "A row outside the table is a defect" and "No other family's module changes", with no way to measure it. Fix: in §12's lead-in, name an `inspection` of each row against the #209 module map at #212, and an `architecture-conformance` changed-paths check on the #221 and #223 landing PRs (the WP11 re-walk AD-016 schedules), with its owning ticket. |
| FND-009 | low | partial | §12.1 adds `case` evaluation on each variant, §12.2 adds runtime `frame_violation` evaluation, and §12.1 drops S7. §12.1 still names no QSpec #115 vector. |
| FND-010 | low | resolved | §4.2 defines `accept(state, clause) -> (state, ClauseResult)` with an explicit arm per pair. §3 lists builder ordering tests. |
| FND-011 | low | resolved | §9 adds the `#[string_edge]` marker and a lint gate on string comparisons outside it. §3 lists wire totality tests. See new FND-014. |
| FND-012 | low | resolved | §5.3 extends mapping mutants to S1 and S4 `catalog_code()`. |
| FND-013 | low | resolved | Consequences scenario 5 names the bound round trip from IR `KaniOutcome` to the FR-331 record. |

New findings:

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-014 | low | The §9 `#[string_edge]` lint gate has no owner and no mechanism. Clippy has no such lint, so it needs a custom lint (for example dylint) or an `xtask` scan. "Every string comparison outside a marked function" also catches comparisons of user values that select no behaviour. Fix: in §14, give the gate to #214 for QSL. State that it flags string comparisons and matches on string literals in non-test code outside `#[string_edge]` functions, with an allow-list for value comparisons. | ADR-012 §9, §14 |

Round 2 verdict: ACCEPT WITH FINDINGS

## Author closure (after the PR review)

Every finding this record left open or partial has one closing line. "Fixed"
names the ADR-012 section in the commit that carries this section. "Routed"
names the owner that holds the remaining work.

| ID | Closure |
| --- | --- |
| FND-001 | Fixed: §5.3 states that every S1–S4 enum and match site is in the one QSL crate, so one probe build reports them all. The seams that cross repositories are probed where their enums are defined. |
| FND-003 | Fixed: §5.3 names the probe carrier for each of S5–S9, and cites AD-016 `make heads` drift checks 1 and 7 as the cross-repository evidence. |
| FND-004 | Fixed: §5.3 "Dispositions end to end" runs the #185 exit corpus, including §3's absent-capability case, in the test harness downstream of CG and asserts §7.3. |
| FND-005 | Fixed: §7.4 names the fault-injection test (tool missing, pin mismatch), asserts the ADR-013 QC-9 result, replaces the IT-010 `expect` and gives it to Codegen #86. |
| FND-007 | Fixed: the Consequences table maps scenarios 1 and 6 and names the ADR-011 layer check that `check` and the family modules do not depend on `route`. |
| FND-008 | Fixed: §12's lead-in names an inspection of each row against the #209 module map at #212, and a changed-paths check on the #187 and #218 landing PRs. Both §12 tables name exact modules or paths per row, including the RT and CG `ValueType` match sites. |
| FND-009 | Fixed: §12.1 tests include the QSpec #115 sum and `case` vectors (FR-143, FR-146). |
| FND-014 | Fixed: §9 scopes the `xtask string-edge` scan to non-test code with an allow-list for value comparisons. #214 owns it, and Contract IR #141, Codegen #86 and the RT ticket run it (§14.2). |

## PR review (QSL PR #234, delta 43677c9..10664aa)

The PR reviewer checked the author-closure lines above against ADR-012 at
10664aa. FND-001, FND-003, FND-004, FND-005, FND-007, FND-008, FND-009 and
FND-014 are confirmed fixed. For FND-008, every §12.1 and §12.2 row names a
module or path, and every RT, CG, IR and QSpec path exists at `origin/main`.
No new evidence finding. The open PR findings are in SR-474.
