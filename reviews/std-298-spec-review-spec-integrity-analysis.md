---
id: SR-15541
title: "integrity review of STD-298 quire-eval leaf and vocabulary placement (QSL PR #1227)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@2013cea1d67df19afbc6f1cd905bf5f6d33a0666; spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md, spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md, spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/assurance/AD-001-shared-evaluation-core-interface.md, spec/test-cases/TC-390-family-outcome-and-refusal-layering.md, spec/test-cases/TC-386-diagnostic-maps-family-refusal-codes-to-refusal.md; cross-PR #1225@dc6e2b20, #1223@c6e3c941"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-390
    type: reviews
---
# SR-15541: integrity review of STD-298 quire-eval leaf and vocabulary placement (QSL PR #1227)

## Summary

Ticket: STD-298. This review checks consistency inside the diff, against the rest of the corpus, and against the
open PRs #1225 (FR-400, head dc6e2b20) and #1223 (NFR-015, head c6e3c941). Textually, #1227 merges cleanly with
#1223. It conflicts with #1225 only in ADR-011's edge table and the X-13/X-14/X-15 rows, and both conflicts are
adjacency only. `git merge-tree` auto-merges the E and C layer rows, the mermaid edges, AD-001 and FR-262. The
semantic conflicts are below. Ids are unique: no open PR head adds FR-401, X-14, X-15 or layer E. The only
existing `FR-401` mentions (FR-183, ADR-023) name QSpec's FR-401.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: FR-401 and FR-400 disagree about `WrongSnapshotCause` and the S6a vocabulary. FR-401's "S1 and S2" bullet says `WrongSnapshotCause` "must leave `check` before FR-400-AC-2 can pass, so whichever of S1 and S2 lands first moves it". X-15 says the same ("When X-13 lands first, X-13 moves it"). But FR-400-AC-2 at #1225's head explicitly allows `WrongSnapshotCause` in `qsl-eval`. FR-400's "What stays with the checker" also says the S6a result vocabulary, `FamilyKind` and `WrongSnapshotCause` do not move in S2, and that "the S1 slice places them". FR-401 instead moves the result types in S0, keeps `FamilyKind`, and lets S2 move `WrongSnapshotCause`. Fix: drop the "whichever lands first" clause from FR-401 and X-15, so S1 alone moves it. Whichever of #1225 and #1227 merges second updates FR-400's sentence to S0/E for the result types, with `FamilyKind` staying. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:217-224 |
| FND-002 | high | major: FR-401 and FR-400 replace `Evaluation::refusal_record` in incompatible ways. FR-401 makes it a `qsl-eval` function "over the evaluation and that graph (after FR-400, the checker-side record)". FR-400 (Source locations) says it takes a region resolver, a function from `Location` to an optional `SourceRegion`, in place of a `CheckedGraph`. FR-400 puts the checker-side record in `qsl-semantics`, and FR-400-AC-2 forbids shipped `qsl-eval` code from naming `qsl_semantics::check` items other than the three it allows. Align FR-401 on the resolver form. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:132-136 |
| FND-003 | medium | major: X-14 breaks NFR-015 (#1223). NFR-015-AC-2 requires `cargo tree -p qsl-foundation --no-default-features` to list "exactly `quire-exact`, `serde`, `sha2`, `thiserror` and the `ix-trace-rs` proc macro". After X-14 that list also contains `quire-semantic-value`, so AC-2 fails as written. NFR-015's Description also says the foundation items the leaf uses (`Code`, `Category`, `CatalogCode`, `InternalFault`, the digest records and the setting limits) move in S1. FR-401 moves them in S0, keeps `Code` in F, and moves no digest records or setting limits. The PR body notes the AC-2 change but leaves it unmade. Whichever PR merges second must amend NFR-015's AC-2 and Description. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1182 |
| FND-004 | low | minor: The corpus still states the old placements. FR-090 lines 275-281 still call `CatalogCoded` and `UndefinedCoded` "F `diagnostic`'s" traits, while line 283 and the Layering section now say SV `catalog`. ADR-011's layer-3 row (line 730) and §6.2 `family` row (line 982) still list `FamilyOutcome`, `FamilyResult` and `EvalOutcome` in the `check` core, and the E6 row (line 263) names QSL `value::expression`. The E row only amends these by reference. The "Depends on" cells of layers 5, 6 and A do not list E. ADR-014:95 names `qsl_foundation::diagnostic::Category`. FR-262's Begin row still lists "Selected checked closure and its registry" as inputs, but FR-401 says Begin takes the registry alone. Edit the rows, or edit them after #1225 merges if the indirection was meant to avoid conflicts. | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:275-281 |
| FND-005 | low | minor: TC-390 is narrower than the AC it verifies. FR-090-AC-9 says "no QSL crate defines any of" the three types, but TC-390 step 1 scans only `src/`, `qsl-foundation`, `qsl-cst`, `qsl-forms`, `qsl-semantics`, `qsl-package` and `qsl-eval`. It omits `qsl-replay`, `qsl-analyze`, `qsl-bench`, `qsl-route`, `qsl-source` and the other workspace members. Step 1's expected result, "each resolves at its `quire_eval::` path", is not something a definition scan of QSL can establish. | spec/test-cases/TC-390-family-outcome-and-refusal-layering.md:38-56 |
| FND-006 | low | minor: Merge-order dependency. FR-401 links `FR-400-separate-checked-terms-from-the-checker.md`, and its ADR-011 E row and rule bullet depend on layer C. On `main`, neither exists until #1225 lands. FR-401's `relationships` do not name FR-400, so the gate passes. If #1227 merges first, though, the link is dangling and E names an undefined layer. Merge #1225 first, or say so in the PR. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:41,241 |

## Verdict

The SV/E/K placement is internally consistent across FR-401, the ADR-011 rows, X-14 and AD-001. The cross-PR
contract with FR-400 is inconsistent in two places (FND-001, FND-002), and with NFR-015 in one (FND-003). Each
must be reconciled in whichever PR merges second, and FND-001 and FND-002 also need an edit to FR-401 itself.
TC-386's widened clause (F may name K and SV) is correct.

`QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` on 2013cea1 exits 0. It reports 17
warnings, none on a touched file.
