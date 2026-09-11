---
id: SR-370
title: "Gap analysis of mixed received and own-attempt observation tests"
type: SpecReview
analysis: gap-analysis
scope: "tests/native_mixed_observation_choices.rs; src/protocol_artifact/native/; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

QUOIN `gap-analysis` reviewed the test-only mixed-observation increment over
`agent-a/owned-attempt-choices`. The new fixture gives one choice an actual
received `M::Plain` record and the chooser's own Attempt record, then verifies
the retained distinct binders/anchors, emitted package, and independent public
reader round trip; its adverse table verifies owner, visible-basis, and
partition refusals after type/proof discharge.

## Verdict

**FAIL** — only under the inherited strict corpus matrix rule: four ordinary
source-backed-required rows are still unbacked outside this increment. The new
TC-121 / FR-042 test traces are backed and the test reaches the real
producer-to-reader path; this is not a full-corpus or terminal-gates claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Four inherited source-backed-required matrix rows remain unbacked and cause the strict corpus verdict; neither is changed by the mixed-observation tests. TC-010 (Manual) and FR-017-AC-2 (Inspection) are `no_source_symbol` exemptions and are not counted as test failures. | TC-115; FR-036-AC-5; FR-036-AC-6; FR-036-AC-8 |
| FND-002 | low | FR-042 remains 9/10 backed: AC-10 still requires the separate real `quire-protocol` public Rust handoff. The local `admit`/`emit`/reader assertion is meaningful A-side evidence but cannot satisfy that integration. | FR-042-AC-10; TC-121:207-221; tests/native_mixed_observation_choices.rs:28-327 |
| FND-003 | low | Coverage status classification is incomplete: seven inherited matrices use `Coverage Status` while the active declaration selects `Status`. Thus empty `status_lies` does not establish a fully checked status column. Inherited untracked NFR-007 tags also remain outside this test-only scope. | spec/model-linking/tests.md:25; spec/native-lowering/tests.md:20; spec/native-packages/tests.md:19; spec/native-readiness/tests.md:19; spec/native-runtime/tests.md:21; spec/native-workflow/tests.md:19; spec/tests.md:25 |

## Coverage

Reconciliation: `quire coverage --scope /home/peter/dev/worktrees/quire-language-mixed-observation-tests --json` using quire 0.31.0 (engine `0.46.0@ca7362d4`) reports 374/383 rows backed. It reports six raw unbacked rows: TC-010 and FR-017-AC-2 are the Manual/Inspection `no_source_symbol` exemptions, leaving the four rows in FND-001. `status_lies` is empty but is qualified by the seven `status-column-matches-nothing` diagnostics; 20 inherited NFR-007 untracked symbols are likewise outside this scope.

The positive test has `#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-4", "FR-042-AC-5", "FR-042-AC-7")]`; the refusal table has `TC-121` and `FR-042-AC-5`. SR-369 removed the extra AC-8 tag because the refusal table does not establish coexistence with an independently admitted declaration. Both assert actual `native::admit`; the positive also performs `native::emit` and `Inputs::read`, while the refusal table asserts `Unsupported::FamilyProof`. No production behavior changed, and no scoped reverse code-to-requirement gap or hollow test was found.

No plan bundle covers protocol-artifact emission (SR-355 precedent), so plan completion is inapplicable and no plan was fabricated. Optional semantic review was not selected. Parent validation completed: both focused mixed-observation tests passed (2 passed, 0 failed), formatting passed, and both strict Clippy configurations passed, as recorded in `/tmp/quire-mixed-observation-gates.log`. These scoped results do not claim a full-corpus suite or discharge the inherited findings above.
