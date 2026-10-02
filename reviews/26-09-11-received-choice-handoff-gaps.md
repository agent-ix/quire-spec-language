---
id: SR-363
title: "Gap analysis of the received-choice producer handoff fixture"
type: SpecReview
analysis: gap-analysis
scope: "examples/native_protocol_handoff.rs; examples/protocol-handoff/workflow.body.native; examples/protocol-handoff/README.md; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

QUOIN `gap-analysis` examined the handoff-fixture diff against
`agent-a/native-boolean-choices`. It adds no producer behavior: an existing
ignored stripped-release fixture now exercises a four-unit source with two
parallel received `Notice` records, an all-branch join, and an owned Service
Boolean choice. Static inspection finds the traced fixture asserts the original
source bytes, channel recipient, distinct receive anchors and fields, authored
guard slices, both choice branches, causal edges, and original control identity.

## Verdict

**FAIL** — only under the strict corpus matrix rule: four inherited ordinary
rows remain unbacked outside this fixture scope. No scoped handoff-fixture gap
or unowned/stub behavior was found. The parent subsequently verified the
stripped-release focused test passed, including the final fixture assertions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Four inherited source-backed-required rows remain unbacked; they are unrelated to the handoff fixture but retain the strict matrix FAIL. | TC-115; FR-036-AC-5; FR-036-AC-6; FR-036-AC-8 |
| FND-002 | low | Full AC-9 per-dimension zero/exact/one-short work vectors remain broader corpus debt; this fixture does not add accounting behavior. | FR-042-AC-9; TC-121:174-176 |
| FND-003 | low | No scoped reverse gap found: the fixture's source/identity assertions implement the documented producer-recipe handoff, and no placeholder or untraced public surface is introduced. | examples/native_protocol_handoff.rs:42-365; README.md:10-20; FR-042:275-280; TC-121:37-47 |

## Coverage

Reconciliation: `quire coverage --scope
/home/peter/dev/worktrees/quire-language-received-choice-handoff --json` (quire
0.31.0, engine `ca7362d4`; raw report
`/tmp/quire-received-choice-handoff-coverage.json`) reports 374/383 rows backed.
TC-010 (Manual) and FR-017-AC-2 (Inspection) are `no_source_symbol` exemptions,
not test-backing failures; the four rows in FND-001 are the remaining ordinary
gaps. `status_lies` is empty. FR-042 is 9/10 backed corpus-wide and the changed
fixture has a real `#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-4",
"FR-042-AC-5", "FR-042-AC-6", "FR-042-AC-7")]` test symbol.

No plan bundle covers protocol-artifact emission (SR-355 precedent), so plan
completion is inapplicable. The optional semantic review was not selected. The
focused ignored stripped-release test passed in the final parent-owned log
`/tmp/quire-received-handoff-gates.log`, alongside formatting, strict minimal
and all-feature Clippy, and a fresh stripped-release producer invocation.
This review does not claim corpus or release acceptance.

### Fixture-to-source check

The fixture parses the bytes supplied to the real producer, not an independent
test string. It checks `Decisions` is Provider-to-Service, then ties each visible
field to its own `LeftReply`/`RightReply` event binder, anchor and original field
slice. It also checks the complementary guards, their retained original source
spans, both body handles, branch/join causal edges, and every relevant control's
original node/locus. This is the intended producer-to-artifact handoff evidence,
not a substitute for B's still-absent public consumer acceptance interface.
