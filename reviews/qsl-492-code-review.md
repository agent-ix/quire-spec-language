---
id: SR-1223
title: "Code review of quire-spec-language PR #602: simulation findings, ExpansionStop and BoundReached (FR-101)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ca788c9bdfee6a0ce072a9a85e249dafae834880; PR #602 diff against origin/main: qsl-eval/src/simulation/{expansion,explore,frontier,mod,order,sample,trace}.rs, qsl-eval/tests/it/finite_simulation.rs, FR-101, spec.md, TC-439, TC-453, TC-454, TC-455, TC-474, TC-536, tests.md"
review_set: subset
---
# Code review of quire-spec-language PR #602

## Summary

Ticket: QSL-492. The PR extends FR-101 in `qsl_eval::simulation`:
- `TransitionSystem::successors` returns `Result<Expansion, ExpansionStop>`, and expansions carry findings.
- `Outcome::Stopped` and `Outcome::BoundReached` are added, and `explore_request` returns `Exploration{outcome, findings}`.
- `Limit::Depth` is deleted. `max_depth` becomes a horizon parameter, and `Limit::{States, Transitions}` carry their values and setting names.
- The sampler preimage gains `choice`.
- Replay checks findings and stops.

What the coordinator asked to check:
- **Limits.**
  - `Limits` is `{max_states, max_transitions}` and defaults to 10,000,000 and 100,000,000.
  - `Limit::setting()` returns `explore.states` and `explore.transitions` (FR-255).
  - `Limit::Depth` is gone, and no `max_depth` member or depth limit is left in `qsl-eval/src/simulation`.
  - One fixed cap remains on this path: state-key encoding still runs under `IDENTITY_LIMITS`, which is `quire_canonical::Limits::MAX_DEPTH` (576 at the locked `9572a21`).
    - A key nested deeper than 576 refuses with `KeyEncoding`.
    - This is in `key.rs`, which the diff leaves untouched. Removing the cap is FR-262-AC-2's specified work (TC-735), so it is not a finding against this PR.
- **Findings and stops.**
  - Findings are pushed only after the successor loop completes, so a state a limit returns to the frontier has no entry.
  - A stopped state contributes no entry.
  - The sampler expands every state, the last included, in the order stop, then step limit, then no successors, as FR-101 states.
  - Replay refuses a differing stop in all three directions.
  - Replay does not refuse every finding mismatch: FND-001.
- **Sampler preimage, spot-checked against QSpec.**
  - `{"choice":"0","draw":"0","seed":"424242","step":"0","trace":"0"}` hashes to `d5160380d7495443315376de306a5d3613f3e010df74a5db92853829205995f2`. That is QSpec TC-210's and TC-354 SM-01's vector on origin/main `c76c6ae`.
  - An independent re-implementation reproduces every vector:
    - n=5, trace 0: `1,3,3,4,4`
    - n=5, trace 1: `4,4,1,3,4`
    - n=3: `1,2,0,0,1`
    - n=2: `0,1,0,1,1`
    - n=7: `6,2,6,6,1`
  - QSpec `simulation-sampler.md` gives a non-workload step one choice, so `STEP_CHOICE = "0"` is right.
- **The category special case.** The shared table is right for what it maps, and FR-101 differs for a stated reason, so the local `stop_category` override stays. The table's own comment misstates the sources: FND-002.
  - QSpec `native-diagnostics.md` on origin/main has no category column. Its `resource_exhausted` row describes a caller work-budget denial (`insufficient-next-charge`) and says "A semantic maximum is not a caller work budget."
  - ADR-013 O-16 classifies by outcome family, not by code:
    - Bound exhaustion of a run is incomplete.
    - ADR-013's "read-only B-2 ceiling" row files `resource_exhausted` carried by a refusal record as a refusal.
  - ADR-014 B-2 says a denied charge yields `Incomplete` carrying `resource_exhausted`.
  - So `resource_exhausted` has two categories by context. `CATALOG_CATEGORIES` is the refusal-record map (O-17):
    - TC-386 asserts that `ModelRefusal`'s `resource_exhausted` codes (`IntakeLimitExceeded`, `FamilySteps`) map to `Refusal`.
    - Changing the table to `Incomplete` would break FR-090-AC-5/TC-386 and ADR-013's refusal row.
  - An `ExpansionStop` is a denied charge that stops a run, not a refusal record, so FR-101 and ADR-014 B-2's incomplete is correct.
  - Other `category_of` callers: no production caller besides `explore.rs`. Test callers are `qsl-foundation/src/diagnostic.rs` (`category_of_reads_the_code_and_refuses_an_unknown_one`), `diagnostic/stage.rs:190`, `qsl-eval/src/value/expression/causes.rs:385,392` (TC-386) and `qsl-semantics/src/check/lowering/model/tests.rs:851`. `qsl-replay/src/result/wire.rs:536` uses `catalog_category` only for membership.
- **Rust lane (rust-review).**
  - Clippy `-p qsl-eval --all-targets -D warnings` is clean.
  - `make string-edge` passes, with the one literal compare marked `#[string_edge]`.
  - `make arch-lint-api-surface-qsl` passes.
  - finite_simulation (35) and the simulation unit tests (5) pass.
  - The new public types derive `Clone, Debug, Eq, PartialEq`, and the `Expanded` enum replaces `ordered_successors` cleanly.
  - There is no `unwrap` on an input path.

## Verdict

Changes requested: one medium finding and two low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Replay reads `trace.findings` only through `find(|entry| entry.depth == index)` for each expanded index. It never refuses: an entry whose depth is past the last expanded state (or at a stopped state's index); a second entry for the same depth (only the first is read); or an entry whose `state` digest is not the state at that depth (the `state` member is never compared). A forged or corrupted trace with extra findings therefore replays `Ok`. FR-101 says replay succeeds only "when every recomputed expansion matches the trace: the same findings". Build the expected `Vec<StateFindings>` while replaying, with digest, depth and findings, then compare it whole with `trace.findings`. Refuse `FindingMismatch{step}` at the first differing entry, the step being the lower of the two entries' depths. | qsl-eval/src/simulation/trace.rs:192-194; qsl-eval/src/simulation/trace.rs:233-241 |
| FND-002 | low | The `CATALOG_CATEGORIES` doc comment makes two false claims. (1) "QSL raises [`resource_exhausted`] only for a semantic maximum, which the catalog states is not a caller work budget." QSpec's row says the reverse: `resource_exhausted` is the caller work-budget cause, and a semantic maximum is not one. QSL now also raises it for a work budget, through `ExpansionStop`. (2) "an exhausted S6a work budget is the kernel `Incomplete` outcome, which carries no catalog code." ADR-014 B-2 says it carries `resource_exhausted`. The table value `Refusal` is right for refusal records, so change only the comment. State that `resource_exhausted` is a refusal when a refusal record carries it (ADR-013's read-only ceiling row) and incomplete when a denied charge stops a run (ADR-014 B-2, FR-101 `Outcome::Stopped`). Also state that `category_of` is the refusal-record map, matching what `stop_category`'s doc already says. | qsl-foundation/src/diagnostic.rs:822-832; qsl-eval/src/simulation/explore.rs:171-182 |
| FND-003 | low | The spec.md index row for FR-101 still begins "Specified:", although FR-101's Status now says AC-1 to AC-15 are implemented and the row itself says TC-453 to TC-455 and TC-474 pass. Other implemented rows begin "Implemented". | spec/spec.md:1022 |

## Dispositions

Round 1, reviewed at `e98d1d87f1c602f22c688b15288b2382fdf10b89`. One fix commit sits directly on the reviewed
head ca788c9b, with no rebase. The committed `reviews/qsl-492-*.md` are
byte-identical to the reviewer's copies.

The commit's only deletion is the depth lookup `recorded_findings`.
Replay's matching-stop path now `break`s instead of returning `Ok`, so
leftover entries are checked on a stopped trace too.

The `reviews/qsl-519-*.md` scope-SHA restore was ordered by the team
leader and is outside E1's change.

Checks run on the head:
- The 35 finite_simulation tests pass.
- Clippy `-D warnings` is clean for qsl-eval and qsl-foundation.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e98d1d87f1c602f22c688b15288b2382fdf10b89 |
| FND-002 | fixed | e98d1d87f1c602f22c688b15288b2382fdf10b89 |
| FND-003 | fixed | e98d1d87f1c602f22c688b15288b2382fdf10b89 |
