---
id: SR-1372
title: "Spec review of quire-spec-language PR #658: content-mismatch and checksum-inventory spec edits (QSL-473)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@384d6898ec47da42782f8d7fa0a6b2158c95c053; spec/functional/FR-357-replay-scalar-parity-claims.md, spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md, spec/test-cases/TC-904-replay-scalar-parity-claims.md, spec/test-cases/TC-907-composite-bounded-shadow-item-settles.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/functional/FR-042-publish-compiled-protocol-artifacts.md, spec/functional/FR-080-registry-evidence-and-gates.md, spec/functional/FR-116-replay-a-frame-counterexample.md; context: spec/functional/FR-050-publish-authenticated-temporal-artifacts.md, spec/test-cases/TC-138-publish-authenticated-temporal-artifacts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-358
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-138
    type: reviews
---
# Spec review of quire-spec-language PR #658

## Summary

Ticket: QSL-473. Base spec review of the spec edits, checking that each states only what is true at the head.

Clean:
- FR-357 (Behavior and AC-11, AC-16, AC-17), FR-358 (step 2 and AC-3), TC-904 (steps 11, 15), TC-907 (step 3) and ADR-013 O-09 now name `content-mismatch` for `ScalarIdentity`, matching `ReplayRefusal::cause()` (qsl-replay/src/execute.rs:326). No spec file outside spec/reviews still names `revision-mismatch` as a live cause; FR-087-AC-11 names it only to say the library has none.
- FR-358's Common steps text ("Both entries SHALL run these steps in order, stopping at the first refusal. A refusal settles as `TerminalValue::from_replay_refusal` gives (FR-069) ... It takes precedence over every settlement row, including the vacuous row V-3.", lines 172-175) is untouched.
- FR-080 and FR-116 lose their "Remaining work" notes, which this PR completes; the Status text left behind is true.
- `quire validate` passes on all eight edited files.

Examined: FR-357 Behavior and AC-11/16/17; FR-358 step 2, AC-3 and lines 168-176; TC-904 steps 11/15; TC-907 step 3; ADR-013 O-09 "Who computes it"; FR-042 Status; FR-080 Status; FR-116 Status; FR-050 member-constant paragraph and AC-7; TC-138 procedure.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-138's procedure still tells the tester to "Verify every entry in the named checksum inventory against the corresponding handoff bytes" and to confirm "every file the written checksum inventory lists is present with the matching digest". This PR deletes the inventory and the tests that did this, so TC-138 now prescribes steps no test performs and no handoff supports. Rewrite both sentences to what the tests now check (each manifest/selection path is a published file; the two writes are byte-identical). | spec/test-cases/TC-138-publish-authenticated-temporal-artifacts.md:57-65 |
| FND-002 | medium | FR-050-AC-7 says "five member filenames". With `PUBLISHED_CHECKSUMS_FILE` deleted there are four (`PUBLISHED_OFFER_FILE`, `PUBLISHED_ARTIFACT_REFERENCE_FILE`, `PUBLISHED_SELECTION_FILE`, `PUBLISHED_MUTATION_MANIFEST_FILE`), which is what FR-050's own Behavior paragraph and the PR's docs edit ("its four consumer-facing members") say. Change the AC to four. | spec/functional/FR-050-publish-authenticated-temporal-artifacts.md:209 |
| FND-003 | low | FR-042's `## Status` section is now an empty heading: the PR deleted its only paragraph. State the status (implemented, verified by TC-121) or remove the heading. | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:488 |

## Verdict

The content-mismatch edits are correct and complete, and the plan-lead-ruled FR-358 sentence is intact. The checksum deletion left two spec statements that are no longer true (FND-001, FND-002) and one empty section (FND-003). Mergeable on the spec side once these three are fixed.

## Dispositions

Round 1, reviewed at 7b326716f68cfb2b7e0086c94704b9382e6175ca (rebased onto main dac0d90f).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 7b326716f: both TC-138 checksum sentences replaced by the base-file existence check and the byte-identical-writes check; no `checksum` text remains in TC-138 |
| FND-002 | fixed | 7b326716f: FR-050-AC-7 says "four member filenames" |
| FND-003 | fixed | 7b326716f: the empty `## Status` heading is removed from FR-042 |
