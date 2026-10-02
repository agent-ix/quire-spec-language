---
id: SR-471
title: "Risk and complexity review of ADR-011 stage DAG and dependency architecture"
type: SpecReview
analysis: risk-complexity
scope: "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# SR-471: Risk and complexity review of ADR-011

## Summary

Reviewed commit: 944a1c8 (branch `task/209-stage-dag`), with the index row in
`spec/spec.md:388`. Evidence baseline: ADR-010 at the same commit, QSL `src/`
at the same commit, issues #205, #209 and #212, and accepted QSpec AD-016
(not reopened). Decisions owned by #210 and #211 are treated as inputs.

The stage DAG, edge contracts and forbidden bypasses are sound and low in
technical risk. The risk sits in the seam retirements and their ordering
against the #205 dependency graph (`#216 → #217 → #218`, `#224 + #216 →
#225`). Two orderings cannot be executed as written:

- SEAM-1 must be gone before gate #216, but IT-010 (SEAM-4, retired in #217,
  after #216) runs on SEAM-1 `lowering` and `runtime`. The record also says
  IT-010 stays until #217 lands (FND-001).
- Removing SEAM-1 moves the CLI onto the spine in Layer 2, but no Layer 2
  ticket owns that move. #225, which owns the CLI design, is sequenced after
  #216 (FND-002).

Four medium items follow. SEAM-1 has members shared with composed lane B.
The CG build carries two QSL revisions between #217 and the IR → QSL
removal. SEAM-3 has an undefined FB-03 binding and a Layer 4 owner. The
biggest slices have no session sizing.

Verdict: REVISE (2 high, 4 medium, 2 low). All fixes are local to ADR-011,
and none needs a compatibility layer.

## Risk register (decision items with elevated risk or volatility)

| Item | Tech risk | Volatility | Drivers | Mitigation named in the record? |
|---|---|---|---|---|
| SEAM-1 / M-6 native-v1 retirement before #216 | High | Low | About 20k lines across 11 modules (ADR-010 §3.1: `runtime` 5356, `lowering` 1662, `native_model` 1231, `model_source` 995, native halves of `linking` 8147, `checking` 8650, `parser`, `syntax`, `package`, `command`). It is the only CLI lane and the input to IT-010. | Partly. Ordering conflicts with SEAM-4 and #225: FND-001, FND-002, FND-003 |
| SEAM-4 IT-010 retired in #217 | High | Low | Only proof-and-replay path. It consumes SEAM-1 `NativeProjection` and `runtime::execute` (ADR-010 A9–A11). | Yes (#217), but the ordering is broken: FND-001 |
| IR root → QSL edge removal (#218, #223, IR #109) | High | High | Needs v2 family forms from #210 and a QSpec wire change. It lands after #217 turns CG → QSL into a normal edge. | Partly: FND-004 |
| SEAM-3 `protocol_artifact` (22,959 lines) | Medium | High | Whether the wires survive is a #211 decision. `state` and `temporal` import its types. Retirement is in Layer 4. | Partly: FND-005 |
| X-1 `quire-exact` extraction | Medium | Low | Replaces the QSL `value` kernel and RT `src/exact` (11,519 lines) in one change per repository | Order stated; no ticket for the RT agreement retarget: FND-008 |
| M-3 + M-5 + SEAM-2 start on #214 | Medium | Medium | New S2 `forms`, SEAM-5 removal, and a split of `value::expression` (6,880 lines). The family contracts depend on #210. | No sizing: FND-006 |
| §2.3 proof-gate acceptance (unreached-module failure) | Medium | Medium | Reads prover transcripts (an external tool format) and needs mutation controls in RT and CG gates across four repositories | Owner for RT only: FND-007 |
| Stage and edge type roles (§2.1, §4) | Low | Medium | #211 names and typestate encoding are still open | Yes: roles only, with questions handed to #211 |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | SEAM-1 and SEAM-4 retirement orders contradict each other. §6.2 and M-6 remove SEAM-1 (including `lowering` and `runtime`) "before gate #216 passes". SEAM-4 (IT-010) retires in #217, which #205 orders after #216. The Consequences say "Until #217 lands, IT-010 remains the only proof-and-replay test". IT-010 lowers through SEAM-1 (`NativeProjection` bytes, ADR-010 A9 `tests/configversion_backends.rs:543,597,871`) and replays through `runtime::execute` (:259). Removing SEAM-1 first deletes IT-010's inputs, so IT-010 dies at #216 and nothing replaces it until #217. The SEAM-1 row is also inconsistent within itself: "removed before gate #216 passes" but "lowering-target removal with #217". Fix: decide one order. Recommended: SEAM-4 (IT-010 and the QSL dev dependencies on CG and IR) is deleted in the same change that removes SEAM-1 `lowering` and `runtime`, before #216. The proof-and-replay gap until #217 is then stated in the Consequences. This matches #205's rule that build-only and stubbed paths are never proof evidence. Remove "lowering-target removal with #217" from the SEAM-1 row and update the §7.1 "QSL tests → RT" row to match. | ADR-011 §6.2 SEAM-1, SEAM-4; §7.3 M-6; §7.1; Consequences bullets 2 and 5; ADR-010 §2.1 A9–A11 |
| FND-002 | high | The CLI move to the spine has no owner in Layer 2. The Consequences say that because SEAM-1 goes before #216, "the CLI therefore moves to the spine within Layer 2". The CLI today reaches only native-v1 (`parse`, `format`, `run`, `compile`, `lower`; ADR-010 OBS-008, `QSL:cli.rs:59-126`). §5 gives CLI design to #225 and implementation to #122 and #232. #205 sequences those as `#224 + #216 → #225 → #232`, so they come after the gate that needs the move. No ticket in the implementing-tickets table builds `run` and `compile` over S0–S6a, retires `lower`, or retargets `format` from the arena to the CST (§6.2 `format` row). #212 fails a design that leaves an ambiguity for implementation. Fix: name a Layer 2 slice, either a new child of #216 or a bounded child of #122 placed before #216. It rewires `run` and `compile` to the spine and deletes `lower` and the native half of `command`. It meets the §5 rules as the interim contract, and #225 later extends them to lifecycle surfaces. Add the slice to the implementing-tickets table and to SEAM-1's "Owning change". | ADR-011 §5; §6.2 SEAM-1, `format` row; Consequences bullet 2; #205 dependency graph; ADR-010 OBS-008 |
| FND-003 | medium | SEAM-1 is defined by whole module, but some of those modules are shared with lane B, which retires later (SEAM-2, through #214, #220, #221 and #223 in Layers 2–4). `formal_source` is listed wholly in SEAM-1, yet composed checking imports it (`QSL:checking/composed.rs:10`, `checking/composed/sources.rs:6`, `checking/composed/proofs/engine.rs:18`; ADR-010 marks it A4, A5, B6 and B7). `syntax`, `parser`, `linking` and `checking` are split "native / composed" with no rule for code the halves share. If SEAM-1 is removed before #216 as written, it breaks lane B or leaves orphaned shared code that belongs to no stage or seam. That breaks Decision 1. Fix: in §6.2, move `formal_source` and any code shared by the native and composed halves to SEAM-2 (it retires with the composed lane). Name SEAM-1 by entry points: arena `ParsedUnit` parser entry, `NativePackage`, `lowering`, `runtime`, `native_model`, `model_source` and `mapped`. Then #226's drift gate can check the split mechanically. | ADR-011 §6.2 SEAM-1, SEAM-2, module table; Decision 1; ADR-010 §3.1 |
| FND-004 | medium | The CG build carries two QSL revisions from #217 until the IR root → QSL edge is removed. §7.1 makes CG → QSL a normal edge in #217, but removes IR root → QSL (f1700a9) only in #218 and #223 with IR #109 (Layers 3–4). CG depends on the IR root (`CG:Cargo.toml:17`), so the #217 replay adapter builds QSL head and QSL f1700a9 side by side. That breaks the AD-016 heads drift check 6 ("one revision per quire crate"), and the OBS-034 pin drift continues through the proof spine. §7.1's one-revision rule covers only the QSL lock, so it does not catch this. Its volatility is high because removal waits on #210 v2 family forms and a QSpec wire change. Fix: make the IR root → QSL removal a precondition of #217. Delete IR `predicate::project` and `temporal::project` over QSL types, and `replay_with_native_runtime` (IR #140), before #217. Re-add predicate and temporal admission from v2 forms in #218 and #223 (prerelease removal, not a shim). Extend the §7.1 one-revision rule to every quire-ecosystem lock (IR, RT, CG). | ADR-011 §7.1 rows 1 and 5 and rules; FB-05; ADR-010 OBS-029, OBS-034, §3.2 |
| FND-005 | medium | SEAM-3 (`protocol_artifact`, 22,959 lines) has an undefined exit and a late owner. FB-03 allows wire data only "with a verified binding to a checked producer", but no ticket defines that binding, and it is not among the questions handed to #211. The seam's end state depends on #211 deciding whether compiled-protocol /1 to /3 survive, and its owners are #223 (Layer 4) and #218. Meanwhile the layer-5 family evaluators import SEAM-3 types directly (`QSL:state/input.rs:6` `ProtocolNumber`, `state/work.rs:4`, `temporal/formula.rs:10` `protocol_artifact::wire`). So §6.1's "`temporal` never imports a wire module" and the break of ADR-010 SCC S3 stay false until Layer 4. #226 may start after #215 and #216, so it would report a violation that the record itself schedules. Fix: (a) add "the FB-03 binding rule for a wire-read package" to the questions handed to #211. (b) Name a slice, placed with M-4 in #216, that moves `ProtocolNumber` and `Locus` to K or F and removes the `state` and `temporal` imports of `protocol_artifact`. (c) List SEAM-3's remaining FB-03 violations as known until #223, so #226 can baseline them by seam id. | ADR-011 §3 FB-03; §6.1 rules; §6.2 SEAM-3, `state`/`temporal` rows; Questions to #211; ADR-010 OBS-015, OBS-016, OBS-037 |
| FND-006 | medium | The big slices are not sized against #205 ("roughly one to three focused agent sessions") or the #212 pass condition. #214 gets M-3 (new S2 `forms` plus SEAM-5 removal), M-5 (splitting `value::expression`, 6,880 lines, into `check` and `evaluate`) and the first SEAM-2 rehoming. M-6 removes about 20k lines across 11 modules and has only a gate as its owning change. X-1 replaces the `value` kernel and RT `src/exact` (11,519 lines) "in one change per repository". Also, #205 requires a single writer for overlapping QSL modules, yet M-5, M-2 and X-1 all touch `value` and `model`, and neither order nor writer is stated. Fix: add a slicing column (or a note) to §7.3. Split M-6 into named slices (CLI rewire per FND-002; `lowering` + `lower` + SEAM-4; `runtime` + `NativePackage` + `native_model` + `model_source` + `mapped`; native halves of `syntax`, `parser`, `linking` and `checking`). Put M-3 and M-5 in separate #214 slices, M-5 after M-3. State the single-writer order for `value` and `model`: X-1 → M-2 → M-5. | ADR-011 §7.3; §6.2 SEAM-1, SEAM-2, SEAM-5; #205 Required process; #212 pass conditions |
| FND-007 | low | The §2.3 proof-gate rule binds gates in four repositories but names a red-to-green owner only for RT (#53). The CG generated-harness gates (CG #58, #59, #60, #73) and the #217 and #219 spine gates gain the same unreached-module failure and a per-module mutation-control requirement. Condition 1 ("discharged check location inside that module") depends on how the prover's transcript attributes locations. That is an external tool format, which makes it volatile. Fix: for each bound gate, name the change that adds its mutation controls. State that module attribution is read from the typed #231 proof-result envelope under the pinned Kani, not from raw transcript text (this also keeps FB-02 intact). | ADR-011 §2.3; FB-10; Consequences bullet 4; #231 |
| FND-008 | low | Two ordering prerequisites have no ticket. X-1 is "1st (AD-016 WP5a and WP5b)", and M-2 comes after it, but X-1 is absent from the implementing-tickets table and #205's graph. §7.1's "RT `qsl-agreement` → QSL (dev) removed" is owned by "RT, after `quire-exact` lands", which names no ticket. Both come before #213 and M-2, so an unowned slip stalls Layer 2. Fix: add X-1 (with its QSL and RT tickets) and the RT agreement-suite retarget ticket to the implementing-tickets table, and link them as blockers of #213. | ADR-011 implementing-tickets table; §7.1 row 4; §7.3 X-1, M-2 |

## Volatility of the decisions

Low: the stage order (§1), the edge-preservation table (§2.2), FB-01, FB-02,
FB-04 and FB-09, the §7.2 extraction criteria, and Decision 9. These follow
from accepted AD-016 and #205. They do not depend on open sibling decisions.

Medium: stage output types and outcome vocabulary. They are named as roles
and handed to #211, which contains the churn well.

High: SEAM-3's end state and the IR predicate and temporal admission from v2
(which wait on #210, #211 and QSpec), and the §2.3 transcript-based proof
acceptance. FND-004, FND-005 and FND-007 isolate these from the Layer 2
critical path.

## Failure-domain gaps

No failure-domain review of ADR-011 exists yet in `spec/reviews/stage-dag/`.
The overlap with this review is FND-001 and FND-003: removal ordering
decides whether a gate fails for lack of any proof path, or because
composed-lane code is orphaned.

## Round 2 (HEAD 5cbd853)

Reviewed the revised ADR-011 at 5cbd853 against the round-1 findings. The
main changes are the M-6 slices (§7.3), the gap statement after them, and the
Owner questions. Choices routed to the owner under "Owner questions" are
accepted at this layer and not re-argued here.

### Resolution of round-1 findings

| ID | Round 2 | Reason |
| --- | --- | --- |
| FND-001 | Resolved | SEAM-4 is now removed with SEAM-1 in M-6 (§6.2 SEAM-4 row, M-6d). The "lowering-target removal with #217" text is gone. §7.1 "QSL tests → RT" is owned by M-6. The proof-and-replay gap until #217 is stated in §7.3 and the Consequences, and routed as Owner question 2. |
| FND-002 | Resolved | M-6a rewires `run` and `compile` to the spine, and M-6c removes `lower` and the native `command` submodules (§7.3, §6.2 SEAM-1). The missing ticket is routed as Owner question 1, and the text states the amendment to the #205 layer plan. |
| FND-003 | Resolved | SEAM-1 is now defined by its entry points, and code shared with SEAM-2, including `formal_source`, belongs to SEAM-2 (§6.2 SEAM-1 and SEAM-2 rows, `formal_source` row). |
| FND-004 | Resolved | The IR root → QSL edge is now removed before #216 (§7.1 row 1, §9 OBS-029), so before #217 makes CG → QSL normal. The CG build no longer carries two QSL revisions. The ticket is routed as Owner question 4. The one-revision rule still checks only QSL's lock. That is acceptable now that the ordering removes the case. |
| FND-005 | Partly | §4 now defines the verified binding that FB-03 cites. M-6c removes the SEAM-3 reads that feed `state` and `temporal`, and the evaluator gap is stated (§7.3, Owner question 2). But the `state` and `temporal` modules are built on SEAM-3 types, not only on reads. The unresolved part is FND-010. |
| FND-006 | Partly | M-6 is split into M-6a to M-6d. M-3 and M-5 are separate slices, and the single-writer order X-1 → M-2 → M-5 is stated (§7.3 preamble). M-6c still deletes every SEAM-1 module, SEAM-2 and SEAM-3 emission, and the `state` imports in one slice. No slice is sized against #205's one to three sessions. This remains medium: M-6c is mostly deletion, and FND-010 decides its size. |
| FND-007 | Partly | Gate owners are listed per gate (§2.3). The CG gate and the `quire-exact` gate are routed as Owner questions 6 and 7. §2.3 condition 1 and the #226 census still read the prover transcript. They do not say that module attribution comes from the typed #231 proof-result envelope. This remains low. |
| FND-008 | Resolved | X-1, M-2 and the RT agreement retarget have no tickets yet, and this is routed as Owner question 6. §7.1 now places X-1 "ahead of #213". |

### New findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | The skeleton spine has no ticket and no order relative to M-6d. The record now rests the M-6 → #217 window on it: "The skeleton spine (§1.1) is the only proof-and-replay evidence in that window" (§7.3, Consequences). Owner question 2 asks the owner to accept the gap on that basis. But nothing orders M-6d (which deletes IT-010) after the skeleton is green in CI. If M-6d lands first, QSL's ecosystem has no proof-and-replay evidence at all. The skeleton also needs M-4 (E5 reads v2 bytes, E9 reads through the I2 reader) and a CG → QSL edge to a revision that has M-4 and the S6a entry. §7.1 makes that edge normal only in #217, and today it is a dev pin to 21c507e. Fix: in §7.3, add "after the skeleton spine is green in CI" to M-6d's order. In §7.1, state that the skeleton moves the CG → QSL dev pin to a revision with M-4, and that #217 makes it normal. Either name the skeleton's ticket or add it to Owner question 1. | ADR-011 §1.1; §7.1 row 5; §7.3 M-6d and the gap paragraph; Consequences bullet 5; Owner question 2 |
| FND-010 | medium | M-6c's scope for `state` and `temporal` is undefined. M-6c removes "the SEAM-3 reads that feed `state` and `temporal`, and the `state` `native_model` and IR imports". But these modules are typed over SEAM-1 and SEAM-3: `state/input.rs:6` (`protocol_artifact::{wire, ProtocolNumber}`), `state/work.rs:4` (`wire::Locus`), `state/evaluation.rs:21-22,56` (`native_model::NativeModel`, `protocol_artifact`), `temporal/formula.rs:10,572` and `temporal/mapping.rs:19` (`protocol_artifact::wire`, `ProtocolNumber`). The SEAM-3 row removes `protocol_artifact`'s reads and emission in M-6, and SEAM-1 removes `native_model`. So M-6c must either delete `state` and `temporal`, or re-type them over K, F or `semantic_value`. The record says only that the evaluators are "unavailable" until #220 and #222. The first choice empties layer 5 apart from `value::expression` and `simulation`. The second adds new design work to a deletion slice. #226 and #212 cannot check M-6c until it is decided. Fix: state in the §6.2 `state` and `temporal` rows and in M-6c which choice is made. If they are deleted, #220 and #222 re-add them, which follows Decision 9. If SEAM-3 is removed completely in M-6, say so in the SEAM-3 row. | ADR-011 §6.2 SEAM-3, `state`, `temporal` and `protocol_artifact` rows; §7.3 M-6c and the gap paragraph; `QSL:state/*.rs`, `QSL:temporal/*.rs` at 5cbd853 |

No new high finding. The revision adds no compatibility layer, and it keeps
AD-016 arrows 1, 2 and 7, Owner decision 5 and the crate graph.

### Round-2 verdict

ACCEPT WITH FINDINGS. Open: 0 high, 3 medium (FND-006, FND-009, FND-010) and
1 low (FND-007). Both round-1 high findings are resolved. FND-005 and FND-006 are partly resolved, and their open parts
are FND-010 and M-6c sizing. FND-007 remains low. FND-009 and FND-010 are
local text fixes to §7.1, §7.3 and §6.2, and neither needs an owner decision
beyond Owner questions 1 and 2.

## Round 3 (HEAD 22fa948)

Delta review f781e32 → 22fa948. The X-1 schedule risk is now explicit: X-1 is
#213 S-1, blocked by the AD-016 amendment (TK-10, QC-15), which may land now
that IR #139 is merged (954c2f2). One effect of the delta is that compiling a
package with dependencies now needs each dependency's source. SR-467 FND-014
records the missing binding rule. No new findings in this analysis area.

### Round-3 verdict

ACCEPT WITH FINDINGS (no change from round 2).
