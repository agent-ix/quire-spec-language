---
id: SR-15532
title: "dependency review of STD-299 no_std build contract (QSL PR #1223)"
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language@44f497b8ced379b560d10c2707c97b643ca609e4; spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md, spec/assurance/AD-001-shared-evaluation-core-interface.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: reviews
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: reviews
---
# SR-15532: dependency review of STD-299 no_std build contract (QSL PR #1223)

## Summary

Ticket: STD-299. New edges:

- NFR-015 depends_on ADR-011.
- NFR-015 references AD-001, ADR-013, ADR-030, FR-262 and NFR-013.
- AD-001 and FR-262 reference NFR-015.
- spec.md contains NFR-015.
- TC-1110 and TC-1111 verify NFR-015.

Every target exists on main, and no edge forms a `depends_on` cycle. The ids NFR-015, TC-1110 and
TC-1111 appear in no other open QSL PR. The check fetched all 19 open heads and grepped `id:` in
each.

Overlap with other open PRs, from `git merge-tree` over main:

- #1224 (IR-583) conflicts textually with this PR in AD-001. It rewrites the slice table, including
  the S3 and S5 rows, and the "no S1-S5 implementation slice can start" paragraph.
- #1225 (STD-297) conflicts textually in AD-001. It merges cleanly in ADR-011 and FR-262.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | minor: this PR's AD-001 still contains, unchanged from main, "no S1-S5 implementation slice can start under the current structured blockers: all are blocked by IR-269". NFR-015's rationale ("lets S3 and S5 start before S4") and the PR body ("so both can start now") disagree with it. #1224 rewrites that paragraph and the S3 and S5 rows, and the two PRs conflict textually there. #1225 (STD-297) also changes ADR-011 so that layer 4 depends on "3, F, C, SV, K", which adds `quire-checked`. NFR-015's `qsl-package` row lists the crate-tier dependencies "exactly", so that row has to add `quire-checked` once #1225 lands. <br>Fix: merge #1224 first and rebase this PR so its S3 and S5 row text sits on #1224's rows. The PR that merges second between #1223 and #1225 adds `quire-checked` to the `qsl-package` row. | spec/assurance/AD-001-shared-evaluation-core-interface.md:154-170, spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:103 |

## Verdict

One minor finding, about merge order. The new edges are sound. The S4 dependency
(`qsl-semantics`), which keeps `qsl-package` at the crate tier, is stated explicitly. SR-15530
FND-001 records that this dependency is incomplete.

## New findings (disposition pass 1)

Round 1 reviewed QSL #1223 at c6e3c941e, with main a89136bfa merged in. #1224 is on main. #1225
(STD-297) is open and still conflicts textually in AD-001. #1227 (STD-298) is newly open and merges
cleanly with this PR.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | minor: #1227 (STD-298) adds X-14, under which "`qsl-foundation` adds SV to its `[dependencies]` (amending X-2)", and moves the catalog vocabulary to SV `catalog`. NFR-015's `qsl-foundation` row and AC-2 (with TC-1110 expected result 3) list the depth-1 dependencies as "exactly `quire-exact`, `serde`, `sha2`, `thiserror` and the `ix-trace-rs` proc macro". Once #1227 lands, that list misses `quire-semantic-value`, and AC-2 fails. The closure tier still holds, because SV is `#![no_std]` with no features. The PR that merges second between #1223 and #1227 adds `quire-semantic-value` to the row, to AC-2 and to TC-1110. | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:110, spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:209 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | The AD-001 half is fixed: #1224 is on main and the merged head no longer has the "no S1-S5 slice can start" paragraph. The `quire-checked` half waits on #1225, which is still open. Whichever of #1223 and #1225 merges second adds `quire-checked` to NFR-015's `qsl-package` row and resolves the AD-001 conflict. |
