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

Reviewed commit: 432e615 (branch `task/206-observed-architecture`), round 2.
Round 1 reviewed faa1731. This round checks each round-1 finding against the
revision (`git diff faa1731 432e615 -- spec/decisions`) and looks for new
risk or complexity problems in that diff only. Spot checks ran against QSL
de627b5 (PR heads pr/200 13b6687, pr/204 6eee1f3, pr/228 a43e951), CG 5e2a6a9
and a4b2a73, and the live ARCH-01 comment on #207 (issuecomment-5743530928).

The blocking defect is fixed. §3.2 now has a `QSL tests → RT 8a4d02b` edge, a
QSL ⇄ RT test-time cycle and a proof-path revision chain. OBS-002 and OBS-040
state that the proof path shares no IR, CG or RT revision with production. The
checks hold: CG 5e2a6a9 `src/oracle.rs:14,17` pins IR 04eb6f8 and RT 8a4d02b,
and the QSL `Cargo.lock:911-913` resolves one `quire-contract-ir` at 04eb6f8.
The revision also pins every PR citation to a head sha, widens the
single-writer hotspot (confirmed with `git diff --stat de627b5...pr/N`), makes
ARCH-01 the authority for merge order (the comment reads #228 → #204 → #200),
adds a 31-module placement table, states the owner load, and adds OBS-037 to
OBS-041.

Two medium problems remain. One is new wording that is wrong: "no CI job runs
IT-010". The manual CI workflow and `make ci` both run IT-010 (FND-001). The
other is that replay ownership is split across two Layer 1 owners, and §7.1
names consumed items under owners that contradict §9.2 (FND-002). Neither
blocks, and both fixes are local.

Verdict: ACCEPT WITH FINDINGS (0 high, 2 medium, 3 low).

## Risk register (decision items with elevated risk or volatility)

| Item | Tech risk | Volatility | Drivers | Mitigation named in the record? |
|---|---|---|---|---|
| OBS-002 / OBS-040 / A9–A11 | High | Medium | Only proof-and-replay path. It is test-only and runs on IR 04eb6f8, CG 5e2a6a9 and RT 8a4d02b, none of them a production pin. It needs a pinned Kani. The full gate runs it. | Yes. CI wording is wrong: FND-001 |
| OBS-028 / OBS-036 / OBS-038 / OBS-039 | High | High | Replay executor is disputed in the specs, every call site uses a stub, and two owners decide it | Partly. FND-002 |
| `src/model/{normalize,conformance,refusal,checked_dispatch}.rs` | Medium | High | Three keep PRs write these files. Merge order #228 → #204 → #200 now cites ARCH-01 | Yes (§4.5, §8). Tag list incomplete: FND-004 |
| X2, OBS-006, OBS-014, OBS-027, OBS-036, OBS-041 | Low | High | Keep PRs change the described state | Yes. PR-sensitive list plus a #208 re-check |
| #211 owner load | n/a | n/a | 33 of 59 items. #213 consumes 8 of #211's DA items | Stated in §9.2. Grouping incomplete: FND-003 |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | "No CI job runs IT-010" is wrong. The record says so in §2.1 ("so no CI job runs IT-010"), OBS-002 ("no CI job runs it") and the Summary counts row ("not run by CI"). But IT-010 is an auto-discovered integration test: it is not `#[ignore]`d and has no `required-features` entry in `QSL:Cargo.toml:62-95`. The only CI job runs `cargo test --locked --workspace` twice (`QSL:.github/workflows/ci.yml:22,26`), and so does `make ci` (`QSL:Makefile:34,38`). When dispatched, both run IT-010 on a runner that installs no cargo-kani (`QSL:.github/workflows/ci.yml:13-16`). There it panics at `QSL:tests/configversion_backends.rs:513` (`expect("cargo-kani 0.67.0 must be installed")`). The real risk is the reverse of the one the record states: IT-010 is on every full gate, and that gate fails in any environment without the pinned Kani and a warm cargo cache. CI also has no automatic trigger (`ci.yml:3-4`). Fix: in §2.1, OBS-002 and the Summary counts row, replace "no CI job runs IT-010" with "CI has only a manual trigger. The manual workflow and `make ci` both run IT-010, which fails unless the pinned cargo-kani is installed and the RT revision is already in the cargo cache." Cite `ci.yml:22,26` and `Makefile:34,38`. | ADR-010 §2.1 (IT-010 preconditions), OBS-002, Summary counts "Proof-and-replay paths" |
| FND-002 | medium | Replay ownership is split across two owners, and §7.1 names consumed items under owners that contradict §9.2. OBS-028, OBS-036 and OBS-038 (who performs replay, the FR-031 vs AD-016 executor conflict, and #205's Runtime replay surface) go to #209. OBS-039 (AD-016 and #205 name different replay owners) goes to #211 with #209 as secondary. That is one question with two deciders, which works against Decision 3 ("exactly one owning ticket"). The new §7.1 column "Layer 1 decision consumed" also names items under the wrong owner: #231 "#211 (OBS-002, OBS-027, OBS-028, X6)", but OBS-002 and OBS-028 are #209's; #217 "#209 (OBS-002, OBS-027, …)", but OBS-027 is #211's; #215 "#209 (OBS-031, OBS-034)", but OBS-034 is #211's. A consumer reading §7.1 would wait on the wrong ticket. Fix: move OBS-039 to #209 (secondary #211), or say which part each ticket decides. In §7.1, group each row's items by their §9.2 owner, for example #231 "#209 (OBS-002, OBS-028); #211 (OBS-027, X6)". | ADR-010 Decision 3, §7.1 rows #215, #217, #231, §9.2 OBS-028, OBS-036, OBS-038, OBS-039 |
| FND-003 | low | The §9.2 load paragraph says #211's items "fall into three groups", but those groups cover 21 of #211's 33 items. Twelve are in no group: DA-09, DA-10, DA-12, DA-13, DA-15, OBS-006, OBS-021, OBS-025, OBS-026, OBS-027, OBS-035 and OBS-039. Six of those twelve (DA-09, DA-10, DA-12, DA-13 and their findings) are the ones #213 consumes, so the ungrouped remainder sits on #211's critical path. Fix: add a fourth group, "outcomes, refusals, budgets, provenance and digests" (DA-09, DA-10, DA-12, DA-13, DA-15, OBS-021, OBS-025, OBS-035), and a fifth, "intake, witness and wire" (OBS-006, OBS-026, OBS-027, OBS-039). Alternatively, say "include" instead of "fall into". | ADR-010 §9.2 Load paragraph |
| FND-004 | low | The §8 PR-sensitive list leaves out evidence sites that the keep PRs rewrite. PR #204@6eee1f3 adds 253 changed lines to `src/model/checked_dispatch.rs`, inside `checked_dispatch_operation`. It adds `object_type_supertypes`, removes `DispatchRoot::receiver_type` and derives receiver types from `object_keys`. That changes the cited lines and the facts of OBS-007, the §2.3 "only non-test constructor" sentence, the §4.2 rows for `DeclarationKey → NodeKey` and model dispatch, OBS-018 and DA-02 (`:653,657`). PR #200@13b6687 changes `src/model/population.rs` (DA-02 `:376`) and `src/model/systems.rs` (OBS-014 `:269`, already tagged). §8 notes the OBS-007 site in the #204 row, but the list that #208 re-checks does not include it. Fix: add OBS-007, OBS-018, DA-02 and the §2.3 and §4.2 rows to the PR-sensitive list. | ADR-010 §8 PR-sensitive items, §2.3, §4.2, OBS-007, OBS-018, DA-02 |
| FND-005 | low | Some positive evidence cells still have no line number, which the evidence convention requires. The ones left are `QSpec:spec/objects/protocol/FR-290-protocol-claim-kind.md` (§1.2, OBS-012, OBS-013, DA-11), `QSpec:proposals/checked-package-v2/schema.json` (§1.1, OBS-001), `PR #200@13b6687:src/model/intake.rs` (§1.1 `model::intake` row) and `QSL:tests/native_protocol_emission.rs` (§2.2, OBS-011). FR-290 backs two load-bearing claims: the 6 claim kinds, and naming QSL's Kani backend as a registrant (OBS-013). Those claims are the hardest to re-verify once QSpec #116 or #229 edits FR-290. Every cell that round 1 FND-010 listed is fixed. Fix: add line numbers, or cite a file-level fact as `absent:` or `§`. | ADR-010 Evidence convention, §1.1, §1.2, §2.2, OBS-011, OBS-012, OBS-013 |

## Round 1 resolution

Counts: 10 resolved, 1 partially resolved, 0 unresolved.

| Round-1 ID | Severity | Status | Reason |
| --- | --- | --- | --- |
| FND-001 | high | resolved | §3.2 adds `QSL tests → RT 8a4d02b` (fixture and IT-010 crates), the QSL ⇄ RT cycle, the proof-path revision chain and a mermaid test-time edge. OBS-002, OBS-040 and A9 name IR 04eb6f8, CG 5e2a6a9 and RT 8a4d02b as separate from 53cc03c. Checked against `CG@5e2a6a9:src/oracle.rs:14,17` and `QSL:Cargo.lock:911-913`. |
| FND-002 | medium | partially resolved | The Kani pin, `CARGO_NET_OFFLINE` and non-`#[ignore]` preconditions are now recorded in §2.1 and OBS-002. The new sentence "no CI job runs IT-010" is wrong, because CI and `make ci` run it (round-2 FND-001). No #209 or #217 routing for the gate gap is stated beyond the OBS-002 owner. |
| FND-003 | medium | resolved | §4.5 names normalize.rs (#228, #204, #200), conformance.rs and refusal.rs (#204, #200) and checked_dispatch.rs (#204). Confirmed with three-dot diffs against de627b5. |
| FND-004 | medium | resolved | Decision 4 and §8 cite issuecomment-5743530928 as the authority for dispositions and merge order. The comment's "Rulings applied" gives #228 → #204 → #200, which matches the table. |
| FND-005 | medium | resolved | Every PR citation is pinned to a head sha. There is a PR-sensitive list, and Consequences has #208 re-check it after each keep PR merges. The list is incomplete: round-2 FND-004. |
| FND-006 | medium | resolved | §9.2 states the load (33 of 59 items; 8 consumed by #213) and names groups. The groups are incomplete: round-2 FND-003. |
| FND-007 | medium | resolved | OBS-031 moves to #209 (secondary #211). OBS-034 and OBS-017 gain #209 as secondary, and the tally is updated. The §7.1 owner columns now contradict §9.2: round-2 FND-002. |
| FND-008 | medium | resolved | §3.1 places all 31 top-level modules with lane or stage, lines and public items, and marks the shared modules. Round 1's "32" counted the `#[path]` test-support module `runtime_test_setup`, and the record rightly excludes it. |
| FND-009 | low | resolved | OBS-041 records the FCD edge, the production `tempfile` and quire-rs 2823a93, owned by #209 with #211 as secondary. Evidence checked at `PR #200@13b6687:Cargo.toml:37-38,42` and `Cargo.lock:1077,1098`. |
| FND-010 | low | resolved | Every cell listed in round 1 now has a line number (`configversion_backends.rs:34`, `wire_format.rs:31`, `temporal.rs:50`, `checked_dispatch.rs:653,657`, `Cargo.toml:26`, `value/accounting.rs:2,143`, `model/key.rs:23,26`), and there is an `absent:` form. Other cells without line numbers remain: round-2 FND-005. |
| FND-011 | low | resolved | `STANDARD "e897f810…"` is in §3.2, OBS-022 and DA-14 (`QSL:package/view.rs:39`, with `ir_revision` at :46). The Evidence convention now defines "Behind" against the Context shas. |

## Failure-domain gaps

`spec/reviews/observed-architecture/failure-domain.md` now exists. The overlap
with this review is FND-001: the proof path's environment preconditions decide
whether the full gate passes.

## Round 2 resolution (author)

Recorded by the authoring agent; the round-2 verdict stands.

- FND-001 resolved: §2.1, OBS-002 and the Summary row state that IT-010 runs on
  every full `cargo test` gate (`ci.yml:22,26`, `Makefile:34,38`) and fails at
  `configversion_backends.rs:513` without cargo-kani.
- FND-002 resolved: OBS-039 moved to #209; §7.1 #215, #217 and #231 cells follow
  §9 owners.
- FND-003 resolved: the §9.2 load paragraph lists four groups covering all 33
  #211 items.
- FND-004 resolved: the PR-sensitive list adds OBS-007, OBS-018, DA-02 and the
  §2.3 and §4.2 `checked_dispatch.rs` rows.
- FND-005 partly resolved: FR-290 and test-file cells carry lines; the
  checked-package-v2 `schema.json` and PR #200 `intake.rs` cells remain
  file-level (existence claims).
