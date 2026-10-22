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

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The merged FR-400 and FR-401, X-13, X-15 and AD-001 now agree: whichever of S1 and S2 lands first moves `WrongSnapshotCause` to SV `call`, and FR-400-AC-2 requires `qsl-eval` to name it there. FR-400 says S0 moves the S6a result types to `quire-eval` and `FamilyKind` stays in `family`. |
| FND-002 | fixed | FR-401 and X-14 now make `Evaluation::refusal_record` a `qsl-eval` free function over the evaluation and a region resolver, matching FR-400's Source locations; `qsl-eval` names no checker-side type. |
| FND-003 | fixed | NFR-015-AC-2 on main (#1223) lists `quire-semantic-value`. NFR-015's Description now names the S0 catalog set, keeps `Code`, `RefusalRecord`, `Locus`, the digest records and setting limits in F, and names S0 rather than S1. New FND-007 covers the remaining ordering question. |
| FND-004 | fixed | FR-090 now calls the traits SV `catalog`'s. ADR-011's layer-3 row, §6.2 `family` row and E6 row are updated, and layers 5, 6 and A depend on E. ADR-014:95 names `quire_semantic_value::catalog::Category`. FR-262's Begin row takes the registry that holds its closure. New FND-008 covers §2.3, which this finding did not list. |
| FND-005 | fixed | TC-390 step 1 scans every workspace member, and the `quire_eval::` resolution is left to FR-401-AC-3. |
| FND-006 | fixed | #1225 merged first. FR-401 now lists FR-400 as `depends_on`, and FR-400 exists on main. |
| FND-007 | fixed | FR-401 now says the first change that moves an F item into SV, S0's (X-14) or S2's (X-13), adds `qsl-foundation`'s SV dependency, and that neither waits for the other. X-14, X-2, NFR-015's row and AC-2, and TC-1110 all attribute the edge to "X-13 or X-14, whichever lands first" (57a293cc). |
| FND-008 | fixed | §2.3 now calls `Evaluation<T>` and `FamilyOutcome` layer-E types, and the diagram adds `QEL --> QCK & QW`. |
| FND-009 | fixed | TC-390 step 4 lists `qsl-package`'s and `qsl-eval`'s `[dependencies]` exactly as the manifests at 57a293cc and `family_outcome_layering.rs` name them. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | major: S0 relies on an F → SV edge that only X-13 creates. FR-401 says `qsl-foundation` names the moved catalog vocabulary "over the F → SV dependency FR-400 states", and that S0 and S1 "both build on FR-400's F → SV dependency". X-14 says "over the F → SV edge of the F row (FR-400)". X-2 adds that edge only "From X-13", and NFR-015-AC-2 attributes it to "FR-400's X-13". But X-14 runs only "after X-11", and AD-001 and FR-262 make IR-583 (S0) and IR-586 (S2) independent. If S0 lands before X-13, no spec text says who adds the SV dependency, which the round-0 text did say ("the first change that moves an F item into SV adds it"). Restore that rule in FR-401 and X-14, and extend X-2 and NFR-015-AC-2's attribution to "X-13 or X-14, whichever lands first". The alternative is to order X-14 after X-13 and say so in AD-001. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:152,230 |
| FND-008 | low | minor: ADR-011 §2.3 still gives the old placement. It describes "The layer-5 `Evaluation<T>`" and "the layer-3 `check`-core `FamilyOutcome`" (lines 385-389), which contradicts the E row, X-14 and FR-090. The mermaid diagram also draws `QEL --> SV & QX` but no `QEL --> QCK` or `QEL --> QW` edge, although the E row lists C and W. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:385-389 |
| FND-009 | low | minor: TC-390's rewritten step-4 expectation contradicts X-8, the manifest and the test it describes. It says `qsl-eval`'s `[dependencies]` "are exactly `qsl-foundation`, `qsl-package`, `qsl-semantics`, `quire-canonical`, `quire-exact`, `serde` and `thiserror`". But X-8, `qsl-eval/Cargo.toml` and `tests/it/family_outcome_layering.rs:436-446` also name `quire-semantic-value` and `ix-trace-rs`. For `qsl-package`, it calls `quire-contract-model` "their one ecosystem crate outside the workspace", but the manifest also names `quire-canonical` and `quire-semantic-value`. The text came from main, and this PR re-authored the paragraph. | spec/test-cases/TC-390-family-outcome-and-refusal-layering.md:66-79 |
