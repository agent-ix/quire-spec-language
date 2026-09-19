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

Reviewed commit faa1731 on `task/206-observed-architecture` (ADR-010 and its
index row in `spec/spec.md`). ADR-010 is a descriptive record, not a requirement
set, so this pass checks the dependency data it records instead of building a
requirement DAG. It covers four things: crate, repository and module dependency
direction, cycles and bypasses (#206 acceptance); the §7 ticket map "consumes"
data compared with the #205 dependency graph and the current issue bodies; how
L1-D1 frames the #185 question; and whether enablement tickets are kept apart
from feature tickets.

The in-crate SCC analysis, the IR root ⇄ QSL cycle, the list of bypass paths and
the 68-row issue coverage all hold. The following do not:

- Two Cargo citations name the wrong crate or dependency kind. One of them hides
  a transitive normal dependency from CG to QSL.
- The repository-cycle count of 1 rests on an edge-kind rule the record never
  states. §3.2 and §6.3 contradict each other on QSL → RT.
- L1-D1 misstates what #185 depends on and says no ladder ticket is ordered
  after #185. The "woven in after" chain orders every one of them after it.
- The §8 merge order contradicts itself.

Verdict: REJECT. FND-001, FND-002, FND-003 and FND-004 are blocking.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | high | §3.2 row "CG → IR model, normal, a5154d3, `CG:Cargo.toml:17`" and the mermaid edge `CG → IRCM` are wrong. `CG:Cargo.toml:17` declares package `quire-contract-ir`, which is the IR root crate, not `quire-contract-model`. At a5154d3 the IR root has a normal dependency on QSL f1700a9 (`IR@a5154d3:Cargo.toml:24`), so CG has a transitive normal dependency on QSL. CG also has a direct dev dependency on QSL 21c507e (`CG:Cargo.toml:28`), and QSL has a dev dependency on CG (`QSL:Cargo.toml:43`). The record shows none of this, and OBS-034's count of QSL revisions misses that f1700a9 reaches CG builds. Fix: relabel the row and the diagram edge as "CG → IR root (package quire-contract-ir)". Add the transitive edge CG → IR root → QSL f1700a9 (normal). Record the QSL ⇄ CG repository relationship in §3.2 and OBS-029, or in a new OBS. | ADR-010 §3.2, §6.3, OBS-029, OBS-034 · `CG:Cargo.toml:17,28` · `IR@a5154d3:Cargo.toml:24` · `QSL:Cargo.toml:43` |
| FND-002 | high | The count "Cross-repository repo cycles: 1" (Summary counts, §3.2) rests on an edge-kind rule the record does not state. The rule used for IR ⇄ QSL is "the repositories depend on each other", with normal edges both ways. Applied to dev and test edges it also yields QSL ⇄ CG (QSL dev → CG 5e2a6a9; CG dev → QSL 21c507e plus the transitive normal edge in FND-001) and QSL ⇄ RT. For QSL ⇄ RT the edges are the QSL test fixture crate at `QSL:tests/fixtures/native-lowering/Cargo.toml:14` → RT 8a4d02b, built by `QSL:tests/native_backend.rs:281`, and `RT:conformance/qsl-agreement` → QSL. #206 acceptance requires known cycles to be explicit. Fix: state which edge kinds (normal, dev, test fixture) define a repository cycle, then list every cycle under that rule with the kind of each edge, and correct the count. | ADR-010 Summary counts, §3.2 · #206 Acceptance · `QSL:tests/fixtures/native-lowering/Cargo.toml:14` · `RT:conformance/qsl-agreement/Cargo.toml:18` |
| FND-003 | high | L1-D1 misstates the #185 question it hands to #210, in two ways. (a) It says #185 depends on "#212, #229 and QSpec #116". The current #185 body (retitled "[V1-A08] Lowering target registry and routing over the canonical Capability type", edited 2026-09-19T16:49Z) also depends on #213 and says #185 consumes #213's `Capability` type and does not own it. (b) It says "None of #186–#198 states one". Every ladder body carries an ordering edge: #186 "Woven in after V1-A08" (A08 is #185), #187 after A09, #188 after A10, #189 after A11, #191 after A12, #192 after the spec-versioning gate, and #198 after A14. So all seven are ordered after #185 by transitive sequencing. It also omits #186 → #231, and #188's statement that an unbounded formula with "no registered liveness backend settles `unsupported`", which is #185 registry behaviour. Fix: restate the observed edges from the current bodies and pin the retrieval time. Separate the "woven in after" sequencing edges from declared technical prerequisites. Reframe L1-D1 as "which of the existing sequencing edges from #185 are real capability-dispatch prerequisites, and which may be relaxed". | ADR-010 §9.1 L1-D1 · #185, #186, #187, #188, #189, #191, #192, #198 bodies · #205 "Existing consumers" |
| FND-004 | high | §8 contradicts itself on merge order. The table notes say #228 "merges first", #204 "merges second" and #200 "merges third". The prose below says "Merge order: #204 → #228 → #200 as set by the #205 coordinator", and says that order governs over #207. The coordinator ruling has no evidence citation. Both ARCH-01 statements on #207 (the Rulings table and §3) give #228 → #204 → #200, and Decision 4 says dispositions follow ARCH-01. Fix: cite the coordinator ruling (comment URL and time) or drop it. Make the table notes and the prose give the same single order. | ADR-010 §8, Decision 4 · #207 ARCH-01 comment (Rulings, §3) |
| FND-005 | medium | The §7.1 "Consumes" column mixes three relations: Layer 1 decision consumed, gate inputs, and prerequisite tickets. Read as dependencies, several rows disagree with the issue bodies. #213 lists only #211, but its body depends on #212 and #222 and consumes #229. #214 lists only #210, but its body lists #210, #211, #212 and #213. #216 lists "#209 and #211", but its body lists #213, #214, #215, #231, #229, #185, #131, #132 and #164. #218 omits #217 and QSpec #101/#106. #220 omits #216 and #219. #221 omits #216. #222 lists #210, but its body and #205 give #212 plus QSpec #112/#113. #223 omits #216 and #218/#219. #225 omits #216 and #224. The #205 graph itself leaves out edges the bodies declare: #213 → #185, #231 → #216, #231 → #186, and #232 → #230. Fix: split the column into "Layer 1 decision owner" and "Declared prerequisites (issue body, retrieval time)". Add a short table recording each disagreement between the #205 graph and the issue bodies as an observation for #208 or #212. | ADR-010 §7.1 · #205 "Dependency graph" · #213, #214, #216, #218, #220–#225, #230, #231 bodies |
| FND-006 | medium | The record does not note that the program disagrees with itself on who owns the Capability type. #205 lists #185 as "sole capability enum/registry implementation owner". The #185 body says it consumes the canonical `Capability` from #213 and must not own a competing enum. #213 and #216 say #213 owns the value type and #185 only registration and routing. The §7.1 row for #185 assigns DA-11 without resolving this, and DA-11 is exactly this ambiguity. Fix: add the disagreement to DA-11's observed owners (or to a new OBS) with the #205, #185, #213 and #216 citations, so #210 or #229 decides it explicitly. | ADR-010 §5 DA-11, §7.1 #185 row · #205 Layer 2 list · #185, #213, #216 bodies |
| FND-007 | medium | DA-11 and OBS-012 (capability vocabulary and its alignment with FR-290) are owned by #210, with #229 as a secondary input. Decision 3 limits owners to #209, #210 and #211. But #229 is a Layer 1 ticket whose scope is exactly the six-kind vocabulary and absence policy. #212 passes only if "Capability vocabulary/absence policy is accepted in #229", and #185 and #213 both depend on #229. With #229 as secondary, the dependency path #229 → #213 → #185 has no decision owner in the record. Fix: allow #229 as an owner in Decision 3 and give it the vocabulary part of DA-11 and OBS-012. Keep backend-selection dispatch with #210, or state why #210 decides the vocabulary before #229 does. | ADR-010 Decision 3, §9.2 OBS-012, §9.3 DA-11 · #229, #212 bodies |
| FND-008 | medium | §3.2 row "IR root → IR model, normal, 53cc03c, `IR:Cargo.toml:37`" and §6.3 "IR model 53cc03c at `IR:Cargo.toml:37`" cite a dev dependency (`quire-contract-model-owner`, `[dev-dependencies]`). The IR root's normal dependency on the model crate is the in-workspace path dependency at `IR:Cargo.toml:21`. Fix: change the normal row to cite `IR:Cargo.toml:21` (path). Record `:37` as a dev-only self-pin to 53cc03c, labelled as such in §6.3. | ADR-010 §3.2, §6.3 · `IR:Cargo.toml:21,37` |
| FND-009 | medium | Two dependency statements contradict each other or the code. (a) §3.2 "QSL → RT: none" against §6.3 "RT 8a4d02b at `QSL:tests/fixtures/native-lowering/Cargo.toml:14`", which is a test fixture crate that QSL tests build. (b) The §3.2 mermaid attaches the edge "normal dep QSL rev f1700a9 78 behind" to node IRH (IR historical 04eb6f8). IR at 04eb6f8 has no QSL dependency, so the only drawn repository cycle sits on the wrong node. The prose correctly names IR root at 553b6d1. Fix: change the QSL → RT row to "test fixture, 8a4d02b". Add an IR-root node at 553b6d1 and move the edge to it. | ADR-010 §3.2 · `IR@04eb6f8:Cargo.toml` · `IR:Cargo.toml:24` · `QSL:tests/fixtures/native-lowering/Cargo.toml:14` |
| FND-010 | medium | Enablement and feature work are not separated. §7 maps every issue to a Layer 1 owner but never classifies it as enablement, feature, gate or unrelated. Enablement placed inside the feature ladder is left implicit: #185 (A08) is capability-registry enablement, #231 is envelope enablement, and #232 is a bounded #122 feature child listed among the architecture tickets. #205 needs this split to put enablement before dependent features. Fix: add a Class column (enablement, feature, gate, unrelated) to §7.1 to §7.5. Record that #185 and #186 sit in the #1 feature ladder while acting as enablement for later rungs. | ADR-010 §7 · #205 Layered delivery · #185, #231, #232 bodies |
| FND-011 | low | L1-D1 limits the #185 question to QSL #186–#198. #205 places #185 before "proof-dependent backend breadth", which also covers the downstream backend work #223 names (Codegen Verus #84, Contract IR SMT/runtime #136) and #217, which already depends on #185. Fix: either widen L1-D1 to name these downstream consumers, or state that they are out of scope and which ticket decides them. | ADR-010 §9.1 · #205 "Existing consumers" · #223 body |
| FND-012 | low | Issue-body evidence is mutable and carries no revision. L1-D1 and §7 cite issue bodies with no retrieval time, and #185 was edited about two seconds after faa1731 was committed, so the record is stale on arrival (FND-003). Several code citations also break the `<prefix>:<path>:<line>` convention: `QSL:Cargo.toml` (quire-rs is `:26`), `QSL:wire_format.rs`, `QSL:temporal.rs`, `QSL:model/checked_dispatch.rs`, and "`QSL:tests/configversion_backends.rs` constants". Fix: give a retrieval timestamp (or an issue `updatedAt`) for every issue-body citation. Add line numbers where a positive claim is made. Keep line-less citations only for claims of absence, and mark them so. | ADR-010 Evidence convention, §1, §3.2, §4.3, §9.1 |
| FND-013 | low | The §3.1 SCC S1 diagram is not strongly connected as drawn: no edge enters `parser`, and `lexer`, `package`, `syntax` and `formal_source` have no path back to the rest. The closing-edge table cites only the edges that close the cycle. The membership looks correct: `QSL:package/reading.rs:10` and `QSL:linking/composed/inventory.rs:127` reach `parser` through the root re-export. Fix: add one cited edge into each S1 member that has none, so the SCC claim can be checked from the record. | ADR-010 §3.1 · `QSL:package/reading.rs:10` · `QSL:linking/composed/inventory.rs:127` |

## Method

- Read ADR-010 at faa1731 in full, together with its `spec/spec.md` relationship
  and index row (line 385).
- Read #206, #205 (including its dependency graph), and the bodies of #185,
  #186–#198, #207–#232 and the ARCH-01 comment on #207, all as of 2026-09-19.
  Confirmed that `gh issue list --state open` returns exactly the 68 issues §7
  maps.
- Spot-checked Cargo manifests with `git show <sha>:<path>`:
  - QSL de627b5 `Cargo.toml` lines 26, 36, 43 and 44, and
    `tests/fixtures/native-lowering/Cargo.toml`.
  - IR 553b6d1, a5154d3 and 04eb6f8 `Cargo.toml`.
  - CG a4b2a73 and 5e2a6a9 `Cargo.toml`.
  - RT 4e33052 and d97bc0b `Cargo.toml`.
  - `QSL:Cargo.toml:36,43,44` and `IR:Cargo.toml:24` are cited correctly.
    `IR:Cargo.toml:37` and `CG:Cargo.toml:17` are mislabelled (FND-001,
    FND-008).
- Spot-checked the in-crate edges into `parser` and `lexer` for SCC S1
  (FND-013).
- The following pass: the stage graph's dependency direction (link runs before
  check, recorded as OBS-009), the bypass inventory (§4.4), the in-crate SCC
  counts, and the one-owner-per-item assignment in §9.
