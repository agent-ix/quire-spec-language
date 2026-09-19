---
id: SR-461
title: "Dependency analysis of ADR-010 observed architecture baseline"
type: SpecReview
analysis: dependency
scope: "spec/decisions/ADR-010-observed-architecture-baseline.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-010
    type: reviews
---
# SR-461: Dependency analysis of ADR-010

## Summary

Round 2. Reviewed commit 432e615 on `task/206-observed-architecture`, against
the round-1 review of faa1731. ADR-010 is a descriptive record, so this pass
checks the dependency data it records: crate, repository and module dependency
direction, cycles and bypasses (#206 acceptance), the §7 prerequisite data
compared with the #205 dependency graph and the current issue bodies, the L1-D1
framing of the #185 question, and the enablement and feature split.

The revision resolves every blocking round-1 finding:

- §3.2 now cites CG → IR root (package quire-contract-ir) and records the
  transitive normal edge CG → IR root a5154d3 → QSL f1700a9.
- §3.2 states an edge rule and lists three cycles under it: IR ⇄ QSL, QSL ⇄ CG
  and QSL ⇄ RT.
- L1-D1 separates the "woven in after" sequencing edges from the declared
  technical prerequisites. It matches the current issue bodies, with their
  `updatedAt` times.
- §8 gives one merge order, #228 → #204 → #200, with the ARCH-01 comment ID and
  time.

Round 1 also asked for four structural changes, and the revision makes all four:

- §7.1 has a separate "Declared prerequisites" column, and every row in it
  matches its issue body.
- DA-11 records the #213 / #185 / #205 statements on who owns the capability
  type.
- Every §7 table has a Class column.
- The SCC S1 diagram is strongly connected.

The open findings are one medium and six low. The medium finding (FND-001) is
that the new edge rule names no set of repositories, so the cycle count of 3 is
complete only for the Context-table repositories. The low findings are
completeness and labelling gaps in the new tables.

Verdict: ACCEPT WITH FINDINGS. No finding is high, so none blocks.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | The §3.2 edge rule ("a Cargo dependency of any kind … or a Cargo manifest that the repository's tests write or build") does not say which repositories it covers, yet the Summary counts row and the cycle table present "3" as the complete list under that rule. Applied as written, the rule also gives: (a) IR root → quire-protocol 34d1752 as a normal edge (`IR:Cargo.toml:23`), where quire-protocol has a normal dependency on QSL f1700a9 (`quire-protocol@34d1752:Cargo.toml:11`) and a dev dependency on IR model 53cc03c (`quire-protocol@34d1752:Cargo.toml:22`). That makes a fourth cycle, IR ⇄ quire-protocol, and a second path from IR, and so from CG, to QSL f1700a9. (b) An IR test-built manifest, `IR:tests/fixtures/bridge-qsl-consumer/Cargo.toml:13` → QSL f1700a9, built at `IR:tests/cycle_free_model.rs:276`. This adds no new QSL cycle, but it is an edge that §3.2 leaves out. OBS-029 says the IR typed handoffs pin QSL f1700a9. quire-protocol is a second consumer inside IR's build that pins the same revision. Fix: limit the edge rule to the Context-table repositories and say that transitive paths through other repositories are recorded but not counted, or add the quire-protocol edges and the IR ⇄ quire-protocol cycle and correct the count. Either way, add the IR fixture edge to the §3.2 table. | ADR-010 Summary counts, §3.2 edge rule and cycle table, OBS-029 · `IR:Cargo.toml:23` · `quire-protocol@34d1752:Cargo.toml:11,22` · `IR:tests/fixtures/bridge-qsl-consumer/Cargo.toml:13` · `IR:tests/cycle_free_model.rs:276` |
| FND-002 | low | The §7.5 rule is "every downstream issue referenced in the body of an issue mapped in §7.1–§7.4", but the table leaves out five downstream references in mapped bodies. QSpec #104 (open) is cited by #155. QSpec #63 (closed; #1 is "Blocked until … #63 Task-010 passes") and quire-research #28 are cited by #1. QSpec #13 is cited by #42. quire-wasm #6 (closed) is cited by #207. The "Count: 28" row and the Summary row "Every downstream issue cited …" therefore overstate how complete the table is. Fix: add these references, or narrow the rule (for example, to open issues other than the #205 non-goals) and recount. | ADR-010 §7.5, Summary counts · #155, #1, #42, #207 bodies |
| FND-003 | low | The table "Declared prerequisites that the #205 'Dependency graph' does not draw" states no rule for whether an edge counts as drawn when it appears only transitively. Two rows are drawn transitively: #232 ← #216 (`#224 + #216 → #225 → #232`) and #231 ← #211 (`#211 → #212 → … → #213 → #231`). Other direct edges from the bodies that the graph draws only transitively are left out: #214 ← #210/#211/#212, #215 ← #209/#211, #185 ← #229/#212/QSpec #116, #216 ← #229/#213/#185/#231, and #230 ← #224. Fix: state "not drawn as a direct edge" and list every such edge, or state "not reachable" and drop the two transitive rows. | ADR-010 §7.1 side table · #205 "Dependency graph" (updated 16:53:56Z) · #214, #215, #185, #216, #230, #231, #232 bodies |
| FND-004 | low | L1-D1 has three residual gaps. (a) It lists "#187 on #172" and "#198 on … #168" as declared technical prerequisites, but #172 and #168 are QSL PRs that merged on 2026-09-19 at 01:33Z and 2026-09-18 at 22:57Z, both before de627b5. They are already met, not open edges. (b) It names only #188 as depending on #185 registry behaviour. #189 says the same: "a claim over it that no registered backend can discharge settles `unsupported`". (c) It does not cite #1's "architecture-aware execution order" checklist, which puts #185 first and says "#185 moves ahead of proof/backend work that requires dispatch". That checklist is a sequencing source separate from the "woven in after" lines. Fix: mark #172 and #168 as merged PRs, add #189 to the registry-behaviour sentence, and cite the #1 ordering. | ADR-010 §9.1 L1-D1 · #1, #187, #189, #198 bodies · QSL PR #168, PR #172 |
| FND-005 | low | OBS-034 says "Five QSL revisions are in use" but lists only four QSL revisions: f1700a9, 21c507e, ea39f91 and de627b5. The fifth item, "QSL's own IR pins", is not a QSL revision. Fix: say four, or name the fifth QSL revision. | ADR-010 OBS-034 · §6.3 |
| FND-006 | low | The §3.2 mermaid draws dependency edges (from dependent to dependency) and data-flow edges in the same arrow style, pointing in opposite directions, with no legend. `IRH --> CG` reads as "IR 04eb6f8 depends on CG", but the dependency runs the other way: CG 5e2a6a9 pins IR 04eb6f8 (`CG@5e2a6a9:src/oracle.rs:14`). `Kani --> QSL` and the `QSpec -.-> QSL` vendoring edges also point in the direction data flows. Fix: add a legend, or draw data-flow edges in a distinct style, for example labelled `-. data .->`. | ADR-010 §3.2 mermaid · `CG@5e2a6a9:src/oracle.rs:14` |
| FND-007 | low | Citation gaps left from round-1 FND-012 and FND-013. (a) OBS-012, OBS-013, DA-11 and the §1 rows cite `QSpec:spec/objects/protocol/FR-290-protocol-claim-kind.md` with no line number, although each makes a positive claim, for example that FR-290 says QSL's enum aligns to its six kinds. (b) The SCC S1 table cites no edge into `native_model`, which is reached only through linking→native_model (`QSL:linking.rs:15`). It also gives no citation for the checking→linking half of that two-cycle (`QSL:checking.rs:15`). (c) The §7.5 rows for RT #51 and IR #137 cite "§8", but §8 names neither RT PR #52 nor IR #137. Fix: add line numbers and the two edge citations, and point the RT #51 and IR #137 rows at the ARCH-01 comment rather than §8. | ADR-010 §1.1, §3.1, §5 DA-11, §7.5, OBS-012, OBS-013 · `QSL:linking.rs:15` · `QSL:checking.rs:15` · ARCH-01 comment on #207 |

## Round 1 resolution

Round 1 reviewed faa1731 and raised 13 findings (4 high, 6 medium, 3 low).
Against 432e615: 11 are resolved, 2 are partially resolved and 0 are unresolved.

| Round-1 ID | Severity | Status | Reason |
|---|---|---|---|
| FND-001 | high | resolved | §3.2 and the mermaid now show CG → IR root (package quire-contract-ir) at `CG:Cargo.toml:17`. The transitive CG → IR a5154d3 → QSL f1700a9 edge (checked at `IR@a5154d3:Cargo.toml:24`), CG dev → QSL 21c507e and the QSL ⇄ CG cycle are recorded. The OBS-034 count left behind is now FND-005. |
| FND-002 | high | resolved | An edge rule is stated, and the three QSL cycles are listed with the kind of each edge. Every edge checks out: QSL→CG `QSL:Cargo.toml:43`; CG→QSL `CG:Cargo.toml:28`; the QSL fixture → RT 8a4d02b, built at `QSL:tests/native_backend.rs:281`; RT qsl-agreement `[dev-dependencies]` → QSL ea39f91. The rule's unbounded scope is now FND-001. |
| FND-003 | high | resolved | L1-D1 now lists #185's dependency on #213 and the fact that it consumes #213's `Capability`. It separates the sequencing edges from the technical prerequisites and adds #186 → #231 and the #188 registry behaviour. It pins the retrieval time and each body's `updatedAt`, and every item matches the current bodies. Remaining gaps are FND-004. |
| FND-004 | high | resolved | §8 and Decision 4 give one order, #228 → #204 → #200, citing issuecomment-5743530928 at 2026-09-19T16:35:16Z. That order matches the ARCH-01 "Rulings applied" section and §3. |
| FND-005 | medium | resolved | §7.1 now separates "Layer 1 decision consumed" from "Declared prerequisites (issue body)", and all 28 rows match the bodies retrieved at 17:02Z. A side table records where the #205 graph differs from the bodies; its criterion is FND-003. |
| FND-006 | medium | resolved | DA-11 records that #213 owns the canonical `Capability`, that #185 owns only registration and routing, and #205's "sole capability registry/routing implementation owner" (the #205 wording has changed since round 1). |
| FND-007 | medium | resolved | Decision 3 now places #229 as secondary on OBS-012, OBS-013 and DA-11: #229 decides the normative vocabulary and #210 decides the architecture boundary. Under the coordinator's constraint (owners limited to #209/#210/#211, with #229 secondary), this covers the #229 → #213 → #185 path. |
| FND-008 | medium | resolved | §3.2 cites the path dependency at `IR:Cargo.toml:21` and labels `:37` as a dev self-pin. §6.3 matches. |
| FND-009 | medium | resolved | §3.2 adds a "QSL tests → RT, test-time, 8a4d02b" row. The mermaid attaches the f1700a9 edge to a new IR-root node at 553b6d1 (IRR). |
| FND-010 | medium | resolved | §7.1–§7.5 carry a Class column (record, design, gate, enablement, conformance, feature, fix, unrelated). #185 is marked enablement inside the #1 ladder, and #232 is marked as a feature child of #122. |
| FND-011 | low | resolved | L1-D1 names #217 and the #223 backend-breadth owners, CG #84 and IR #136, under "Outside the ladder". |
| FND-012 | low | partially resolved | Issue-body retrieval time and `updatedAt` are recorded, and the line-less QSL citations from round 1 now carry lines. The FR-290 positive citations still have none (FND-007 a). |
| FND-013 | low | partially resolved | Edges into eight S1 members are now cited, and the diagram is strongly connected. The entry into `native_model` and the checking→linking edge are still uncited (FND-007 b). |

## Method

- Read ADR-010 at 432e615 in full and the diff `faa1731..432e615 -- spec/decisions`.
- Checked the Cargo manifests with `git grep` and `git show` at QSL de627b5, IR
  553b6d1 and a5154d3, IR model 53cc03c, CG a4b2a73, CG 5e2a6a9
  (`src/oracle.rs`), RT d97bc0b (`conformance/qsl-agreement`), quire-protocol
  34d1752, quire-observation 9ac80e9 and quire-rs 8b8020e.
- Checked the SCC S1 entering edges against `use crate::` lines at de627b5.
- Re-read the bodies and `updatedAt` times of #1, #185–#198 and #205–#232 as of
  2026-09-19, plus the ARCH-01 comment on #207. Checked the GitHub blocked-by
  and blocking links on #185 and #186, both empty. Checked the state of QSL PRs
  #168 and #172, QSpec #63 and #104, and quire-wasm #6.
- Scanned every open QSL issue body for downstream issue references and compared
  them with §7.5.

## Round 2 resolution (author)

Recorded by the authoring agent; the round-2 verdict stands.

- FND-001 resolved: the §3.2 edge rule is limited to the seven Context-table
  repositories and names quire-protocol as out of scope; the IR test fixture
  edge to QSL f1700a9 is added.
- FND-002 resolved: §7.5 completed (34 rows).
- FND-003 resolved: the undrawn-edges table states it is a sample and that
  transitive-only edges count as not drawn.
- FND-004 resolved: L1-D1 marks #172 and #168 as merged PRs, names #189 and
  cites #1's execution order.
- FND-005 resolved: OBS-034 says four QSL revisions.
- FND-006 resolved: mermaid legend added; data-flow arrows labelled.
- FND-007 resolved: FR-290 lines, `linking.rs:15` and `checking/proof.rs:20`
  cited; §7.5 IR #137 and RT #51 cite the ARCH-01 comment.
