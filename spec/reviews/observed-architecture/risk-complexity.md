---
id: SR-463
title: "Risk and complexity review of ADR-010 observed architecture baseline"
type: SpecReview
analysis: risk-complexity
scope: "spec/decisions/ADR-010-observed-architecture-baseline.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-010
    type: reviews
---
# SR-463: Risk and complexity review of ADR-010

## Summary

Reviewed commit: faa1731 (branch `task/206-observed-architecture`), ADR-010 and
its `spec/spec.md` index row. Adapted to a descriptive baseline: the review asks
whether the record surfaces the hotspots Layer 1 (#209, #210, #211) needs,
whether any high-risk fact is missing or mis-stated, whether the record will go
stale in a misleading way, and whether owner assignments are balanced and
plausible. Spot checks ran against QSL de627b5, IR 553b6d1, CG a4b2a73 and
CG 5e2a6a9 (the QSL dev pin), plus live `gh` state for PRs #200, #204, #228 and
issue #207.

The record is strong. Counts reconcile: 14 QSL-side AD-016 claims (5/1/8), 32
downstream claims (24/2/6), 17 DA items (13/4), 36 findings (13/6/17) and 68 open
issues (confirmed live). The `Diagnostic` embedding (`QSL:diagnostic.rs:306-324`)
and the `ir_revision` literal (`QSL:package/view.rs:39-46`) check out. It flags
the main hotspots: the `Diagnostic` SCC, `ir::ValueType`, IR definedness
inversion, the single-writer file `normalize.rs`, pin staleness, and the one
test-only proof path.

One blocking defect remains. The record says QSL has no dependency on the
Contract Runtime (RT), but the only proof path compiles and runs RT at a pinned
revision (FND-001). Two more gaps weaken the proof-path hotspot: the record does
not say whether that path ever runs (FND-002), and it does not say that the
path's IR, RT and CG revisions differ from the production pins. Other risk-side
gaps: PR-sourced evidence is unpinned and will change as the next three merges
land, writer contention is understated, and the owner load leans heavily on #211,
with some owners that look wrong.

Verdict: REJECT (one blocking finding, FND-001; the fix is small and local to
§3.2, §2.1 and OBS-002).

## Risk register (decision items with elevated risk or volatility)

| Item | Tech risk | Volatility | Drivers | Mitigation named in the record? |
|---|---|---|---|---|
| OBS-002 / A9–A11 | High | Medium | Only proof-and-replay path. It is test-only, needs an installed pinned Kani and offline cargo, and runs on CG, IR and RT revisions that differ from production. | Partly. Environment and revision gaps: FND-001, FND-002 |
| OBS-028 / OBS-030 / OBS-036 | High | High | Replay executor is disputed in the specs (IR FR-031 vs AD-016). Every CG replay call site uses a stub. | Yes, routed to #209 |
| OBS-016 / `Diagnostic` | Medium | Medium | Fan-in 20 type embeds `quire_contract_ir::Diagnostic`, so every IR pin bump reaches every module | Yes, hotspot row |
| `src/model/{normalize,conformance,refusal}.rs` | Medium | High | Three open PRs write these files, and the merge order is disputed | Partly. FND-003, FND-004 |
| OBS-006, OBS-014, X2 | Low | High | PR #200 is expected to merge soon and will change what these items describe | No. FND-005 |
| OBS-022 / OBS-034 / DA-14 | Medium | High | Five QSL revisions in use, hard-coded revision literals, pins 5–144 commits behind | Yes. Partly incomplete: FND-011 |
| #211 owner load | n/a | n/a | 32 of 54 decision items go to one Layer 1 ticket, and it feeds #213 | No. FND-006 |

## Top hazards

1. The proof spine: IT-010 runs on CG 5e2a6a9, IR root 04eb6f8 and RT 8a4d02b.
   None of these is on a production edge. §3.2 says QSL has no RT dependency.
   (FND-001, FND-002)
2. Replay-executor conflict (OBS-028, OBS-036): the specs disagree and every
   call site uses a stub.
3. Model-file writer contention across PRs #200, #204 and #228, with two merge
   orders on record (FND-003, FND-004).
4. #211 load concentration, which is on the critical path to #213 (FND-006).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Dependency direction is mis-stated for the only proof path. §3.2 gives `QSL → RT: none`, and the mermaid shows `QSL -.-x no dep RT`. But IT-010 writes a Kani crate that depends on `quire-contract-runtime` at CG 5e2a6a9's `RUNTIME_REVISION` 8a4d02b and runs it (`QSL:tests/configversion_backends.rs:807-841`, `CG@5e2a6a9:src/oracle.rs:17`). QSL's fixture `tests/fixtures/native-lowering/Cargo.toml:14` also pins RT 8a4d02b, which §6.3 already lists, so the record contradicts itself. The QSL `Cargo.lock` also shows CG 5e2a6a9 resolving to IR root 04eb6f8, a different crate from the production pin, model 53cc03c. So the proof path shares no IR, RT or CG revision with production. This breaks the #206 criterion "dependency direction ... explicit" on the hotspot #209 must rule on. Fix: in §3.2 and its diagram, add `QSL (tests) → RT 8a4d02b` (generated Kani crate and native-lowering fixture, test-time only). In OBS-002 and §2.1 A9, state that the proof path's IR (04eb6f8), CG (5e2a6a9) and RT (8a4d02b) revisions are all separate from the production IR pin 53cc03c. | ADR-010 §3.2, §6.3, §2.1 A9, OBS-002 |
| FND-002 | medium | The record never says whether the only proof path is executed. IT-010 is not `#[ignore]`d. It `expect`s `cargo-kani 0.67.0` on PATH with an exact sha256 (`QSL:tests/configversion_backends.rs:502-526`), and it runs Kani with `CARGO_NET_OFFLINE=true` against an RT git revision that must already be in the cargo cache (:820-838). QSL CI is `workflow_dispatch` only and installs no cargo-kani (`QSL:.github/workflows/ci.yml:3-26`). So the one proof-and-replay path has no automated gate and fails in a default environment. Layer 1 could read "1 path, test-only" as "1 path, exercised". Fix: in §2.1 A9 and OBS-002, record these environment preconditions and the fact that no CI job runs IT-010. Route the gap as a #209 input, with #217 as consumer. | ADR-010 §2.1, OBS-002, Summary counts row "Proof-and-replay paths" |
| FND-003 | medium | Writer contention is understated. The §4.5 hotspot row names only `src/model/normalize.rs`. Live PR file lists show #200 and #204 also both write `src/model/conformance.rs` and `src/model/refusal.rs` (the other two single-writer files in §8). #204 also writes `src/model/checked_dispatch.rs`, the evidence site for OBS-007 and §2.3, where it is named as the only non-test `Expression` producer. Fix: widen the §4.5 row to all three single-writer files with the PRs that touch each one. Note that #204 changes the OBS-007 evidence site. | ADR-010 §4.5, §8, OBS-007 |
| FND-004 | medium | The §8 merge order contradicts Decision 4 and the §8 table. Decision 4 makes the ARCH-01 comment on #207 the authority for WIP dispositions. The §8 table Notes follow #207 (#228 first, #204 second). The prose then says a coordinator order #204 → #228 → #200 governs, and cites nothing. A reader cannot tell which order holds for the hottest file in the repo. Fix: cite the coordinator decision (comment URL), then either amend Decision 4 to name it as the authority for merge order, or make the table Notes match the governing order. | ADR-010 Decision 4, §8 |
| FND-005 | medium | PR-sourced evidence is unpinned and will change soon. PR #200, #204, #228 and IR PR #139 are cited with no head sha (for example "PR #200 adds it (2104 lines)"). #207 recorded #204's head as 8d0a939. The live head is already 6eee1f3. OBS-006, OBS-014 (the `"allocation"` site), X2, §1 rows 3–4, §4.3 row 1 and the §8 notes all describe state that these "keep" PRs change, and those PRs are next to merge. Once they merge, main-state claims in the record become false while it still reads as current. Fix: pin every PR citation to a head sha (#200@13b6687, #204@6eee1f3, #228@a43e951, IR #139@64982f1). Tag PR-sensitive items (for example "PR-SENSITIVE: #200"). State in Consequences that #208 re-checks the tagged items after each of these merges. | ADR-010 §1, §2.6 X2, §4.3, §8, OBS-006, OBS-014, OBS-027 |
| FND-006 | medium | Owner load is unbalanced. #211 owns 17 of 36 findings and 15 of 17 DA items: 32 of 54 decision items (59%). #209 owns 13 and #210 owns 8 (including L1-D1 and DA-11). #213 (Layer 2) consumes 8 of #211's DA items, which puts #211 on the critical path. The record does not flag this. Fix: state the load in §9. Either split #211's items into named decision groups (identity and typestate DA-01..04, DA-08, DA-17; values, types and kernel DA-05..07, DA-16, OBS-032; versions, pins and vendoring DA-14, OBS-022..024, OBS-034), or move the items that are about dependency direction to #209 (see FND-007). | ADR-010 §9.2 owner tally, §9.3, §7.1 #213 |
| FND-007 | medium | Some owners look wrong. OBS-031 (no current-head integration lane in QI) goes to #211. But the #207 ARCH-01 classification, which Decision 4 names as authority, says QI PR #2 waits on #209, "which decides whether quire-integration owns the current-head integration lane". #209's scope is "legal dependency direction". OBS-034 (five QSL revisions in use, pins trailing targets) is also about cross-repo dependency direction, but names no #209 input. OBS-017 (two `CheckedPackage` types) bears directly on the #209 acceptance criterion "unchecked and checked objects cannot share an ambiguous public type", but names no secondary. Fix: reassign OBS-031 to #209. Add #209 as a secondary on OBS-034 and OBS-017. Update the §9.2 tally and the Summary counts. | ADR-010 OBS-017, OBS-031, OBS-034, §9.2, Decision 4 |
| FND-008 | medium | The module-to-stage inventory is incomplete for #209. #209 acceptance requires "every current module/crate maps to one stage". `QSL:lib.rs` declares 32 top-level modules. The record places modules only through stage entry cells. It has no placement for `format`, `located_json`, `json_number`, `serde_object`, `token`, `digest`, `source_map`, `wire_format` or `model_source`. §3.1 leaves the public items of `checking`, `complete` and `runtime` "not counted". #206 acceptance says Layer 1 must not need another census. Fix: add one table listing all 32 top-level modules with lane or stage, line count and public-item count. Mark the modules shared across lanes (`diagnostic`, `source`, `digest`). | ADR-010 §2, §3.1; #209 Acceptance; #206 Acceptance |
| FND-009 | low | Risks from PR #200 appear in §8 prose with no decision item: a new QSL → FCD edge at 7dcb2f2, which #209 must rule on for direction, `tempfile` promoted to a production dependency, and a second quire-rs revision (2823a93) in the lock. These leave a new cross-repo edge and a duplicate version with no owner. Fix: add OBS items (FCD edge → #209; duplicate quire-rs revision and the production `tempfile` → #211), or state that they are out of Layer 1 scope. | ADR-010 §8, §3.2 row "QSL → FCD" |
| FND-010 | low | Several evidence cells lack the line number that the evidence convention requires. These are the cells that are hardest to re-verify after the code changes: `QSL:tests/configversion_backends.rs constants` (§1 Kani sha, actually :34), `QSL:wire_format.rs` (§3.3, OBS-026), `QSL:temporal.rs` (`clock:`, OBS-014), `QSL:model/checked_dispatch.rs` (DA-02, §4.2), `QSL:Cargo.toml` (quire-rs row), `QSL:value/accounting.rs`, `QSL:model/key.rs` (DA-12, DA-15). Fix: add line numbers. For evidence of absence, cite the file and state what the search looked for. | ADR-010 §1, §3.2, §3.3, §4.2, §5, OBS-014, OBS-026 |
| FND-011 | low | Version-literal and staleness coverage is incomplete. `QSL:package/view.rs:39` hard-codes a second pinned revision literal, `STANDARD = "e897f810…"`, next to `ir_revision`. Neither OBS-022 nor DA-14 records it. The "Behind" counts in §3.2, §6.3 and the vendoring table do not say which head they were measured against (the Context-table sha or live `origin/main`). Fix: add the `STANDARD` literal to OBS-022 and DA-14. State that "behind" counts are measured against the Context-table shas. | ADR-010 §3.2, §6.3, OBS-022, DA-14 |

## Failure-domain gaps

No failure-domain SpecReview exists yet in `spec/reviews/observed-architecture/`
(only base and ears-conformance are present). FND-001 and FND-002 are the
failure-domain overlaps: the proof path's environment and revision identity.
