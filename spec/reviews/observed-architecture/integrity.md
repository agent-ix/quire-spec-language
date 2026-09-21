---
id: SR-460
title: "Integrity review of ADR-010 observed architecture baseline"
type: SpecReview
analysis: integrity
scope: "spec/decisions/ADR-010-observed-architecture-baseline.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-010
    type: reviews
---
# SR-460: Integrity review of ADR-010 observed architecture baseline

## Summary

Reviewed commit faa1731 on `task/206-observed-architecture`
(agent-ix/quire-spec-language): ADR-010 and its index row in `spec/spec.md`.
ADR-010 is a descriptive record of the architecture at pinned revisions, not a
requirement set. So this integrity pass checks four things: completeness against
the #206 deliverables and acceptance criteria, internal consistency of every
count and tally, atomicity and unique ids of the decision items, and whether
every item has evidence and one owner.

The core counts hold when checked against the code. The stage count
(28 present, 7 absent), the module SCCs, the ownership tally (13 DUPLICATE,
4 AMBIGUOUS, 1 ABSENT), the finding owner tally (13/6/17 = 36) and the 68-row
ticket map all reconcile. The ticket map matches the open-issue list one to one.
The SCCs and fan-in/fan-out figures reproduce from a fresh `crate::` edge
extraction. Every pin-staleness figure reproduces with `git rev-list --count`.

The defects are in the mapping and evidence layers:
- One handoff table cites the wrong file.
- Several ticket-map rows name a Layer 1 owner that disagrees with §9.
- #232 is placed in the wrong layer, and #229 is left out of the Layer 1 set.
- The program flow is attributed to the wrong issue.
- §8 contradicts itself on merge order.
- Several evidence cells break the record's own `<prefix>:<path>:<line>`
  convention or cite PR content without a revision.

None of these makes a stage, owner or dependency claim false in substance, so
no finding is blocking.

Verdict: ACCEPT WITH FINDINGS (no high-severity findings; FND-001 to FND-007 are
medium and should be fixed before the #208 gate).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Wrong evidence file for the §2.2 checked handoffs. The predicate and temporal-subject rows cite `QSL:protocol_artifact/checked_handoff.rs:139`, `:156`, `:177` and `:193`. At de627b5 those lines are a closing brace, a `Usage` field, `Report::result` and `self.usage`. The public `derive`/`read` entries are at `QSL:protocol_artifact/checked_predicate.rs:139,156` and `QSL:protocol_artifact/temporal_subject.rs:177,193`, and both delegate to `checked_handoff.rs:1687` (`derive`) and `:1753` (`read`). Fix: change the paths in the two rows to those files. Keep `checked_handoff.rs:1585-1586` for the format constants. | ADR-010 §2.2 |
| FND-002 | medium | In the ticket map, the Layer 1 owner disagrees with the §9 owner of the items it cites. Cases: #222 cites DA-12 but says "consumes #210", while §9.3 gives DA-12 to #211. §7.2 gives #189 (DA-12) to #210. #217 cites OBS-027 (owner #211) but consumes only #209 and #185. #231 cites OBS-002 and OBS-028 (owner #209) but consumes only #211. So a reader cannot tell which Layer 1 ticket a consumer waits on. Fix: for each row, list every owner of the cited items, or name the §9 owner and mark the other ticket as secondary. Then add a note that the §7 owners are derived from §9. | ADR-010 §7.1, §7.2, §9.2, §9.3 |
| FND-003 | medium | #232 is mapped to Layer "2 (bounded child of #122)". Epic #205 lists #232 under Layer 5 (lifecycle and qualification readiness), and its dependency graph orders `#225 → #232 → #230`. Fix: change the #232 row's layer to 5. | ADR-010 §7.1 · #205 "Active architecture children" |
| FND-004 | medium | The Layer 1 ticket set is incomplete. Decision 1, Context and Consequences name only #209, #210 and #211. #205 also lists #229 (ARCH-13, align the QSL Capability spec with FR-290) under Layer 1. OBS-012 (the FR-290 disagreement) and DA-11 are #229's subject, yet §9 gives them to #210, and §7.1 calls #229 only a secondary input. Fix: add #229 to the Layer 1 set in Decision 1. Then either make #229 the owner of OBS-012 (and DA-11, if the capability-alignment decision is #229's), or state that #229 owns no §9 item and why. Recompute the owner tally if ownership moves. | ADR-010 Decision 1, §7.1, §9.2 OBS-012, §9.3 DA-11 · #205 |
| FND-005 | medium | The intended program flow is attributed to the wrong issue. §2.5 and OBS-009 quote "the #205 program flow" (`source → CST → … → replay`). That sequence is not in the #205 body or comments. It is in #209's body ("source → CST → parsed semantic forms → checked semantic graph → linked/package form → …"), and the ADR also abbreviates its element names. The Summary counts row "Absent stages" also relies on "the #205 program flow". Fix: cite #209 as the source of the flow, quote it exactly, and add #209 to OBS-009's evidence. | ADR-010 §2.5, §2.6, Summary counts, OBS-009 |
| FND-006 | medium | §8 contradicts itself and Decision 4. Decision 4 says WIP dispositions follow the ARCH-01 classification on #207. The §8 table notes say #228 merges first and #204 second, as #207 does. The prose below the table says "#204 → #228 → #200 … the coordinator's order governs". That order has no cited source, and the prose narrates the change. Fix: pick one order. If the coordinator's order stands, cite where it is recorded, update the table notes, and amend Decision 4 to name that authority. Otherwise drop the override sentence. | ADR-010 Decision 4, §8 · #207 ARCH-01 comment |
| FND-007 | medium | PR-based claims carry no revision identifiers. #206 requires "evidence paths and revision identifiers for every assertion", and the record pins every main at a sha. But these cite PRs with no head sha: §1 rows on `model::intake` and ValueTypeRef ("PR #200"), OBS-006 (PR #200), OBS-027 (IR PR #139), and the §8 PR #200 observations (FCD rev, `tempfile`, quire-rs 2823a93). #207 records the heads as `13b6687` (QSL #200) and `64982f1` (IR #139). Fix: add a PR-head row to the Revisions table or a `PR#n@<sha>:` evidence prefix, and cite the heads. | ADR-010 §1, §8, OBS-006, OBS-027 |
| FND-008 | low | Evidence cells break the stated `<prefix>:<path>:<line>` convention for claims that do have a line. Examples: `QSL:wire_format.rs` (native-run/1 :27, native-compile/1 :29, native-run-result/1 :31, native-state-input/1 :37, native-linked-package/1 :39); `QSL:temporal.rs` for `CLOCK_PREFIX` (:50); `QSL:complete/diagnostic.rs` for 1-draft.3 (:13); `QSL:tests/configversion_backends.rs constants` (KANI_SHA256 :34). Also `QSL:model/checked_dispatch.rs` (DA-02, §4.2), `QSL:checking/types.rs` (DA-05 NativeType), `QSL:model/key.rs` (DA-15), `QSL:value/accounting.rs` / `QSL:model/accounting.rs` (DA-12), `QSL:Cargo.toml` (quire-rs pin), `QSL:source.rs` (DA-13) and `CG:assurance/pins.json`. §6.4 cites a bare `output_mapping.rs` with no prefix or directory. Fix: add the line numbers for presence claims and the full path for `output_mapping.rs`. Also amend the convention to allow a path-only cell for absence claims (for example `QSL:Cargo.toml` for "no quire-exact dependency"). | ADR-010 §1, §2.6, §3.2, §3.3, §4.2, §4.3, §5, §6.4, OBS-014, OBS-026, OBS-034 |
| FND-009 | low | The lane D rows record the stage outputs as "none recorded", but the outputs exist. `sample` returns `Result<Trace<_>, EmptyInitial>` (`QSL:simulation/sample.rs:95-99`), and `replay` returns `Result<(), ReplayError<_>>` (`QSL:simulation/trace.rs:97-100`). The lane D table also lacks the Refusal column the other lanes carry. The Summary counts a present stage as "entry function and output type", so D2 and D3 are counted without meeting their own rule as written. Fix: fill in the outputs and add the Refusal column (`EmptyInitial`, `ReplayError`). | ADR-010 §2.4, Summary counts |
| FND-010 | low | Some counts and wordings are imprecise. (a) The Summary says "3 SCCs … plus 7 two-cycles", but S2 and S3 are two of those 7. OBS-016 says it correctly ("7 two-cycles in total"). (b) DA-10 says `Code` has 44 variants; `QSL:diagnostic.rs:51` has 45 (through `CardinalityOutOfBound`). (c) §6.1 "DIFFERS / ABSENT 6" lists `quire-exact` and `model::intake` as one entry, making 7 claims. §6.1 "downstream" also re-counts QSL-side claims already in §1 (QSL→contract-model, QSL dev-dependencies, quire-exact, model::intake), so "46 claims checked" double-counts. (d) OBS-028 calls the sole caller of `replay_with_native_runtime` an "IR unit test"; it is the integration test `IR:tests/kani_replay.rs:240`. Fix: correct each figure, split the bundled entry, remove or mark the overlapping claims, and cite `IR:tests/kani_replay.rs:240`. | ADR-010 Summary counts, §5 DA-10, §6.1, OBS-016, OBS-028 |
| FND-011 | low | The SCC S3 closing edge `temporal→protocol_artifact` is cited at `QSL:temporal.rs:15`. That line is a doc comment, not a `use crate::` edge, and §3.1's stated method counts only `use crate::…` edges. The import is at `QSL:temporal.rs:29`. Fix: cite `:29`. | ADR-010 §3.1 |
| FND-012 | low | The #206 ownership list includes "identities/names", but §5 has no row for names. `ir::SymbolName` (103 uses, §3.2) is the name vocabulary native-v1 uses, and QSL has its own declaration and function-name keys (`&str` value calls, §4.3). Fix: add a Names row to §5 with its owner(s) and flag, and a DA id if it is DUPLICATE or AMBIGUOUS. Update the 13/4 tally and §9.3 to match. | ADR-010 §5 · #206 Deliverables |
| FND-013 | low | Some items are not atomic. OBS-034 bundles three separate observations: pins trailing their targets, pins duplicated as literals (`IR_CANDIDATE_REVISION`, `RUNTIME_REVISION`), and the false CG `assurance/pins.json` statement about Kani code. OBS-028 bundles IR-side native replay, CG stub executors and the AD-016 disagreement. Each part can be decided separately. Fix: split OBS-034 into two or three findings, and consider splitting OBS-028 into IR replay placement and CG stub executors. Keep one owner each and update the tally. | ADR-010 §9.2 OBS-028, OBS-034 |
| FND-014 | low | §7.2 is titled "A04–A15 feature ladder" but omits A08 (#185, "[V1-A08]"), which is mapped in §7.1 instead. #206 requires every open A04–A15 ticket to be mapped. #185 is mapped, but a reader scanning §7.2 sees a gap. Fix: add a cross-reference row "#185 (A08) — see §7.1", or note in §7.2 that A08 appears in §7.1 as a program ticket. | ADR-010 §7.1, §7.2 · #206 Deliverables |

## Method

- **Counts reconciled by recount.**
  - §1 has 14 rows: 5 AGREES, 1 PRESENT unwired, 8 DISAGREES/ABSENT.
  - Stages: A1–A8 = 8, B1–B10 = 10, C1–C6 + C9 = 7, D1–D3 = 3, for 28. X1–X7 = 7.
  - §5: 13 DUPLICATE (DA-01, 03–10, 12, 15–17), 4 AMBIGUOUS (DA-02, 11, 13, 14), 1 ABSENT.
  - §9.2 primary owners: #209 13, #210 6, #211 17, total 36.
  - §6.1: 24 + 2 + 6 = 32.
  - §7: 28 + 14 + 16 + 10 = 68.
- **Ticket map.** Compared with
  `gh issue list -R agent-ix/quire-spec-language --state open --limit 200` on
  2026-09-19. There are 68 open issues, and the set equals the §7 rows exactly.
  Layers were compared with the #205 "Active architecture children" list.
- **Module graph.** Extracted `crate::` references (grouped imports and `lib.rs`
  re-exports resolved, `#[cfg(test)] mod` blocks and `*tests.rs` excluded) from
  `git archive de627b5 src`, then ran Tarjan's SCC algorithm. Result: one
  11-module SCC with the listed members (lexer is reached via parser), plus
  {model, value} and {protocol_artifact, temporal}. There are 7 two-cycles.
  Fan-out: command 16, package 11, runtime/protocol_artifact/mapped 10,
  lowering 9. Fan-in: diagnostic 20, source 18, syntax 12. All match §3.1.
- **Evidence spot-checks.** About 120 cited `path:line` cells were read with
  `git show <sha>:<path>` at QSL de627b5, IR 553b6d1, CG a4b2a73 and QSpec
  dccddab. All resolved to the named item except those in FND-001 and FND-011.
  Enum variant counts were rechecked for QSL `Value` 13, `ValueType` 14,
  `value::Refusal` 15, `Capability` 4, `Family` 4 and `KaniOutcomeKind` 10; all
  hold except the `Code` count in FND-010.
- **Pin staleness.** Every commits-behind figure in §3.2, §3.2 Vendored and
  §6.3 reproduces with `git rev-list --count <pin>..<main>` in the local clones
  that exist: 26, 33, 7, 78, 19, 5, 15, 144 and 10.
- **Not verified.** RT, FCD and QI evidence was not spot-checked, because no
  local clones were provided.
- **Ids and ownership.** L1-D1, OBS-001…OBS-036 and DA-01…DA-17 are sequential
  with no gaps or duplicates. Every §9 item has evidence and exactly one primary
  owner. The `ABSENT` witnesses row carries no DA id and defers to OBS-027
  (owner #211).
- **Artifact kind.** The US→FR→StR traceability and EARS probes do not apply to a
  descriptive ADR. The hidden-assumption probes were applied as a check for
  intended design described as present. No such case was found: every
  AD-016 / #205 element is stated as a reference with an observed verdict.

## Round 1 resolution (author, commit after 432e615)

Recorded by the authoring agent; this analysis was ACCEPT WITH FINDINGS and was
not rerun.

- FND-001 resolved: §2.2 cites `checked_predicate.rs:139,156` and
  `temporal_subject.rs:177,193`.
- FND-002 resolved: §7 states that a cited §9 item keeps its §9 owner; the
  #222, #217, #231 and #189 rows name both owners.
- FND-003 resolved: #232 is Layer 5.
- FND-004 partly resolved: #229 is named as a Layer 1 ticket in Context,
  Decision 1 and Consequences, and as secondary input on OBS-012, OBS-013 and
  DA-11. Primary owners stay within #209, #210 and #211 per the #206 brief;
  Decision 3 states why.
- FND-005 resolved: §2.5, §2.6, OBS-009 and Summary cite #209 and quote it.
- FND-006 resolved: one merge order, #228 → #204 → #200, from ARCH-01.
- FND-007 resolved: PR heads pinned; `PR #n@<sha>:` prefix added.
- FND-008 resolved: line numbers added; negative-evidence form for absences;
  `output_mapping.rs` full path.
- FND-009 resolved: lane D outputs and Refusal column.
- FND-010 resolved: two-cycle wording, 45-variant `Code`, §6.1 no longer
  double-counts QSL-side claims (29 downstream), `IR:tests/kani_replay.rs:240`.
- FND-011 resolved: `QSL:temporal.rs:29`.
- FND-012 resolved: DA-18 Names added (14 DUPLICATE, 4 AMBIGUOUS).
- FND-013 open (low): OBS-028 and OBS-034 remain bundled; each still has one
  owner.
- FND-014 resolved: §7.2 notes #185 (A08) is mapped in §7.1.
